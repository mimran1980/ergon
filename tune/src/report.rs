//! The one switch over every host-check warning: `ERGON_HOST_CHECKS`.
//!
//! The checks always run and their results are always returned. The switch
//! only says how they are reported: `off` drops the warning lines, `on`
//! (the default) prints them, `require` makes a failing check an error at
//! start. Every warning line that is printed ends with how to silence it.
//!
//! `require` is a readiness gate, not a blanket refusal. It refuses on a
//! **warning** (a check fired) and on an **unproven** check whose job is to
//! prove a property of this pod or process
//! ([`CheckId::unknown_stops_require`](crate::CheckId::unknown_stops_require)):
//! starting on an unproven isolation claim is the false pass the check
//! exists to stop. An unknown of a check whose subsystem the node may not
//! have — PSI, a NUMA node, a NIC knob — is printed and allowed through,
//! because refusing there would stop every start on such a host.

use std::fmt;

use crate::finding::Finding;

/// The one switch over every host-check warning.
pub const HOST_CHECKS_ENV: &str = "ERGON_HOST_CHECKS";

/// What every printed warning line ends with.
pub const SILENCE: &str = "(ERGON_HOST_CHECKS=off silences host-check warnings)";

/// What [`HOST_CHECKS_ENV`] is set to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum HostCheckPolicy {
    /// Print no warning line. The results are still returned, and info and
    /// unknown lines are still printed.
    Off,
    /// Print the warning lines (the default).
    #[default]
    On,
    /// Print them, and make a warning an error at start. An **unproven** check
    /// is an error too, but only where the check must prove something: see
    /// [`CheckId::unknown_stops_require`](crate::CheckId::unknown_stops_require).
    Require,
}

/// What a caller does with a host-check policy or its report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HostCheckError {
    /// [`HOST_CHECKS_ENV`] is set to something other than `off`, `on`/`warn`
    /// or `require`.
    Invalid {
        /// The value it is set to.
        value: String,
    },
    /// [`HostCheckPolicy::Require`] and a check warned, or could not prove a
    /// property that check must prove: the start must fail.
    Warnings {
        /// How many findings warned or were left unproven.
        count: usize,
        /// Their check ids, joined with `, `.
        ids: String,
    },
}

impl fmt::Display for HostCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid { value } => write!(
                f,
                "{HOST_CHECKS_ENV}={value}: set {HOST_CHECKS_ENV} to off, on or require (off prints \
                 no host-check warning line; require refuses to start on a warning, or on a check \
                 that could not be proven)"
            ),
            Self::Warnings { count, ids } => write!(
                f,
                "{HOST_CHECKS_ENV}=require and {count} host check(s) warned or could not be \
                 proven ({ids}), printed above: fix them, or set {HOST_CHECKS_ENV}=off to start \
                 anyway"
            ),
        }
    }
}

impl std::error::Error for HostCheckError {}

impl HostCheckPolicy {
    /// `off`, `on` (or `warn`, the same as `on`) or `require`, any case.
    ///
    /// # Errors
    ///
    /// [`HostCheckError::Invalid`] when `value` is none of those.
    pub fn parse(value: &str) -> Result<Self, HostCheckError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Ok(Self::Off),
            "on" | "warn" => Ok(Self::On),
            "require" => Ok(Self::Require),
            _ => Err(HostCheckError::Invalid {
                value: value.to_owned(),
            }),
        }
    }

    /// From [`HOST_CHECKS_ENV`]'s value: [`Self::On`] when it is unset
    /// (`None`), else [`Self::parse`].
    ///
    /// # Errors
    ///
    /// [`HostCheckError::Invalid`] when it is set to none of `off`, `on`,
    /// `warn` or `require`.
    pub fn from_var(value: Option<&str>) -> Result<Self, HostCheckError> {
        value.map_or(Ok(Self::On), Self::parse)
    }

    /// From the process environment: [`HOST_CHECKS_ENV`], [`Self::On`] when
    /// it is unset. No other variable is read.
    ///
    /// # Errors
    ///
    /// [`HostCheckError::Invalid`] when it is set to none of `off`, `on`,
    /// `warn` or `require`, or is not valid Unicode.
    pub fn from_env() -> Result<Self, HostCheckError> {
        match std::env::var(HOST_CHECKS_ENV) {
            Ok(v) => Self::from_var(Some(&v)),
            Err(std::env::VarError::NotPresent) => Self::from_var(None),
            Err(std::env::VarError::NotUnicode(_)) => Err(HostCheckError::Invalid {
                value: "not valid Unicode".to_owned(),
            }),
        }
    }

    /// The warning lines of `findings` under this policy, as the caller
    /// prints them (`<prefix> warning: …`), each ending with [`SILENCE`].
    /// [`Self::Off`] drops them; the findings themselves are untouched.
    #[must_use]
    pub fn warn_lines(&self, prefix: &str, findings: &[Finding]) -> Vec<String> {
        if *self == Self::Off {
            return Vec::new();
        }
        findings
            .iter()
            .filter_map(|f| warn_line(prefix, f))
            .collect()
    }

    /// Every line to print for `findings` under this policy: the info and
    /// unknown lines always, and the warning lines (each ending with
    /// [`SILENCE`]) only when this is not [`Self::Off`]. An ok finding
    /// prints nothing.
    #[must_use]
    pub fn lines(&self, prefix: &str, findings: &[Finding]) -> Vec<String> {
        findings
            .iter()
            .filter(|f| *self != Self::Off || !f.is_warning())
            .filter_map(|f| host_check_line(prefix, f))
            .collect()
    }

    /// Under [`Self::Require`], the error a start must fail with when any
    /// finding is a warning, or is an unknown of a check that must be proven
    /// ([`CheckId::unknown_stops_require`](crate::CheckId::unknown_stops_require)).
    /// An unknown of a check whose subsystem the node may not have is
    /// reported and allowed through: that is a real host, not a gap.
    #[must_use]
    pub fn refusal(&self, findings: &[Finding]) -> Option<HostCheckError> {
        if *self != Self::Require {
            return None;
        }
        let mut ids: Vec<&str> = Vec::new();
        let mut count = 0;
        for f in findings
            .iter()
            .filter(|f| f.is_warning() || (f.is_unknown() && f.id.unknown_stops_require()))
        {
            count += 1;
            let id = f.id.as_str();
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        if count == 0 {
            None
        } else {
            Some(HostCheckError::Warnings {
                count,
                ids: ids.join(", "),
            })
        }
    }
}

/// The one warning line of `f`, if it is a warning: `<prefix> warning: <the
/// finding> (ERGON_HOST_CHECKS=off silences host-check warnings)`.
#[must_use]
pub fn warn_line(prefix: &str, f: &Finding) -> Option<String> {
    if f.is_warning() {
        Some(format!("{prefix} warning: {f} {SILENCE}"))
    } else {
        None
    }
}

/// The one line of `f` as the caller prints it: `<prefix> warning: …` with
/// [`SILENCE`] on a warning, `<prefix> info: …` on an info finding or an
/// unknown, and nothing on an ok.
#[must_use]
pub fn host_check_line(prefix: &str, f: &Finding) -> Option<String> {
    if f.is_warning() {
        warn_line(prefix, f)
    } else if f.is_unknown() || f.is_finding() {
        Some(format!("{prefix} info: {f}"))
    } else {
        None
    }
}
