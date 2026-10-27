//! Text checks on WebSocket text and HTTP bodies, and [`AsciiText`].
//!
//! The check is shared by WebSocket text messages
//! ([`crate::WsOptions::text_check`]) and HTTP response bodies
//! ([`crate::HttpOptions::body_check`]); [`crate::RxMeta::text`] says what
//! it found. [`AsciiText`] is a byte view for text the application trusts
//! to be ASCII.
//!
//! # The default: [`TextCheck::None`]
//!
//! | | |
//! |---|---|
//! | what it does | no check: the payload is delivered as the bytes that arrived, [`TextStatus::Unchecked`] |
//! | what it does not promise | ASCII, or valid UTF-8: a WebSocket text message may break RFC 6455's UTF-8 rule and still be delivered |
//! | why it is the default | it reads nothing, so it costs nothing on the hot path; a parser that works on `&[u8]` needs no check |
//! | when you need a guarantee | [`TextCheck::Utf8`] (any character, RFC 6455) or [`TextCheck::AsciiChecked`] (every byte below 0x80): each reads every text byte once more |
//!
//! [`TextCheck::Ascii`] reads nothing either and costs the same as `None`:
//! it is a promise about the source, not a check. An HTTP response head has
//! its own rule ([`crate::NonAsciiHead`]).
//!
//! The one `unsafe` here is [`AsciiText::as_str_unchecked`], so this module
//! allows `unsafe_code`.
#![allow(unsafe_code)]

use core::ops::{Bound, Index, RangeBounds};

/// How payloads are checked before delivery. WebSocket checks kept text
/// messages (never binary); HTTP checks every response body.
///
/// A parser that works on `&[u8]` needs no check at all: use
/// [`TextCheck::None`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextCheck {
    /// The default. No check: the payload is delivered as the bytes that
    /// arrived, and [`crate::RxMeta::text`] is [`TextStatus::Unchecked`].
    /// Nothing promises ASCII or valid UTF-8. It reads nothing, so it costs
    /// nothing on the hot path; use [`TextCheck::Utf8`] or
    /// [`TextCheck::AsciiChecked`] when the parser needs a guarantee.
    #[default]
    None,
    /// UTF-8 (what RFC 6455 requires of WebSocket text): invalid text closes
    /// the connection.
    Utf8,
    /// A trust declaration, not a check: the source is known to send ASCII.
    /// The payload is never read, so this costs the same as
    /// [`TextCheck::None`]. It is delivered unchanged as
    /// [`TextStatus::AssumedAscii`], whatever its bytes, and never closes or
    /// drops anything. Wrap it in [`AsciiText`] to index it by byte, or take
    /// a `&str` with the `unsafe` [`AsciiText::as_str_unchecked`].
    Ascii,
    /// One pass checking that every byte is below 0x80. An ASCII payload is
    /// delivered as [`TextStatus::Ascii`]: every byte is one character, so
    /// any byte index is a character boundary and
    /// [`core::str::from_utf8`] cannot fail. A payload with another byte is
    /// handled as the policy says.
    AsciiChecked(NonAscii),
}

/// What [`TextCheck::AsciiChecked`] does with a payload that has a byte at
/// or above 0x80.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonAscii {
    /// Close the connection ([`crate::Error::Ws`] or [`crate::Error::Http`]),
    /// as invalid input would.
    Close,
    /// WebSocket only: drop the message ([`crate::WsStats::skipped`]); the
    /// connection stays up. Refused for HTTP bodies ([`crate::Error::Config`]):
    /// a dropped response would leave its request unanswered.
    ///
    /// Under [`crate::Interest::Stream`] each fragment is checked as it
    /// arrives, so a message whose fragment `k` is not ASCII stops mid-way:
    /// fragments `0..k` were already delivered through
    /// [`crate::WsHandler::on_fragment`], no `fin` fragment follows, and
    /// no callback signals the drop (the message counts in
    /// [`crate::WsStats::skipped`] once its last frame arrives, and on Linux
    /// the first one per connection queues one warning line). The next
    /// message starts again at index 0: a handler that sees index 0 while
    /// it holds a partial message should discard that partial message.
    Skip,
    /// Deliver it as plain bytes, flagged [`TextStatus::NotAscii`]: a stray
    /// byte never drops a feed.
    #[default]
    DeliverUnchecked,
}

