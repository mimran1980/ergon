//! `/proc/interrupts` and `/proc/softirqs`: one CPU's column of each, and the
//! counters that grew between two reads.

use std::fmt;

/// A table that is not the kernel's per-CPU counter format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// no `CPU0 CPU1 ...` header line
    NoHeader,
    /// the header has no column for this CPU (offline, or out of range)
    NoColumn(usize),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHeader => write!(f, "no CPU header line"),
            Self::NoColumn(cpu) => write!(f, "no column for CPU{cpu}"),
        }
    }
}

impl std::error::Error for ParseError {}

/// One CPU's counters from a `/proc/interrupts` or `/proc/softirqs` text,
/// in file order. A row is named by its label and, for interrupts, the
/// description after the counts (`11 GICv3 27 Level arch_timer`, `IPI0
/// Rescheduling interrupts`); `prefix` goes before every name
/// (`softirq:`). Rows with no count for this CPU (`ERR`, `MIS`) are left out.
///
/// # Errors
/// No header, or no column for `cpu`.
pub fn column(text: &str, cpu: usize, prefix: &str) -> Result<Vec<(String, u64)>, ParseError> {
    let mut lines = text.lines();
    let header = lines.next().ok_or(ParseError::NoHeader)?;
    let cpus: Vec<&str> = header.split_whitespace().collect();
    if cpus.is_empty() || !cpus.iter().all(|c| c.starts_with("CPU")) {
        return Err(ParseError::NoHeader);
    }
    let want = format!("CPU{cpu}");
    let at = cpus
        .iter()
        .position(|c| *c == want)
        .ok_or(ParseError::NoColumn(cpu))?;
    let mut out = Vec::new();
    for line in lines {
        let Some((label, rest)) = line.split_once(':') else {
            continue;
        };
        let label = label.trim();
        if label.is_empty() {
            continue;
        }
        let fields: Vec<&str> = rest.split_whitespace().collect();
        // the counts: the leading numeric fields, one per header column
        let counts = fields
            .iter()
            .take(cpus.len())
            .take_while(|f| f.parse::<u64>().is_ok())
            .count();
        if counts < cpus.len() {
            continue;
        }
        let Some(value) = fields.get(at).and_then(|f| f.parse::<u64>().ok()) else {
            continue;
        };
        let desc = fields
            .get(counts..)
            .map(|d| d.join(" "))
            .unwrap_or_default();
        let name = if desc.is_empty() {
            format!("{prefix}{label}")
        } else {
            format!("{prefix}{label} {desc}")
        };
        out.push((name, value));
    }
    Ok(out)
}

/// The counters that grew from `before` to `after`, by how much, in
/// `after`'s order. A counter new in `after` grew from 0; one that went
/// down (a hot-unplug) is left out.
#[must_use]
pub fn deltas(before: &[(String, u64)], after: &[(String, u64)]) -> Vec<(String, u64)> {
    let old: std::collections::HashMap<&str, u64> =
        before.iter().map(|(n, v)| (n.as_str(), *v)).collect();
    after
        .iter()
        .filter_map(|(n, v)| {
            let was = old.get(n.as_str()).copied().unwrap_or(0);
            (*v > was).then(|| (n.clone(), v - was))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ARM64: &str = "           CPU0       CPU1       CPU2       CPU3       
 10:      51232      12034     900001         17     GICv3  27 Level     arch_timer
 12:          0          0          0          0     GICv3  33 Level     uart-pl011
 24:        731          0          3          0   ITS-PCI-MSIX-0000:00:00.0   0 Edge      mana_hwc@pci:7870:00:00.0
IPI0:       120        340         55          9       Rescheduling interrupts
IPI1:      4000       3000          1          2       Function call interrupts
Err:          0
";

    const SOFTIRQS: &str = "                    CPU0       CPU1       CPU2       CPU3
          HI:          0          0          0          0
       TIMER:      10442       2209         12          1
      NET_RX:        100          5          0          0
         RCU:       5000       4000          3          0
";

    #[test]
    fn interrupts_column_by_label_and_description() {
        let c = column(ARM64, 2, "").unwrap_or_default();
        assert_eq!(
            c,
            vec![
                ("10 GICv3 27 Level arch_timer".to_string(), 900_001),
                ("12 GICv3 33 Level uart-pl011".to_string(), 0),
                (
                    "24 ITS-PCI-MSIX-0000:00:00.0 0 Edge mana_hwc@pci:7870:00:00.0".to_string(),
                    3
                ),
                ("IPI0 Rescheduling interrupts".to_string(), 55),
                ("IPI1 Function call interrupts".to_string(), 1),
            ]
        );
    }

    #[test]
    fn softirqs_column_with_prefix() {
        let c = column(SOFTIRQS, 1, "softirq:").unwrap_or_default();
        assert_eq!(
            c,
            vec![
                ("softirq:HI".to_string(), 0),
                ("softirq:TIMER".to_string(), 2209),
                ("softirq:NET_RX".to_string(), 5),
                ("softirq:RCU".to_string(), 4000),
            ]
        );
    }

    #[test]
    fn missing_header_or_column_is_an_error() {
        assert_eq!(column("", 0, ""), Err(ParseError::NoHeader));
        assert_eq!(column(" 10: 1 2\n", 0, ""), Err(ParseError::NoHeader));
        assert_eq!(column(SOFTIRQS, 4, ""), Err(ParseError::NoColumn(4)));
    }

    #[test]
    fn header_columns_name_the_cpu_not_the_position() {
        // CPU1 offline: the second column is CPU2
        let text = "  CPU0 CPU2\n 5: 7 9 x y\n";
        assert_eq!(
            column(text, 2, "").unwrap_or_default(),
            vec![("5 x y".to_string(), 9)]
        );
        assert_eq!(column(text, 1, ""), Err(ParseError::NoColumn(1)));
    }

    #[test]
    fn deltas_only_what_grew() {
        let before = vec![
            ("a".to_string(), 10),
            ("b".to_string(), 5),
            ("c".to_string(), 9),
        ];
        let after = vec![
            ("a".to_string(), 10),
            ("b".to_string(), 8),
            ("c".to_string(), 1),
            ("d".to_string(), 2),
        ];
        assert_eq!(
            deltas(&before, &after),
            vec![("b".to_string(), 3), ("d".to_string(), 2)]
        );
    }
}
