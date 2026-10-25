//! The one switch over every host-check warning: `ERGON_HOST_CHECKS`.
//! Off prints no warning line (the results are still there), on prints it,
//! require refuses to start on one.
#![allow(missing_docs)]

use ergon_tune::{
    Finding, HOST_CHECKS_ENV, HostCheckError, HostCheckPolicy, SILENCE, Severity, Status,
    host_check_line, unknown_stops_require, warn_line,
};

type R = Result<(), Box<dyn std::error::Error>>;

fn warn() -> Finding {
    Finding {
        id: "T1",
        severity: Severity::Warn,
        status: Status::Finding,
        message: "a warning".to_owned(),
        evidence: "somewhere",
    }
}

fn info() -> Finding {
    Finding {
        id: "T1",
        severity: Severity::Info,
        status: Status::Finding,
        message: "just so you know".to_owned(),
        evidence: "somewhere",
    }
}

fn ok() -> Finding {
    Finding {
        id: "T1",
        severity: Severity::Warn,
        status: Status::Ok,
        message: "all well".to_owned(),
        evidence: "somewhere",
    }
}

fn unknown(id: &'static str) -> Finding {
    Finding {
        id,
        severity: Severity::Warn,
        status: Status::Unknown("no such file".to_owned()),
        message: "cannot tell".to_owned(),
        evidence: "somewhere",
    }
}

#[test]
fn the_policy_parses_off_on_warn_and_require() -> R {
    assert_eq!(
        HostCheckPolicy::parse(HOST_CHECKS_ENV, "off"),
        Ok(HostCheckPolicy::Off)
    );
    assert_eq!(
        HostCheckPolicy::parse(HOST_CHECKS_ENV, "on"),
        Ok(HostCheckPolicy::On)
    );
    assert_eq!(
        HostCheckPolicy::parse(HOST_CHECKS_ENV, "warn"),
        Ok(HostCheckPolicy::On),
        "warn is the media driver's word for on"
    );
    assert_eq!(
        HostCheckPolicy::parse(HOST_CHECKS_ENV, "REQUIRE"),
        Ok(HostCheckPolicy::Require)
    );
    assert_eq!(HostCheckPolicy::default(), HostCheckPolicy::On);
    for bad in ["", "strict", "no", "0"] {
        let Err(e) = HostCheckPolicy::parse(HOST_CHECKS_ENV, bad) else {
            return Err(format!("{bad} parsed").into());
        };
        let text = e.to_string();
        assert!(text.contains(HOST_CHECKS_ENV), "{text}");
        assert!(text.contains(bad), "{text}");
    }
    Ok(())
}

#[test]
fn the_primary_variable_wins_and_the_alias_is_next() -> R {
    assert_eq!(
        HostCheckPolicy::from_vars(
            Some("require"),
            "RUSTERON_MEDIA_DRIVER_HOST_CHECK",
            Some("off")
        )?,
        HostCheckPolicy::Require
    );
    assert_eq!(
        HostCheckPolicy::from_vars(None, "RUSTERON_MEDIA_DRIVER_HOST_CHECK", Some("off"))?,
        HostCheckPolicy::Off
    );
    assert_eq!(
        HostCheckPolicy::from_vars(None, "RUSTERON_MEDIA_DRIVER_HOST_CHECK", None)?,
        HostCheckPolicy::On
    );
    let Err(e) =
        HostCheckPolicy::from_vars(None, "RUSTERON_MEDIA_DRIVER_HOST_CHECK", Some("strict"))
    else {
        return Err("strict parsed".into());
    };
    assert!(
        e.to_string().contains("RUSTERON_MEDIA_DRIVER_HOST_CHECK"),
        "{e}"
    );
    Ok(())
}

#[test]
fn every_warning_line_says_how_to_silence_it() -> R {
    let line = host_check_line("ergon-io", &warn()).ok_or("no line")?;
    assert!(
        line.starts_with("ergon-io warning: T1 warn: a warning"),
        "{line}"
    );
    assert!(line.ends_with(SILENCE), "{line}");
    assert_eq!(
        SILENCE,
        "(ERGON_HOST_CHECKS=off silences host-check warnings)"
    );
    // An info line is not a warning line: no silence clause.
    let line = host_check_line("ergon-io", &info()).ok_or("no line")?;
    assert!(line.starts_with("ergon-io info: T1 info:"), "{line}");
    assert!(!line.contains("ERGON_HOST_CHECKS"), "{line}");
    // An ok prints nothing.
    assert_eq!(host_check_line("ergon-io", &ok()), None);
    Ok(())
}

#[test]
fn off_keeps_the_results_and_drops_only_the_warning_lines() {
    let findings = vec![warn(), info(), ok()];
    assert_eq!(
        HostCheckPolicy::Off.warn_lines("ergon-io", &findings),
        Vec::<String>::new(),
        "no warning line is printed"
    );
    assert_eq!(findings.len(), 3, "the results are still returned");
    assert_eq!(HostCheckPolicy::Off.refusal(&findings), None);
    let off = HostCheckPolicy::Off.lines("ergon-io", &findings);
    assert_eq!(off.len(), 1, "off keeps the info line: {off:#?}");
    assert!(off[0].starts_with("ergon-io info:"), "{}", off[0]);
    let on = HostCheckPolicy::On.lines("ergon-io", &findings);
    assert_eq!(on.len(), 2, "{on:#?}");
    let on = HostCheckPolicy::On.warn_lines("ergon-io", &findings);
    assert_eq!(on.len(), 1);
    assert!(on[0].ends_with(SILENCE), "{}", on[0]);
    assert_eq!(HostCheckPolicy::On.refusal(&findings), None);
}