/// What the text check found for a delivered payload ([`crate::RxMeta::text`]).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextStatus {
    /// Not checked: a WebSocket binary message, [`TextCheck::None`], or not
    /// yet checked ([`crate::WsHandler::on_message_start`]).
    #[default]
    Unchecked,
    /// Valid UTF-8 ([`TextCheck::Utf8`]); a streamed WebSocket fragment may
    /// start or end inside a character.
    Utf8,
    /// Every byte is below 0x80 ([`TextCheck::AsciiChecked`]): each byte is
    /// one character, so a parser may index it without boundary checks.
    Ascii,
    /// Trusted to be ASCII, never read ([`TextCheck::Ascii`]). Nothing was
    /// verified: the bytes are whatever the source sent.
    AssumedAscii,
    /// At least one byte is at or above 0x80, delivered under
    /// [`NonAscii::DeliverUnchecked`]: neither ASCII nor checked UTF-8.
    NotAscii,
}

/// What [`TextCheck::scan`] found: a status, a UTF-8 check the caller runs
/// (it differs between whole and streamed payloads), or a non-ASCII byte
/// with the policy for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scan {
    /// Deliver with this status.
    Status(TextStatus),
    /// [`TextCheck::Utf8`]: the caller checks.
    Utf8,
    /// A byte at or above 0x80 under [`TextCheck::AsciiChecked`].
    NotAscii(NonAscii),
}

impl TextCheck {
    /// The one place a payload's bytes are read for the ASCII check. Under
    /// [`TextCheck::None`] and [`TextCheck::Ascii`], `parts` is never
    /// iterated (a unit test below proves it).
    #[inline]
    pub(crate) fn scan<'a, I>(self, parts: I) -> Scan
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        match self {
            Self::None => Scan::Status(TextStatus::Unchecked),
            Self::Ascii => Scan::Status(TextStatus::AssumedAscii),
            Self::Utf8 => Scan::Utf8,
            Self::AsciiChecked(policy) => {
                if parts.into_iter().fold(0, |acc, s| acc | high_bits(s)) == 0 {
                    Scan::Status(TextStatus::Ascii)
                } else {
                    Scan::NotAscii(policy)
                }
            }
        }
    }
}

/// The top bit of every byte of `s`, OR-ed together: 0 when `s` is ASCII.
#[inline]
pub(crate) fn high_bits(s: &[u8]) -> u64 {
    const HIGH: u64 = 0x8080_8080_8080_8080;
    let (words, tail) = s.as_chunks::<8>();
    let mut acc = words.iter().fold(0, |acc, w| acc | u64::from_ne_bytes(*w));
    for &b in tail {
        acc |= u64::from(b);
    }
    acc & HIGH
}

/// Bytes the application trusts to be ASCII (a payload delivered under
/// [`TextCheck::Ascii`], or any slice): a `&[u8]` that reads as text.
///
/// Built from a payload at no cost ([`AsciiText::new`] reads nothing) and
/// never allocates. Every method is a byte operation, safe whatever the
/// bytes are; only [`AsciiText::as_str_unchecked`] relies on them being
/// ASCII.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct AsciiText<'a>(&'a [u8]);

