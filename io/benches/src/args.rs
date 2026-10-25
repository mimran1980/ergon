//! Tiny `--key value` argument parser shared by the binaries.

use std::collections::HashMap;

use crate::{Error, Result};

/// Parsed arguments: a subcommand followed by `--key value` pairs and `--flag`s.
#[derive(Debug, Default, Clone)]
pub struct Args {
    /// First positional argument.
    pub command: String,
    /// Remaining positional arguments.
    pub positional: Vec<String>,
    map: HashMap<String, String>,
}

impl Args {
    /// Parses `std::env::args()`.
    #[must_use]
    pub fn from_env() -> Self {
        Self::parse(std::env::args().skip(1))
    }

    /// Parses an argument list.
    #[must_use]
    pub fn parse(it: impl IntoIterator<Item = String>) -> Self {
        let mut a = Self::default();
        let mut it = it.into_iter().peekable();
        while let Some(s) = it.next() {
            if let Some(k) = s.strip_prefix("--") {
                let v = match it.peek() {
                    Some(n) if !n.starts_with("--") => it.next().unwrap_or_default(),
                    _ => "true".to_owned(),
                };
                a.map.insert(k.to_owned(), v);
            } else if a.command.is_empty() {
                a.command = s;
            } else {
                a.positional.push(s);
            }
        }
        a
    }

    /// A string value or the default.
    #[must_use]
    pub fn str_or(&self, k: &str, d: &str) -> String {
        self.map.get(k).cloned().unwrap_or_else(|| d.to_owned())
    }

    /// A required string value.
    ///
    /// # Errors
    /// Missing key.
    pub fn req(&self, k: &str) -> Result<String> {
        self.map
            .get(k)
            .cloned()
            .ok_or_else(|| Error::Config(format!("missing --{k}")))
    }

    /// A parsed value or the default.
    ///
    /// # Errors
    /// The value does not parse.
    pub fn num_or<T: std::str::FromStr>(&self, k: &str, d: T) -> Result<T> {
        match self.map.get(k) {
            None => Ok(d),
            Some(v) => v
                .parse()
                .map_err(|_| Error::Config(format!("bad --{k} {v}"))),
        }
    }

    /// A comma-separated list (empty when absent or empty).
    #[must_use]
    pub fn list(&self, k: &str, d: &str) -> Vec<String> {
        let v = self.str_or(k, d);
        v.split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }

    /// Whether a flag is present.
    #[must_use]
    pub fn flag(&self, k: &str) -> bool {
        self.map.get(k).is_some_and(|v| v != "false")
    }
}
