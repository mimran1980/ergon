//! `outcome`: `Retry-After` and usage-header parsing, `Sent` and `Verdict`,
//! and `generic`. Pure functions over a `Response`: every platform.
#![allow(missing_docs)]

use ergon_io::Response;
use ergon_io::limit::{BucketSpec, Budget};
use ergon_io::outcome::{self, Now, Sent, Verdict};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// 1994-11-06 08:49:37 UTC, in ms.
const T0_MS: i64 = 784_111_777_000;

const fn resp<'a>(headers: &'a [httparse::Header<'a>]) -> Response<'a> {
    Response {
        status: 429,
        headers,
        body: b"",
    }
}

const fn h<'a>(name: &'a str, value: &'a str) -> httparse::Header<'a> {
    httparse::Header {
        name,
        value: value.as_bytes(),
    }
}

#[test]
fn retry_after_in_seconds() -> R {
    for (value, want) in [("120", 120_000), ("0", 0), (" 7 ", 7_000)] {
        let hs = [h("Retry-After", value)];
        assert_eq!(
            outcome::retry_after_ms(&resp(&hs), T0_MS)?,
            Some(want),
            "{value:?}"
        );
    }
    // Header names are case-insensitive.
    let hs = [h("retry-after", "3")];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), T0_MS)?, Some(3_000));
    Ok(())
}

#[test]
fn retry_after_is_none_when_absent() -> R {
    let hs = [h("Content-Length", "0")];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), T0_MS)?, None);
    assert_eq!(outcome::retry_after_ms(&resp(&[]), T0_MS)?, None);
    Ok(())
}

#[test]
fn retry_after_dates_in_all_three_formats_are_measured_against_date() -> R {
    // One minute after the response's Date, written three ways (RFC 9110
    // 5.6.7). The caller's clock is a day off: it must not matter.
    let date = h("Date", "Sun, 06 Nov 1994 08:49:37 GMT");
    for later in [
        "Sun, 06 Nov 1994 08:50:37 GMT",
        "Sunday, 06-Nov-94 08:50:37 GMT",
        "Sun Nov  6 08:50:37 1994",
    ] {
        let hs = [date, h("Retry-After", later)];
        let off_by_a_day = T0_MS + 86_400_000;
        assert_eq!(
            outcome::retry_after_ms(&resp(&hs), off_by_a_day)?,
            Some(60_000),
            "{later}"
        );
    }
    // The Date header may itself use an obsolete format.
    let hs = [
        h("Date", "Sun Nov  6 08:49:37 1994"),
        h("Retry-After", "Sun, 06 Nov 1994 08:49:47 GMT"),
    ];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), 0)?, Some(10_000));
    Ok(())
}

#[test]
fn retry_after_date_without_date_uses_the_callers_clock() -> R {
    let hs = [h("Retry-After", "Sun, 06 Nov 1994 08:50:37 GMT")];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), T0_MS)?, Some(60_000));
    // Measured to the millisecond, rounded up to whole ms.
    assert_eq!(
        outcome::retry_after_ms(&resp(&hs), T0_MS + 250)?,
        Some(59_750)
    );
    // A date already past is no wait.
    assert_eq!(
        outcome::retry_after_ms(&resp(&hs), T0_MS + 120_000)?,
        Some(0)
    );
    Ok(())
}

#[test]
fn rfc850_two_digit_years_are_read_near_now() -> R {
    // 2026-11-06 was a Friday: "26" is 2026 seen from 2026, not 1926.
    let now_2026 = 1_792_238_400_000; // 2026-10-17 12:00:00 UTC
    let hs = [
        h("Date", "Sat, 17 Oct 2026 12:00:00 GMT"),
        h("Retry-After", "Friday, 06-Nov-26 00:00:00 GMT"),
    ];
    let want = (20 * 86_400 - 12 * 3_600) * 1_000;
    assert_eq!(outcome::retry_after_ms(&resp(&hs), now_2026)?, Some(want));
    Ok(())
}

#[test]
fn two_digit_years_are_read_within_50_years_of_now() -> R {
    // Seen from 1999-12-31: "10" is 2010 (ahead, within 50 years), "49" is
    // 2049 (exactly 50 ahead), "50" is 1950 (2050 would be 51 ahead).
    let now_1999 = 946_598_400_000; // 1999-12-31 00:00:00 UTC
    let date = h("Date", "Fri, 31 Dec 1999 00:00:00 GMT");
    for (later, want_days) in [
        ("Sunday, 10-Jan-10 00:00:00 GMT", 3_663),
        ("Friday, 31-Dec-49 00:00:00 GMT", 18_263),
    ] {
        let hs = [date, h("Retry-After", later)];
        let want = u64::try_from(want_days * 86_400_000_i64)?;
        assert_eq!(
            outcome::retry_after_ms(&resp(&hs), now_1999)?,
            Some(want),
            "{later}"
        );
    }
    let hs = [date, h("Retry-After", "Sunday, 01-Jan-50 00:00:00 GMT")];
    assert_eq!(
        outcome::retry_after_ms(&resp(&hs), now_1999)?,
        Some(0),
        "1950 is past"
    );
    // A weekday that fits only the other century is refused: 2050-01-01
    // was a Saturday.
    let hs = [date, h("Retry-After", "Saturday, 01-Jan-50 00:00:00 GMT")];
    assert!(outcome::retry_after_ms(&resp(&hs), now_1999).is_err());
    Ok(())
}

