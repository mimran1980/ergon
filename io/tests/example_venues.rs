//! Runs `examples/venues.rs`, the Binance, Bybit and OKX adapters, against
//! the recorded responses in `tests/fixtures/venues/`: each response's
//! verdict and what it did to the budget. Pure code: every platform.
#![allow(missing_docs)]

// The example's `main` is not called here.
#[path = "../examples/venues.rs"]
#[allow(dead_code)]
mod example;

use ergon_io::limit::{BucketId, Budget, Charge, Class, Why};
use ergon_io::outcome::{self, Now};
use ergon_io::{Response, Verdict};
use example::{Next, Order, binance, bybit, json, okx};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The budget's clock (monotonic) when each response arrives.
const T0: i64 = 1_000_000_000;
const MS: i64 = 1_000_000;
const S: i64 = 1_000 * MS;
/// Server time when the fixtures' requests went out, ns since 1970.
const SERVER_NS: i64 = 1_700_000_000_000 * MS;
/// This host's UTC is 2 s ahead of the server's: a wait the server states
/// as a time must be measured on the server's clock.
const NOW: Now = Now {
    ns: T0,
    real_ns: SERVER_NS + 2 * S,
};

/// `adapter` on fixture `name` of `file`.
fn on<T>(file: &str, name: &str, adapter: impl FnOnce(&Response<'_>) -> T) -> R<T> {
    Ok(example::fixture(file, name)?.with(adapter))
}

fn id(budget: &Budget, name: &str) -> R<BucketId> {
    Ok(budget.id(name).ok_or_else(|| format!("no bucket {name}"))?)
}

fn binance() -> R<(binance::Binance, Budget)> {
    Ok(binance::Binance::new(binance::limits(
        example::BINANCE_EXCHANGE_INFO.as_bytes(),
    )?)?)
}

fn why(budget: &mut Budget, parts: &example::Parts, now: i64) -> Option<Why> {
    budget
        .try_charge(parts.as_slice(), Class::Normal, now)
        .err()
        .map(|l| l.why)
}

#[test]
fn binance_limits_come_from_exchange_info() -> R {
    let (_, budget) = binance()?;
    for (name, limit) in [
        ("REQUEST_WEIGHT 1M", 6_000),
        ("ORDERS 10S", 100),
        ("ORDERS 1D", 200_000),
        ("RAW_REQUESTS 5M", 300_000),
    ] {
        assert_eq!(budget.stats(id(&budget, name)?, T0)?.limit, limit, "{name}");
    }
    Ok(())
}

#[test]
fn binance_keeps_two_percent_of_the_ip_weight_for_cancels() -> R {
    let (_, mut budget) = binance()?;
    let weight = id(&budget, "REQUEST_WEIGHT 1M")?;
    let one = [Charge::new(weight, 1)];
    budget.try_charge(&[Charge::new(weight, 5_880)], Class::Normal, T0)?;
    let refused = budget.try_charge(&one, Class::Normal, T0).err();
    assert_eq!(refused.map(|l| l.why), Some(Why::Full));
    assert!(budget.try_charge(&one, Class::Reduce, T0).is_ok());
    Ok(())
}

#[test]
fn binance_exchange_info_that_cannot_be_read_is_an_error() {
    // A good weight and order limit, plus one bad entry.
    let entry = |kind: &str, interval: &str, num: &str, limit: &str| {
        format!(
            r#"{{"rateLimits":[{{"rateLimitType":"REQUEST_WEIGHT","interval":"MINUTE","intervalNum":1,"limit":6000}},
               {{"rateLimitType":"ORDERS","interval":"DAY","intervalNum":1,"limit":200000}},
               {{"rateLimitType":"{kind}","interval":"{interval}","intervalNum":{num},"limit":{limit}}}]}}"#
        )
    };
    let good = entry("ORDERS", "SECOND", "10", "100");
    assert!(
        binance::limits(good.as_bytes())
            .and_then(binance::Binance::new)
            .is_ok()
    );
    let bad = [
        ("no rateLimits", r#"{"symbols":[]}"#.to_owned()),
        ("cut short", r#"{"rateLimits":[{"rateLimitType":"#.to_owned()),
        ("a kind it does not know", entry("CONNECTIONS", "MINUTE", "1", "10")),
        ("an interval it does not know", entry("ORDERS", "WEEK", "1", "10")),
        ("a limit as text", entry("ORDERS", "SECOND", "10", r#""100""#)),
        ("a zero interval", entry("ORDERS", "SECOND", "0", "100")),
        ("a negative limit", entry("ORDERS", "SECOND", "10", "-1")),
        (
            "a second weight limit",
            entry("REQUEST_WEIGHT", "SECOND", "10", "100"),
        ),
        (
            "no REQUEST_WEIGHT",
            r#"{"rateLimits":[{"rateLimitType":"ORDERS","interval":"SECOND","intervalNum":10,"limit":100}]}"#
                .to_owned(),
        ),
    ];
    for (what, body) in bad {
        let built = binance::limits(body.as_bytes()).and_then(binance::Binance::new);
        assert!(built.is_err(), "{what}: accepted");
    }
}

#[test]
fn binance_an_order_charges_weight_raw_and_every_order_bucket_a_cancel_does_not() -> R {
    let (b, budget) = binance()?;
    let order = b.new_order()?;
    let mut names: Vec<_> = order
        .as_slice()
        .iter()
        .map(|c| (budget.name(c.bucket).unwrap_or("?"), c.cost))
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            ("ORDERS 10S", 1),
            ("ORDERS 1D", 1),
            ("RAW_REQUESTS 5M", 1),
            ("REQUEST_WEIGHT 1M", 1),
            ("ip", 1)
        ]
    );
    let cancel = b.request(1)?;
    let mut names: Vec<_> = cancel
        .as_slice()
        .iter()
        .map(|c| budget.name(c.bucket).unwrap_or("?"))
        .collect();
    names.sort_unstable();
    assert_eq!(names, ["RAW_REQUESTS 5M", "REQUEST_WEIGHT 1M", "ip"]);
    Ok(())
}

#[test]
fn binance_usage_headers_raise_the_local_count() -> R {
    // ANCHOR: start
    // At start: Binance's limits from `GET /api/v3/exchangeInfo`.
    let limits = binance::limits(example::BINANCE_EXCHANGE_INFO.as_bytes())?;
    let (b, mut budget) = binance::Binance::new(limits)?;
    // Before sending: every bucket the order counts against, or none.
    let ticket = budget.try_charge(b.new_order()?.as_slice(), Class::Normal, T0)?;
    // On its response (`on` replays a recorded one): usage, then the verdict.
    let v = on(example::BINANCE, "order-ack", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // ANCHOR_END: start
    assert_eq!(ticket.parts().len(), 5);
    assert_eq!(v, Verdict::Done);
    let weight = budget.stats(id(&budget, "REQUEST_WEIGHT 1M")?, T0)?;
    // The server counted 3; this process charged 1: 2 came from elsewhere.
    assert_eq!((weight.used, weight.foreign), (3, 2));
    let orders = budget.stats(id(&budget, "ORDERS 10S")?, T0)?;
    assert_eq!(
        (orders.used, orders.foreign, orders.server_used),
        (1, 0, Some(1))
    );
    Ok(())
}

#[test]
fn binance_429_on_weight_holds_the_ip_for_retry_after_and_the_guard() -> R {
    let (b, mut budget) = binance()?;
    let weight = id(&budget, "REQUEST_WEIGHT 1M")?;
    let v = on(example::BINANCE, "weight-429", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // Retry-After 20 s, plus the guard: one sub-window (1 s).
    assert_eq!(
        v,
        Verdict::Throttled {
            bucket: weight,
            until_ns: T0 + 21 * S
        }
    );
    // Everything on the IP stops, cancels included, and stays stopped until
    // the minute the server counted full has passed: no burst at the end.
    let cancel = b.request(1)?;
    assert_eq!(why(&mut budget, &cancel, T0 + 20 * S), Some(Why::Held));
    assert_eq!(why(&mut budget, &cancel, T0 + 30 * S), Some(Why::Full));
    assert_eq!(why(&mut budget, &cancel, T0 + 63 * S), None);
    Ok(())
}

#[test]
fn binance_429_on_orders_holds_only_the_bucket_its_message_names() -> R {
    let (b, mut budget) = binance()?;
    let ten_s = id(&budget, "ORDERS 10S")?;
    let day = id(&budget, "ORDERS 1D")?;
    let v = on(example::BINANCE, "orders-429", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // Retry-After 3 s plus the guard (100 ms).
    assert_eq!(
        v,
        Verdict::Throttled {
            bucket: ten_s,
            until_ns: T0 + 3_100 * MS
        }
    );
    assert_eq!(
        budget.held_until(day, T0)?,
        None,
        "the day bucket is not touched"
    );
    // Cancels do not count as orders: they still go.
    assert_eq!(why(&mut budget, &b.request(1)?, T0 + MS), None);
    assert_eq!(why(&mut budget, &b.new_order()?, T0 + MS), Some(Why::Held));

    let (b, mut budget) = binance()?;
    let v = on(example::BINANCE, "orders-day-429", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // Retry-After 30 s plus the day bucket's guard (one 60 s sub-window).
    assert_eq!(
        v,
        Verdict::Throttled {
            bucket: day,
            until_ns: T0 + 90 * S
        }
    );
    assert_eq!(budget.held_until(ten_s, T0)?, None);
    Ok(())
}

#[test]
fn binance_429_on_orders_naming_no_interval_holds_every_order_bucket() -> R {
    let (b, mut budget) = binance()?;
    let v = on(example::BINANCE, "orders-429-unnamed", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    assert!(matches!(v, Verdict::Throttled { .. }), "{v:?}");
    for name in ["ORDERS 10S", "ORDERS 1D"] {
        assert!(
            budget.held_until(id(&budget, name)?, T0)?.is_some(),
            "{name}"
        );
    }
    assert_eq!(why(&mut budget, &b.request(1)?, T0 + MS), None);
    Ok(())
}

#[test]
fn binance_418_halts_the_ip_until_a_person_clears_it() -> R {
    let (b, mut budget) = binance()?;
    // ANCHOR: ban
    let v = on(example::BINANCE, "ban-418", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // Retry-After 300 s. The IP gate has no window to align to, so no
    // guard: this is the end of the server's wait. The halt does not end
    // there; a person clears it.
    assert_eq!(
        v,
        Verdict::Banned {
            bucket: b.ip(),
            until_ns: T0 + 300 * S
        }
    );
    // Ten minutes later the server's wait is long over: still stopped.
    let later = T0 + 600 * S;
    assert_eq!(why(&mut budget, &b.request(1)?, later), Some(Why::Banned));
    // Every connect is stopped too: it charges the same gate.
    assert_eq!(
        why(&mut budget, &b.connect(), later),
        Some(Why::Banned),
        "a banned IP must not reconnect"
    );
    // The loop's alert check names it.
    let mut alerted = Vec::new();
    example::alerts(&budget, later, |name, stats| {
        alerted.push((name.to_owned(), stats.bans));
    })?;
    assert_eq!(alerted, [("ip".to_owned(), 1)]);
    // A person decides; then sending resumes.
    budget.clear_halt(b.ip())?;
    assert_eq!(why(&mut budget, &b.request(1)?, later), None);
    assert_eq!(why(&mut budget, &b.connect(), later), None);
    // ANCHOR_END: ban
    Ok(())
}

#[test]
fn binance_waf_403_is_a_ban_with_the_documented_minimum() -> R {
    let (b, mut budget) = binance()?;
    let v = on(example::BINANCE, "waf-403", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    // No Retry-After: the documented minimum, 2 minutes. The IP gate has
    // no guard.
    assert_eq!(
        v,
        Verdict::Banned {
            bucket: b.ip(),
            until_ns: T0 + 120 * S
        }
    );
    assert_eq!(budget.halted().collect::<Vec<_>>(), [b.ip()]);
    Ok(())
}

#[test]
fn binance_codes_decide_before_the_status() -> R {
    for (name, want) in [
        ("order-ack", Verdict::Done),
        ("timestamp-400", Verdict::ClockSkew),
        ("timeout-1007", Verdict::Unknown),
        ("unexpected-1006", Verdict::Unknown),
        ("busy-1008", Verdict::Unknown),
        ("cancel-replace-409", Verdict::Unknown),
        ("cancel-replace-both-failed-409", Verdict::NotRun),
        ("bad-gateway-502", Verdict::Unknown),
        ("bad-symbol-400", Verdict::Rejected),
    ] {
        let (b, mut budget) = binance()?;
        let v = on(example::BINANCE, name, |r| {
            b.on_response(r, &mut budget, NOW)
        })??;
        assert_eq!(v, want, "{name}");
        assert_eq!(budget.halted().count(), 0, "{name}");
    }
    // A 408 is not a free re-send: the backend may have taken the order.
    // `generic` and the adapter agree on Unknown.
    let (_, mut budget) = binance()?;
    let weight = id(&budget, "REQUEST_WEIGHT 1M")?;
    let generic = on(example::BINANCE, "timeout-1007", |r| {
        outcome::generic(r, weight, &mut budget, NOW)
    })??;
    assert_eq!(generic, Verdict::Unknown);
    Ok(())
}

#[test]
fn binance_a_usage_header_that_is_not_a_number_is_an_error() -> R {
    let (b, mut budget) = binance()?;
    let v = on(example::BINANCE, "bad-usage-header", |r| {
        b.on_response(r, &mut budget, NOW)
    })?;
    assert!(v.is_err(), "{v:?}");
    Ok(())
}

const CREATE: &str = "/v5/order/create";
const CANCEL: &str = "/v5/order/cancel";

fn bybit() -> R<(bybit::Bybit, Budget)> {
    Ok(bybit::Bybit::new(&[(CREATE, 20), (CANCEL, 20)])?)
}

#[test]
fn bybit_usage_headers_set_the_limit_and_the_count() -> R {
    let (b, mut budget) = bybit()?;
    let create = b.endpoint(CREATE).ok_or("no endpoint")?;
    let v = on(example::BYBIT, "order-ok", |r| {
        b.on_response(create, r, &mut budget, NOW)
    })??;
    assert_eq!(v, Verdict::Done);
    let s = budget.stats(create, T0)?;
    // X-Bapi-Limit 10, X-Bapi-Limit-Status 9 left: 1 used.
    assert_eq!((s.limit, s.used, s.server_used), (10, 1, Some(1)));
    Ok(())
}

#[test]
fn bybit_10006_holds_the_endpoint_until_the_reset() -> R {
    let (b, mut budget) = bybit()?;
    let create = b.endpoint(CREATE).ok_or("no endpoint")?;
    let v = on(example::BYBIT, "too-many-visits", |r| {
        b.on_response(create, r, &mut budget, NOW)
    })??;
    // The reset is 400 ms after the response's own `time`; plus the guard.
    assert_eq!(
        v,
        Verdict::Throttled {
            bucket: create,
            until_ns: T0 + 500 * MS
        }
    );
    assert_eq!(why(&mut budget, &b.request(CANCEL)?, T0 + MS), None);
    assert_eq!(
        why(&mut budget, &b.request(CREATE)?, T0 + MS),
        Some(Why::Held)
    );
    Ok(())
}

#[test]
fn bybit_403_halts_the_ip_whether_or_not_the_body_names_the_ban() -> R {
    let (b, mut budget) = bybit()?;
    let create = b.endpoint(CREATE).ok_or("no endpoint")?;
    let v = on(example::BYBIT, "ip-ban-403", |r| {
        b.on_response(create, r, &mut budget, NOW)
    })??;
    // "Wait for at least 10 minutes". The IP gate has no window to align
    // to, so no guard: this is the end of the server's wait. The halt does
    // not end there; a person clears it.
    assert_eq!(
        v,
        Verdict::Banned {
            bucket: b.gate(),
            until_ns: T0 + 600 * S
        }
    );
    assert_eq!(budget.halted().collect::<Vec<_>>(), [b.gate()]);
    // A banned IP must not reconnect: every connect charges the gate.
    assert_eq!(
        why(&mut budget, &b.connect(), T0 + 600 * S),
        Some(Why::Banned)
    );

    // An unrecognized 403 ("Forbidden", an HTML ban page, a U.S. IP) is
    // halted too: the safe side, until a person clears it.
    let (b, mut budget) = bybit()?;
    let v = on(example::BYBIT, "forbidden-403", |r| {
        b.on_response(create, r, &mut budget, NOW)
    })??;
    assert_eq!(
        v,
        Verdict::Banned {
            bucket: b.gate(),
            until_ns: T0 + 600 * S
        }
    );
    assert_eq!(budget.halted().collect::<Vec<_>>(), [b.gate()]);
    Ok(())
}

#[test]
fn bybit_codes() -> R {
    let (probe, _) = bybit()?;
    let create = probe.endpoint(CREATE).ok_or("no endpoint")?;
    let ip = probe.ip();
    let system = probe.system();
    for (name, want) in [
        // No wait given: the IP bucket's first backoff (1 s) plus its guard.
        (
            "ip-limit-10018",
            Verdict::Throttled {
                bucket: ip,
                until_ns: T0 + 1_100 * MS,
            },
        ),
        // The system gate, not this IP's count: its hold is the first
        // backoff (1 s). A gate has no window to align to, so no guard.
        (
            "protection-429",
            Verdict::Throttled {
                bucket: system,
                until_ns: T0 + 1_000 * MS,
            },
        ),
        ("timestamp-10002", Verdict::ClockSkew),
        ("server-error-10016", Verdict::Unknown),
        ("insufficient-balance", Verdict::Rejected),
    ] {
        let (b, mut budget) = bybit()?;
        let v = on(example::BYBIT, name, |r| {
            b.on_response(create, r, &mut budget, NOW)
        })??;
        assert_eq!(v, want, "{name}");
    }
    let (b, mut budget) = bybit()?;
    let v = on(example::BYBIT, "remaining-above-limit", |r| {
        b.on_response(create, r, &mut budget, NOW)
    })?;
    assert!(v.is_err(), "{v:?}");
    Ok(())
}

fn okx() -> R<(okx::Okx, Budget)> {
    let (mut o, mut budget) = okx::Okx::new()?;
    o.add_instrument(&mut budget, "BTC-USDT")?;
    o.add_instrument(&mut budget, "ETH-USDT")?;
    Ok((o, budget))
}

#[test]
fn okx_buckets_are_per_instrument_and_operation() -> R {
    let (o, budget) = okx()?;
    let names = |p: &example::Parts| -> Vec<String> {
        p.as_slice()
            .iter()
            .map(|c| budget.name(c.bucket).unwrap_or("?").to_owned())
            .collect()
    };
    assert_eq!(
        names(&o.request(okx::Op::Place, "BTC-USDT")?),
        ["place BTC-USDT", "sub-account"]
    );
    assert_eq!(
        names(&o.request(okx::Op::Amend, "ETH-USDT")?),
        ["amend ETH-USDT", "sub-account"]
    );
    assert_eq!(
        names(&o.request(okx::Op::Cancel, "BTC-USDT")?),
        ["cancel BTC-USDT"]
    );
    assert!(o.request(okx::Op::Place, "SOL-USDT").is_err());
    Ok(())
}

#[test]
fn okx_50011_holds_only_that_instrument_and_operation() -> R {
    let (o, mut budget) = okx()?;
    let place_btc = o.bucket(okx::Op::Place, "BTC-USDT").ok_or("no bucket")?;
    let v = on(example::OKX, "rate-limit-50011", |r| {
        o.on_response(place_btc, r, &mut budget, NOW)
    })??;
    // No wait given: the first backoff (1 s) plus the guard (100 ms).
    assert_eq!(
        v,
        Verdict::Throttled {
            bucket: place_btc,
            until_ns: T0 + 1_100 * MS
        }
    );
    let at = T0 + MS;
    let place = |inst| o.request(okx::Op::Place, inst);
    assert_eq!(why(&mut budget, &place("BTC-USDT")?, at), Some(Why::Held));
    assert_eq!(why(&mut budget, &place("ETH-USDT")?, at), None);
    let cancel = o.request(okx::Op::Cancel, "BTC-USDT")?;
    assert_eq!(why(&mut budget, &cancel, at), None);
    Ok(())
}

#[test]
fn okx_codes() -> R {
    let (probe, _) = okx()?;
    let place_btc = probe
        .bucket(okx::Op::Place, "BTC-USDT")
        .ok_or("no bucket")?;
    for (name, want) in [
        ("order-ok", Verdict::Done),
        (
            "sub-account-50061",
            Verdict::Throttled {
                bucket: probe.sub_account(),
                until_ns: T0 + 1_100 * MS,
            },
        ),
        ("timestamp-50102", Verdict::ClockSkew),
        ("timeout-50004", Verdict::Unknown),
        ("order-failed-1", Verdict::Rejected),
        ("unavailable-503", Verdict::Unknown),
    ] {
        let (o, mut budget) = okx()?;
        let v = on(example::OKX, name, |r| {
            o.on_response(place_btc, r, &mut budget, NOW)
        })??;
        assert_eq!(v, want, "{name}");
    }
    let (o, mut budget) = okx()?;
    let v = on(example::OKX, "truncated-body", |r| {
        o.on_response(place_btc, r, &mut budget, NOW)
    })?;
    assert!(v.is_err(), "{v:?}");
    Ok(())
}

#[test]
fn only_what_did_not_run_is_sent_again_and_only_inside_its_deadline() -> R {
    let (b, mut budget) = binance()?;
    let order = |deadline_ns| -> R<Order> {
        Ok(Order {
            client_order_id: 7,
            parts: b.new_order()?,
            class: Class::Normal,
            deadline_ns,
        })
    };
    // Not run, inside the deadline: sent again (the budget charged it).
    let soon = order(T0 + 5 * MS)?;
    assert!(matches!(
        example::next(&soon, Verdict::NotRun, &mut budget, T0),
        Next::SendNow(_)
    ));
    assert!(matches!(
        example::next(&soon, Verdict::ClockSkew, &mut budget, T0 + MS),
        Next::SendNow(_)
    ));
    // Past the deadline: stale, dropped, nothing charged.
    let charged = budget.stats(id(&budget, "ORDERS 10S")?, T0)?.charged;
    assert_eq!(
        example::next(&soon, Verdict::NotRun, &mut budget, T0 + 6 * MS),
        Next::Dropped
    );
    assert_eq!(
        budget.stats(id(&budget, "ORDERS 10S")?, T0)?.charged,
        charged
    );
    // Throttled: the hold (3.1 s) outlasts a 5 ms deadline: dropped.
    let v = on(example::BINANCE, "orders-429", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    assert_eq!(example::next(&soon, v, &mut budget, T0), Next::Dropped);
    // A deadline past the hold and the full window: wait, then send.
    let patient = order(T0 + 30 * S)?;
    let Next::RetryAt(at) = example::next(&patient, v, &mut budget, T0) else {
        return Err("expected RetryAt".into());
    };
    assert!(at <= patient.deadline_ns);
    assert!(matches!(
        example::next(&patient, v, &mut budget, at),
        Next::SendNow(_)
    ));
    // Unknown: never sent again, however much budget and time is left.
    assert_eq!(
        example::next(&patient, Verdict::Unknown, &mut budget, at),
        Next::Reconcile(7)
    );
    assert_eq!(
        example::next(&patient, Verdict::Rejected, &mut budget, at),
        Next::Dropped
    );
    assert_eq!(
        example::next(&patient, Verdict::Done, &mut budget, at),
        Next::Applied
    );
    Ok(())
}

#[test]
fn a_ban_stops_every_order_and_raises_the_alert() -> R {
    let (b, mut budget) = binance()?;
    let v = on(example::BINANCE, "ban-418", |r| {
        b.on_response(r, &mut budget, NOW)
    })??;
    let o = Order {
        client_order_id: 9,
        parts: b.new_order()?,
        class: Class::Reduce,
        deadline_ns: T0 + 3_600 * S,
    };
    assert_eq!(example::next(&o, v, &mut budget, T0), Next::Halted(b.ip()));
    // Even a re-send of something that did not run waits for the person.
    assert_eq!(
        example::next(&o, Verdict::NotRun, &mut budget, T0 + 1_000 * S),
        Next::Dropped
    );
    assert_eq!(example::alerts(&budget, T0, |_, _| {})?, 1);
    Ok(())
}

#[test]
fn the_json_reader_is_strict() -> R {
    let v = json::Value::parse(
        br#" {"a": [1, -2.5e3, true, null, {"b": "x\"\u00e9"}], "code": "50011", "n": -1003} "#,
    )?;
    assert_eq!(v.get("n")?.ok_or("n")?.as_i64()?, -1_003);
    assert_eq!(v.get("code")?.ok_or("code")?.code()?, 50_011);
    assert_eq!(v.get("n")?.ok_or("n")?.code()?, -1_003);
    assert_eq!(v.get("a")?.ok_or("a")?.items()?.len(), 5);
    assert_eq!(v.get("missing")?, None);
    for bad in [
        &br#"{"a":1,}"#[..],
        br#"{"a":"open}"#,
        br#"{"a":01}"#,
        br#"{"a":"\x"}"#,
        b"{\"a\":\"\x01\"}",
        br#"{"a":1} x"#,
        br#"{"a":1.}"#,
        br#"{"a":tru}"#,
        b"",
    ] {
        assert!(
            json::Value::parse(bad).is_err(),
            "{:?}",
            String::from_utf8_lossy(bad)
        );
    }
    let deep = format!("{}{}", "[".repeat(100), "]".repeat(100));
    assert!(json::Value::parse(deep.as_bytes()).is_err(), "too deep");
    let dup = json::Value::parse(br#"{"code":1,"code":2}"#)?;
    assert!(dup.get("code").is_err(), "a duplicate key is ambiguous");
    let text = json::Value::parse(br#"{"n":"12x","f":1.5,"big":99999999999999999999}"#)?;
    assert!(text.get("n")?.ok_or("n")?.code().is_err());
    assert!(text.get("f")?.ok_or("f")?.as_i64().is_err());
    assert!(text.get("big")?.ok_or("big")?.as_i64().is_err());
    Ok(())
}

#[test]
fn the_example_replays_every_recorded_response() -> R {
    let mut out = Vec::new();
    example::replay(&mut out)?;
    let out = String::from_utf8(out)?;
    for (file, venue) in [
        (example::BINANCE, "binance"),
        (example::BYBIT, "bybit"),
        (example::OKX, "okx"),
    ] {
        for (name, _) in example::recorded(file)? {
            assert!(
                out.contains(&format!("{venue} {name}:")),
                "{venue} {name} missing from:\n{out}"
            );
        }
    }
    assert!(out.contains("ALERT"), "a ban raises the alert:\n{out}");
    Ok(())
}
