//! The generic-netlink codec for ethtool's `COALESCE_GET` and `RINGS_GET`,
//! and the controller's `GETFAMILY` that finds ethtool's family id.
//!
//! Numbers from the Linux UAPI headers `linux/netlink.h`,
//! `linux/genetlink.h` and `linux/ethtool_netlink.h` of Linux 6.12
//! (Debian's `linux-libc-dev` 6.12.111-1): a C program compiled against
//! them printed exactly these values. They are ABI, appended to and never
//! renumbered since ethtool netlink arrived in Linux 5.6; `tests/live.rs`
//! exchanges these requests with the running kernel.
//!
//! Every length is checked; nothing in a reply can make a decoder read out
//! of bounds or panic.

use super::{Coalesce, EthtoolError, Rings};

pub(super) const NLMSG_HDRLEN: usize = 16;
const GENL_HDRLEN: usize = 4;
const NLMSG_ERROR: u16 = 2;
const NLM_F_REQUEST: u16 = 1;
const NLA_F_NESTED: u16 = 1 << 15;
const NLA_F_NET_BYTEORDER: u16 = 1 << 14;
const NLA_TYPE_MASK: u16 = !(NLA_F_NESTED | NLA_F_NET_BYTEORDER);

/// `GENL_ID_CTRL`: the controller family (`NLMSG_MIN_TYPE`).
const GENL_ID_CTRL: u16 = 0x10;
const CTRL_CMD_NEWFAMILY: u8 = 1;
const CTRL_CMD_GETFAMILY: u8 = 3;
const CTRL_ATTR_FAMILY_ID: u16 = 1;
const CTRL_ATTR_FAMILY_NAME: u16 = 2;

/// `ETHTOOL_GENL_NAME`, with its terminating NUL.
const ETHTOOL_GENL_NAME: &[u8] = b"ethtool\0";
const ETHTOOL_GENL_VERSION: u8 = 1;

pub(super) const ETHTOOL_MSG_RINGS_GET: u8 = 15;
pub(super) const ETHTOOL_MSG_COALESCE_GET: u8 = 19;
const ETHTOOL_MSG_RINGS_GET_REPLY: u8 = 16;
const ETHTOOL_MSG_COALESCE_GET_REPLY: u8 = 20;

/// `ETHTOOL_A_RINGS_HEADER` and `ETHTOOL_A_COALESCE_HEADER`.
const ETHTOOL_A_HEADER: u16 = 1;
const ETHTOOL_A_HEADER_DEV_INDEX: u16 = 1;
const ETHTOOL_A_RINGS_RX_MAX: u16 = 2;
const ETHTOOL_A_RINGS_RX: u16 = 6;
const ETHTOOL_A_COALESCE_RX_USECS: u16 = 2;
const ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX: u16 = 11;

/// `ENOENT`: the controller knows no such family.
const ENOENT: i32 = 2;

const fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// A request: the netlink header (its length filled in by `finish`), the
/// generic-netlink header, then attributes.
struct Message {
    buf: Vec<u8>,
}

impl Message {
    fn new(kind: u16, seq: u32, cmd: u8, version: u8) -> Self {
        let mut buf = Vec::with_capacity(64);
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(&kind.to_ne_bytes());
        buf.extend_from_slice(&NLM_F_REQUEST.to_ne_bytes());
        buf.extend_from_slice(&seq.to_ne_bytes());
        buf.extend_from_slice(&0u32.to_ne_bytes());
        buf.extend_from_slice(&[cmd, version, 0, 0]);
        Self { buf }
    }

    fn attr(&mut self, kind: u16, payload: &[u8]) {
        let len = u16::try_from(4 + payload.len()).unwrap_or(u16::MAX);
        self.buf.extend_from_slice(&len.to_ne_bytes());
        self.buf.extend_from_slice(&kind.to_ne_bytes());
        self.buf.extend_from_slice(payload);
        self.buf.resize(align4(self.buf.len()), 0);
    }

