//! Which threads write each transport counter kind, derived from the native
//! sources: every `rusteron_dpdk_counters_{add,max,min_counted,burst}` call,
//! the function it sits in, and every thread that can reach that function
//! through the native call graph. A kind only the agents write has one
//! writer, the agent that polls and sends on its port (each port's poller and
//! TX queue are one agent's: DEDICATED's sender or receiver, the one network
//! agent otherwise), so its add is a relaxed load and a release store; a kind
//! another thread writes too keeps the atomic read-modify-write. The header's
//! list of shared kinds (`rusteron_dpdk_counter_shared`) must be exactly what
//! the sources say, and a max or a min (a load and a store, never atomic) is
//! never a shared kind.
//!
//! Threads: the bindings' agent callbacks (`send_func`, `poller_poll_func`,
//! `recvmmsg_func`); the conductor's (every other binding); each thread the
//! runtime starts (`pthread_create`) and each EAL callback it registers
//! (`rte_eth_dev_callback_register`, the EAL's interrupt thread); and every
//! exported function nothing native calls (the Rust side, from whichever
//! thread runs it). The start-time gateway check is apart: it refuses once a
//! transport is open, and the counters exist only while one is, so it never
//! writes beside an agent. So are the test hooks (`*_test_*`), which only the
//! single-threaded tests call, never the library.
//!
//! Source only: runs on every platform.
#![allow(missing_docs, clippy::all, clippy::pedantic, clippy::nursery, clippy::restriction)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::Path;

const NATIVE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/native/dpdk");

/// The bindings' callbacks that run on an agent.
const AGENT_BINDINGS: [&str; 3] = ["send_func", "poller_poll_func", "recvmmsg_func"];
/// Refuses to run once a transport is open (see the module's documentation).
const BEFORE_ANY_TRANSPORT: &str = "rusteron_dpdk_transport_gateway_check";
const BUMPS: [&str; 4] = [
    "rusteron_dpdk_counters_add",
    "rusteron_dpdk_counters_max",
    "rusteron_dpdk_counters_min_counted",
    "rusteron_dpdk_counters_burst",
];

/// The text with comments and string and character literals blanked (their
/// length kept, newlines too), so braces and parentheses in them do not count.
fn strip(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            let end = src[i + 2..].find("*/").map_or(b.len(), |e| i + 2 + e + 2);
            out.extend(b[i..end].iter().map(|&c| if c == b'\n' { b'\n' } else { b' ' }));
            i = end;
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                out.push(b' ');
                i += 1;
            }
        } else if b[i] == b'"' || b[i] == b'\'' {
            let quote = b[i];
            out.push(b' ');
            i += 1;
            while i < b.len() && b[i] != quote {
                if b[i] == b'\\' {
                    out.push(b' ');
                    i += 1;
                }
                out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
            out.push(b' ');
            i += 1;
        } else if b[i] == b'#' && (i == 0 || b[i - 1] == b'\n') {
            // a preprocessor line (and its continuations)
            while i < b.len() && !(b[i] == b'\n' && b[i - 1] != b'\\') {
                out.push(if b[i] == b'\n' { b'\n' } else { b' ' });
                i += 1;
            }
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap()
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_'
}

/// Every identifier in `text` followed by `(`: the calls (and some casts and
/// keywords, which match no function).
fn calls(text: &str) -> Vec<(usize, String)> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if is_ident(b[i]) && (i == 0 || !is_ident(b[i - 1])) {
            let start = i;
            while i < b.len() && is_ident(b[i]) {
                i += 1;
            }
            let mut j = i;
            while j < b.len() && b[j].is_ascii_whitespace() {
                j += 1;
            }
            if b.get(j) == Some(&b'(') && !b[start].is_ascii_digit() {
                out.push((j, text[start..i].to_string()));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The arguments of the call whose `(` is at `open`, split on its top-level commas.
fn arguments(text: &str, open: usize) -> Vec<String> {
    let b = text.as_bytes();
    let (mut depth, mut i, mut start, mut args) = (0, open, open + 1, Vec::new());
    while i < b.len() {
        match b[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    args.push(text[start..i].to_string());
                    return args;
                }
            }
            b',' if depth == 1 => {
                args.push(text[start..i].to_string());
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    panic!("unbalanced call at {open}")
}

fn kinds_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("RD_COUNTER_") {
        let tail = &rest[at..];
        let end = tail.bytes().position(|c| !is_ident(c)).unwrap_or(tail.len());
        out.push(tail[..end].to_string());
        rest = &tail[end..];
    }
    out
}

/// A function defined at file scope: its name, whether it is static, its body.
struct Function {
    name: String,
    exported: bool,
    body: String,
}

/// Every function defined in `text` (stripped): at file scope, a `{` whose
/// signature since the last `;` or `}` holds a `(`.
fn functions(text: &str) -> Vec<Function> {
    let b = text.as_bytes();
    let (mut out, mut depth, mut sig_start, mut body_start, mut current) = (Vec::new(), 0, 0, 0, None);
    // `extern "C" {` (its string blanked) holds file-scope definitions
    let mut linkage = 0;
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'{' if depth == 0 && text[sig_start..i].trim_end().ends_with("extern") => {
                linkage += 1;
                sig_start = i + 1;
            }
            b'}' if depth == 0 && linkage > 0 => {
                linkage -= 1;
                sig_start = i + 1;
            }
            b'{' => {
                if depth == 0 {
                    let sig = &text[sig_start..i];
                    current = if sig.contains('=') { None } else { signature(sig) };
                    body_start = i;
                }
                depth += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    if let Some((name, exported)) = current.take() {
                        out.push(Function {
                            name,
                            exported,
                            body: text[body_start..=i].to_string(),
                        });
                    }
                    sig_start = i + 1;
                }
            }
            b';' if depth == 0 => sig_start = i + 1,
            _ => {}
        }
    }
    out
}

