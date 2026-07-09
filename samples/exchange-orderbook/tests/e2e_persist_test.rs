
#[test]
fn e2e_persist_orderbook_snapshot() {
    use ergo_clickhouse_persist::ClickhouseSinkBuilder;
    use exchange_orderbook::persist::OrderbookSnapshot;
    use chrono::Utc;
    use rust_decimal::Decimal;
    
    let url = std::env::var("CLICKHOUSE_URL").unwrap_or("http://localhost:8123".into());
    let sink = ClickhouseSinkBuilder::new()
        .url(&url)
        .batch_size(1)
        .build()
        .expect("sink build");
    
    let sender = sink.sender("orderbook_snapshots")
        .metadata("host", "e2e_test")
        .build::<OrderbookSnapshot>();
    
    let snap = OrderbookSnapshot {
        exchange: "bitget".into(),
        instrument: "BTCUSDT".into(),
        timestamp: Utc::now(),
        best_bid: Decimal::new(50000, 0),
        best_ask: Decimal::new(50100, 0),
        spread: Decimal::new(100, 0),
    };
    
    sender.persist(&snap).expect("persist");
    sink.flush().expect("flush");
}