    fn finish(mut self) -> Vec<u8> {
        let len = u32::try_from(self.buf.len()).unwrap_or(u32::MAX);
        self.buf[0..4].copy_from_slice(&len.to_ne_bytes());
        self.buf
    }
}

/// The controller's `GETFAMILY` for `ethtool`.
pub(super) fn get_family(seq: u32) -> Vec<u8> {
    let mut m = Message::new(GENL_ID_CTRL, seq, CTRL_CMD_GETFAMILY, 1);
    m.attr(CTRL_ATTR_FAMILY_NAME, ETHTOOL_GENL_NAME);
    m.finish()
}

/// An ethtool `cmd` (`COALESCE_GET`, `RINGS_GET`) for interface `ifindex`:
/// its request header, nested, holds the index.
pub(super) fn ethtool_get(family: u16, cmd: u8, ifindex: u32, seq: u32) -> Vec<u8> {
    let mut m = Message::new(family, seq, cmd, ETHTOOL_GENL_VERSION);
    let mut header = Vec::with_capacity(8);
    header.extend_from_slice(&8u16.to_ne_bytes());
    header.extend_from_slice(&ETHTOOL_A_HEADER_DEV_INDEX.to_ne_bytes());
    header.extend_from_slice(&ifindex.to_ne_bytes());
    m.attr(ETHTOOL_A_HEADER | NLA_F_NESTED, &header);
    m.finish()
}

fn ne_u16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_ne_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn ne_u32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_ne_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// Netlink attributes of `b`, each as (type without flags, payload).
fn attrs(mut b: &[u8]) -> impl Iterator<Item = Result<(u16, &[u8]), EthtoolError>> {
    std::iter::from_fn(move || {
        if b.is_empty() {
            return None;
        }
        let (Some(len), Some(kind)) = (ne_u16(b, 0), ne_u16(b, 2)) else {
            b = &[];
            return Some(Err(EthtoolError::Malformed("truncated attribute header")));
        };
        let len = usize::from(len);
        if len < 4 || len > b.len() {
            b = &[];
            return Some(Err(EthtoolError::Malformed(
                "attribute length outside its message",
            )));
        }
        let payload = &b[4..len];
        b = b.get(align4(len)..).unwrap_or_default();
        Some(Ok((kind & NLA_TYPE_MASK, payload)))
    })
}

/// The generic-netlink command and attributes of the kernel's answer to
/// request `seq` of family `kind`, or the kernel's refusal.
fn answer(reply: &[u8], seq: u32, kind: u16) -> Result<(u8, &[u8]), EthtoolError> {
    let malformed = EthtoolError::Malformed;
    let len = ne_u32(reply, 0).ok_or(EthtoolError::Malformed("shorter than a netlink header"))?;
    let len = usize::try_from(len).map_err(|_| malformed("netlink length out of range"))?;
    if len < NLMSG_HDRLEN {
        return Err(malformed("netlink length below its header"));
    }
    if len > reply.len() {
        return Err(malformed("netlink message longer than the reply"));
    }
    let msg_kind =
        ne_u16(reply, 4).ok_or(EthtoolError::Malformed("shorter than a netlink header"))?;
    if ne_u32(reply, 8) != Some(seq) {
        return Err(malformed("answer to another request"));
    }
    let body = &reply[NLMSG_HDRLEN..len];
    if msg_kind == NLMSG_ERROR {
        let code =
            ne_u32(body, 0).ok_or(EthtoolError::Malformed("error message without its code"))?;
        let code = i32::from_ne_bytes(code.to_ne_bytes());
        return Err(match code.checked_neg() {
            Some(errno) if errno > 0 => EthtoolError::Kernel(errno),
            _ if code == 0 => malformed("acknowledgement instead of an answer"),
            _ => malformed("error code out of range"),
        });
    }
    if msg_kind != kind {
        return Err(malformed("not an answer of the family asked"));
    }
    let genl = body
        .get(..GENL_HDRLEN)
        .ok_or(EthtoolError::Malformed("truncated generic netlink header"))?;
    Ok((genl[0], &body[GENL_HDRLEN..]))
}