impl<'a> AsciiText<'a> {
    /// Wraps `bytes`; reads nothing.
    #[inline]
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }

    /// The bytes.
    #[inline]
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.0
    }

    /// Length in bytes (one byte per character when the bytes are ASCII).
    #[inline]
    #[must_use]
    pub const fn len(&self) -> usize {
        self.0.len()
    }

    /// `true` when there are no bytes.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The byte at `index`, or `None` past the end.
    #[inline]
    #[must_use]
    pub const fn get(&self, index: usize) -> Option<u8> {
        if index < self.0.len() {
            Some(self.0[index])
        } else {
            None
        }
    }

    /// The bytes in `range`, or `None` when it is out of bounds or reversed.
    #[inline]
    #[must_use]
    pub fn slice<R: RangeBounds<usize>>(&self, range: R) -> Option<Self> {
        let start = match range.start_bound() {
            Bound::Included(&s) => s,
            Bound::Excluded(&s) => s.checked_add(1)?,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&e) => e.checked_add(1)?,
            Bound::Excluded(&e) => e,
            Bound::Unbounded => self.0.len(),
        };
        self.0.get(start..end).map(Self)
    }

    /// The index of the first `byte`.
    #[inline]
    #[must_use]
    pub fn find(&self, byte: u8) -> Option<usize> {
        self.0.iter().position(|&b| b == byte)
    }

    /// The index of the first byte `pred` accepts.
    #[inline]
    pub fn position<P: FnMut(u8) -> bool>(&self, mut pred: P) -> Option<usize> {
        self.0.iter().position(|&b| pred(b))
    }

    /// `true` when the bytes start with `prefix`.
    #[inline]
    #[must_use]
    pub fn starts_with(&self, prefix: &[u8]) -> bool {
        self.0.starts_with(prefix)
    }

    /// The bytes before and from `mid`, or `None` when `mid` is past the end.
    #[inline]
    #[must_use]
    pub const fn split_at(&self, mid: usize) -> Option<(Self, Self)> {
        match self.0.split_at_checked(mid) {
            Some((a, b)) => Some((Self(a), Self(b))),
            None => None,
        }
    }

    /// The bytes as a `&str`, without a check. Debug builds assert the bytes
    /// are ASCII; release builds read nothing.
    ///
    /// # Safety
    ///
    /// The caller vouches that every byte is below 0x80 (a source trusted
    /// under [`TextCheck::Ascii`], or one checked otherwise). Calling it on
    /// bytes that are not ASCII breaks this contract and is undefined
    /// behaviour: a `&str` must be UTF-8, and nothing here checks. Under
    /// [`TextStatus::Ascii`], or [`TextStatus::Utf8`] on a whole message,
    /// use the safe [`core::str::from_utf8`] instead: it cannot fail there
    /// (a streamed `Utf8` fragment may start or end inside a character).
    #[inline]
    #[must_use]
    pub unsafe fn as_str_unchecked(&self) -> &'a str {
        debug_assert!(
            self.0.is_ascii(),
            "AsciiText::as_str_unchecked on non-ASCII bytes"
        );
        // SAFETY: the caller guarantees the bytes are ASCII (this function's
        // contract), and ASCII is valid UTF-8.
        unsafe { core::str::from_utf8_unchecked(self.0) }
    }
}

impl Index<usize> for AsciiText<'_> {
    type Output = u8;

    /// The byte at `index`; out of bounds panics, as a slice does
    /// ([`AsciiText::get`] does not).
    #[inline]
    fn index(&self, index: usize) -> &u8 {
        &self.0[index]
    }
}

