//! poll_pass: what one poll pass costs over N connected TCP sockets, for
//! raw epoll and for the loops built on it. Linux only.
//!
//! Loops (each over the same N loopback sockets):
//! - `epoll-lt`: level-triggered `epoll_wait(0)`, one read per ready socket;
//! - `epoll-et-drain`: edge-triggered, reads until `EAGAIN` (two reads a
//!   message: mio's idiom);
//! - `epoll-et-short`: edge-triggered, stops at a short read (one read);
//! - `mio`: `Poll::poll` with a zero timeout, reads until `WouldBlock`;
//! - `driver`: ergon-io's `Driver<FrameConn>` on `epoll-spin`.
//!
//! Modes: `idle` (no data: the cost of finding nothing) and `ready` (one
//! 12-byte frame written to socket `pass % N` before each pass, outside the
//! timed region; the pass must read exactly that frame or the run fails).
//!
//! Each pass is timed with the raw counter (rdtsc / cntvct). Output: one row
//! per loop, N and mode with p50 / p99 / p99.9 / max in ns and the passes
//! over the mode's slow mark (1 µs idle, 10 µs ready); with `--out <dir>`
//! every such pass is written to `<dir>/slow-<loop>-<n>-<mode>.csv` (pass
//! index, ns, raw counter at its start) so each max can be placed. The
//! timing buffers are written once before the run, so the loop thread does
//! not fault them in while it is timed. Not a latency verdict: run it pinned (`taskset -c <cpu>`), and
//! compare loops within one run.
//!
//! `poll_pass [--passes P] [--n 1,4,16,64,256] [--out DIR]`

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("poll_pass: Linux only (epoll)");
    std::process::exit(2);
}

#[cfg(target_os = "linux")]
fn main() {
    if let Err(e) = linux::run() {
        eprintln!("poll_pass: {e}");
        std::process::exit(1);
    }
}

#[cfg(target_os = "linux")]
mod linux {
    use std::io::{ErrorKind, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::os::fd::AsRawFd;
    use std::path::PathBuf;
    use std::time::Duration;

    use ergon_io::{
        Backend, Config, ConnId, Driver, FrameConn, FrameHandler, FrameOptions, Handler, RxMeta,
    };
    use io_bench::clock::{rdtsc, ticks_per_us};

    type R<T> = Result<T, Box<dyn std::error::Error>>;

    const LOOPS: [&str; 5] = [
        "epoll-lt",
        "epoll-et-drain",
        "epoll-et-short",
        "mio",
        "driver",
    ];
    /// A frame as `FrameConn` reads it: u32 little-endian length 8, then a u64.
    const FRAME_LEN: usize = 12;

    struct Args {
        passes: usize,
        ns: Vec<usize>,
        out: Option<PathBuf>,
    }

    fn args() -> R<Args> {
        let mut a = Args {
            passes: 100_000,
            ns: vec![1, 4, 16, 64, 256],
            out: None,
        };
        let mut it = std::env::args().skip(1);
        while let Some(k) = it.next() {
            let v = it.next().ok_or(format!("{k} needs a value"))?;
            match k.as_str() {
                "--passes" => a.passes = v.parse()?,
                "--n" => {
                    a.ns = v.split(',').map(str::parse).collect::<Result<_, _>>()?;
                }
                "--out" => a.out = Some(PathBuf::from(v)),
                _ => return Err(format!("unknown argument {k}").into()),
            }
        }
        if a.passes == 0 || a.ns.contains(&0) {
            return Err("--passes and every --n must be at least 1".into());
        }
        Ok(a)
    }

    /// N connected loopback pairs: (the polled, non-blocking end; the writer).
    fn pairs(n: usize) -> R<Vec<(TcpStream, TcpStream)>> {
        let l = TcpListener::bind("127.0.0.1:0")?;
        let addr = l.local_addr()?;
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            let w = TcpStream::connect(addr)?;
            w.set_nodelay(true)?;
            let (r, _) = l.accept()?;
            r.set_nonblocking(true)?;
            out.push((r, w));
        }
        Ok(out)
    }