/// The ethtool family's id from the controller's answer.
pub(super) fn decode_family(reply: &[u8], seq: u32) -> Result<u16, EthtoolError> {
    let (cmd, body) = answer(reply, seq, GENL_ID_CTRL).map_err(|e| match e {
        EthtoolError::Kernel(ENOENT) => EthtoolError::NoFamily,
        other => other,
    })?;
    if cmd != CTRL_CMD_NEWFAMILY {
        return Err(EthtoolError::Malformed("not a family description"));
    }
    for a in attrs(body) {
        let (kind, p) = a?;
        if kind == CTRL_ATTR_FAMILY_ID {
            let id = <[u8; 2]>::try_from(p)
                .map(u16::from_ne_bytes)
                .map_err(|_| EthtoolError::Malformed("family id of the wrong length"))?;
            return Ok(id);
        }
    }
    Err(EthtoolError::Malformed("family description without its id"))
}

fn u32_of(p: &[u8]) -> Result<u32, EthtoolError> {
    <[u8; 4]>::try_from(p)
        .map(u32::from_ne_bytes)
        .map_err(|_| EthtoolError::Malformed("32-bit attribute of the wrong length"))
}

const fn u8_of(p: &[u8]) -> Result<u8, EthtoolError> {
    match p {
        [v] => Ok(*v),
        _ => Err(EthtoolError::Malformed(
            "8-bit attribute of the wrong length",
        )),
    }
}

/// The reply's attributes, after checking its command and that its header
/// names `ifindex`.
fn ethtool_answer(
    reply: &[u8],
    seq: u32,
    family: u16,
    cmd: u8,
    ifindex: u32,
) -> Result<Vec<(u16, &[u8])>, EthtoolError> {
    let (got, body) = answer(reply, seq, family)?;
    if got != cmd {
        return Err(EthtoolError::Malformed("not the reply to the command sent"));
    }
    let list: Vec<(u16, &[u8])> = attrs(body).collect::<Result<_, _>>()?;
    for (kind, p) in &list {
        if *kind == ETHTOOL_A_HEADER {
            for h in attrs(p) {
                let (k, v) = h?;
                if k == ETHTOOL_A_HEADER_DEV_INDEX && u32_of(v)? != ifindex {
                    return Err(EthtoolError::Malformed("an answer about another interface"));
                }
            }
        }
    }
    Ok(list)
}

/// `COALESCE_GET`'s reply. An attribute the driver does not support is
/// absent, so `None`.
pub(super) fn decode_coalesce(
    reply: &[u8],
    seq: u32,
    family: u16,
    ifindex: u32,
) -> Result<Coalesce, EthtoolError> {
    let mut c = Coalesce::default();
    for (kind, p) in ethtool_answer(reply, seq, family, ETHTOOL_MSG_COALESCE_GET_REPLY, ifindex)? {
        match kind {
            ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX => c.use_adaptive_rx = Some(u8_of(p)? != 0),
            ETHTOOL_A_COALESCE_RX_USECS => c.rx_usecs = Some(u32_of(p)?),
            _ => {}
        }
    }
    Ok(c)
}

/// `RINGS_GET`'s reply.
pub(super) fn decode_rings(
    reply: &[u8],
    seq: u32,
    family: u16,
    ifindex: u32,
) -> Result<Rings, EthtoolError> {
    let mut r = Rings::default();
    for (kind, p) in ethtool_answer(reply, seq, family, ETHTOOL_MSG_RINGS_GET_REPLY, ifindex)? {
        match kind {
            ETHTOOL_A_RINGS_RX => r.rx = Some(u32_of(p)?),
            ETHTOOL_A_RINGS_RX_MAX => r.rx_max = Some(u32_of(p)?),
            _ => {}
        }
    }
    Ok(r)
}

#[cfg(test)]
mod tests {
    use super::{
        Coalesce, EthtoolError, GENL_HDRLEN, Message, NLMSG_HDRLEN, Rings, decode_coalesce,
        decode_family, decode_rings, ethtool_get, get_family, ne_u16, ne_u32,
    };

