//! RFC 6455 codec: the client handshake, a server handshake (for test and lab
//! servers), frame headers, frame encoding and Close frames.
//!
//! Server frames are unmasked, so a parsed payload is a sub-slice of the
//! decrypted receive buffer.

use std::fmt;

use crate::error::{Error, Result};
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as B64;

const GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Length marker: a 16-bit length follows.
const LEN16: u8 = 0x7E;
/// Length marker: a 64-bit length follows.
const LEN64: u8 = 0x7F;

/// Continuation frame.
pub const OP_CONT: u8 = 0x0;
/// Text frame.
pub const OP_TEXT: u8 = 0x1;
/// Binary frame.
pub const OP_BINARY: u8 = 0x2;
/// Close frame.
pub const OP_CLOSE: u8 = 0x8;
/// Ping frame.
pub const OP_PING: u8 = 0x9;
/// Pong frame.
pub const OP_PONG: u8 = 0xA;

/// Close code: normal closure.
pub const CLOSE_NORMAL: u16 = 1000;
/// Close code: the endpoint is going away (a server restart).
pub const CLOSE_GOING_AWAY: u16 = 1001;

/// Largest control frame payload (Ping, Pong, Close).
pub const MAX_CONTROL: usize = 125;

/// How a close handshake ended a connection ([`Error::WsClose`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Close {
    /// Status code; `None` when the Close frame had no body.
    pub code: Option<u16>,
    /// Reason text (UTF-8, at most 123 bytes).
    pub reason: String,
    /// `true`: the server sent the first Close. `false`: the client did
    /// (`WsConn::close`), and this is the server's answer.
    pub by_server: bool,
}

impl fmt::Display for Close {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let by = if self.by_server { "server" } else { "client" };
        match self.code {
            Some(c) if self.reason.is_empty() => write!(f, "closed by the {by}: {c}"),
            Some(c) => write!(f, "closed by the {by}: {c} {}", self.reason),
            None => write!(f, "closed by the {by}: no status"),
        }
    }
}

/// Whether a status code may appear in a Close frame (RFC 6455 section 7.4):
/// 1000-1003, 1007-1014 and 3000-4999.
#[must_use]
pub const fn close_code_ok(code: u16) -> bool {
    matches!(code, 1000..=1003 | 1007..=1014 | 3000..=4999)
}

/// Parses a Close frame's payload: the code (if any) and the reason.
///
/// # Errors
/// A one-byte body, a code [`close_code_ok`] refuses, or a reason that is
/// not UTF-8.
pub fn parse_close(payload: &[u8]) -> Result<(Option<u16>, &str)> {
    match payload {
        [] => Ok((None, "")),
        [_] => Err(Error::Ws("close frame with a one-byte body")),
        [hi, lo, reason @ ..] => {
            let code = u16::from_be_bytes([*hi, *lo]);
            if !close_code_ok(code) {
                return Err(Error::Ws("invalid close code"));
            }
            let reason =
                std::str::from_utf8(reason).map_err(|_| Error::Ws("close reason is not UTF-8"))?;
            Ok((Some(code), reason))
        }
    }
}

/// A Close frame's payload: the code then the reason. Returns the buffer and
/// the length used.
///
/// # Errors
/// A reason without a code, a code [`close_code_ok`] refuses, or more than
/// [`MAX_CONTROL`] bytes.
pub fn close_payload(code: Option<u16>, reason: &str) -> Result<([u8; MAX_CONTROL], usize)> {
    let mut out = [0u8; MAX_CONTROL];
    let Some(code) = code else {
        if !reason.is_empty() {
            return Err(Error::Ws("a close reason needs a code"));
        }
        return Ok((out, 0));
    };
    if !close_code_ok(code) {
        return Err(Error::Ws("invalid close code"));
    }
    let n = 2 + reason.len();
    if n > MAX_CONTROL {
        return Err(Error::Ws("close reason longer than 123 bytes"));
    }
    out[..2].copy_from_slice(&code.to_be_bytes());
    out[2..n].copy_from_slice(reason.as_bytes());
    Ok((out, n))
}

/// A parsed frame header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameHeader {
    /// FIN bit.
    pub fin: bool,
    /// Opcode (low four bits).
    pub opcode: u8,
    /// Masking key, when the frame is masked.
    pub mask: Option<[u8; 4]>,
    /// Header length in bytes, including the masking key.
    pub header_len: usize,
    /// Payload length in bytes.
    pub payload_len: usize,
}

impl FrameHeader {
    /// Total frame length (header plus payload).
    #[must_use]
    pub const fn frame_len(&self) -> usize {
        self.header_len + self.payload_len
    }
}

