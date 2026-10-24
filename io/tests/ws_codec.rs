//! WebSocket frame codec.
#![allow(
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::unnecessary_wraps
)]

type R = Result<(), Box<dyn std::error::Error>>;

use ergon_io::ws::{self, OP_BINARY, OP_CLOSE, OP_CONT, OP_PING, OP_TEXT};

#[test]
fn every_length_encoding_round_trips() -> R {
    for &len in &[0usize, 1, 125, 126, 127, 1000, 65_535, 65_536, 200_000] {
        for mask in [None, Some([1, 2, 3, 4])] {
            let payload: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
            let mut out = vec![0u8; ws::header_len(len, mask.is_some()) + len];
            let n = ws::encode_frame(OP_BINARY, &payload, mask, &mut out)?;
            assert_eq!(n, out.len());
            let h = ws::parse_header(&out)?.ok_or("incomplete header")?;
            assert!(h.fin);
            assert_eq!(h.opcode, OP_BINARY);
            assert_eq!(h.payload_len, len);
            assert_eq!(h.mask, mask);
            let mut body = out[h.header_len..h.frame_len()].to_vec();
            if let Some(m) = h.mask {
                ws::apply_mask(&mut body, m);
            }
            assert_eq!(body, payload, "len {len} mask {mask:?}");
            // Every strict prefix of the header is "need more".
            for cut in 0..h.header_len {
                assert_eq!(ws::parse_header(&out[..cut])?, None, "len {len} cut {cut}");
            }
        }
    }
    Ok(())
}

#[test]
fn rejects_protocol_violations() -> R {
    assert!(
        ws::parse_header(&[0x80 | 0x40 | OP_TEXT, 0]).is_err(),
        "RSV1 without extension"
    );
    assert!(ws::parse_header(&[0x83, 0]).is_err(), "reserved opcode");
    assert!(
        ws::parse_header(&[OP_PING, 0]).is_err(),
        "fragmented control frame"
    );
    assert!(
        ws::parse_header(&[0x80 | OP_CLOSE, 126, 0, 200]).is_err(),
        "control frame over 125 bytes"
    );
    let mut big = [0u8; 10];
    big[0] = 0x80 | OP_BINARY;
    big[1] = 127;
    big[2] = 0x80;
    assert!(
        ws::parse_header(&big).is_err(),
        "64-bit length with top bit set"
    );
    Ok(())
}

#[test]
fn continuation_frames_parse() -> R {
    let h = ws::parse_header(&[OP_TEXT, 2, b'a', b'b'])?.ok_or("incomplete header")?;
    assert!(!h.fin);
    let h = ws::parse_header(&[0x80 | OP_CONT, 2, b'c', b'd'])?.ok_or("incomplete header")?;
    assert!(h.fin);
    assert_eq!(h.opcode, OP_CONT);
    Ok(())
}

#[test]
fn mask_matches_bytewise_xor() -> R {
    let mask = [0xAA, 0x55, 0x0F, 0xF0];
    for len in 0u8..20 {
        let orig: Vec<u8> = (0..len).collect();
        let mut fast = orig.clone();
        ws::apply_mask(&mut fast, mask);
        let slow: Vec<u8> = orig
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ mask[i % 4])
            .collect();
        assert_eq!(fast, slow);
    }
    Ok(())
}

#[test]
fn handshake_accept_key_matches_rfc_example() -> R {
    // RFC 6455 section 1.3.
    assert_eq!(
        ws::accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
        "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
    );
    Ok(())
}

