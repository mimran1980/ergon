//! The system calls the benchmarks need beyond ergon-clock's.
//!
//! Pinning a thread, its context switches, and the kernel clocks by id: the
//! bench crate's only `unsafe`.
#![allow(unsafe_code)]

/// Pins the calling thread to `cpu` (Linux `sched_setaffinity`).
///
/// Elsewhere (macOS has no CPU affinity) it does nothing and says so
/// (`Ok(false)`).
///
/// # Errors
/// The kernel refused (a CPU that does not exist, or is not allowed).
#[allow(clippy::missing_const_for_fn)] // const only where the Linux body is compiled out
pub fn pin(cpu: usize) -> std::io::Result<bool> {
    #[cfg(target_os = "linux")]
    {
        if cpu >= 1024 {
            return Err(std::io::Error::from_raw_os_error(libc::EINVAL));
        }
        // SAFETY: a zeroed cpu_set_t is a valid empty set; CPU_SET stays in
        // bounds for cpu < CPU_SETSIZE (1024), checked above.
        let rc = unsafe {
            let mut set: libc::cpu_set_t = std::mem::zeroed();
            libc::CPU_SET(cpu, &mut set);
            libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw const set)
        };
        if rc != 0 {
            return Err(std::io::Error::last_os_error());
        }
        Ok(true)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = cpu;
        Ok(false)
    }
}

/// The CPUs the calling thread may run on (Linux `sched_getaffinity`),
/// ascending; empty elsewhere (macOS has no CPU affinity) or where the
/// kernel refused.
#[must_use]
#[allow(clippy::missing_const_for_fn)] // const only where the Linux body is compiled out
pub fn allowed_cpus() -> Vec<usize> {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: a zeroed cpu_set_t is a valid empty set, and the kernel
        // writes at most its size.
        let mut set: libc::cpu_set_t = unsafe { std::mem::zeroed() };
        // SAFETY: a valid pointer to a cpu_set_t of the size given.
        let rc = unsafe {
            libc::sched_getaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &raw mut set)
        };
        if rc != 0 {
            return Vec::new();
        }
        // SAFETY: CPU_ISSET reads a set the kernel filled, cpu < CPU_SETSIZE.
        (0..1024)
            .filter(|&cpu| unsafe { libc::CPU_ISSET(cpu, &set) })
            .collect()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Vec::new()
    }
}

/// The calling thread's voluntary and involuntary context switches so far.
///
/// Linux `getrusage(RUSAGE_THREAD)`; zeros elsewhere. A measurement's growth
/// in them says whether the thread slept or was preempted in it.
#[must_use]
#[allow(clippy::missing_const_for_fn)] // const only where the Linux body is compiled out
pub fn context_switches() -> (u64, u64) {
    #[cfg(target_os = "linux")]
    {
        // SAFETY: a zeroed rusage is a valid out-parameter.
        let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
        // SAFETY: a valid pointer to a rusage.
        if unsafe { libc::getrusage(libc::RUSAGE_THREAD, &raw mut ru) } == 0 {
            return (
                u64::try_from(ru.ru_nvcsw).unwrap_or(0),
                u64::try_from(ru.ru_nivcsw).unwrap_or(0),
            );
        }
    }
    (0, 0)
}

/// The page faults served without I/O so far.
///
/// `getrusage`'s `ru_minflt`: the calling thread's on Linux
/// (`RUSAGE_THREAD`), the process's elsewhere. A page's first touch is one;
/// a measurement's growth in them says whether it paid for its own memory.
#[must_use]
pub fn minor_faults() -> u64 {
    #[cfg(target_os = "linux")]
    let who = libc::RUSAGE_THREAD;
    #[cfg(not(target_os = "linux"))]
    let who = libc::RUSAGE_SELF;
    // SAFETY: a zeroed rusage is a valid out-parameter.
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: a valid pointer to a rusage.
    if unsafe { libc::getrusage(who, &raw mut ru) } == 0 {
        return u64::try_from(ru.ru_minflt).unwrap_or(0);
    }
    0
}

/// Whether [`minor_faults`] counts only the calling thread.
///
/// Linux `getrusage(RUSAGE_THREAD)` does. Darwin has no `RUSAGE_THREAD`, so
/// there the count is the whole process's: a delta over a measurement cannot
/// be attributed to that measurement, and must not be asserted as if it
/// were. Where this is `false`, use [`pages_resident`] instead — it is
/// scoped to the buffer, so nothing else in the process moves it.
#[must_use]
pub const fn minor_faults_are_thread_local() -> bool {
    cfg!(target_os = "linux")
}

/// The kernel's page size in bytes (0 where it will not say).
#[must_use]
pub fn page_size() -> usize {
    // SAFETY: sysconf is a pure query; _SC_PAGESIZE is a valid name.
    let n = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    usize::try_from(n).unwrap_or(0)
}

/// How many pages lie wholly inside `items`.
///
/// A page counts only where its whole range is a subset of `items`: the
/// first and last partial pages do not count. This is what
/// [`pages_resident`] reports when every one of them is resident, so it is
/// the count a caller should expect after touching the slice — not
/// `size_of_val(items) / page_size`, which is wrong for a slice that is not
/// page-aligned at both ends (a `Vec` is only element-aligned).
#[must_use]
pub fn pages_wholly_inside<T>(items: &[T]) -> usize {
    let page = page_size();
    if page == 0 || items.is_empty() {
        return 0;
    }
    let start = items.as_ptr() as usize;
    let end = start.saturating_add(std::mem::size_of_val(items));
    // The first page starting inside the slice, and the end of the last
    // page ending inside it.
    let lo = start.saturating_add(page - 1) & !(page - 1);
    let hi = end & !(page - 1);
    if hi <= lo {
        return 0;
    }
    (hi - lo) / page
}

