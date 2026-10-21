//! The order request every HTTP client in the lab sends (`http_bench`,
//! `mio_bench http`): the same bytes from every client.

/// The `X-MBX-APIKEY` header value (a made-up key of Binance's length).
pub const API_KEY: &str = "vmPUZE6mv9SD5VNHk4HlWFsOr6aKE2zvsw0MuIgwCIPy6utIco14y7Ju91duEh8A";

/// The request body: Binance new-order parameters plus a signature.
#[must_use]
pub fn order_body(seq: u64) -> String {
    format!(
        "symbol=BTCUSDT&side=BUY&type=LIMIT&timeInForce=GTC&quantity=0.00100000&price=50000.00\
         &newClientOrderId=x-ergon-{seq:016}&recvWindow=5000&timestamp=1760400000000\
         &signature=c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71"
    )
}
