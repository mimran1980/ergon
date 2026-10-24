//! Request building: `RequestTemplate` writes the same bytes as the request
//! builder it replaces, plaintext and ciphertext, for a table of requests
//! and for every request size up to one TLS record. Every platform: no
//! driver.
#![allow(missing_docs, clippy::cast_possible_truncation)]

mod http_rig;
mod support;

use ergon_io::http::MAX_REQUEST;
use ergon_io::{
    ConnId, Handler, HttpHandler, HttpOptions, RequestTemplate, Response, RxMeta, Target,
};
use http_rig::HttpRig;
use support::{Lab, R};

/// The request builder as it was before templates (`HttpConn::queue` at
/// a003bf60), verbatim but for the receiver: `fixed` is the connection's
/// `Host` and target headers.
fn old_build(
    out: &mut Vec<u8>,
    fixed: &[u8],
    method: &str,
    path: &str,
    query: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let with_length = !body.is_empty() || matches!(method, "POST" | "PUT" | "PATCH");
    out.extend_from_slice(method.as_bytes());
    out.push(b' ');
    out.extend_from_slice(path.as_bytes());
    out.extend_from_slice(query.as_bytes());
    out.extend_from_slice(b" HTTP/1.1\r\n");
    out.extend_from_slice(fixed);
    for (k, v) in headers {
        out.extend_from_slice(k.as_bytes());
        out.extend_from_slice(b": ");
        out.extend_from_slice(v.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    if with_length {
        out.extend_from_slice(b"Content-Length: ");
        out.extend_from_slice(body.len().to_string().as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
}

/// The fixed headers `HttpDial` serialises for `t`.
fn fixed_of(t: &Target) -> Vec<u8> {
    let mut f = format!("Host: {}\r\n", t.host_header()).into_bytes();
    for (k, v) in &t.headers {
        f.extend_from_slice(format!("{k}: {v}\r\n").as_bytes());
    }
    f
}

type Case<'a> = (&'a str, &'a str, &'a str, Vec<(&'a str, &'a str)>, Vec<u8>);

fn table() -> Vec<Case<'static>> {
    let twenty: Vec<(&str, &str)> = (0..20)
        .map(|i| {
            (
                ["X-A", "X-B", "Accept", "X-Request-Id", "User-Agent"][i % 5],
                ["1", "application/json", "abc-def-0123", "ergon-io/0.1", ""][i % 5],
            )
        })
        .collect();
    vec![
        ("GET", "/", "", vec![], vec![]),
        ("GET", "/api/v3/time", "", vec![], vec![]),
        ("GET", "/api/v3/time", "?a=1&b=two", vec![], vec![]),
        ("HEAD", "/x", "", vec![("Accept", "*/*")], vec![]),
        ("DELETE", "/order", "/42", vec![], vec![]),
        ("POST", "/order", "", vec![], vec![]),
        (
            "PUT",
            "/order",
            "",
            vec![("Content-Type", "application/json")],
            b"{}".to_vec(),
        ),
        ("PATCH", "/o", "", vec![], vec![]),
        ("OPTIONS", "*", "", vec![], vec![]),
        (
            "POST",
            "/operation",
            "",
            vec![
                ("Content-Type", "application/json"),
                ("X-Request-Id", "7"),
                ("Accept", "application/json"),
            ],
            br#"{"order":1,"side":"buy"}"#.to_vec(),
        ),
        ("POST", "/bulk", "?x=1", twenty, vec![b'z'; 1000]),
        (
            "GET",
            "/q",
            "",
            vec![("X-Empty", ""), ("X-Tab", "a\tb")],
            b"body on GET".to_vec(),
        ),
    ]
}

#[test]
fn a_template_writes_the_old_builders_bytes_for_a_table_of_requests() -> R {
    for target in [
        Target::new("api.example.com", 443, ""),
        Target::new("localhost", 8443, "").with_header("X-API-Key", "key-1"),
        Target::new("h", 1, "")
            .with_header("A", "1")
            .with_header("Accept-Encoding", "identity"),
    ] {
        let fixed = fixed_of(&target);
        for (method, path, suffix, headers, body) in table() {
            let mut want = Vec::new();
            old_build(&mut want, &fixed, method, path, suffix, &headers, &body);
            let t = RequestTemplate::new(&target, method, path)?;
            let mut got = b"prefix".to_vec();
            t.write(&mut got, suffix, &headers, &body)?;
            assert_eq!(&got[..6], b"prefix", "appends");
            assert_eq!(
                String::from_utf8_lossy(&got[6..]),
                String::from_utf8_lossy(&want),
                "{method} {path}{suffix}"
            );
        }
    }
    Ok(())
}

#[derive(Default)]
struct Count(usize);
impl Handler for Count {}
impl HttpHandler for Count {
    fn on_response(&mut self, _: ConnId, _: u64, _: &Response<'_>, _: &RxMeta) {
        self.0 += 1;
    }
}

/// Every request size from the smallest to one full record: `request` and
/// `request_with` queue exactly the record the old builder's plaintext
/// seals to.
#[test]
fn request_and_request_with_queue_the_old_ciphertext_at_every_size() -> R {
    let lab = Lab::new()?;
    let target = lab.target("")?;
    let fixed = fixed_of(&target);
    let mut r = HttpRig::new(&lab, HttpOptions::default())?;
    let tpl = RequestTemplate::new(&target, "POST", "/operation")?;
    let headers = [("Content-Type", "application/octet-stream")];
    let mut h = Count::default();
    let mut fitting = 0;
    let mut base = Vec::new();
    old_build(
        &mut base,
        &fixed,
        "POST",
        "/operation",
        "?n=1",
        &headers,
        &[],
    );
    let room = MAX_REQUEST - base.len();
    let body: Vec<u8> = (0..room + 8).map(|i| (i % 251) as u8).collect();
    for len in 0..=room + 8 {
        let b = &body[..len];
        let mut plain = Vec::new();
        old_build(
            &mut plain,
            &fixed,
            "POST",
            "/operation",
            "?n=1",
            &headers,
            b,
        );
        let fits = plain.len() <= MAX_REQUEST;
        for with_template in [false, true] {
            let got = if with_template {
                r.conn.request_with(&tpl, "?n=1", &headers, b)
            } else {
                r.conn.request("POST", "/operation?n=1", &headers, b)
            };
            assert_eq!(got.is_ok(), fits, "{len} bytes, template {with_template}");
            if !fits {
                assert_eq!(r.conn.queued_tx(), []);
                continue;
            }
            fitting += 1;
            let want = r.tx.seal(&plain)?;
            assert!(
                r.conn.queued_tx() == want.as_slice(),
                "{len}-byte body, template {with_template}: ciphertext differs"
            );
            r.conn.clear_tx();
            let answer = r.records(b"HTTP/1.1 204 No Content\r\n\r\n", &[])?.concat();
            r.feed(&answer, &mut h)?;
        }
    }
    assert_eq!(h.0, fitting);
    assert!(fitting > 2 * (room - 8), "{fitting}");
    Ok(())
}

#[test]
fn a_template_is_checked_once_and_refused_on_another_target() -> R {
    let lab = Lab::new()?;
    let target = lab.target("")?;
    let mut r = HttpRig::new(&lab, HttpOptions::default())?;
    for (method, path) in [("GET /x", "/"), ("GET", "/a b"), ("GET", ""), ("", "/")] {
        assert!(
            RequestTemplate::new(&target, method, path).is_err(),
            "{method:?} {path:?}"
        );
    }
    assert!(RequestTemplate::new(&target.clone().with_header("Host", "x"), "GET", "/").is_err());
    let tpl = RequestTemplate::new(&target, "GET", "/a")?;
    assert!(r.conn.request_with(&tpl, "\r\nX: 1", &[], &[]).is_err());
    assert!(r.conn.request_with(&tpl, " x", &[], &[]).is_err());
    assert!(
        r.conn
            .request_with(&tpl, "", &[("X", "a\nb")], &[])
            .is_err()
    );
    assert!(
        r.conn
            .request_with(&tpl, "", &[("Content-Length", "0")], &[])
            .is_err()
    );
    let other = RequestTemplate::new(&target.with_header("X-Other", "1"), "GET", "/a")?;
    assert!(r.conn.request_with(&other, "", &[], &[]).is_err());
    let elsewhere = RequestTemplate::new(&Target::new("example.com", 443, ""), "GET", "/a")?;
    assert!(r.conn.request_with(&elsewhere, "", &[], &[]).is_err());
    assert_eq!(r.conn.in_flight(), 0);
    assert_eq!(r.conn.queued_tx(), []);
    r.conn.request_with(&tpl, "", &[], &[])?;
    // A clone is the same template.
    let mut h = Count::default();
    let answer = r.records(b"HTTP/1.1 204 No Content\r\n\r\n", &[])?.concat();
    r.feed(&answer, &mut h)?;
    r.conn.request_with(&tpl.clone(), "?b", &[], &[])?;
    assert_eq!(r.conn.in_flight(), 1);
    Ok(())
}

#[test]
fn a_signed_template_request_equals_the_signed_request() -> R {
    let lab = Lab::new()?;
    let signer = ergon_io::Signer::hmac_sha256(b"secret");
    let opts = HttpOptions {
        max_in_flight: 4,
        ..HttpOptions::default()
    };
    let mut a = HttpRig::new(&lab, opts)?;
    let tpl = RequestTemplate::new(&lab.target("")?, "POST", "/operation")?;
    let with_query = RequestTemplate::new(&lab.target("")?, "POST", "/operation?x=1")?;
    let ts = 1_700_000_000_000;
    let hdr = [("X-Id", "9")];
    a.conn
        .signed_request_at("POST", "/operation", "a=1&b=2", &hdr, &signer, ts)?;
    a.conn
        .signed_request_with(&tpl, "a=1&b=2", &hdr, &signer, ts)?;
    let plain = a.tx_open.open_all(a.conn.queued_tx())?;
    assert_eq!(plain.len(), 2);
    assert!(plain[0] == plain[1], "signed template request differs");
    assert!(plain[0].starts_with(b"POST /operation?a=1&b=2&timestamp=1700000000000&signature="));
    assert!(
        a.conn
            .signed_request_with(&with_query, "a=1", &[], &signer, 1)
            .is_err()
    );
    Ok(())
}

/// Header checks refuse CR, LF and NUL at every offset of a value, and a
/// non-token byte at every offset of a name; tab, space and obs-text pass.
#[test]
fn header_checks_hold_at_every_offset() -> R {
    let target = Target::new("h", 443, "");
    let tpl = RequestTemplate::new(&target, "GET", "/")?;
    let mut out = Vec::new();
    for len in 1..40usize {
        for at in 0..len {
            for bad in ['\r', '\n', '\0'] {
                let mut v: Vec<char> = "abcdefghij\tklmnop qrstuvwxyz0123456789!~"
                    .chars()
                    .collect();
                v.truncate(len);
                v[at] = bad;
                let v: String = v.into_iter().collect();
                assert!(
                    tpl.write(&mut out, "", &[("X-V", &v)], &[]).is_err(),
                    "{v:?}"
                );
            }
            let mut name: Vec<char> = "X-Abcdefghijklmnopqrstuvwxyz0123456789!#$"
                .chars()
                .collect();
            name.truncate(len);
            for bad in [' ', ':', '(', '\u{e9}', '"'] {
                let mut n = name.clone();
                n[at] = bad;
                let n: String = n.into_iter().collect();
                assert!(tpl.write(&mut out, "", &[(&n, "1")], &[]).is_err(), "{n:?}");
            }
        }
        let ok: String = "a\t \u{e9}\u{7f}~".chars().cycle().take(len).collect();
        tpl.write(&mut out, "", &[("X-Ok", &ok)], &[])?;
    }
    for name in [
        "TE",
        "host",
        "Connection",
        "Expect",
        "Upgrade",
        "Trailer",
        "CONTENT-LENGTH",
        "Transfer-Encoding",
    ] {
        assert!(
            tpl.write(&mut out, "", &[(name, "1")], &[]).is_err(),
            "{name}"
        );
    }
    for name in ["Hosts", "T", "Content-Lengths", "X-Host"] {
        tpl.write(&mut out, "", &[(name, "1")], &[])?;
    }
    Ok(())
}