#[test]
fn require_refuses_to_start_on_a_warning() -> R {
    let findings = vec![ok(), info()];
    assert_eq!(HostCheckPolicy::Require.refusal(&findings), None);
    let findings = vec![warn(), info(), warn()];
    let e = HostCheckPolicy::Require
        .refusal(&findings)
        .ok_or("no refusal")?;
    let text = e.to_string();
    assert_eq!(
        e,
        HostCheckError::Warnings {
            count: 2,
            ids: "T1".to_owned(),
        }
    );
    assert!(text.contains("ERGON_HOST_CHECKS=require"), "{text}");
    assert!(
        text.contains("2 host check(s) warned or could not be proven (T1)"),
        "{text}"
    );
    assert!(
        text.contains("ERGON_HOST_CHECKS=off to start anyway"),
        "{text}"
    );
    // Two checks: both ids.
    let findings = vec![warn(), Finding { id: "K1", ..warn() }];
    let e = HostCheckPolicy::Require
        .refusal(&findings)
        .ok_or("no refusal")?;
    assert_eq!(
        e,
        HostCheckError::Warnings {
            count: 2,
            ids: "T1, K1".to_owned(),
        }
    );
    Ok(())
}

#[test]
fn warn_line_is_none_for_everything_but_a_warning() {
    assert!(warn_line("ergon-io", &warn()).is_some());
    assert!(warn_line("ergon-io", &info()).is_none());
    assert!(warn_line("ergon-io", &ok()).is_none());
}

/// `require` is a readiness gate, not a blanket refusal. An unknown of a
/// check that must prove a property of this pod stops the start: unproven is
/// not "fine". An unknown of a check whose subsystem the node may not have
/// does not: that host is a real host, and refusing there would stop every
/// start on it.
#[test]
fn require_refuses_on_an_unproven_proof_but_not_on_a_missing_subsystem() -> R {
    assert!(
        unknown_stops_require("K2"),
        "K2 proves this pod owns its CPUs"
    );
    assert!(
        !unknown_stops_require("K6"),
        "K6 reads PSI, which a kernel may not have"
    );
    // The proof: refused, and the ids are named.
    let findings = vec![ok(), unknown("K2")];
    let e = HostCheckPolicy::Require
        .refusal(&findings)
        .ok_or("an unproven K2 must stop a require start")?;
    assert_eq!(
        e,
        HostCheckError::Warnings {
            count: 1,
            ids: "K2".to_owned(),
        },
        "{e}"
    );
    // The missing subsystem: reported, not refused.
    let findings = vec![ok(), unknown("K6"), unknown("K7")];
    assert_eq!(
        HostCheckPolicy::Require.refusal(&findings),
        None,
        "a host with no PSI and no NIC node is a real host"
    );
    // And the unknowns still print.
    let lines = HostCheckPolicy::Require.lines("ergon-io", &findings);
    assert!(
        lines.iter().any(|l| l.contains("K6 unknown:")),
        "{lines:#?}"
    );
    // Off and on never refuse, whatever the findings are.
    let findings = vec![warn(), unknown("K2")];
    assert_eq!(HostCheckPolicy::Off.refusal(&findings), None);
    assert_eq!(HostCheckPolicy::On.refusal(&findings), None);
    Ok(())
}

/// A check id is a string, so a caller adds its own checks (`X1`) beside
/// the crate's. `require` fails closed on one it does not know: an unproven
/// result of an unclassified check stops the start.
#[test]
fn require_fails_closed_on_an_unknown_result_of_a_callers_own_check() -> R {
    assert!(
        unknown_stops_require("X1"),
        "an unclassified check must block"
    );
    let findings = vec![ok(), unknown("X1")];
    let e = HostCheckPolicy::Require
        .refusal(&findings)
        .ok_or("an unproven caller check must stop a require start")?;
    assert_eq!(
        e,
        HostCheckError::Warnings {
            count: 1,
            ids: "X1".to_owned(),
        }
    );
    let line = host_check_line("app", &Finding { id: "X1", ..warn() }).ok_or("no line")?;
    assert!(
        line.starts_with("app warning: X1 warn: a warning"),
        "{line}"
    );
    Ok(())
}

/// The whole classification, pinned: the exhaustive `match` that used to
/// force it is gone, so a check dropped from (or added to) the subsystem
/// list must fail here. T11 matters most: a pod normally cannot see the
/// time daemon, and blocking on it would refuse every pod start.
#[test]
fn unknown_stops_require_classifies_every_built_in_check() {
    let blocks = [
        "T1", "T2", "T7", "T8", "T9", "T10", "T12", "K1", "K2", "K3", "K4", "K5", "M1", "M2", "M3",
    ];
    let allowed = [
        "T3", "T4", "T5", "T6", "T11", "K6", "K7", "N1", "N2", "N3", "N4", "M4",
    ];
    for id in blocks {
        assert!(unknown_stops_require(id), "{id} must block when unknown");
    }
    for id in allowed {
        assert!(
            !unknown_stops_require(id),
            "{id} must not block when unknown"
        );
    }
    // Ids are exact: no case folding or trimming lets a typo through.
    for id in ["t3", " T3", "T3 ", "T03", ""] {
        assert!(unknown_stops_require(id), "{id:?} is not T3");
    }
}