/// How many pages of `items` are resident (`mincore`).
///
/// Scoped to the buffer, so unlike [`minor_faults`] off Linux nothing else
/// in the process can move the answer. Counts only pages wholly inside
/// `items` (see [`pages_wholly_inside`]: the first and last partial pages
/// do not count). Returns 0 where the kernel will not answer, so a caller
/// that expects every page fails closed rather than passing on an unknown.
#[must_use]
pub fn pages_resident<T>(items: &[T]) -> usize {
    let page = page_size();
    let n = pages_wholly_inside(items);
    if page == 0 || n == 0 {
        return 0;
    }
    let start = items.as_ptr() as usize;
    let lo = start.saturating_add(page - 1) & !(page - 1);
    let mut vec = vec![0_u8; n];
    #[cfg(target_os = "linux")]
    let vec_ptr = vec.as_mut_ptr().cast::<libc::c_uchar>();
    #[cfg(not(target_os = "linux"))]
    let vec_ptr = vec.as_mut_ptr().cast::<libc::c_char>();
    // SAFETY: lo..hi is n whole pages inside the slice's mapped bytes, and
    // vec holds one byte per page for the kernel to fill.
    let rc = unsafe { libc::mincore(lo as *mut libc::c_void, n * page, vec_ptr) };
    if rc != 0 {
        return 0;
    }
    vec.iter().filter(|&&b| b & 1 == 1).count()
}

/// `clock_gettime(id)` in ns (0 on failure: the rows that read it are
/// checked for a sane rate first).
#[inline]
#[must_use]
pub fn clock_ns(id: libc::clockid_t) -> u64 {
    let mut ts = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: a valid, writable timespec.
    unsafe { libc::clock_gettime(id, &raw mut ts) };
    u64::try_from(ts.tv_sec)
        .unwrap_or(0)
        .wrapping_mul(1_000_000_000)
        .wrapping_add(u64::try_from(ts.tv_nsec).unwrap_or(0))
}

/// The kernel's release (`uname -r`).
#[must_use]
pub fn kernel() -> String {
    // SAFETY: a zeroed utsname is a valid out-parameter.
    let mut u: libc::utsname = unsafe { std::mem::zeroed() };
    // SAFETY: a valid pointer to a utsname.
    if unsafe { libc::uname(&raw mut u) } != 0 {
        return "-".to_owned();
    }
    let release: Vec<u8> = u
        .release
        .iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c.to_ne_bytes()[0])
        .collect();
    String::from_utf8_lossy(&release).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `mincore` counts exactly the pages a slice has resident, so a cold
    /// page in a sample buffer is visible before a timed pass runs over it.
    /// The region is a fresh anonymous mapping, which the kernel leaves
    /// zero-fill-on-demand: a heap buffer would do as well once touched, but
    /// the allocator may hand back memory that is already resident, so it
    /// cannot show the cold end of the count.
    #[test]
    fn pages_resident_counts_the_pages_actually_touched() {
        let page = page_size();
        assert!(page > 0, "no page size");
        let pages = 64_usize;
        let len = pages * page;
        // SAFETY: a fresh anonymous mapping of len bytes, viewed and
        // unmapped again below.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap of {len} bytes failed");
        // SAFETY: base is our mapping of exactly len bytes.
        let slice = unsafe { std::slice::from_raw_parts_mut(base.cast::<u8>(), len) };
        assert_eq!(pages_resident(slice), 0, "a fresh mapping must be cold");
        for b in &mut slice[..len / 2] {
            *b = 1;
        }
        assert_eq!(pages_resident(slice), pages / 2);
        for b in &mut slice[len / 2..] {
            *b = 1;
        }
        assert_eq!(pages_resident(slice), pages);
        // SAFETY: our mapping, its exact length.
        assert_eq!(unsafe { libc::munmap(base, len) }, 0);
    }

    /// A fully-touched slice that is not page-aligned reports the pages
    /// wholly inside it, not `length / page_size`. A mid-page offset into a
    /// three-page mapping, two pages long, has one page wholly inside: the
    /// review's `expected 2, reported 1`.
    #[test]
    fn pages_resident_counts_wholly_inside_pages_of_an_offset_slice() {
        let page = page_size();
        assert!(page > 0, "no page size");
        let len = 3 * page;
        // SAFETY: a fresh anonymous mapping of len bytes, viewed and
        // unmapped again below.
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                len,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANON,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap of {len} bytes failed");
        // SAFETY: base is our mapping of exactly len bytes.
        let region = unsafe { std::slice::from_raw_parts_mut(base.cast::<u8>(), len) };
        let offset = page / 2;
        let slice = &mut region[offset..offset + 2 * page];
        for b in slice.iter_mut() {
            *b = 1;
        }
        // Pages wholly inside: the middle page only. The first and last are
        // half in the slice and do not count.
        assert_eq!(pages_wholly_inside(slice), 1);
        assert_eq!(pages_resident(slice), 1);
        // The naive length count is the wrong expectation for this slice.
        assert_eq!(slice.len() / page, 2);
        // SAFETY: our mapping, its exact length.
        assert_eq!(unsafe { libc::munmap(base, len) }, 0);
    }
}