    // The kernel's numbers written out, not the constants under test: the
    // Linux 6.12 UAPI headers, printed by a C program compiled against them.
    const NLMSG_ERROR: u16 = 2;
    const NLM_F_REQUEST: u16 = 1;
    const NLA_F_NESTED: u16 = 0x8000;
    const GENL_ID_CTRL: u16 = 0x10;
    const CTRL_CMD_NEWFAMILY: u8 = 1;
    const CTRL_CMD_GETFAMILY: u8 = 3;
    const CTRL_ATTR_FAMILY_ID: u16 = 1;
    const CTRL_ATTR_FAMILY_NAME: u16 = 2;
    const ETHTOOL_GENL_VERSION: u8 = 1;
    const ETHTOOL_MSG_RINGS_GET: u8 = 15;
    const ETHTOOL_MSG_COALESCE_GET: u8 = 19;
    const ETHTOOL_MSG_RINGS_GET_REPLY: u8 = 16;
    const ETHTOOL_MSG_COALESCE_GET_REPLY: u8 = 20;
    const ETHTOOL_A_HEADER: u16 = 1;
    const ETHTOOL_A_HEADER_DEV_INDEX: u16 = 1;
    const ETHTOOL_A_RINGS_RX_MAX: u16 = 2;
    const ETHTOOL_A_RINGS_RX: u16 = 6;
    const ETHTOOL_A_COALESCE_RX_USECS: u16 = 2;
    const ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX: u16 = 11;
    const ENOENT: i32 = 2;

    const FAMILY: u16 = 0x1b;

    /// A reply as the kernel builds one: header, generic header, attributes.
    fn reply(kind: u16, seq: u32, cmd: u8, attributes: &[(u16, Vec<u8>)]) -> Vec<u8> {
        let mut m = Message::new(kind, seq, cmd, 1);
        for (k, p) in attributes {
            m.attr(*k, p);
        }
        m.finish()
    }

    fn header(ifindex: u32) -> (u16, Vec<u8>) {
        let mut h = Vec::new();
        h.extend_from_slice(&8u16.to_ne_bytes());
        h.extend_from_slice(&ETHTOOL_A_HEADER_DEV_INDEX.to_ne_bytes());
        h.extend_from_slice(&ifindex.to_ne_bytes());
        h.extend_from_slice(&9u16.to_ne_bytes());
        h.extend_from_slice(&2u16.to_ne_bytes()); // ETHTOOL_A_HEADER_DEV_NAME
        h.extend_from_slice(b"ens6\0\0\0\0");
        (ETHTOOL_A_HEADER | NLA_F_NESTED, h)
    }