/// A definition's name and whether it is exported, from its signature: the
/// identifier before its last parenthesised group that is not an
/// `__attribute__((...))`; `None` for anything without one.
fn signature(sig: &str) -> Option<(String, bool)> {
    let b = sig.as_bytes();
    let mut end = sig.len();
    loop {
        let close = sig[..end].rfind(')')?;
        let mut depth = 0;
        let mut open = close;
        loop {
            match b[open] {
                b')' => depth += 1,
                b'(' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            open = open.checked_sub(1)?;
        }
        let head = sig[..open].trim_end();
        let name_start = head.bytes().rposition(|c| !is_ident(c)).map_or(0, |p| p + 1);
        let name = &head[name_start..];
        if name.is_empty() || name == "__attribute__" {
            end = open;
            continue;
        }
        let words: Vec<&str> = head.split_whitespace().collect();
        return Some((name.to_string(), !words.contains(&"static")));
    }
}

struct Sources {
    /// name -> the functions it calls (by name, every definition of it merged)
    graph: BTreeMap<String, BTreeSet<String>>,
    /// (function, kind, the bump that writes it)
    bumps: Vec<(String, String, String)>,
    exported: BTreeSet<String>,
    /// every stripped source, joined
    all: String,
    enum_order: Vec<String>,
}

fn read_sources() -> Sources {
    let mut texts = Vec::new();
    for entry in std::fs::read_dir(NATIVE).unwrap() {
        let path = entry.unwrap().path();
        if matches!(path.extension().and_then(|e| e.to_str()), Some("c" | "h")) {
            texts.push(strip(&std::fs::read_to_string(&path).unwrap()));
        }
    }
    let all = texts.join("\n");
    let mut graph: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut bumps = Vec::new();
    let mut exported = BTreeSet::new();
    let mut defined = Vec::new();
    for text in &texts {
        for f in functions(text) {
            if f.exported {
                exported.insert(f.name.clone());
            }
            defined.push(f);
        }
    }
    let names: BTreeSet<String> = defined.iter().map(|f| f.name.clone()).collect();
    for f in &defined {
        let callees = graph.entry(f.name.clone()).or_default();
        for (open, callee) in calls(&f.body) {
            if names.contains(&callee) && callee != f.name {
                callees.insert(callee.clone());
            }
            let Some(&bump) = BUMPS.iter().find(|b| **b == callee) else {
                continue;
            };
            if BUMPS.contains(&f.name.as_str()) {
                continue; // the helpers themselves, on their kind parameter
            }
            let args = arguments(&f.body, open);
            // min_counted(c, kind, v, count): `count` is only read
            let written = if bump.ends_with("min_counted") {
                &args[1]
            } else {
                &args[1..].join(",")
            };
            let kinds = kinds_in(written);
            assert!(
                !kinds.is_empty(),
                "{}: a {bump} whose kind is not an RD_COUNTER_* constant: it cannot be attributed",
                f.name
            );
            for kind in kinds {
                bumps.push((f.name.clone(), kind, bump.to_string()));
            }
        }
    }
    // the kinds' order (burst writes the two after its max kind)
    let header = strip(&std::fs::read_to_string(Path::new(NATIVE).join("rusteron_dpdk_counters.h")).unwrap());
    let body = &header[header.find("RD_COUNTER_PORT_INFO").unwrap()..header.find("RD_COUNTER_COUNT").unwrap()];
    let enum_order = body
        .split(',')
        .map(|e| e.split('=').next().unwrap().trim().to_string())
        .filter(|e| e.starts_with("RD_COUNTER_"))
        .collect();
    Sources {
        graph,
        bumps,
        exported,
        all,
        enum_order,
    }
}

/// `binding` -> the function the bindings table installs for it.
fn bindings(s: &Sources) -> BTreeMap<String, String> {
    let at = s
        .all
        .find("rusteron_dpdk_transport_bindings_instance")
        .expect("the bindings table");
    let at = at + s.all[at..].find('{').unwrap() + 1;
    let table = &s.all[at..at + s.all[at..].find("};").unwrap()];
    table
        .split(',')
        .filter_map(|e| {
            let (k, v) = e.split_once('=')?;
            let k = k.trim().strip_prefix('.')?;
            k.ends_with("_func").then(|| (k.to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Every thread or caller that starts native code, by role, and its roots.
fn roots(s: &Sources) -> BTreeMap<String, BTreeSet<String>> {
    let mut roles: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let table = bindings(s);
    assert!(table.len() >= 12, "the bindings table not read: {table:?}");
    for (binding, function) in &table {
        let role = if AGENT_BINDINGS.contains(&binding.as_str()) {
            "agent"
        } else {
            "conductor"
        };
        roles.entry(role.into()).or_default().insert(function.clone());
    }
    for (open, callee) in calls(&s.all) {
        let thread_arg = match callee.as_str() {
            "pthread_create" => 2,
            "rte_eth_dev_callback_register" => 2,
            "rte_eal_alarm_set" => 1,
            _ => continue,
        };
        let args = arguments(&s.all, open);
        let entry = args[thread_arg].trim().to_string();
        assert!(
            s.graph.contains_key(&entry),
            "{callee}'s thread entry {entry} is not a native function"
        );
        roles.entry(format!("thread {entry}")).or_default().insert(entry);
    }
    // exported and called by nothing native: the Rust side's
    let called: BTreeSet<&String> = s.graph.values().flatten().collect();
    let started: BTreeSet<String> = roles.values().flatten().cloned().collect();
    for f in &s.exported {
        if !called.contains(f) && !started.contains(f) && f != BEFORE_ANY_TRANSPORT && !f.contains("_test_") {
            roles.entry(format!("external {f}")).or_default().insert(f.clone());
        }
    }
    roles
        .entry("before any transport".into())
        .or_default()
        .insert(BEFORE_ANY_TRANSPORT.into());
    roles
}

/// Kind -> the roles that write it.
fn writers(s: &Sources) -> BTreeMap<String, BTreeSet<String>> {
    let roots = roots(s);
    let mut reached: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for (role, starts) in &roots {
        let mut queue: VecDeque<&str> = starts.iter().map(String::as_str).collect();
        let mut seen = BTreeSet::new();
        while let Some(f) = queue.pop_front() {
            if !seen.insert(f) {
                continue;
            }
            reached.entry(f).or_default().insert(role.clone());
            for callee in s.graph.get(f).into_iter().flatten() {
                queue.push_back(callee);
            }
        }
    }
    let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (function, kind, bump) in &s.bumps {
        let roles = reached
            .get(function.as_str())
            .unwrap_or_else(|| panic!("{function} bumps {kind} but no thread reaches it"));
        let mut kinds = vec![kind.clone()];
        if bump.ends_with("burst") {
            let at = s.enum_order.iter().position(|k| k == kind).unwrap();
            kinds.extend(s.enum_order[at + 1..at + 3].iter().cloned());
        }
        for k in kinds {
            out.entry(k).or_default().extend(roles.iter().cloned());
        }
    }
    out
}

/// The header's shared kinds: the cases of `rusteron_dpdk_counter_shared`.
fn declared_shared() -> BTreeSet<String> {
    let header = strip(&std::fs::read_to_string(Path::new(NATIVE).join("rusteron_dpdk_counters.h")).unwrap());
    let f = functions(&header)
        .into_iter()
        .find(|f| f.name == "rusteron_dpdk_counter_shared")
        .expect("rusteron_dpdk_counter_shared in rusteron_dpdk_counters.h");
    f.body
        .split("case ")
        .skip(1)
        .map(|c| c.split(':').next().unwrap().trim().to_string())
        .collect()
}

fn single_writer(roles: &BTreeSet<String>) -> bool {
    roles.iter().all(|r| r == "agent" || r == "before any transport")
}

#[test]
fn the_header_names_exactly_the_kinds_more_than_the_agents_write() {
    let s = read_sources();
    let writers = writers(&s);
    let shared: BTreeSet<String> = writers
        .iter()
        .filter(|(_, roles)| !single_writer(roles))
        .map(|(k, _)| k.clone())
        .collect();
    let detail: Vec<String> = writers
        .iter()
        .filter(|(k, _)| shared.contains(*k))
        .map(|(k, roles)| format!("{k}: {roles:?}"))
        .collect();
    assert_eq!(
        declared_shared(),
        shared,
        "rusteron_dpdk_counter_shared must list exactly the kinds a thread other than the agents writes:\n{}",
        detail.join("\n")
    );
    // the analysis saw the agents' hot paths, and attributed them to agents alone
    for kind in [
        "RD_COUNTER_TX_PKTS",
        "RD_COUNTER_TX_BYTES",
        "RD_COUNTER_RX_PKTS",
        "RD_COUNTER_RX_BYTES",
        "RD_COUNTER_POLLER",
    ] {
        assert_eq!(
            writers.get(kind).map(|r| r.iter().cloned().collect::<Vec<_>>()),
            Some(vec!["agent".to_string()]),
            "{kind}"
        );
    }
}

#[test]
fn a_max_or_a_min_is_never_a_shared_kind() {
    let s = read_sources();
    let writers = writers(&s);
    let shared = declared_shared();
    for (function, kind, bump) in &s.bumps {
        if bump.ends_with("_max") || bump.ends_with("min_counted") || bump.ends_with("burst") {
            assert!(
                single_writer(&writers[kind]) && !shared.contains(kind),
                "{function}: {bump} on {kind} is a load and a store, but {:?} write it",
                writers[kind]
            );
        }
    }
}

#[test]
fn the_analysis_sees_every_bump_and_thread() {
    let s = read_sources();
    // a call graph that missed the bumps would pass the checks above vacuously
    assert!(s.bumps.len() >= 60, "only {} bumps found", s.bumps.len());
    let roots = roots(&s);
    for want in [
        "agent",
        "conductor",
        "thread rusteron_dpdk_housekeeping_main",
        "thread rusteron_dpdk_local_io_main",
    ] {
        assert!(roots.contains_key(want), "no {want} among {:?}", roots.keys());
    }
    assert!(
        roots.keys().any(|r| r == "thread rusteron_dpdk_port_on_reset"),
        "the EAL's reset callback not seen: {:?}",
        roots.keys()
    );
    // the test hooks left out of the roots: the library never calls one
    let mut dirs = vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("src")];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                dirs.push(path);
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            for hook in s.exported.iter().filter(|f| f.contains("_test_")) {
                assert!(
                    !text.contains(hook.as_str()),
                    "{} calls the test hook {hook}",
                    path.display()
                );
            }
        }
    }
    let table = bindings(&s);
    assert_eq!(table["send_func"], "rusteron_dpdk_transport_send");
    assert_eq!(table["poller_poll_func"], "rusteron_dpdk_transport_poller_poll");
}
