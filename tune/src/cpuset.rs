//! CPU sets in the kernel's two text forms: lists (`0-3,8`, as in
//! `isolated` or `smp_affinity_list`) and hexadecimal masks (`ff,00000000`,
//! as in `default_smp_affinity`).

use std::fmt::{self, Write as _};

/// The highest CPU number accepted, exclusive.
///
/// The kernel's `NR_CPUS` is at most 8,192, so anything larger is malformed
/// input, not a CPU. Bounds the bitmap a hostile or corrupt file could make
/// this allocate.
pub const CPU_LIMIT: u32 = 1 << 16;

/// A set of CPU numbers. Its bitmap never ends in an empty word, so the
/// derived equality is set equality.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct CpuSet {
    words: Vec<u64>,
}

/// Why a CPU list or mask did not parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CpuSetError {
    /// Not a number, a range or a hexadecimal group where one was expected.
    Syntax,
    /// A range whose end is below its start.
    Reversed,
    /// A CPU number at or above [`CPU_LIMIT`].
    TooLarge,
}

impl fmt::Display for CpuSetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Syntax => "not a CPU list or mask",
            Self::Reversed => "a CPU range that ends before it starts",
            Self::TooLarge => "a CPU number beyond any kernel's limit",
        })
    }
}

impl std::error::Error for CpuSetError {}