    fn frame(seq: u64) -> [u8; FRAME_LEN] {
        let mut f = [0u8; FRAME_LEN];
        f[..4].copy_from_slice(&8u32.to_le_bytes());
        f[4..].copy_from_slice(&seq.to_le_bytes());
        f
    }

    /// One loop over the polled ends: `pass` returns the bytes it read.
    trait Pass {
        fn pass(&mut self) -> R<usize>;
    }

    struct Epoll {
        ep: libc::c_int,
        socks: Vec<TcpStream>,
        events: Vec<libc::epoll_event>,
        buf: [u8; 4096],
        edge: bool,
        drain: bool,
    }

    impl Epoll {
        fn new(socks: Vec<TcpStream>, edge: bool, drain: bool) -> R<Self> {
            // SAFETY: plain syscall, checked below.
            let ep = unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) };
            if ep < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            for (i, s) in socks.iter().enumerate() {
                let flags = libc::EPOLLIN as u32 | if edge { libc::EPOLLET as u32 } else { 0 };
                let mut ev = libc::epoll_event {
                    events: flags,
                    u64: i as u64,
                };
                // SAFETY: valid epoll fd, socket fd and event.
                if unsafe { libc::epoll_ctl(ep, libc::EPOLL_CTL_ADD, s.as_raw_fd(), &mut ev) } < 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
            let n = socks.len();
            Ok(Self {
                ep,
                socks,
                events: vec![libc::epoll_event { events: 0, u64: 0 }; n],
                buf: [0; 4096],
                edge,
                drain,
            })
        }
    }

    impl Drop for Epoll {
        fn drop(&mut self) {
            // SAFETY: the fd this struct created.
            unsafe { libc::close(self.ep) };
        }
    }

    impl Pass for Epoll {
        fn pass(&mut self) -> R<usize> {
            let cap = i32::try_from(self.events.len())?;
            // SAFETY: the buffer holds `cap` events.
            let ready = unsafe { libc::epoll_wait(self.ep, self.events.as_mut_ptr(), cap, 0) };
            if ready < 0 {
                return Err(std::io::Error::last_os_error().into());
            }
            let mut got = 0;
            for e in &self.events[..usize::try_from(ready)?] {
                let s = &mut self.socks[usize::try_from(e.u64)?];
                loop {
                    match s.read(&mut self.buf) {
                        Ok(0) => return Err("peer closed".into()),
                        Ok(k) => {
                            got += k;
                            // level-triggered: one read; edge short: stop on a short read
                            if !self.edge || (!self.drain && k < self.buf.len()) {
                                break;
                            }
                        }
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            Ok(got)
        }
    }

    struct Mio {
        poll: mio::Poll,
        events: mio::Events,
        socks: Vec<mio::net::TcpStream>,
        buf: [u8; 4096],
    }

    impl Mio {
        fn new(socks: Vec<TcpStream>) -> R<Self> {
            let poll = mio::Poll::new()?;
            let mut out = Vec::with_capacity(socks.len());
            for (i, s) in socks.into_iter().enumerate() {
                let mut m = mio::net::TcpStream::from_std(s);
                poll.registry()
                    .register(&mut m, mio::Token(i), mio::Interest::READABLE)?;
                out.push(m);
            }
            let n = out.len();
            Ok(Self {
                poll,
                events: mio::Events::with_capacity(n),
                socks: out,
                buf: [0; 4096],
            })
        }
    }

    impl Pass for Mio {
        fn pass(&mut self) -> R<usize> {
            self.poll.poll(&mut self.events, Some(Duration::ZERO))?;
            let mut got = 0;
            for e in &self.events {
                let s = &mut self.socks[e.token().0];
                loop {
                    match s.read(&mut self.buf) {
                        Ok(0) => return Err("peer closed".into()),
                        Ok(k) => got += k,
                        Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                        Err(e) => return Err(e.into()),
                    }
                }
            }
            Ok(got)
        }
    }

    #[derive(Default)]
    struct Count {
        bytes: usize,
        closed: Option<String>,
    }

    impl Handler for Count {
        fn on_closed(&mut self, id: ConnId, error: ergon_io::Error) {
            self.closed = Some(format!("{id:?}: {error}"));
        }
    }

    impl FrameHandler for Count {
        fn on_frame(&mut self, _conn: ConnId, payload: &[u8], _meta: &RxMeta) {
            self.bytes += payload.len() + 4;
        }
    }

    struct Drv {
        d: Driver<FrameConn>,
        h: Count,
    }

    impl Drv {
        fn new(socks: Vec<TcpStream>) -> R<Self> {
            let mut cfg = Config::new(Backend::EpollSpin);
            cfg.rx_timestamps = false;
            cfg.keepalive_idle_s = 0;
            cfg.tcp_info_interval_ms = 0;
            cfg.epoll_events = u32::try_from(socks.len().max(16))?;
            let mut d: Driver<FrameConn> = Driver::new(cfg)?;
            for s in socks {
                d.add(FrameConn::from_stream(s, FrameOptions::default())?)?;
            }
            Ok(Self {
                d,
                h: Count::default(),
            })
        }
    }

    impl Pass for Drv {
        fn pass(&mut self) -> R<usize> {
            self.h.bytes = 0;
            self.d.poll(&mut self.h)?;
            if let Some(c) = self.h.closed.take() {
                return Err(format!("connection closed: {c}").into());
            }
            Ok(self.h.bytes)
        }
    }

    fn build(name: &str, socks: Vec<TcpStream>) -> R<Box<dyn Pass>> {
        Ok(match name {
            "epoll-lt" => Box::new(Epoll::new(socks, false, false)?),
            "epoll-et-drain" => Box::new(Epoll::new(socks, true, true)?),
            "epoll-et-short" => Box::new(Epoll::new(socks, true, false)?),
            "mio" => Box::new(Mio::new(socks)?),
            "driver" => Box::new(Drv::new(socks)?),
            _ => return Err(format!("no loop {name}").into()),
        })
    }

    /// This thread's minor page faults so far (`/proc/thread-self/stat`, field 10).
    fn minflt() -> R<u64> {
        let st = std::fs::read_to_string("/proc/thread-self/stat")?;
        // fields after the command name, which is in parentheses and may hold spaces
        let rest = st.rsplit_once(')').ok_or("bad stat")?.1;
        Ok(rest
            .split_whitespace()
            .nth(7)
            .ok_or("short stat")?
            .parse()?)
    }

    struct Stats {
        p50: f64,
        p99: f64,
        p999: f64,
        max: f64,
        over_mark: usize,
        faults: u64,
    }

    fn stats(ns: &mut [f64], mark: f64) -> Stats {
        let over_mark = ns.iter().filter(|&&v| v > mark).count();
        ns.sort_by(f64::total_cmp);
        let at = |q: f64| {
            // nearest rank
            let i = ((q * ns.len() as f64).ceil() as usize).clamp(1, ns.len()) - 1;
            ns[i]
        };
        Stats {
            p50: at(0.50),
            p99: at(0.99),
            p999: at(0.999),
            max: ns[ns.len() - 1],
            over_mark,
            faults: 0,
        }
    }

    fn one(name: &str, n: usize, ready: bool, a: &Args, tpns: f64) -> R<Stats> {
        let ps = pairs(n)?;
        let (polled, mut writers): (Vec<_>, Vec<_>) = ps.into_iter().unzip();
        let mut lp = build(name, polled)?;
        // correctness first: every socket's frame is read, and an idle pass reads nothing
        for (i, w) in writers.iter_mut().enumerate() {
            w.write_all(&frame(i as u64))?;
            let mut got = 0;
            for _ in 0..1_000_000 {
                got += lp.pass()?;
                if got >= FRAME_LEN {
                    break;
                }
            }
            if got != FRAME_LEN {
                return Err(
                    format!("{name}: socket {i} read {got} bytes, want {FRAME_LEN}").into(),
                );
            }
        }
        if lp.pass()? != 0 {
            return Err(format!("{name}: an idle pass read data").into());
        }
        // warm-up, untimed
        for _ in 0..a.passes / 10 {
            std::hint::black_box(lp.pass()?);
        }
        // written once now: a page first touched inside the timed loop is a
        // fault. A zeroed vec! is a lazily mapped calloc, so fill with a
        // value the compiler cannot fold away.
        let one = std::hint::black_box(1.0f64);
        let mut ns = vec![one; a.passes];
        let mut slow = vec![(1usize, one, 1u64); a.passes];
        std::hint::black_box((&mut ns, &mut slow));
        let mut kept = 0usize;
        let mut slow_n = 0usize;
        let mark = if ready { 10_000.0 } else { 1_000.0 };
        let f0 = minflt()?;
        // POLL_PASS_FAULT_DIAG=1: name every timed pass that took a page fault
        // (two /proc reads around each pass, outside the timed region)
        let diag = std::env::var_os("POLL_PASS_FAULT_DIAG").is_some();
        let mut late = 0u64;
        for p in 0..a.passes {
            if ready {
                writers[p % n].write_all(&frame(p as u64))?;
            }
            let fa = if diag { minflt()? } else { 0 };
            let t0 = rdtsc();
            let got = std::hint::black_box(lp.pass()?);
            let t1 = rdtsc();
            if diag && minflt()? != fa {
                eprintln!("fault {name} n={n} pass {p}");
            }
            let v = t1.wrapping_sub(t0) as f64 / tpns;
            if ready && got != FRAME_LEN {
                // loopback delivery not yet done when the pass ran: drain untimed, count it
                late += 1;
                let mut left = FRAME_LEN - got.min(FRAME_LEN);
                for _ in 0..1_000_000 {
                    if left == 0 {
                        break;
                    }
                    left = left.saturating_sub(lp.pass()?);
                }
                if left != 0 {
                    return Err(format!("{name}: pass {p} lost a frame").into());
                }
                continue;
            }
            if !ready && got != 0 {
                return Err(format!("{name}: idle pass {p} read {got} bytes").into());
            }
            if v > mark {
                slow[slow_n] = (p, v, t0);
                slow_n += 1;
            }
            ns[kept] = v;
            kept += 1;
        }
        let faults = minflt()? - f0;
        if late * 100 > a.passes as u64 {
            return Err(format!(
                "{name} n={n}: {late} of {} ready passes found no frame",
                a.passes
            )
            .into());
        }
        if let Some(dir) = &a.out {
            let mode = if ready { "ready" } else { "idle" };
            let mut f = std::fs::File::create(dir.join(format!("slow-{name}-{n}-{mode}.csv")))?;
            writeln!(f, "pass,ns,counter")?;
            for (p, v, t) in &slow[..slow_n] {
                writeln!(f, "{p},{v:.0},{t}")?;
            }
        }
        let mut st = stats(&mut ns[..kept], mark);
        st.faults = faults;
        Ok(st)
    }

    pub fn run() -> R<()> {
        let a = args()?;
        if let Some(dir) = &a.out {
            std::fs::create_dir_all(dir)?;
        }
        let tpns = ticks_per_us() / 1000.0;
        println!("loop,n,mode,passes,p50_ns,p99_ns,p999_ns,max_ns,over_mark,faults");
        for &n in &a.ns {
            for ready in [false, true] {
                for name in LOOPS {
                    let s = one(name, n, ready, &a, tpns)?;
                    println!(
                        "{name},{n},{},{},{:.0},{:.0},{:.0},{:.0},{},{}",
                        if ready { "ready" } else { "idle" },
                        a.passes,
                        s.p50,
                        s.p99,
                        s.p999,
                        s.max,
                        s.over_mark,
                        s.faults
                    );
                }
            }
        }
        Ok(())
    }
}
