//! The `ll-verify` output: one tab-separated line per check, a header and a
//! result line. `scripts/verify.sh` and the e2e read it; its format is part
//! of the documentation.
//!
//! Fields are tab-separated (two spaces here):
//!
//! ```text
//! ll-verify  v1  contract=placement  node=<n>  window_s=60.0  samples=600
//! ll-verify  pod=trading/ll-ping-x  P3.affinity  ll-hot/4242  PASS  allowed=3 expected=3
//! ll-verify  pod=trading-driver/ll-media-driver-y  P5.affinity  aeron-md-nra/318  PASS  allowed=1 expected=shared(1)
//! ll-verify  node=<n>  F2.device_irqs  cpu=3  NOT-IN-CONTRACT  delta=0
//! ll-verify  RESULT  PASS  checks=<k>  failed=0
//! ```

use crate::probe::verdict::{Check, Contract, Outcome};

/// The header line.
#[must_use]
pub fn header(contract: Contract, node: &str, window_s: f64, samples: u64) -> String {
    format!(
        "ll-verify\tv1\tcontract={contract}\tnode={node}\twindow_s={window_s:.1}\tsamples={samples}"
    )
}

/// One check's line.
#[must_use]
pub fn line(check: &Check) -> String {
    // a tab inside a field would shift every later column
    let clean = |s: &str| s.replace(['\t', '\n'], " ");
    format!(
        "ll-verify\t{}\t{}\t{}\t{}\t{}",
        clean(&check.scope),
        clean(&check.id),
        clean(&check.subject),
        check.outcome,
        clean(&check.detail)
    )
}

/// The result line.
#[must_use]
pub fn result_line(outcome: Outcome, checks: usize, failed: usize) -> String {
    format!("ll-verify\tRESULT\t{outcome}\tchecks={checks}\tfailed={failed}")
}

/// A parsed check line (what `verify.sh` and the tests read back).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Parsed {
    /// The scope.
    pub scope: String,
    /// The check id.
    pub id: String,
    /// The subject.
    pub subject: String,
    /// `PASS`, `FAIL`, `NOT-IN-CONTRACT` or `INFO`.
    pub outcome: String,
    /// The detail.
    pub detail: String,
}

/// Parses a check line; `None` for the header, the result line or anything
/// else.
#[must_use]
pub fn parse_line(text: &str) -> Option<Parsed> {
    let mut f = text.trim_end_matches(['\r', '\n']).split('\t');
    if f.next()? != "ll-verify" {
        return None;
    }
    let scope = f.next()?;
    if scope == "v1" || scope == "RESULT" {
        return None;
    }
    let parsed = Parsed {
        scope: scope.to_owned(),
        id: f.next()?.to_owned(),
        subject: f.next()?.to_owned(),
        outcome: f.next()?.to_owned(),
        detail: f.next().unwrap_or_default().to_owned(),
    };
    ["PASS", "FAIL", "NOT-IN-CONTRACT", "INFO"]
        .contains(&parsed.outcome.as_str())
        .then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_golden() {
        assert_eq!(
            header(Contract::Placement, "kind-ll", 60.0, 600),
            "ll-verify\tv1\tcontract=placement\tnode=kind-ll\twindow_s=60.0\tsamples=600"
        );
        let c = Check::judged(
            Contract::Placement,
            "pod=trading/ll-ping-x",
            "P3.affinity",
            "ll-hot/4242",
            true,
            "allowed=3 expected=3".into(),
        );
        let text = line(&c);
        assert_eq!(
            text,
            "ll-verify\tpod=trading/ll-ping-x\tP3.affinity\tll-hot/4242\tPASS\tallowed=3 expected=3"
        );
        assert_eq!(
            result_line(Outcome::Pass, 12, 0),
            "ll-verify\tRESULT\tPASS\tchecks=12\tfailed=0"
        );
        let back = parse_line(&text);
        assert_eq!(
            back,
            Some(Parsed {
                scope: "pod=trading/ll-ping-x".into(),
                id: "P3.affinity".into(),
                subject: "ll-hot/4242".into(),
                outcome: "PASS".into(),
                detail: "allowed=3 expected=3".into(),
            })
        );
        assert_eq!(parse_line(&result_line(Outcome::Pass, 1, 0)), None);
        assert_eq!(parse_line(&header(Contract::Full, "n", 1.0, 1)), None);
        assert_eq!(parse_line("random"), None);
    }

    /// The real capture the README shows (scripts/e2e-vdev.sh --capture):
    /// every ll-verify line parses, and the run passed.
    #[test]
    fn the_captured_e2e_output_parses() {
        let text = include_str!("../../tests/fixtures/e2e-docker-arm64-generic.txt");
        let verify: Vec<&str> = text
            .lines()
            .filter(|l| l.starts_with("ll-verify\t"))
            .collect();
        assert!(verify.len() > 10);
        let checks: Vec<Parsed> = verify.iter().filter_map(|l| parse_line(l)).collect();
        // all but the header and the result line
        assert_eq!(checks.len(), verify.len() - 2);
        assert!(checks.iter().all(|c| c.outcome != "FAIL"));
        for id in [
            "P1.env",
            "P2.present",
            "P3.affinity",
            "P4.ran_on",
            "P5.affinity",
            "P6.ran_on",
        ] {
            assert!(checks.iter().any(|c| c.id == id), "{id} missing");
        }
        assert!(
            verify
                .iter()
                .any(|l| l.starts_with("ll-verify\tRESULT\tPASS\t"))
        );
        assert!(verify[0].starts_with("ll-verify\tv1\tcontract=placement\t"));
    }
}