/// Parses a frame header. Returns `Ok(None)` when more bytes are needed.
///
/// # Errors
/// Reserved bits set, an unknown opcode, an oversized or fragmented control frame,
/// or a 64-bit length with the top bit set.
pub fn parse_header(buf: &[u8]) -> Result<Option<FrameHeader>> {
    if buf.len() < 2 {
        return Ok(None);
    }
    let b0 = buf[0];
    let b1 = buf[1];
    if b0 & 0x70 != 0 {
        return Err(Error::Ws(
            "reserved bits set (no extensions were negotiated)",
        ));
    }
    let fin = b0 & 0x80 != 0;
    let opcode = b0 & 0x0F;
    match opcode {
        OP_CONT | OP_TEXT | OP_BINARY => {}
        OP_CLOSE | OP_PING | OP_PONG => {
            if !fin {
                return Err(Error::Ws("fragmented control frame"));
            }
            if b1 & 0x7F > 125 {
                return Err(Error::Ws("control frame longer than 125 bytes"));
            }
        }
        _ => return Err(Error::Ws("unknown opcode")),
    }
    let masked = b1 & 0x80 != 0;
    let (payload_len, mut header_len) = match b1 & 0x7F {
        126 => {
            if buf.len() < 4 {
                return Ok(None);
            }
            (usize::from(u16::from_be_bytes([buf[2], buf[3]])), 4)
        }
        127 => {
            if buf.len() < 10 {
                return Ok(None);
            }
            let mut be = [0u8; 8];
            be.copy_from_slice(&buf[2..10]);
            let len = u64::from_be_bytes(be);
            if len >> 63 != 0 {
                return Err(Error::Ws("64-bit length with the top bit set"));
            }
            let len = usize::try_from(len).map_err(|_| Error::Ws("frame too large"))?;
            (len, 10)
        }
        n => (usize::from(n), 2),
    };
    let mask = if masked {
        if buf.len() < header_len + 4 {
            return Ok(None);
        }
        let mut m = [0u8; 4];
        m.copy_from_slice(&buf[header_len..header_len + 4]);
        header_len += 4;
        Some(m)
    } else {
        None
    };
    Ok(Some(FrameHeader {
        fin,
        opcode,
        mask,
        header_len,
        payload_len,
    }))
}

/// XORs `data` with the masking key, starting at key offset 0.
pub fn apply_mask(data: &mut [u8], mask: [u8; 4]) {
    let word = u32::from_ne_bytes(mask);
    let (chunks, rest) = data.as_chunks_mut::<4>();
    for c in chunks {
        *c = (u32::from_ne_bytes(*c) ^ word).to_ne_bytes();
    }
    for (i, b) in rest.iter_mut().enumerate() {
        *b ^= mask[i];
    }
}

/// XORs `data` with the masking key, starting at key offset `offset`: for a
/// payload masked in pieces, `offset` is the piece's position in it.
pub fn apply_mask_at(data: &mut [u8], mask: [u8; 4], offset: usize) {
    let mut rotated = mask;
    rotated.rotate_left(offset % 4);
    apply_mask(data, rotated);
}

/// Length of a frame header for `payload_len` bytes.
#[must_use]
pub const fn header_len(payload_len: usize, masked: bool) -> usize {
    let base = if payload_len < 126 {
        2
    } else if payload_len <= 0xFFFF {
        4
    } else {
        10
    };
    if masked { base + 4 } else { base }
}

/// Encodes the header of one FIN frame of `payload_len` bytes into `out`.
/// Returns the header length.
///
/// # Errors
/// `out` is too small.
#[inline]
pub fn encode_header(
    opcode: u8,
    payload_len: usize,
    mask: Option<[u8; 4]>,
    out: &mut [u8],
) -> Result<usize> {
    let hl = header_len(payload_len, mask.is_some());
    if out.len() < hl {
        return Err(Error::Ws("output buffer too small for frame"));
    }
    out[0] = 0x80 | opcode;
    let mbit = if mask.is_some() { 0x80 } else { 0x00 };
    let mut pos = 2;
    if let Ok(small @ 0..126) = u8::try_from(payload_len) {
        out[1] = mbit | small;
    } else if let Ok(medium) = u16::try_from(payload_len) {
        out[1] = mbit | LEN16;
        out[2..4].copy_from_slice(&medium.to_be_bytes());
        pos = 4;
    } else {
        out[1] = mbit | LEN64;
        out[2..10].copy_from_slice(&(payload_len as u64).to_be_bytes());
        pos = 10;
    }
    if let Some(m) = mask {
        out[pos..pos + 4].copy_from_slice(&m);
    }
    Ok(hl)
}