#[test]
fn client_response_checks() -> R {
    let key = "dGhlIHNhbXBsZSBub25jZQ==";
    let ok = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n\r\nXY";
    assert_eq!(ws::parse_client_response(ok, key)?, Some(ok.len() - 2));
    assert_eq!(ws::parse_client_response(&ok[..20], key)?, None);
    // Each refusal below differs from `ok` in one header only.
    let wrong = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: nope\r\n\r\n";
    let e = ws::parse_client_response(wrong, key)
        .err()
        .ok_or("wrong accept taken")?;
    assert!(e.to_string().contains("Sec-WebSocket-Accept"), "{e}");
    let deflate = b"HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\nSec-WebSocket-Extensions: permessage-deflate\r\n\r\n";
    let e = ws::parse_client_response(deflate, key)
        .err()
        .ok_or("extension taken")?;
    assert!(e.to_string().contains("extension"), "{e}");
    let forbidden = b"HTTP/1.1 451 Unavailable\r\nContent-Length: 0\r\n\r\n";
    assert!(ws::parse_client_response(forbidden, key).is_err());
    Ok(())
}

#[test]
fn upgrade_requests_reject_injected_or_reserved_fields() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = ergon_io::tls::client_config(rustls::RootCertStore::empty())?;
    let target = |path: &str| ergon_io::Target::new("stream.example.test", 443, path);
    for (t, why) in [
        (
            target("/ws").with_header("X-Key", "v\r\nHost: evil"),
            "CR/LF in a value",
        ),
        (
            target("/ws").with_header("Bad Name", "1"),
            "space in a name",
        ),
        (
            target("/ws").with_header("Sec-WebSocket-Key", "x"),
            "a handshake field",
        ),
        (
            target("/ws").with_header("Upgrade", "h2c"),
            "a reserved field",
        ),
        (target("/a b"), "space in the path"),
        (target("/a\r\nX: 1"), "CR/LF in the path"),
    ] {
        assert!(
            ergon_io::WsDial::new(t, cfg.clone(), 1 << 20).is_err(),
            "accepted {why}"
        );
    }
    assert!(ergon_io::WsDial::new(target("/ws?x=1"), cfg, 1 << 20).is_ok());
    Ok(())
}

#[test]
fn upgrade_request_sends_only_the_callers_user_agent() {
    let none = String::from_utf8_lossy(&ws::client_request("h", "/", "k", &[])).to_lowercase();
    assert!(!none.contains("user-agent"), "{none}");
    let one = String::from_utf8_lossy(&ws::client_request("h", "/", "k", &[("User-Agent", "me")]))
        .to_lowercase();
    assert_eq!(one.matches("user-agent").count(), 1, "{one}");
}

#[test]
fn close_payloads_parse_and_validate() -> R {
    assert_eq!(ws::parse_close(&[])?, (None, ""));
    assert_eq!(
        ws::parse_close(&[0x03, 0xE9, b'b', b'y', b'e'])?,
        (Some(1001), "bye")
    );
    assert!(ws::parse_close(&[0x03]).is_err(), "one byte");
    // 1005, 1006 and 1015 are never sent; 1004 and 1016-2999 are reserved.
    for bad in [999u16, 1004, 1005, 1006, 1015, 1016, 2999, 5000] {
        assert!(!ws::close_code_ok(bad), "{bad}");
        assert!(ws::parse_close(&bad.to_be_bytes()).is_err(), "{bad}");
    }
    for good in [1000u16, 1001, 1002, 1003, 1007, 1011, 1014, 3000, 4999] {
        assert!(ws::close_code_ok(good), "{good}");
        assert_eq!(ws::parse_close(&good.to_be_bytes())?, (Some(good), ""));
    }
    assert!(
        ws::parse_close(&[0x03, 0xE8, 0xFF]).is_err(),
        "reason not UTF-8"
    );
    Ok(())
}

#[test]
fn close_payloads_encode() -> R {
    let (buf, n) = ws::close_payload(Some(1000), "done")?;
    assert_eq!(&buf[..n], b"\x03\xE8done");
    let (_, n) = ws::close_payload(None, "")?;
    assert_eq!(n, 0);
    assert!(
        ws::close_payload(None, "x").is_err(),
        "a reason needs a code"
    );
    assert!(
        ws::close_payload(Some(1005), "").is_err(),
        "1005 is never sent"
    );
    let long = "r".repeat(124);
    assert!(
        ws::close_payload(Some(1000), &long).is_err(),
        "over 125 bytes"
    );
    let (_, n) = ws::close_payload(Some(1000), &long[..123])?;
    assert_eq!(n, 125);
    Ok(())
}

