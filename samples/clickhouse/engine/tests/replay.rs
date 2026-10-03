//! The engine replayed with no infrastructure: a checked-in input gives the
//! checked-in output byte for byte, and any input gives the same output twice.

use std::collections::BTreeMap;
use std::error::Error;

use engine::replay::{self, Market};
use ergon_runtime::frames;
use proptest::prelude::*;

const INBOUND: &[u8] = include_bytes!("fixtures/engine-inbound.bin");
const GOLDEN: &[u8] = include_bytes!("fixtures/engine-outbound.golden");

/// Records per stream: enough to say what changed when bytes differ.
fn summary(log: &[u8]) -> Result<BTreeMap<String, usize>, Box<dyn Error>> {
    let parsed = frames::parse(log)?;
    let mut counts = BTreeMap::new();
    for r in parsed.records {
        let name = parsed
            .names
            .get(r.stream as usize)
            .cloned()
            .unwrap_or_default();
        *counts.entry(name).or_default() += 1;
    }
    Ok(counts)
}

#[test]
fn the_fixture_replays_to_the_golden_output() -> Result<(), Box<dyn Error>> {
    assert!(
        replay::inbound(&Market::FIXTURE) == INBOUND,
        "the generator drifted from engine-inbound.bin: run `just engine-fixtures`"
    );
    let outbound = replay::run(INBOUND.to_vec())?;
    if outbound != GOLDEN {
        return Err(format!(
            "the engine's output changed: {:?} against the golden {:?}; if intended, run `just engine-fixtures`",
            summary(&outbound)?,
            summary(GOLDEN)?
        )
        .into());
    }
    let counts = summary(GOLDEN)?;
    assert!(
        counts.get("engine-r1/orders").is_some_and(|&n| n > 0),
        "the fixture trades: {counts:?}"
    );
    assert!(
        counts.get("engine-r1/signals").is_some_and(|&n| n > 0),
        "and publishes signals: {counts:?}"
    );
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 16, .. ProptestConfig::default() })]
    #[test]
    fn the_same_input_gives_the_same_output(
        seed in 1u64..u64::MAX,
        seconds in 60u32..400,
        repeat_every in prop_oneof![Just(0u32), 5u32..60],
    ) {
        let input = replay::inbound(&Market { seed, seconds, repeat_every });
        let first = replay::run(input.clone());
        let second = replay::run(input);
        prop_assert!(first.is_ok(), "{first:?}");
        prop_assert_eq!(first.ok(), second.ok());
    }
}
