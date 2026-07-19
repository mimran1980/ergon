//! Dynamic stream 1002 — real `DynamicSchemaV2` + `DynamicRowV2` messages,
//! isolated from typed AppMessage traffic on stream 1001.
//!
//! No literal byte strings: every payload is a real generated SBE message
//! published through the `ClaimPublisher` and decoded by the
//! `ForegroundPersistor` on the other side.

use rusteron_client::cformat;
use std::sync::Mutex;
use std::time::Duration;

use advanced_bitget::config::{CHANNEL, STREAM_DYNAMIC, STREAM_TYPED};
use advanced_bitget::market::{Level, NormalizedEventRef, WireDec};
use advanced_bitget::persistence::{ForegroundPersistor, InMemorySink};
use advanced_bitget::publication::{AeronPublication, ClaimPublisher, PublishOutcome};

static DRIVER_LOCK: Mutex<()> = Mutex::new(());

#[test]
fn dynamic_stream_carries_schema_then_rows_isolated_from_typed() -> Result<(), Box<dyn std::error::Error>> {
    let _guard = DRIVER_LOCK.lock().unwrap();
    let driver = rusteron_media_driver::testing::EmbeddedDriver::launch().expect("driver");
    let ctx = rusteron_client::AeronContext::new().expect("ctx");
    ctx.set_dir(&cformat!("{}", driver.dir()))
        .expect("dir");
    let aeron = rusteron_client::Aeron::new(&ctx).expect("aeron");
    aeron.start().expect("start");
    let ch = CHANNEL;

    let pub_typed = aeron
        .async_add_exclusive_publication(&ch, STREAM_TYPED)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let pub_dyn = aeron
        .async_add_exclusive_publication(&ch, STREAM_DYNAMIC)
        .expect("pub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let sub_typed = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, STREAM_TYPED, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");
    let sub_dyn = aeron
        .async_add_subscription::<rusteron_client::AeronAvailableImageLogger, rusteron_client::AeronUnavailableImageLogger>(
            &ch, STREAM_DYNAMIC, None, None,
        )
        .expect("sub")
        .poll_blocking(Duration::from_secs(5))
        .expect("connect");

    let mut publisher = ClaimPublisher::new(AeronPublication(pub_typed), AeronPublication(pub_dyn))
        .expect("publisher");

    // Schema first (after subscribers connect, before data).
    assert_eq!(publisher.publish_schema(), PublishOutcome::Published);

    // One book: typed AppMessage on 1001 + dynamic row on 1002.
    let bids = [Level {
        price: WireDec::new(500005, -1),
        size: WireDec::new(15, -1),
    }];
    publisher.publish(&NormalizedEventRef::L2Book {
        symbol: "BTCUSDT",
        exchange_ts_ns: 1,
        receive_ts_ns: 2,
        sequence: 7,
        bids: &bids,
        asks: &[],
    });

    let mut persistor = ForegroundPersistor::new(InMemorySink::default());
    let mut asm = rusteron_client::AeronFragmentClosureAssembler::new().expect("asm");

    fn on_typed(
        p: &mut ForegroundPersistor<InMemorySink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        p.on_typed(buf)
            .expect("typed stream must carry only AppMessage");
    }
    fn on_dynamic(
        p: &mut ForegroundPersistor<InMemorySink>,
        buf: &[u8],
        _h: rusteron_client::AeronHeader,
    ) {
        p.on_dynamic(buf)
            .expect("dynamic stream must carry only V2 schema/rows");
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while (persistor.counters().schemas_seen < 1 || persistor.counters().persisted_typed < 1)
        && std::time::Instant::now() < deadline
    {
        asm.poll(&sub_typed, &mut persistor, on_typed, 16)
            .expect("poll");
        asm.poll(&sub_dyn, &mut persistor, on_dynamic, 16)
            .expect("poll");
        std::thread::sleep(Duration::from_millis(1));
    }

    let c = persistor.counters();
    assert_eq!(c.schemas_seen, 1, "schema announcement decoded on 1002");
    assert_eq!(
        (c.persisted_typed, c.persisted_dynamic),
        (1, 1),
        "book matched across both streams"
    );
    assert_eq!(c.decode_failures, 0, "no cross-stream contamination");
    assert_eq!(persistor.sink().l2book_typed[0].sequence, 7);

    Ok(())
}
