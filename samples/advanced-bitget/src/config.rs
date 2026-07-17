//! Approved runtime configuration constants.

/// Aeron IPC channel used for both streams.
pub const CHANNEL: &str = "aeron:ipc";
/// AppMessage-wrapped L2Book/Trade stream.
pub const STREAM_TYPED: i32 = 1001;
/// Unwrapped DynamicSchemaV2/DynamicRowV2 stream.
pub const STREAM_DYNAMIC: i32 = 1002;
/// Envelope application name.
pub const APP_NAME: &[u8] = b"ergosbe";
/// Traded instrument.
pub const SYMBOL: &str = "BTCUSDT";
/// Bitget public WebSocket endpoint.
pub const WS_URL: &str = "wss://ws.bitget.com/v2/ws/public";