#[test]
fn masking_from_an_offset_continues_the_key() -> R {
    let mask = [1, 2, 3, 4];
    let data: Vec<u8> = (0..41).collect();
    let mut whole = data.clone();
    ws::apply_mask(&mut whole, mask);
    for split in [0, 1, 3, 4, 5, 17, 40, 41] {
        let mut parts = data.clone();
        let (a, b) = parts.split_at_mut(split);
        ws::apply_mask_at(a, mask, 0);
        ws::apply_mask_at(b, mask, split);
        assert_eq!(parts, whole, "split {split}");
    }
    Ok(())
}

#[test]
fn header_only_encoding_matches_the_frame() -> R {
    for len in [0usize, 125, 126, 65_535, 65_536] {
        let payload = vec![7u8; len];
        let mask = Some([9, 8, 7, 6]);
        let mut frame = vec![0u8; ws::header_len(len, true) + len];
        ws::encode_frame(OP_BINARY, &payload, mask, &mut frame)?;
        let mut head = [0u8; 14];
        let hl = ws::encode_header(OP_BINARY, len, mask, &mut head)?;
        assert_eq!(hl, ws::header_len(len, true));
        assert_eq!(&head[..hl], &frame[..hl], "len {len}");
    }
    Ok(())
}

const ACCEPT: &str = "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=";
const KEY: &str = "dGhlIHNhbXBsZSBub25jZQ==";

fn response(extra: &str) -> Vec<u8> {
    format!("HTTP/1.1 101 Switching Protocols\r\n{extra}Sec-WebSocket-Accept: {ACCEPT}\r\n\r\n")
        .into_bytes()
}

#[test]
fn the_upgrade_needs_upgrade_and_connection_headers() -> R {
    let both = response("Upgrade: websocket\r\nConnection: keep-alive, Upgrade\r\n");
    assert!(ws::parse_upgrade(&both, KEY, &[])?.is_some());
    let mixed_case = response("upgrade: WebSocket\r\nconnection: upgrade\r\n");
    assert!(ws::parse_upgrade(&mixed_case, KEY, &[])?.is_some());
    for (extra, why) in [
        ("Connection: Upgrade\r\n", "no Upgrade"),
        ("Upgrade: h2c\r\nConnection: Upgrade\r\n", "wrong Upgrade"),
        ("Upgrade: websocket\r\n", "no Connection"),
        (
            "Upgrade: websocket\r\nConnection: close\r\n",
            "Connection without upgrade",
        ),
    ] {
        assert!(
            ws::parse_upgrade(&response(extra), KEY, &[]).is_err(),
            "{why}"
        );
    }
    Ok(())
}

#[test]
fn the_upgrade_checks_the_subprotocol() -> R {
    let ok = "Upgrade: websocket\r\nConnection: Upgrade\r\n";
    let offered = vec!["v2.json".to_owned(), "v1.json".to_owned()];
    let picked = response(&format!("{ok}Sec-WebSocket-Protocol: v1.json\r\n"));
    let up = ws::parse_upgrade(&picked, KEY, &offered)?.ok_or("incomplete")?;
    assert_eq!(up.protocol.as_deref(), Some("v1.json"));
    assert_eq!(up.header_len, picked.len());
    // The server may pick none; it may never pick one that was not offered.
    let none = ws::parse_upgrade(&response(ok), KEY, &offered)?.ok_or("incomplete")?;
    assert_eq!(none.protocol, None);
    assert!(
        ws::parse_upgrade(&picked, KEY, &[]).is_err(),
        "nothing offered"
    );
    let other = response(&format!("{ok}Sec-WebSocket-Protocol: v3.json\r\n"));
    assert!(
        ws::parse_upgrade(&other, KEY, &offered).is_err(),
        "not offered"
    );
    Ok(())
}