impl CpuSet {
    /// The empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self { words: Vec::new() }
    }

    /// The set of `cpus`; numbers at or above [`CPU_LIMIT`] are left out.
    #[must_use]
    pub fn of(cpus: impl IntoIterator<Item = u32>) -> Self {
        let mut s = Self::new();
        for c in cpus {
            s.insert(c);
        }
        s
    }

    /// Parses a CPU list: comma-separated numbers and inclusive ranges
    /// (`0-3,8,10-11`). Empty text, and `(null)` (older kernels' empty
    /// `nohz_full`), are the empty set. Surrounding whitespace is ignored.
    ///
    /// # Errors
    /// [`CpuSetError`] for anything else.
    pub fn parse_list(text: &str) -> Result<Self, CpuSetError> {
        let text = text.trim();
        let mut s = Self::new();
        if text.is_empty() || text == "(null)" {
            return Ok(s);
        }
        for part in text.split(',') {
            let (lo, hi) = if let Some((a, b)) = part.split_once('-') {
                (cpu_number(a)?, cpu_number(b)?)
            } else {
                let c = cpu_number(part)?;
                (c, c)
            };
            if hi < lo {
                return Err(CpuSetError::Reversed);
            }
            for c in lo..=hi {
                s.insert(c);
            }
        }
        Ok(s)
    }

    /// Parses a hexadecimal CPU mask: groups of at most eight hex digits
    /// separated by commas, the most significant first (`ff,00000000` is
    /// CPUs 32 to 39).
    ///
    /// # Errors
    /// [`CpuSetError`] for anything else.
    pub fn parse_mask(text: &str) -> Result<Self, CpuSetError> {
        let text = text.trim();
        if text.is_empty() {
            return Err(CpuSetError::Syntax);
        }
        let groups: Vec<&str> = text.split(',').collect();
        let max_groups = usize::try_from(CPU_LIMIT / 32).unwrap_or(usize::MAX);
        if groups.len() > max_groups {
            return Err(CpuSetError::TooLarge);
        }
        let mut s = Self::new();
        for (i, g) in groups.iter().rev().enumerate() {
            if g.is_empty() || g.len() > 8 || !g.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err(CpuSetError::Syntax);
            }
            let bits = u32::from_str_radix(g, 16).map_err(|_| CpuSetError::Syntax)?;
            let base = u32::try_from(i).map_err(|_| CpuSetError::TooLarge)? * 32;
            for b in 0..32 {
                if bits & (1 << b) != 0 {
                    s.insert(base + b);
                }
            }
        }
        Ok(s)
    }

    /// Adds `cpu`; a number at or above [`CPU_LIMIT`] is ignored.
    pub fn insert(&mut self, cpu: u32) {
        if cpu >= CPU_LIMIT {
            return;
        }
        let (w, b) = split(cpu);
        if self.words.len() <= w {
            self.words.resize(w + 1, 0);
        }
        self.words[w] |= 1 << b;
    }

    /// Whether `cpu` is in the set.
    #[must_use]
    pub fn contains(&self, cpu: u32) -> bool {
        let (w, b) = split(cpu);
        self.words.get(w).is_some_and(|word| word & (1 << b) != 0)
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.words.iter().all(|w| *w == 0)
    }

    /// The number of CPUs in the set.
    #[must_use]
    pub fn len(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }

    /// The CPUs in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = u32> + '_ {
        self.words.iter().enumerate().flat_map(|(i, w)| {
            let w = *w;
            (0..64u32).filter_map(move |b| {
                if w & (1 << b) == 0 {
                    return None;
                }
                let i = u32::try_from(i).ok()?;
                Some(i * 64 + b)
            })
        })
    }

    /// The CPUs in either set.
    #[must_use]
    pub fn union(&self, other: &Self) -> Self {
        let n = self.words.len().max(other.words.len());
        Self::trimmed((0..n).map(|i| self.word(i) | other.word(i)).collect())
    }

    /// The CPUs in both sets.
    #[must_use]
    pub fn intersection(&self, other: &Self) -> Self {
        let n = self.words.len().min(other.words.len());
        Self::trimmed((0..n).map(|i| self.word(i) & other.word(i)).collect())
    }

    /// The CPUs of this set that are not in `other`.
    #[must_use]
    pub fn difference(&self, other: &Self) -> Self {
        Self::trimmed(
            (0..self.words.len())
                .map(|i| self.word(i) & !other.word(i))
                .collect(),
        )
    }

    /// The set as a hexadecimal mask in the kernel's form (`3`,
    /// `ff,00000000`), as `/proc/irq/default_smp_affinity` takes it. The
    /// empty set is `0`.
    #[must_use]
    pub fn to_mask(&self) -> String {
        let mut groups: Vec<u32> = Vec::new();
        for w in &self.words {
            // Two 32-bit groups per word, low first.
            #[allow(clippy::cast_possible_truncation)] // the low 32 bits, wanted
            groups.push(*w as u32);
            groups.push(u32::try_from(w >> 32).unwrap_or(u32::MAX));
        }
        while groups.len() > 1 && groups.last() == Some(&0) {
            groups.pop();
        }
        let mut out = String::new();
        for (i, g) in groups.iter().rev().enumerate() {
            // Writing to a String cannot fail.
            let _ = if i == 0 {
                write!(out, "{g:x}")
            } else {
                write!(out, ",{g:08x}")
            };
        }
        if out.is_empty() {
            out.push('0');
        }
        out
    }

    /// Without trailing empty words, so equal sets compare equal.
    fn trimmed(mut words: Vec<u64>) -> Self {
        while words.last() == Some(&0) {
            words.pop();
        }
        Self { words }
    }

    fn word(&self, i: usize) -> u64 {
        self.words.get(i).copied().unwrap_or(0)
    }
}

/// The list form (`0-3,8`); the empty set is empty text.
impl fmt::Display for CpuSet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut run: Option<(u32, u32)> = None;
        let mut flush = |f: &mut fmt::Formatter<'_>, (lo, hi): (u32, u32)| -> fmt::Result {
            if !first {
                f.write_str(",")?;
            }
            first = false;
            if lo == hi {
                write!(f, "{lo}")
            } else {
                write!(f, "{lo}-{hi}")
            }
        };
        for c in self.iter() {
            run = match run {
                Some((lo, hi)) if hi + 1 == c => Some((lo, c)),
                Some(r) => {
                    flush(f, r)?;
                    Some((c, c))
                }
                None => Some((c, c)),
            };
        }
        if let Some(r) = run {
            flush(f, r)?;
        }
        Ok(())
    }
}