/// Every day from 1970 to 2199 against a plain calendar walk.
#[test]
fn every_date_from_1970_to_2199_is_counted_right() -> R {
    const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let leap = |y: u32| (y.is_multiple_of(4) && !y.is_multiple_of(100)) || y.is_multiple_of(400);
    let mut ms = 0_u64;
    let mut weekday = 4; // 1970-01-01 was a Thursday
    for year in 1970..2200 {
        for (month, name) in MONTHS.iter().enumerate() {
            let len = match month {
                1 if leap(year) => 29,
                1 => 28,
                3 | 5 | 8 | 10 => 30,
                _ => 31,
            };
            for day in 1..=len {
                let date = format!("{}, {day:02} {name} {year} 00:00:00 GMT", DAYS[weekday]);
                let hs = [h("Retry-After", &date)];
                assert_eq!(outcome::retry_after_ms(&resp(&hs), 0)?, Some(ms), "{date}");
                ms += 86_400_000;
                weekday = (weekday + 1) % 7;
            }
        }
    }
    Ok(())
}

#[test]
fn malformed_retry_after_is_an_error_never_a_default() {
    for bad in [
        "",
        "soon",
        "1.5",
        "-1",
        "+1",
        "1 2",
        // Overflows u64 ms.
        "18446744073709552",
        "99999999999999999999999",
        // Each date part out of range or misspelt, and a wrong weekday.
        "Sun, 06 Nov 1994 24:00:00 GMT",
        "Sun, 06 Nov 1994 08:60:00 GMT",
        "Sun, 06 Nov 1994 08:49:61 GMT",
        "Sun, 31 Nov 1994 08:49:37 GMT",
        "Tue, 29 Feb 2022 08:49:37 GMT",
        "Sun, 06 Nov 1994 08:49:37 UTC",
        "sun, 06 nov 1994 08:49:37 GMT",
        "Mon, 06 Nov 1994 08:49:37 GMT",
        "Sun, 6 Nov 1994 08:49:37 GMT",
        "Sun, 06 Nov 1994 08:49:37 GMT extra",
        "Sunday, 06-Nov-1994 08:49:37 GMT",
        "Sun Nov 6 08:49:37 1994",
    ] {
        let hs = [h("Retry-After", bad)];
        assert!(
            outcome::retry_after_ms(&resp(&hs), T0_MS).is_err(),
            "{bad:?} was accepted"
        );
    }
}

#[test]
fn a_leap_day_and_a_leap_second_are_dates() -> R {
    let hs = [
        h("Date", "Thu, 29 Feb 2024 23:59:59 GMT"),
        h("Retry-After", "Thu, 29 Feb 2024 23:59:60 GMT"),
    ];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), 0)?, Some(1_000));
    Ok(())
}

#[test]
fn a_malformed_date_is_an_error_only_when_it_is_needed() -> R {
    let bad_date = h("Date", "yesterday");
    let hs = [bad_date, h("Retry-After", "5")];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), T0_MS)?, Some(5_000));
    let hs = [bad_date, h("Retry-After", "Sun, 06 Nov 1994 08:50:37 GMT")];
    assert!(outcome::retry_after_ms(&resp(&hs), T0_MS).is_err());
    Ok(())
}

#[test]
fn repeated_headers_must_agree() -> R {
    let hs = [h("Retry-After", "5"), h("retry-after", "5")];
    assert_eq!(outcome::retry_after_ms(&resp(&hs), T0_MS)?, Some(5_000));
    let hs = [h("Retry-After", "5"), h("Retry-After", "50")];
    assert!(outcome::retry_after_ms(&resp(&hs), T0_MS).is_err());
    let hs = [h("X-Used", "1"), h("x-used", "2")];
    assert!(outcome::header_u64(&resp(&hs), "X-Used").is_err());
    Ok(())
}

#[test]
fn integer_headers() -> R {
    let hs = [
        h("X-Used-Units", "1200"),
        h("X-Units-Left", " 42 "),
        h("X-Max", "18446744073709551615"),
    ];
    let r = resp(&hs);
    assert_eq!(outcome::header_u64(&r, "x-used-units")?, Some(1200));
    assert_eq!(outcome::header_u64(&r, "X-Units-Left")?, Some(42));
    assert_eq!(outcome::header_u64(&r, "X-Max")?, Some(u64::MAX));
    assert_eq!(outcome::header_u64(&r, "X-Absent")?, None);
    for bad in ["", "12a", "-1", "+1", "1 2", "0x10", "18446744073709551616"] {
        let hs = [h("X-Used", bad)];
        assert!(
            outcome::header_u64(&resp(&hs), "X-Used").is_err(),
            "{bad:?} was accepted"
        );
    }
    Ok(())
}

