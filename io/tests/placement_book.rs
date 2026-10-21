//! The snippets `book/src/io/kubernetes.md` includes, compiled and run
//! here: what a pod with `cpu: 1`, with two CPUs, or with shared CPUs
//! configures, and how the loop thread is pinned inside the pod.
#![allow(missing_docs)]

use ergon_io::placement::{self, HELPER_CPUS_ENV};
use ergon_io::{Backend, Config};
use ergon_tune::{CpuSet, Host};

type R<T = ()> = Result<T, Box<dyn std::error::Error>>;

// ANCHOR: pin_loop
/// Pin the loop thread to one of the pod's CPUs, before `Driver::new`.
/// The kubelet gives the pod its CPUs (its cpuset); it never pins threads
/// to them. `Driver::new` then knows the polling CPU, for its host checks
/// and for the helper-thread choice.
fn pin_loop() -> R {
    let (cpus, source) = placement::process_cpus(&Host::system());
    let cpu = cpus
        .iter()
        .next()
        .ok_or_else(|| format!("no CPU to pin to: {source}"))?;
    ergon_io::clock::pin_current_thread(usize::try_from(cpu)?)?;
    Ok(())
}
// ANCHOR_END: pin_loop

// ANCHOR: one_cpu
/// A pod with one CPU (`cpu: 1`). `connect_background` refuses: its dial
/// helper would time-slice the polling thread. With
/// `inline_connect_fallback=true` it connects as `Driver::connect` does
/// (the TLS handshake on the loop, about 0.7 ms of CPU once per connect
/// or reconnect). Never `uring-sqpoll` here: it is refused at
/// `Driver::new`.
fn one_cpu_pod() -> ergon_io::Result<Config> {
    Config::parse("recv-busy,inline_connect_fallback=true")
}
// ANCHOR_END: one_cpu

// ANCHOR: x86_two_cpus
/// x86 with SMT, `cpu: 2` and the kubelet's `full-pcpus-only`: one
/// physical core, its two hyperthreads. The loop takes one; `SQPOLL`, the
/// dial helper and the warning thread take the other (the loop's SMT
/// sibling). Nothing to configure.
fn x86_two_cpu_pod() -> ergon_io::Result<Config> {
    Config::parse("uring-sqpoll")
}
// ANCHOR_END: x86_two_cpus

// ANCHOR: graviton_two_cpus
/// Graviton (no SMT), `cpu: 2`: two cores. The loop takes one; the
/// helpers take the other. Nothing to configure.
fn graviton_two_cpu_pod() -> ergon_io::Result<Config> {
    Config::parse("recv-busy")
}
// ANCHOR_END: graviton_two_cpus

// ANCHOR: helper_cpus
/// Shared CPUs a node-level mechanism added to the pod's cpuset: name
/// them for the helper threads. A CPU outside the process's CPUs is
/// refused (`Error::Cpu`), never used.
fn shared_cpu_pod() -> ergon_io::Result<Config> {
    Config::parse("recv-busy,helper_cpus=3-4")
}
// ANCHOR_END: helper_cpus

// ANCHOR: helper_cpus_env
/// The same from the pod's environment: `ERGON_HELPER_CPUS` is a kernel
/// CPU list (`3`, `3,7`, `2-3`). `Config::helper_cpus` wins when both are
/// set. `Driver::new` reads the variable; the application does not.
const fn helper_cpus_env_name() -> &'static str {
    HELPER_CPUS_ENV
}
// ANCHOR_END: helper_cpus_env

#[test]
fn pin_loop_pins_to_a_cpu_of_this_process() -> R {
    let (cpus, _) = placement::process_cpus(&Host::system());
    match cpus.iter().next() {
        Some(_) => pin_loop()?,
        None => assert!(pin_loop().is_err(), "nowhere to pin: it must say so"),
    }
    Ok(())
}

#[test]
fn each_pod_layout_configures_what_the_book_shows() -> R {
    let one = one_cpu_pod()?;
    assert_eq!(one.backend, Backend::RecvBusy);
    assert!(one.inline_connect_fallback);
    assert_eq!(x86_two_cpu_pod()?.backend, Backend::UringSqpoll);
    assert_eq!(graviton_two_cpu_pod()?.backend, Backend::RecvBusy);
    assert_eq!(shared_cpu_pod()?.helper_cpus, vec![3, 4]);
    assert_eq!(helper_cpus_env_name(), "ERGON_HELPER_CPUS");
    // The variable's value is a kernel CPU list, the one `placement`
    // parses; the book's `3-4` must parse to CPUs 3 and 4.
    assert_eq!(
        CpuSet::parse_list("3-4")?.iter().collect::<Vec<_>>(),
        vec![3, 4]
    );
    Ok(())
}