impl<'a> From<&'a [u8]> for AsciiText<'a> {
    #[inline]
    fn from(bytes: &'a [u8]) -> Self {
        Self(bytes)
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::{AsciiText, NonAscii, Scan, TextCheck, TextStatus};

    /// Parts that count every time one is read.
    #[derive(Clone)]
    struct Probe<'a> {
        parts: &'a [&'a [u8]],
        reads: &'a Cell<usize>,
    }

    impl<'a> Iterator for Probe<'a> {
        type Item = &'a [u8];

        fn next(&mut self) -> Option<&'a [u8]> {
            let (first, rest) = self.parts.split_first()?;
            self.reads.set(self.reads.get() + 1);
            self.parts = rest;
            Some(first)
        }
    }

    fn scan(check: TextCheck, parts: &[&[u8]]) -> (Scan, usize) {
        let reads = Cell::new(0);
        let found = check.scan(Probe {
            parts,
            reads: &reads,
        });
        (found, reads.get())
    }

    /// `Ascii` and `None` never read a part; `AsciiChecked` reads every
    /// one (the probe counts, so a scan added to `Ascii` fails here).
    #[test]
    fn ascii_never_reads_the_payload() {
        let mixed: &[&[u8]] = &[b"price=", "\u{e9}".as_bytes(), b"1"];
        let ascii: &[&[u8]] = &[b"price=", b"1"];
        for parts in [mixed, ascii] {
            assert_eq!(
                scan(TextCheck::Ascii, parts),
                (Scan::Status(TextStatus::AssumedAscii), 0)
            );
            assert_eq!(
                scan(TextCheck::None, parts),
                (Scan::Status(TextStatus::Unchecked), 0)
            );
            assert_eq!(scan(TextCheck::Utf8, parts), (Scan::Utf8, 0));
        }
        let close = TextCheck::AsciiChecked(NonAscii::Close);
        assert_eq!(
            scan(close, ascii),
            (Scan::Status(TextStatus::Ascii), ascii.len())
        );
        assert_eq!(
            scan(close, mixed),
            (Scan::NotAscii(NonAscii::Close), mixed.len())
        );
    }

    #[test]
    fn ascii_text_methods_are_byte_operations() {
        let t = AsciiText::new(b"key=value;x");
        assert_eq!(t.as_bytes(), b"key=value;x");
        assert_eq!((t.len(), t.is_empty()), (11, false));
        assert!(AsciiText::new(b"").is_empty());
        assert_eq!((t[0], t.get(3), t.get(11)), (b'k', Some(b'='), None));
        assert_eq!(t.find(b'='), Some(3));
        assert_eq!(t.find(b'#'), None);
        assert_eq!(t.position(|b| b == b';'), Some(9));
        assert!(t.starts_with(b"key"));
        assert!(!t.starts_with(b"value"));
        let (k, v) = t.split_at(3).unwrap_or_default();
        assert_eq!(
            (k.as_bytes(), v.as_bytes()),
            (&b"key"[..], &b"=value;x"[..])
        );
        assert_eq!(t.split_at(12), None);
        assert_eq!(t.slice(4..9).map(|s| s.as_bytes()), Some(&b"value"[..]));
        assert_eq!(t.slice(4..=8).map(|s| s.as_bytes()), Some(&b"value"[..]));
        assert_eq!(t.slice(..3).map(|s| s.as_bytes()), Some(&b"key"[..]));
        assert_eq!(t.slice(10..).map(|s| s.as_bytes()), Some(&b"x"[..]));
        assert_eq!(t.slice(11..).map(|s| s.as_bytes()), Some(&b""[..]));
        let (from, to) = (5, 4);
        assert_eq!(t.slice(from..to), None);
        assert_eq!(t.slice(..12), None);
        assert_eq!(t.slice(..=usize::MAX), None);
        assert_eq!(AsciiText::from(&b"ab"[..]), AsciiText::new(b"ab"));
        // Byte operations stay safe on bytes that are not ASCII.
        let latin = AsciiText::new("\u{e9}t\u{e9}".as_bytes());
        assert_eq!((latin.len(), latin.find(b't')), (5, Some(2)));
        assert_eq!(latin.split_at(1).map(|(a, _)| a.len()), Some(1));
    }

    #[test]
    fn as_str_unchecked_on_ascii() {
        let t = AsciiText::new(b"{\"p\":\"1.5\"}");
        // SAFETY: the literal is ASCII.
        let s = unsafe { t.as_str_unchecked() };
        assert_eq!(s, "{\"p\":\"1.5\"}");
        assert_eq!(s.as_ptr(), t.as_bytes().as_ptr());
        // SAFETY: an empty slice is ASCII.
        assert_eq!(unsafe { AsciiText::default().as_str_unchecked() }, "");
    }

    /// Debug builds catch a broken promise before it becomes a `&str`. The
    /// call is undefined behaviour without the assertion, so this test
    /// exists only where the assertion does.
    #[cfg(debug_assertions)]
    #[test]
    fn as_str_unchecked_asserts_ascii_in_debug() {
        let bytes = "caf\u{e9}".as_bytes();
        let caught = std::panic::catch_unwind(|| {
            // SAFETY: not upheld on purpose; the debug assertion fires
            // before `from_utf8_unchecked` runs.
            let s = unsafe { AsciiText::new(bytes).as_str_unchecked() };
            s.len()
        });
        assert!(caught.is_err());
    }
}