impl FromIterator<u32> for CpuSet {
    fn from_iter<I: IntoIterator<Item = u32>>(iter: I) -> Self {
        Self::of(iter)
    }
}

const fn split(cpu: u32) -> (usize, u32) {
    ((cpu / 64) as usize, cpu % 64)
}

fn cpu_number(s: &str) -> Result<u32, CpuSetError> {
    let s = s.trim();
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(CpuSetError::Syntax);
    }
    let n: u64 = s.parse().map_err(|_| CpuSetError::TooLarge)?;
    u32::try_from(n)
        .ok()
        .filter(|c| *c < CPU_LIMIT)
        .ok_or(CpuSetError::TooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_parse_and_print_in_the_kernels_form() -> Result<(), CpuSetError> {
        let s = CpuSet::parse_list("0-3,8,10-11\n")?;
        assert_eq!(s.iter().collect::<Vec<_>>(), [0, 1, 2, 3, 8, 10, 11]);
        assert_eq!(s.to_string(), "0-3,8,10-11");
        assert_eq!(s.len(), 7);
        assert!(CpuSet::parse_list("\n")?.is_empty());
        assert!(CpuSet::parse_list("(null)\n")?.is_empty());
        assert_eq!(CpuSet::parse_list("70")?.to_string(), "70");
        Ok(())
    }

    #[test]
    fn malformed_lists_are_refused() {
        assert_eq!(CpuSet::parse_list("a"), Err(CpuSetError::Syntax));
        assert_eq!(CpuSet::parse_list("1,,2"), Err(CpuSetError::Syntax));
        assert_eq!(CpuSet::parse_list("3-1"), Err(CpuSetError::Reversed));
        assert_eq!(CpuSet::parse_list("-1"), Err(CpuSetError::Syntax));
        assert_eq!(
            CpuSet::parse_list("0-4294967295"),
            Err(CpuSetError::TooLarge)
        );
        assert_eq!(
            CpuSet::parse_list("99999999999999999999"),
            Err(CpuSetError::TooLarge)
        );
    }

    #[test]
    fn masks_parse_most_significant_group_first() -> Result<(), CpuSetError> {
        assert_eq!(CpuSet::parse_mask("3\n")?.to_string(), "0-1");
        assert_eq!(CpuSet::parse_mask("ff,00000000")?.to_string(), "32-39");
        assert_eq!(CpuSet::parse_mask("00000001,00000001")?.to_string(), "0,32");
        assert_eq!(CpuSet::parse_mask("0")?.to_string(), "");
        assert_eq!(CpuSet::parse_mask("xyz"), Err(CpuSetError::Syntax));
        assert_eq!(CpuSet::parse_mask("123456789"), Err(CpuSetError::Syntax));
        assert_eq!(CpuSet::parse_mask(""), Err(CpuSetError::Syntax));
        Ok(())
    }

    #[test]
    fn masks_print_as_the_kernel_takes_them() -> Result<(), CpuSetError> {
        assert_eq!(CpuSet::of([0, 1]).to_mask(), "3");
        assert_eq!(CpuSet::of([32]).to_mask(), "1,00000000");
        assert_eq!(CpuSet::of([64]).to_mask(), "1,00000000,00000000");
        assert_eq!(CpuSet::new().to_mask(), "0");
        let s = CpuSet::parse_list("0-1,40,70-71")?;
        assert_eq!(CpuSet::parse_mask(&s.to_mask())?, s);
        Ok(())
    }

    #[test]
    fn set_operations() -> Result<(), CpuSetError> {
        let a = CpuSet::parse_list("0-5")?;
        let b = CpuSet::parse_list("4-7,100")?;
        assert_eq!(a.union(&b).to_string(), "0-7,100");
        assert_eq!(a.intersection(&b).to_string(), "4-5");
        assert_eq!(a.difference(&b).to_string(), "0-3");
        assert!(b.contains(100) && !b.contains(99) && !b.contains(1_000_000));
        let mut c = CpuSet::new();
        c.insert(CPU_LIMIT);
        assert!(c.is_empty());
        Ok(())
    }
}