    fn error(seq: u32, code: i32) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&36u32.to_ne_bytes());
        b.extend_from_slice(&NLMSG_ERROR.to_ne_bytes());
        b.extend_from_slice(&0u16.to_ne_bytes());
        b.extend_from_slice(&seq.to_ne_bytes());
        b.extend_from_slice(&0u32.to_ne_bytes());
        b.extend_from_slice(&code.to_ne_bytes());
        b.extend_from_slice(&[0; 16]); // the request's header, echoed
        b
    }

    fn coalesce_reply(seq: u32, ifindex: u32, adaptive: u8) -> Vec<u8> {
        reply(
            FAMILY,
            seq,
            ETHTOOL_MSG_COALESCE_GET_REPLY,
            &[
                header(ifindex),
                (ETHTOOL_A_COALESCE_RX_USECS, 20u32.to_ne_bytes().to_vec()),
                (ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX, vec![adaptive]),
                (
                    ETHTOOL_A_COALESCE_TX_USECS_FOR_TEST,
                    64u32.to_ne_bytes().to_vec(),
                ),
            ],
        )
    }

    /// `ETHTOOL_A_COALESCE_TX_USECS`: an attribute the decoder skips.
    const ETHTOOL_A_COALESCE_TX_USECS_FOR_TEST: u16 = 6;

    #[test]
    fn requests_have_the_kernels_layout() {
        let req = get_family(7);
        assert_eq!(req.len(), 16 + 4 + 12);
        assert_eq!(ne_u32(&req, 0), Some(32));
        assert_eq!(ne_u16(&req, 4), Some(GENL_ID_CTRL));
        assert_eq!(ne_u16(&req, 6), Some(NLM_F_REQUEST));
        assert_eq!(ne_u32(&req, 8), Some(7));
        assert_eq!(req[16], CTRL_CMD_GETFAMILY);
        assert_eq!(ne_u16(&req, 20), Some(12)); // 4 + "ethtool\0"
        assert_eq!(ne_u16(&req, 22), Some(CTRL_ATTR_FAMILY_NAME));
        assert_eq!(&req[24..32], b"ethtool\0");

        // The commands the client sends.
        assert_eq!(super::ETHTOOL_MSG_COALESCE_GET, ETHTOOL_MSG_COALESCE_GET);
        assert_eq!(super::ETHTOOL_MSG_RINGS_GET, ETHTOOL_MSG_RINGS_GET);
        let req = ethtool_get(FAMILY, ETHTOOL_MSG_COALESCE_GET, 3, 9);
        assert_eq!(req.len(), 16 + 4 + 12);
        assert_eq!(ne_u16(&req, 4), Some(FAMILY));
        assert_eq!(
            (req[16], req[17]),
            (ETHTOOL_MSG_COALESCE_GET, ETHTOOL_GENL_VERSION)
        );
        assert_eq!(ne_u16(&req, 20), Some(12));
        assert_eq!(ne_u16(&req, 22), Some(ETHTOOL_A_HEADER | NLA_F_NESTED));
        assert_eq!(ne_u16(&req, 24), Some(8));
        assert_eq!(ne_u16(&req, 26), Some(ETHTOOL_A_HEADER_DEV_INDEX));
        assert_eq!(ne_u32(&req, 28), Some(3));
    }

    #[test]
    fn decodes_the_family_id() {
        let r = reply(
            GENL_ID_CTRL,
            5,
            CTRL_CMD_NEWFAMILY,
            &[
                (CTRL_ATTR_FAMILY_NAME, b"ethtool\0".to_vec()),
                (CTRL_ATTR_FAMILY_ID, FAMILY.to_ne_bytes().to_vec()),
            ],
        );
        assert_eq!(decode_family(&r, 5), Ok(FAMILY));
        assert_eq!(
            decode_family(&error(5, -ENOENT), 5),
            Err(EthtoolError::NoFamily)
        );
        let no_id = reply(GENL_ID_CTRL, 5, CTRL_CMD_NEWFAMILY, &[]);
        assert!(matches!(
            decode_family(&no_id, 5),
            Err(EthtoolError::Malformed(_))
        ));
        let wide = reply(
            GENL_ID_CTRL,
            5,
            CTRL_CMD_NEWFAMILY,
            &[(CTRL_ATTR_FAMILY_ID, vec![1, 2, 3, 4])],
        );
        assert!(matches!(
            decode_family(&wide, 5),
            Err(EthtoolError::Malformed(_))
        ));
    }

    #[test]
    fn decodes_coalescing_and_rings() {
        let r = coalesce_reply(9, 3, 1);
        assert_eq!(
            decode_coalesce(&r, 9, FAMILY, 3),
            Ok(Coalesce {
                use_adaptive_rx: Some(true),
                rx_usecs: Some(20),
            })
        );
        let r = coalesce_reply(9, 3, 0);
        assert_eq!(
            decode_coalesce(&r, 9, FAMILY, 3).map(|c| c.use_adaptive_rx),
            Ok(Some(false))
        );
        // A driver without adaptive moderation leaves the attribute out.
        let r = reply(FAMILY, 9, ETHTOOL_MSG_COALESCE_GET_REPLY, &[header(3)]);
        assert_eq!(decode_coalesce(&r, 9, FAMILY, 3), Ok(Coalesce::default()));

        let r = reply(
            FAMILY,
            4,
            ETHTOOL_MSG_RINGS_GET_REPLY,
            &[
                header(3),
                (ETHTOOL_A_RINGS_RX_MAX, 8192u32.to_ne_bytes().to_vec()),
                (5, 1024u32.to_ne_bytes().to_vec()), // TX_MAX
                (ETHTOOL_A_RINGS_RX, 1024u32.to_ne_bytes().to_vec()),
            ],
        );
        assert_eq!(
            decode_rings(&r, 4, FAMILY, 3),
            Ok(Rings {
                rx: Some(1024),
                rx_max: Some(8192),
            })
        );
    }

    #[test]
    fn kernel_refusals_carry_their_error_number() {
        assert_eq!(
            decode_coalesce(&error(9, -95), 9, FAMILY, 3),
            Err(EthtoolError::Kernel(95))
        );
        assert_eq!(
            decode_rings(&error(4, -19), 4, FAMILY, 3),
            Err(EthtoolError::Kernel(19))
        );
        assert!(matches!(
            decode_rings(&error(4, 0), 4, FAMILY, 3),
            Err(EthtoolError::Malformed(_))
        ));
        assert!(matches!(
            decode_rings(&error(4, 5), 4, FAMILY, 3),
            Err(EthtoolError::Malformed(_))
        ));
        assert!(matches!(
            decode_rings(&error(4, i32::MIN), 4, FAMILY, 3),
            Err(EthtoolError::Malformed(_))
        ));
    }

    #[test]
    fn mismatched_answers_are_refused() {
        let r = coalesce_reply(9, 3, 1);
        let bad =
            |res: Result<Coalesce, EthtoolError>| matches!(res, Err(EthtoolError::Malformed(_)));
        assert!(
            bad(decode_coalesce(&r, 10, FAMILY, 3)),
            "another sequence number"
        );
        assert!(bad(decode_coalesce(&r, 9, FAMILY + 1, 3)), "another family");
        assert!(bad(decode_coalesce(&r, 9, FAMILY, 4)), "another interface");
        let rings = reply(FAMILY, 9, ETHTOOL_MSG_RINGS_GET_REPLY, &[header(3)]);
        assert!(
            bad(decode_coalesce(&rings, 9, FAMILY, 3)),
            "another command's reply"
        );
        let short = reply(
            FAMILY,
            9,
            ETHTOOL_MSG_COALESCE_GET_REPLY,
            &[(ETHTOOL_A_COALESCE_USE_ADAPTIVE_RX, vec![])],
        );
        assert!(bad(decode_coalesce(&short, 9, FAMILY, 3)), "empty u8");
        let long = reply(
            FAMILY,
            9,
            ETHTOOL_MSG_COALESCE_GET_REPLY,
            &[(ETHTOOL_A_COALESCE_RX_USECS, vec![1, 2])],
        );
        assert!(bad(decode_coalesce(&long, 9, FAMILY, 3)), "short u32");
    }

    #[test]
    fn no_truncation_or_corruption_panics_or_decodes_wrongly_long() {
        let good = coalesce_reply(9, 3, 1);
        for n in 0..good.len() {
            let res = decode_coalesce(&good[..n], 9, FAMILY, 3);
            assert!(res.is_err(), "a reply cut to {n} bytes decoded: {res:?}");
        }
        // Every single-byte corruption either fails cleanly or still decodes.
        for i in 0..good.len() {
            for v in [0u8, 1, 0x7f, 0x80, 0xff] {
                let mut b = good.clone();
                b[i] = v;
                let _ = decode_coalesce(&b, 9, FAMILY, 3);
                let _ = decode_rings(&b, 9, FAMILY, 3);
                let _ = decode_family(&b, 9);
            }
        }
        // An attribute claiming more than the message holds.
        let mut b = good;
        let first_attr = NLMSG_HDRLEN + GENL_HDRLEN;
        b[first_attr..first_attr + 2].copy_from_slice(&0xfff0u16.to_ne_bytes());
        assert!(matches!(
            decode_coalesce(&b, 9, FAMILY, 3),
            Err(EthtoolError::Malformed(_))
        ));
    }
}