/// Encodes one FIN frame into `out`. Returns the number of bytes written.
///
/// # Errors
/// `out` is too small.
#[inline]
pub fn encode_frame(
    opcode: u8,
    payload: &[u8],
    mask: Option<[u8; 4]>,
    out: &mut [u8],
) -> Result<usize> {
    let n = payload.len();
    if out.len() < header_len(n, mask.is_some()) + n {
        return Err(Error::Ws("output buffer too small for frame"));
    }
    let pos = encode_header(opcode, n, mask, out)?;
    out[pos..pos + n].copy_from_slice(payload);
    if let Some(m) = mask {
        apply_mask(&mut out[pos..pos + n], m);
    }
    Ok(pos + n)
}

/// A FIN frame header for `payload_len` bytes on the stack: the bytes and
/// their length (2 to 14).
#[inline]
#[must_use]
pub fn header_bytes(opcode: u8, payload_len: usize, mask: Option<[u8; 4]>) -> ([u8; 14], usize) {
    let mut h = [0u8; 14];
    h[0] = 0x80 | opcode;
    let mbit = if mask.is_some() { 0x80 } else { 0x00 };
    let mut pos = 2;
    if let Ok(small @ 0..126) = u8::try_from(payload_len) {
        h[1] = mbit | small;
    } else if let Ok(medium) = u16::try_from(payload_len) {
        h[1] = mbit | LEN16;
        h[2..4].copy_from_slice(&medium.to_be_bytes());
        pos = 4;
    } else {
        h[1] = mbit | LEN64;
        h[2..10].copy_from_slice(&(payload_len as u64).to_be_bytes());
        pos = 10;
    }
    if let Some(m) = mask {
        h[pos..pos + 4].copy_from_slice(&m);
        pos += 4;
    }
    (h, pos)
}

/// Appends a FIN frame header for `payload_len` bytes ([`header_bytes`]).
#[inline]
pub fn push_header(out: &mut Vec<u8>, opcode: u8, payload_len: usize, mask: Option<[u8; 4]>) {
    let (h, hl) = header_bytes(opcode, payload_len, mask);
    // All 14 bytes (one fixed-size store), then the unused tail is dropped.
    out.extend_from_slice(&h);
    out.truncate(out.len() - (h.len() - hl));
}

/// Appends `data` XOR-ed with the masking key, starting at key offset
/// `offset`: one copy into the vector's spare capacity, then the XOR in
/// place over the bytes just written (still in L1). No zero fill.
#[inline]
pub fn push_masked(out: &mut Vec<u8>, data: &[u8], mask: [u8; 4], offset: usize) {
    let start = out.len();
    out.extend_from_slice(data);
    apply_mask_at(&mut out[start..], mask, offset);
}

/// Appends one masked FIN frame: the header from the stack, then the
/// payload copied and masked ([`push_masked`]). The bytes equal
/// [`encode_frame`]'s with the same mask.
#[inline]
pub fn push_frame(out: &mut Vec<u8>, opcode: u8, payload: &[u8], mask: [u8; 4]) {
    push_header(out, opcode, payload.len(), Some(mask));
    push_masked(out, payload, mask, 0);
}

/// `Sec-WebSocket-Accept` value for a client key.
#[must_use]
pub fn accept_key(client_key: &str) -> String {
    let mut ctx = ring::digest::Context::new(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY);
    ctx.update(client_key.as_bytes());
    ctx.update(GUID.as_bytes());
    B64.encode(ctx.finish().as_ref())
}

/// Base64 encoding of a 16-byte client nonce.
#[must_use]
pub fn client_key(nonce: [u8; 16]) -> String {
    B64.encode(nonce)
}

/// Builds the client upgrade request. No extensions are offered, so the server
/// never compresses.
#[must_use]
pub fn client_request(
    host_header: &str,
    path: &str,
    key: &str,
    extra_headers: &[(&str, &str)],
) -> Vec<u8> {
    let mut s = format!(
        "GET {path} HTTP/1.1\r\nHost: {host_header}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n\
         Sec-WebSocket-Key: {key}\r\nSec-WebSocket-Version: 13\r\n"
    );
    for (k, v) in extra_headers {
        s.push_str(k);
        s.push_str(": ");
        s.push_str(v);
        s.push_str("\r\n");
    }
    s.push_str("\r\n");
    s.into_bytes()
}

