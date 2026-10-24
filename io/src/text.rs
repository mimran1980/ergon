//! The text check shared by WebSocket text messages
//! ([`crate::WsOptions::text_check`]) and HTTP response bodies
//! ([`crate::HttpOptions::body_check`]), and what it found
//! ([`crate::RxMeta::text`]).
//!
//! Off by default on both: a check reads every byte once more. An HTTP
//! response head has its own rule ([`crate::NonAsciiHead`]).

/// How payloads are checked before delivery. WebSocket checks kept text
/// messages (never binary); HTTP checks every response body.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextCheck {
    /// No check: [`crate::RxMeta::text`] is [`TextStatus::Unchecked`].
    #[default]
    None,
    /// UTF-8 (what RFC 6455 requires of WebSocket text): invalid text closes
    /// the connection.
    Utf8,
    /// One pass checking that every byte is below 0x80. An ASCII payload is
    /// delivered as [`TextStatus::Ascii`]: every byte is one character, so
    /// any byte index is a character boundary. A payload with another byte
    /// is handled as the policy says.
    Ascii(NonAscii),
}

/// What [`TextCheck::Ascii`] does with a payload that has a byte at or above
/// 0x80.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NonAscii {
    /// Close the connection ([`crate::Error::Ws`] or [`crate::Error::Http`]),
    /// as invalid input would.
    Close,
    /// WebSocket only: drop the message ([`crate::WsStats::skipped`]); the
    /// connection stays up. Refused for HTTP bodies ([`crate::Error::Config`]):
    /// a dropped response would leave its request unanswered.
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
    /// Every byte is below 0x80 ([`TextCheck::Ascii`]): each byte is one
    /// character, so a parser may index it without boundary checks.
    Ascii,
    /// At least one byte is at or above 0x80, delivered under
    /// [`NonAscii::DeliverUnchecked`]: neither ASCII nor checked UTF-8.
    NotAscii,
}