#[test]
fn a_refused_upgrade_reports_its_status() -> R {
    let refused = b"HTTP/1.1 429 Too Many Requests\r\nContent-Length: 4\r\n\r\nslow";
    match ws::parse_upgrade(refused, KEY, &[]) {
        Err(ergon_io::Error::Upgrade {
            status,
            body,
            retry_after_ms,
            retry_after_invalid,
        }) => {
            assert_eq!(status, 429);
            assert_eq!(body, "slow");
            assert_eq!((retry_after_ms, retry_after_invalid), (None, false));
            Ok(())
        }
        other => Err(format!("{other:?}").into()),
    }
}

/// The server's wait on a refused upgrade, so a reconnect can honour it.
#[test]
fn a_refused_upgrade_keeps_the_servers_wait() -> R {
    let refusal = |extra: &str| {
        format!("HTTP/1.1 418 I'm a teapot\r\n{extra}Content-Length: 0\r\n\r\n").into_bytes()
    };
    let wait = |extra: &str| match ws::parse_upgrade(&refusal(extra), KEY, &[]) {
        Err(ergon_io::Error::Upgrade {
            status: 418,
            retry_after_ms,
            retry_after_invalid,
            ..
        }) => Ok((retry_after_ms, retry_after_invalid)),
        other => Err(format!("{other:?}")),
    };
    assert_eq!(wait("Retry-After: 120\r\n")?, (Some(120_000), false));
    // A date is measured against the refusal's own Date header.
    assert_eq!(
        wait(
            "Date: Sun, 06 Nov 1994 08:49:37 GMT\r\n\
             Retry-After: Sun, 06 Nov 1994 08:51:37 GMT\r\n"
        )?,
        (Some(120_000), false)
    );
    // Unreadable: reported as such, never as "no wait".
    assert_eq!(wait("Retry-After: soon\r\n")?, (None, true));
    let shown = ws::parse_upgrade(&refusal("Retry-After: soon\r\n"), KEY, &[])
        .err()
        .ok_or("accepted")?
        .to_string();
    assert!(shown.contains("Retry-After"), "{shown}");
    Ok(())
}

#[test]
fn ws_options_are_checked() -> R {
    let cfg = ergon_io::tls::client_config(rustls::RootCertStore::empty())?;
    let t = ergon_io::Target::new("stream.example.test", 443, "/ws");
    let ok = ergon_io::WsOptions::default();
    assert!(ergon_io::WsDial::with_options(t.clone(), cfg.clone(), ok.clone()).is_ok());
    for (opts, why) in [
        (
            ergon_io::WsOptions {
                buf_size: 1024,
                ..ok.clone()
            },
            "buffer under MIN_BUF",
        ),
        (
            ergon_io::WsOptions {
                protocols: vec!["a b".to_owned()],
                ..ok.clone()
            },
            "space in a subprotocol",
        ),
        (
            ergon_io::WsOptions {
                protocols: vec![String::new()],
                ..ok
            },
            "empty subprotocol",
        ),
    ] {
        assert!(
            ergon_io::WsDial::with_options(t.clone(), cfg.clone(), opts).is_err(),
            "accepted {why}"
        );
    }
    Ok(())
}

#[test]
fn ws_option_defaults() {
    let d = ergon_io::WsOptions::default();
    assert_eq!(d.buf_size, 1 << 20);
    assert_eq!(d.max_message, 0, "0: the largest the buffer holds");
    assert_eq!(d.ping_interval_ms, 10_000);
    assert_eq!(d.pong_timeout_ms, 5_000);
    assert_eq!(d.idle_timeout_ms, 0, "idle timeout off");
    assert_eq!(d.close_timeout_ms, 1_000);
    assert_eq!(
        d.text_check,
        ergon_io::TextCheck::None,
        "a text check costs time per text message"
    );
    assert_eq!(d.protocols, Vec::<String>::new());
}