#[test]
fn waits_until_a_server_epoch_are_measured_on_the_server_clock() {
    // A server's own epoch ms (a reset timestamp): the
    // wait counts from the server time passed in, whatever the host says.
    let server_now = 1_792_238_400_000;
    assert_eq!(
        outcome::ms_until_epoch(server_now + 5_000, server_now),
        5_000
    );
    assert_eq!(outcome::ms_until_epoch(server_now - 5_000, server_now), 0);
    assert_eq!(outcome::ms_until_epoch(i64::MAX, i64::MIN), u64::MAX);
}

#[test]
fn a_failure_without_a_response_is_not_run_only_when_never_sent() {
    assert_eq!(Verdict::from_sent(Sent::No), Verdict::NotRun);
    assert_eq!(Verdict::from_sent(Sent::Maybe), Verdict::Unknown);
}

#[test]
fn only_requests_that_did_not_run_may_be_sent_again() -> R {
    let b = Budget::new(vec![BucketSpec::rolling("ip", 1_200, 60_000, 60)])?;
    let ip = b.id("ip").ok_or("no bucket")?;
    // Not run, throttled, or refused for its timestamp: again, through the
    // budget and inside the request's own deadline (`Budget::try_resend`).
    for v in [
        Verdict::NotRun,
        Verdict::ClockSkew,
        Verdict::Throttled {
            bucket: ip,
            until_ns: 5,
        },
    ] {
        assert!(v.may_resend(), "{v:?}");
    }
    // Never: it ran, was refused for good, may have run (reconcile an order
    // by its client order id), or the venue banned the scope.
    for v in [
        Verdict::Done,
        Verdict::Rejected,
        Verdict::Unknown,
        Verdict::Banned {
            bucket: ip,
            until_ns: 5,
        },
    ] {
        assert!(!v.may_resend(), "{v:?}");
    }
    Ok(())
}

const fn status(status: u16) -> Response<'static> {
    Response {
        status,
        headers: &[],
        body: b"",
    }
}

#[test]
fn generic_sorts_statuses_alone() -> R {
    let mut b = Budget::new(vec![BucketSpec::rolling("ip", 1_200, 60_000, 60)])?;
    let ip = b.id("ip").ok_or("no bucket")?;
    let now = Now {
        ns: 1_000_000_000,
        real_ns: T0_MS * 1_000_000,
    };
    for (code, want) in [
        (200, Verdict::Done),
        (204, Verdict::Done),
        (301, Verdict::Rejected),
        (304, Verdict::Rejected),
        (400, Verdict::Rejected),
        (401, Verdict::Rejected),
        (403, Verdict::Rejected),
        (404, Verdict::Rejected),
        (408, Verdict::Unknown),
        (500, Verdict::Unknown),
        (503, Verdict::Unknown),
        (0, Verdict::Unknown),
        (600, Verdict::Unknown),
    ] {
        assert_eq!(
            outcome::generic(&status(code), ip, &mut b, now)?,
            want,
            "{code}"
        );
    }
    assert_eq!(
        b.held_until(ip, now.ns)?,
        None,
        "ordinary responses do not hold the scope"
    );
    Ok(())
}

#[test]
fn generic_does_not_interpret_service_specific_statuses_as_bans() -> R {
    let mut budget = Budget::new(vec![BucketSpec::gate("service")])?;
    let scope = budget.id("service").ok_or("no bucket")?;
    for code in [403, 418] {
        assert_eq!(
            outcome::generic(&status(code), scope, &mut budget, Now::default())?,
            Verdict::Rejected
        );
        assert!(!budget.stats(scope, 0)?.halted);
    }
    Ok(())
}

#[test]
fn generic_holds_the_scope_on_a_429_for_the_servers_wait() -> R {
    let mut b = Budget::new(vec![BucketSpec::rolling("ip", 1_200, 60_000, 60)])?;
    let ip = b.id("ip").ok_or("no bucket")?;
    let now = Now {
        ns: 1_000_000_000,
        real_ns: T0_MS * 1_000_000,
    };
    let hs = [h("Retry-After", "30")];
    match outcome::generic(&resp(&hs), ip, &mut b, now)? {
        Verdict::Throttled { bucket, until_ns } => {
            assert_eq!(bucket, ip);
            assert!(until_ns >= now.ns + 30_000_000_000, "{until_ns}");
            assert_eq!(b.held_until(ip, now.ns)?, Some(until_ns));
        }
        v => return Err(format!("{v:?}").into()),
    }
    Ok(())
}

#[test]
fn generic_holds_the_scope_even_when_retry_after_is_unreadable() -> R {
    let mut b = Budget::new(vec![BucketSpec::rolling("ip", 1_200, 60_000, 60)])?;
    let ip = b.id("ip").ok_or("no bucket")?;
    let now = Now {
        ns: 1_000_000_000,
        real_ns: T0_MS * 1_000_000,
    };
    let hs = [h("Retry-After", "soon")];
    assert!(outcome::generic(&resp(&hs), ip, &mut b, now).is_err());
    // Reported, and nothing may be sent into the 429 meanwhile.
    assert!(b.held_until(ip, now.ns)?.is_some());
    Ok(())
}
