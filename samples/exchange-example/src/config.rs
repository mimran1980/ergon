//! Approved runtime configuration constants.

/// Aeron IPC channel for both streams — rusteron's zero-cost `c"aeron:ipc"`.
/// Do not invent another local `c"aeron:ipc"` constant.
pub use rusteron_client::AERON_IPC_STREAM as CHANNEL;
/// AppMessage-wrapped L2Book/Trade stream.
pub const STREAM_TYPED: i32 = 1001;
/// Unwrapped DynamicSchemaV2/DynamicRowV2 stream.
pub const STREAM_DYNAMIC: i32 = 1002;
/// Envelope application name.
pub const APP_NAME: &[u8] = b"ergon";
/// Traded instrument.
pub const SYMBOL: &str = "BTCUSDT";
/// Bitget public WebSocket endpoint.
pub const WS_URL: &str = "wss://ws.bitget.com/v2/ws/public";
/// Maximum maintained levels per book side. Books deeper than this are
/// truncated to the best N levels before publication; the IPC MTU is derived
/// from this bound so every book fits one claim.
pub const MAX_BOOK_LEVELS: usize = 128;