/// Parses the server's upgrade response. Returns the header length once the full
/// header block has arrived, or `Ok(None)` if more bytes are needed.
///
/// # Errors
/// As [`parse_upgrade`] with no subprotocol offered.
pub fn parse_client_response(buf: &[u8], key: &str) -> Result<Option<usize>> {
    Ok(parse_upgrade(buf, key, &[])?.map(|u| u.header_len))
}

/// A completed upgrade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgraded {
    /// Length of the response header block; WebSocket frames follow it.
    pub header_len: usize,
    /// The subprotocol the server picked from those offered, if any.
    pub protocol: Option<String>,
}

/// Parses the server's upgrade response to a request with `key` that offered
/// `protocols`. `Ok(None)` while the header block is incomplete.
///
/// # Errors
/// [`Error::Upgrade`] for a status other than 101, with the server's
/// `Retry-After`; [`Error::Http`] for a
/// missing `Upgrade: websocket` or `Connection: Upgrade`, a missing or wrong
/// `Sec-WebSocket-Accept`, any extension (none is offered), or a subprotocol
/// that was not offered.
pub fn parse_upgrade(buf: &[u8], key: &str, protocols: &[String]) -> Result<Option<Upgraded>> {
    let mut headers = [httparse::EMPTY_HEADER; 64];
    let mut resp = httparse::Response::new(&mut headers);
    let n = match resp.parse(buf).map_err(|e| Error::Http(e.to_string()))? {
        httparse::Status::Partial => return Ok(None),
        httparse::Status::Complete(n) => n,
    };
    if resp.code != Some(101) {
        let body = String::from_utf8_lossy(&buf[n..buf.len().min(n + 300)]).into_owned();
        // A refusal (429, 418) may say how long to stay away. A date there
        // is measured against the response's Date, else this host's UTC.
        let now_ms = crate::clock::realtime_ns().div_euclid(1_000_000);
        let wait = crate::outcome::retry_after_in(resp.headers, now_ms);
        return Err(Error::Upgrade {
            status: resp.code.unwrap_or(0),
            body,
            retry_after_invalid: wait.is_err(),
            retry_after_ms: wait.ok().flatten(),
        });
    }
    let want = accept_key(key);
    let (mut accepted, mut upgrade, mut connection) = (false, false, false);
    let mut protocol = None;
    for h in resp.headers.iter() {
        let name = h.name;
        if name.eq_ignore_ascii_case("sec-websocket-accept") {
            accepted = h.value == want.as_bytes();
        } else if name.eq_ignore_ascii_case("upgrade") {
            upgrade = h.value.trim_ascii().eq_ignore_ascii_case(b"websocket");
        } else if name.eq_ignore_ascii_case("connection") {
            connection |= h
                .value
                .split(|&b| b == b',')
                .any(|t| t.trim_ascii().eq_ignore_ascii_case(b"upgrade"));
        } else if name.eq_ignore_ascii_case("sec-websocket-extensions") && !h.value.is_empty() {
            return Err(Error::Http(
                "server negotiated an extension we did not offer".into(),
            ));
        } else if name.eq_ignore_ascii_case("sec-websocket-protocol") {
            let p = std::str::from_utf8(h.value).unwrap_or_default().trim();
            if !protocols.iter().any(|o| o == p) {
                return Err(Error::Http(format!(
                    "server picked subprotocol {p:?}, which was not offered"
                )));
            }
            protocol = Some(p.to_owned());
        }
    }
    if !upgrade {
        return Err(Error::Http(
            "upgrade response without Upgrade: websocket".into(),
        ));
    }
    if !connection {
        return Err(Error::Http(
            "upgrade response without Connection: Upgrade".into(),
        ));
    }
    if !accepted {
        return Err(Error::Http("missing or wrong Sec-WebSocket-Accept".into()));
    }
    Ok(Some(Upgraded {
        header_len: n,
        protocol,
    }))
}

/// Parses a client upgrade request (lab replay server). Returns
/// `(header_len, sec_websocket_key, path)` once complete.
///
/// # Errors
/// Malformed request or missing key.
pub fn parse_server_request(buf: &[u8]) -> Result<Option<(usize, String, String)>> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let n = match req.parse(buf).map_err(|e| Error::Http(e.to_string()))? {
        httparse::Status::Partial => return Ok(None),
        httparse::Status::Complete(n) => n,
    };
    let key = req
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("sec-websocket-key"))
        .map(|h| String::from_utf8_lossy(h.value).into_owned())
        .ok_or_else(|| Error::Http("missing Sec-WebSocket-Key".into()))?;
    Ok(Some((n, key, req.path.unwrap_or("/").to_owned())))
}

/// The server's 101 response for a client key.
#[must_use]
pub fn server_response(client_key: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept_key(client_key)
    )
    .into_bytes()
}
