// Generated file: edits here are overwritten by the next export. sha256:15df979bb3a817e32b57e4dc0073620e4e82b510ce3ea6f53f87250ff9b9acb8
// Pacing with gates and verdicts: what ergon-io reports, and what it asks.
// Sources: every state below comes from a port of this code, run in order.
//   ergon-io/src/outcome.rs:36-43    Sent: No (record never wholly with the kernel), Maybe
//   ergon-io/src/outcome.rs:48-75    Verdict; Throttled { retry_after_ms }
//   ergon-io/src/outcome.rs:81-97    from_sent (No -> NotRun, Maybe -> Unknown), may_resend
//   ergon-io/src/outcome.rs:181-191  generic: 429 -> Throttled with Retry-After
//   ergon-io/src/outcome.rs:209-228  refused_upgrade: a refused handshake's 429
//   ergon-io/src/client.rs:645-656   Sent::No only when first_unsent <= the request
//   ergon-io/src/reconnect.rs:29-30  cost: one branch while nothing waits
//   ergon-io/src/reconnect.rs:66-77  Retry::default (250 ms, x2, 30 s, jitter 0.2)
//   ergon-io/src/reconnect.rs:111-116  delay_ms
//   ergon-io/src/reconnect.rs:327-335  on_open_at: reconnect = was open before
//   ergon-io/src/reconnect.rs:346-358  on_closed_at -> schedule
//   ergon-io/src/reconnect.rs:539-572  run: a gate's Err(wait_ns) keeps the attempt count;
//                                      :547, :570 a pass starts after the last link the gate passed
//   ergon-io/src/reconnect.rs:619-631  defer_at: not_before
//   ergon-io/src/reconnect.rs:634-662  schedule: attempt + 1, at = max(backoff, not_before)
//   ergon-io/tests/reconnect.rs:417-466  Pace, the test's pacer (the application's)
//   ergon-io/tests/reconnect.rs:690-704  a refused handshake holds the gate and defers the link
// Text: book/src/ergon-io/rate-limits.md:3-22 and :71-75 (Limits).
// Illustrative: the times, the 3-per-second gate, the jitter draw 0.5, the
// Retry-After of 2 s and request 1's 3 of 8 parts. The page records no latency for any of this.
(() => {
  'use strict';
  // Retry::default; R stands in for the seeded xorshift draw.
  const RETRY = { initial: 250, max: 30000, mult: 2, jitter: 0.2 };
  const R = 0.5;
  const SPAN = 2300; // the timeline's right edge, ms
  const fmt = (n) => n.toLocaleString('en-GB');
  const TITLE = 'Pacing with gates and verdicts: Sent, Verdict and the connect gate';
  const NARROW = 560; // px of the figure, padding included: under it, the narrow layout
  const SVG_NS = 'http://www.w3.org/2000/svg';

  // The drawing code below names its fills and text sizes with the old runtime's
  // classes; OLD_CLASS maps each to anim.css's vocabulary (cls), an inline style or a
  // font-size. Outline-only kinds set fill:none, which no anim.css class does
  // (an SVG rect fills black by default). ea-on-a is gone: text in the text
  // colour reads on a 0.22-opacity fill. Unlisted classes pass through.
  const OLD_CLASS = {
    'ea-a': { cls: 'ea-k1' },
    'ea-b': { cls: 'ea-k2' },
    'ea-c': { cls: 'ea-k3' },
    'ea-d': { cls: 'ea-k4' },
    'ea-ghost': { style: 'fill:var(--table-header-bg);opacity:0.6' },
    'ea-empty': { style: 'fill:none;stroke:var(--table-header-bg)' },
    'ea-stroke-hl': { style: 'fill:none;stroke:var(--ea-c5);stroke-width:3' },
    'ea-stroke-c': { cls: 'ea-ok', style: 'fill:none' },
    'ea-stroke-d': { style: 'fill:none;stroke:var(--ea-c4);stroke-width:2;stroke-dasharray:2 3' },
    'ea-wire': { style: 'fill:none;stroke:var(--fg);stroke-width:1.2;opacity:0.6' },
    'ea-title': { cls: 'ea-strong', size: 14 },
    'ea-small': { size: 10 },
    'ea-tiny': { size: 8 },
    'ea-dim': { cls: 'ea-muted' },
    'ea-on-a': {},
  };
  // Whenever `cls` is given, class, style and font-size are all set (or removed),
  // so a keyed element that changes kind between steps keeps nothing of the old one.
  function translate(attrs) {
    if (!attrs || !('cls' in attrs)) return attrs;
    const { cls, style, ...rest } = attrs;
    const classes = [];
    const styles = [];
    let size;
    for (const t of String(cls || '').split(/\s+/).filter(Boolean)) {
      const m = OLD_CLASS[t];
      if (!m) { classes.push(t); continue; }
      if (m.cls) classes.push(m.cls);
      if (m.style) styles.push(m.style);
      if (m.size) size = m.size;
    }
    if (style) styles.push(style);
    // anim.css's ea-k* rules beat a fill-opacity attribute, so it goes inline.
    if (rest['fill-opacity'] != null && classes.some((c) => /^ea-k\d$/.test(c))) {
      styles.push(`fill-opacity:${rest['fill-opacity']}`);
      delete rest['fill-opacity'];
    }
    return { cls: classes.join(' ') || undefined, style: styles.join(';') || undefined, 'font-size': size, ...rest };
  }

  // Keyed SVG elements: put() creates a child once and updates it after, so
  // the CSS transitions animate the change; sweep() removes the ones a step
  // did not draw. Attributes set to undefined are removed; `at` translates.
  function keyed() {
    let gen = 0;
    function put(parent, key, tag, attrs) {
      let e = null;
      for (const c of parent.children) if (c.dataset && c.dataset.k === key) { e = c; break; }
      if (!e) {
        e = document.createElementNS(SVG_NS, tag);
        e.dataset.k = key;
        parent.appendChild(e);
      }
      e.eaGen = gen;
      for (const [k, v] of Object.entries(translate(attrs) || {})) {
        if (v === undefined || v === null || v === false) {
          if (k !== 'text' && k !== 'at') e.removeAttribute(k === 'cls' ? 'class' : k);
        } else if (k === 'at') e.style.transform = `translate(${v[0]}px, ${v[1]}px)`;
        else if (k === 'text') { if (e.textContent !== String(v)) e.textContent = v; }
        else e.setAttribute(k === 'cls' ? 'class' : k, v);
      }
      return e;
    }
    return {
      put,
      begin() { gen += 1; },
      sweep(root) { root.querySelectorAll('[data-k]').forEach((e) => { if (e.eaGen !== gen && e.isConnected) e.remove(); }); },
    };
  }


  const delayMs = (attempt) => Math.max(1, Math.round(
    Math.min(RETRY.initial * RETRY.mult ** (attempt - 1), RETRY.max) * (1 - RETRY.jitter * R)));

  // The test's Pace: at most `limit` starts per window, a hold until a time.
  function pace(limit, windowMs) { return { limit, windowMs, starts: [], held: -Infinity }; }
  function tryStart(p, now) {
    if (now < p.held) return { err: p.held - now };
    while (p.starts.length && p.starts[0] + p.windowMs <= now) p.starts.shift();
    if (p.starts.length >= p.limit) return { err: p.starts[0] + p.windowMs - now };
    p.starts.push(now);
    return { ok: true };
  }

  // One Reconnect link: state, failed attempts in a row, not_before.
  const link = (name, state, wasOpen) => ({ name, state, at: null, attempts: 0, notBefore: -Infinity, wasOpen });
  function schedule(l, now) {
    const attempt = l.attempts + 1;
    l.attempts = attempt;
    const backoff = delayMs(attempt);
    const at = Math.max(now + backoff, l.notBefore);
    l.state = 'Waiting';
    l.at = at;
    return { attempt, backoff, delay: at - now };
  }
  function deferAt(l, wait, now) {
    l.notBefore = Math.max(l.notBefore, now + wait);
    if (l.state === 'Waiting') l.at = Math.max(l.at, l.notBefore);
  }
  // run(): from the cursor, every due link asks the gate; a refusal moves
  // `at`, not `attempts`; a pass moves the cursor past that link.
  function poll(rc, p, now) {
    const out = [];
    const n = rc.links.length;
    const first = rc.cursor < n ? rc.cursor : 0;
    for (let step = 0; step < n; step += 1) {
      const i = (first + step) % n;
      const l = rc.links[i];
      if (l.state !== 'Waiting' || l.at > now) continue;
      const g = tryStart(p, now);
      if (g.err !== undefined) {
        l.at = now + Math.min(g.err, RETRY.max);
        out.push({ l, err: g.err });
        continue;
      }
      rc.cursor = (i + 1) % n;
      l.state = 'Connecting';
      l.at = null;
      out.push({ l, ok: true });
    }
    return out;
  }
  function onOpen(l) { const reconnect = l.wasOpen; l.wasOpen = true; l.state = 'Open'; return reconnect; }

  // Verdict::from_sent and may_resend.
  const fromSent = (sent) => (sent === 'No' ? 'NotRun' : 'Unknown');
  const mayResend = (v) => v === 'NotRun' || v.startsWith('Throttled') || v === 'ClockSkew';

  const snap = (o) => JSON.parse(JSON.stringify(o));

  function buildSteps() {
    const steps = [];
    const reqs = [];
    const add = (s) => steps.push(Object.assign({ reqs: snap(reqs) }, s));

    add({
      part: 1, op: 'ergon-io has no rate limiter', math: 'it reports what happened; your pacer decides when to send',
      caption: 'Three parties: your application (its pacer included), ergon-io, and the server. ergon-io never counts a venue’s budget: it hands back two facts per request, Sent and Verdict, and asks your gate before every connect.',
      arrows: [], changed: [],
    });

    // Request 1: its one TLS record only partly reached the kernel.
    reqs.push({ id: 1, parts: 3, sent: 'No', verdict: fromSent('No') });
    add({
      part: 1, op: 'on_error(1, Sent::No, err)', math: 'first_unsent <= request 1  =>  Sent::No  =>  Verdict::NotRun',
      caption: 'Request 1 is one TLS record. Only 3 of its 8 parts (illustrative) were with the kernel when the connection dropped. The server cannot decrypt part of a record, so it cannot have run: Sent::No, NotRun, safe to send again inside its deadline.',
      arrows: [['app', 'io', 'request 1', 'send #1'], ['io', 'srv', '3/8 of the record, then drop', '3/8 of record, drop', 'cut'], ['io', 'app', 'on_error(1, Sent::No, err)', 'on_error(Sent::No)', 'ok']],
      changed: ['r1'], cost: 'ergon-io: one comparison of the request with first_unsent',
    });

    reqs.push({ id: 2, parts: 8, sent: 'Maybe', verdict: fromSent('Maybe') });
    add({
      part: 1, op: 'on_error(2, Sent::Maybe, err)', math: 'the whole record was with the kernel  =>  Sent::Maybe  =>  Verdict::Unknown',
      caption: 'Request 2’s whole record was with the kernel before the connection was lost, with no response. It may have run: Sent::Maybe, Unknown. may_resend() is false: look the order up by its client order id; never send an order again blindly.',
      arrows: [['app', 'io', 'request 2', 'send #2'], ['io', 'srv', 'whole record, then silence', 'whole record, silence', 'cut'], ['io', 'app', 'on_error(2, Sent::Maybe, err)', 'on_error(Sent::Maybe)', 'maybe']],
      changed: ['r2'], cost: 'ergon-io: same comparison; nothing re-sent for you',
    });

    reqs.push({ id: 3, parts: 8, status: 429, verdict: 'Throttled { Some(2000) }' });
    add({
      part: 1, op: 'outcome::generic(resp, now) on a 429', math: '429 + "Retry-After: 2"  =>  Throttled { retry_after_ms: Some(2000) }',
      caption: 'Request 3 is answered 429 with Retry-After: 2. generic turns it into Throttled { retry_after_ms: Some(2000) }. Your pacer holds that scope 2,000 ms and re-sends after. No Retry-After gives None (use your own backoff); a malformed one is Error::Http, still a 429.',
      arrows: [['app', 'io', 'request 3', 'send #3'], ['srv', 'io', '429, Retry-After: 2', '429, Retry-After: 2', 'wait'], ['io', 'app', 'Throttled { Some(2000) }', 'Throttled{Some(2000)}', 'wait']],
      changed: ['r3'], cost: 'ergon-io: status match + Retry-After parsed with no allocation',
    });

    // Part 2: two links on one IP share one pacer (the application's gate).
    const p = pace(3, 1000);
    const A = link('A', 'Waiting', false);
    A.at = 0; // added: its first attempt is due at once, and asks the gate too
    const B = link('B', 'Open', true);
    const rc = { links: [A, B], cursor: 0 };
    poll(rc, p, 0); // A passes; the cursor moves to B
    const add2 = (s) => steps.push(Object.assign({ part: 2, links: snap([A, B]), pace: snap(p) }, s));

    add2({
      t: 0, op: 'Reconnect::poll_gated(driver, gate)', math: 'gate: the test’s Pace, 3 connects per 1 s (illustrative), one for the IP',
      caption: 'Now connects. Links A and B are on one IP, so one pacer gates both. A’s first connect passed the gate at t = 0 and is in its handshake; B is open. The backoff is Retry::default: 250 ms, doubling, up to 30 s, up to 20 % jitter off.',
      arrows: [['app', 'io', 'poll_gated(driver, || pace.try_start(now))', 'poll_gated(.., gate)'], ['io', 'srv', 'A: handshake', 'A: handshake']],
      changed: ['pace'],
    });

    const throttled = 2000;
    p.held = 80 + throttled;
    deferAt(A, throttled, 80);
    add2({
      t: 80, op: 'refused_upgrade(&err)  =>  Throttled { Some(2000) }', math: 'your on_closed: pace.held_until = now + 2,000; defer_at(A, 2 s)',
      caption: 'A’s handshake is refused: 429, Retry-After: 2. refused_upgrade gives Throttled { retry_after_ms: Some(2000) }. Your handler holds the gate until 2,080 ms and defers A: not_before = 2,080, kept with the link across later closes.',
      arrows: [['srv', 'io', 'A: 429, Retry-After: 2', 'A: 429, Retry-After: 2', 'wait'], ['io', 'app', 'on_closed(A, Error::Upgrade)', 'on_closed(A, Upgrade)'], ['app', 'io', 'defer_at(A, 2 s); hold gate', 'defer(A, 2 s); hold', 'wait']],
      changed: ['pace', 'A-nb'],
    });

    const sa = schedule(A, 80);
    add2({
      t: 80, op: `on_closed(A)  =>  Closed::Retry { attempt ${sa.attempt}, delay_ms ${fmt(sa.delay)} }`,
      math: `backoff ${sa.backoff} ms (jitter draw ${R}, illustrative); at = max(80 + ${sa.backoff}, not_before) = ${fmt(A.at)}`,
      caption: `The refused handshake was a real attempt, so it counts: attempt ${sa.attempt}. Its backoff is only ${sa.backoff} ms, but not_before wins: A waits until ${fmt(A.at)} ms, the server’s wait, whatever the backoff says.`,
      arrows: [['io', 'app', `Closed::Retry { attempt ${sa.attempt}, ${fmt(sa.delay)} ms }`, `Retry{attempt ${sa.attempt}, ${fmt(sa.delay)} ms}`]],
      changed: ['A-st'], ghostA: 80 + sa.backoff, cost: 'ergon-io: one jitter draw, delay_ms, then max(backoff, not_before)',
    });

    const sb = schedule(B, 400);
    add2({
      t: 400, op: `on_closed(B)  =>  Closed::Retry { attempt ${sb.attempt}, delay_ms ${sb.delay} }`,
      math: `B was open; no server wait on B, so at = 400 + ${sb.backoff} = ${B.at}`,
      caption: `B’s connection drops at 400 ms. Nothing deferred B, so its first backoff, ${sb.backoff} ms, makes it due at ${B.at} ms: well inside the hold your pacer set for the IP.`,
      arrows: [['srv', 'io', 'B: connection lost', 'B: connection lost', 'cut'], ['io', 'app', `Closed::Retry { attempt ${sb.attempt}, ${sb.delay} ms }`, `Retry{attempt ${sb.attempt}, ${sb.delay} ms}`]],
      changed: ['B-st'],
    });

    const r1 = poll(rc, p, 625);
    add2({
      t: 625, op: `gate()  =>  Err(${fmt(r1[0].err)} ms)`, math: `B due: the gate refuses; at = 625 + ${fmt(r1[0].err)} = ${fmt(B.at)}; attempt stays ${B.attempts}`,
      caption: `B is due, so poll_gated asks your gate. The IP is held: Err(wait_ns), ${fmt(r1[0].err)} ms. A gate refusal is not a failed attempt (unlike A’s refused handshake): B keeps attempt ${B.attempts} and max_attempts does not count it. Had it counted, B’s next backoff would double to ${delayMs(2)} ms, then ${delayMs(3)}.`,
      arrows: [['io', 'app', 'gate()', 'gate()'], ['app', 'io', `Err(${fmt(r1[0].err)} ms)`, `Err(${fmt(r1[0].err)} ms)`, 'wait']],
      changed: ['B-st'], ghost: true, cost: 'ergon-io: the gate is asked once, for the one due link',
    });

    const r2 = poll(rc, p, 2080);
    const order = r2.map((r) => r.l.name);
    add2({
      t: 2080, op: `gate()  =>  Ok(())  (${order.join(', then ')})`, math: `hold over; the window drops the start at 0; ${r2.length} passes, ${p.starts.length} of ${p.limit} used`,
      caption: `At 2,080 ms both links are due. The hold is over and the window has room. The pass starts after the last link the gate passed (A, at 0 ms), so it asks for ${order[0]} first, then ${order[1]}: links that keep failing cannot starve the others. Each gets a fresh dial from its factory.`,
      arrows: [['io', 'app', 'gate() twice', 'gate() x2'], ['app', 'io', 'Ok(()), Ok(())', 'Ok, Ok', 'ok'], ['io', 'srv', 'A and B: fresh handshakes', 'A, B: handshakes']],
      changed: ['A-st', 'B-st', 'pace'],
    });

    const ra = onOpen(A);
    const rb = onOpen(B);
    add2({
      t: 2160, op: `on_open: A { reconnect: ${ra} }, B { reconnect: ${rb} }`, math: 'attempts reset at the next close, if the connection stayed open stable_ms (60 s)',
      caption: 'Both open. A has never been open, so reconnect is false: subscribe. B is a reconnect: resubscribe and resync any state kept from the old connection. Their attempt counts stay at 1; each resets at its next close if that connection stayed open 60 s.',
      arrows: [['srv', 'io', 'A, B: 101 Switching Protocols', 'A, B: 101', 'ok'], ['io', 'app', `Opened { A: ${ra}, B: ${rb} }`, `Opened{A:${ra}, B:${rb}}`, 'ok']],
      changed: ['A-st', 'B-st'],
    });

    steps.push({
      part: 3, hold: 10000, op: 'Why this wins here, and when it loses', math: '',
      caption: 'Why it wins: one pacer of yours sees every 429 and every connect on the IP, and gate refusals never inflate the backoff. Cost: one branch per poll while nothing waits. When it loses: one process per account and IP only, Unknown needs a lookup, and you write the pacer. The book records no latency for this.',
    });
    return steps;
  }

  const LANES = { app: 0, io: 1, srv: 2 };
  const LANE_NAMES = [['your application (pacer)', 'your app'], ['ergon-io', 'ergon-io'], ['server', 'server']];

  function layout(narrow) {
    if (narrow) return { W: 360, H: 670, lane: [60, 180, 300], headW: 108, laneY: 72, arrowY: [112, 146, 180], low: 222, small: true };
    return { W: 720, H: 470, lane: [120, 360, 600], headW: 200, laneY: 58, arrowY: [98, 134, 170], low: 214, small: false };
  }

  // A cut connection is neutral: request 1's cut did not run, request 2's may have.
  const ARROW_STROKE = { ok: 'var(--ea-c3)', maybe: 'var(--ea-c4)', wait: 'var(--ea-c2)', cut: 'var(--table-header-bg)' };

  function drawArrows(svg, put, L, st, idx) {
    (st.arrows || []).forEach((a, n) => {
      const [from, to, wide, slim, kind] = a;
      const x1 = L.lane[LANES[from]];
      const x2 = L.lane[LANES[to]];
      const y = L.arrowY[n];
      const dir = Math.sign(x2 - x1);
      // Keyed by step: each step's arrows fade in fresh (cg-in), old ones go.
      const g = put(svg, `ar-${idx}-${n}`, 'g', { cls: 'cg-in' });
      const stroke = kind ? ARROW_STROKE[kind] : 'var(--fg)';
      put(g, 'l', 'line', { x1: x1 + dir * 6, y1: y, x2: x2 - dir * 10, y2: y, style: `stroke:${stroke};stroke-width:2;${kind === 'cut' ? 'stroke-dasharray:6 4;' : ''}` });
      const hx = x2 - dir * 4;
      put(g, 'h', 'path', { d: `M${hx},${y} l${-dir * 9},-5 v10 z`, style: `fill:${stroke}` });
      put(g, 't', 'text', { x: (x1 + x2) / 2, y: y - 6, 'text-anchor': 'middle', cls: 'ea-small', text: L.small ? slim : wide });
    });
  }

  function drawLanes(svg, put, L) {
    LANE_NAMES.forEach((names, i) => {
      const x = L.lane[i];
      put(svg, `lh-${i}`, 'rect', { x: x - L.headW / 2, y: L.laneY - 16, width: L.headW, height: 22, rx: 4, cls: 'ea-box' });
      put(svg, `lt-${i}`, 'text', { x, y: L.laneY, 'text-anchor': 'middle', cls: L.small ? 'ea-small' : '', text: names[L.small ? 1 : 0] });
      put(svg, `ll-${i}`, 'line', { x1: x, y1: L.laneY + 6, x2: x, y2: L.arrowY[2] + 16, cls: 'ea-wire', style: 'stroke-dasharray:2 4' });
    });
  }

  // Words split into at most two lines of `max` characters.
  function wrap(put, parent, key, x, y, text, max, lh, cls, style) {
    const lines = ['', ''];
    let n = 0;
    for (const w of text.split(' ')) {
      if (lines[n] && (lines[n] + ' ' + w).length > max && n === 0) n = 1;
      lines[n] = lines[n] ? `${lines[n]} ${w}` : w;
    }
    lines.forEach((t, k) => put(parent, `${key}${k}`, 'text', { x, y: y + k * lh, cls, style, text: t }));
  }

  // A highlight frame around whatever changed this step.
  const frame = (put, parent, key, on, x, y, w, h) =>
    put(parent, key, 'rect', { x, y, width: w, height: h, rx: 4, cls: on ? 'ea-stroke-hl' : 'ea-hidden' });

  const VERDICT_CLS = (v) => (v === 'NotRun' ? 'ea-c' : v === 'Unknown' ? 'ea-d' : 'ea-b');
  const next = (v) => (!mayResend(v) ? 'may_resend: no; look it up' : v === 'NotRun' ? 'may_resend: yes, in deadline' : 'may_resend: yes, after 2 s');

  function drawRequests(svg, put, L, st) {
    const changed = new Set(st.changed);
    const y0 = L.low;
    put(svg, 'rq-h', 'text', { x: 16, y: y0, cls: 'ea-small', text: L.small ? 'each request: its record, then Sent and Verdict' : 'each request is one TLS record: parts with the kernel, then what ergon-io reports' });
    const rowH = L.small ? 66 : 52;
    for (let i = 0; i < 3; i += 1) {
      const r = st.reqs[i];
      const y = y0 + 14 + i * rowH;
      const g = put(svg, `rq-${i}`, 'g', { at: [16, y], cls: r ? '' : 'ea-hidden' });
      if (!r) continue;
      const W = L.W - 32;
      put(g, 'bg', 'rect', { width: W, height: rowH - 8, rx: 4, cls: 'ea-box' });
      put(g, 'n', 'text', { x: 8, y: 18, cls: 'ea-title', text: `#${r.id}` });
      for (let s = 0; s < 8; s += 1) put(g, `p${s}`, 'rect', { x: 40 + s * 13, y: 8, width: 11, height: 12, rx: 2, cls: s < r.parts ? 'ea-a' : 'ea-empty' });
      put(g, 'pl', 'text', { x: 40, y: 34, cls: 'ea-tiny', text: r.status ? 'sent; answered' : `${r.parts}/8 with the kernel` });
      const vx = L.small ? 8 : 160;
      const vy = L.small ? 44 : 8;
      const word = r.status ? `HTTP ${r.status}` : `Sent::${r.sent}`;
      put(g, 'sent', 'text', { x: L.small ? 160 : vx, y: L.small ? 18 : vy + 10, text: word });
      const v = r.verdict;
      const vw = L.small ? 190 : 200;
      const vgx = L.small ? vx : 290;
      put(g, 'vb', 'rect', { x: vgx, y: vy - 2, width: vw, height: 20, rx: 4, cls: VERDICT_CLS(v) });
      put(g, 'vt', 'text', { x: vgx + 8, y: vy + 12, cls: 'ea-on-a', text: v });
      put(g, 'nx', 'text', { x: L.small ? vgx + vw + 4 : 500, y: vy + 12, cls: L.small ? 'ea-tiny' : 'ea-small', text: L.small ? next(v).replace('may_resend: ', 'resend? ') : next(v) });
      frame(put, g, 'hl', changed.has(`r${r.id}`), -3, -3, W + 6, rowH - 2);
    }
  }

  function drawTimeline(svg, put, L, st) {
    const changed = new Set(st.changed);
    const x0 = L.small ? 44 : 70;
    const x1 = L.W - 20;
    const X = (ms) => x0 + (Math.min(ms, SPAN) / SPAN) * (x1 - x0);
    const y0 = L.low + 6;
    const rows = { pace: y0 + 26, A: y0 + 58, B: y0 + 90 };
    put(svg, 'tl-h', 'text', { x: 16, y: L.low, cls: 'ea-small', text: L.small ? 'time, ms (illustrative)' : 'time on the pacer’s clock, ms (illustrative values)' });
    // Axis and ticks every 500 ms.
    put(svg, 'ax', 'line', { x1: x0, y1: rows.B + 18, x2: x1, y2: rows.B + 18, cls: 'ea-wire' });
    for (let ms = 0; ms <= 2000; ms += 500) {
      put(svg, `tk-${ms}`, 'line', { x1: X(ms), y1: rows.B + 15, x2: X(ms), y2: rows.B + 21, cls: 'ea-wire' });
      put(svg, `tl-${ms}`, 'text', { x: X(ms), y: rows.B + 32, 'text-anchor': 'middle', cls: 'ea-tiny', text: fmt(ms) });
    }
    put(svg, 'rl-p', 'text', { x: 16, y: rows.pace + 4, cls: 'ea-small', text: 'gate' });
    put(svg, 'rl-a', 'text', { x: 16, y: rows.A + 4, cls: 'ea-small', text: 'A' });
    put(svg, 'rl-b', 'text', { x: 16, y: rows.B + 4, cls: 'ea-small', text: 'B' });
    const p = st.pace;
    const held = p.held !== null && Number.isFinite(p.held) && p.held > 0;
    put(svg, 'held', 'rect', { x: X(80), y: rows.pace - 8, width: held ? X(p.held) - X(80) : 0, height: 16, rx: 3, cls: held ? undefined : 'ea-hidden', style: held ? 'fill:var(--ea-c2);opacity:0.35' : undefined });
    put(svg, 'held-t', 'text', { x: X(1080), y: rows.pace + 4, 'text-anchor': 'middle', cls: held ? 'ea-tiny' : 'ea-hidden', text: 'held: Retry-After 2 s' });
    // Starts the gate passed, as history (the pacer's window forgets old ones).
    const shown = st.t >= 2080 ? [0, 2080] : [0];
    [0, 2080].forEach((ms, n) => {
      put(svg, `st-${n}`, 'rect', { x: X(ms) - 3, y: rows.pace - 8, width: 6, height: 16, rx: 1, cls: shown.includes(ms) ? 'ea-c' : 'ea-hidden' });
    });
    if (st.ghost) put(svg, 'refuse', 'path', { d: `M${X(625) - 5},${rows.pace - 6} l10,12 m0,-12 l-10,12`, style: 'stroke:var(--fg);stroke-width:2.5;fill:none' });
    // "now": a cursor that slides with each step.
    const now = put(svg, 'now', 'g', { at: [X(st.t), 0] });
    put(now, 'l', 'line', { x1: 0, y1: rows.pace - 16, x2: 0, y2: rows.B + 16, style: 'stroke:var(--ea-c5);stroke-width:2' });
    put(now, 't', 'text', { x: 0, y: rows.pace - 19, 'text-anchor': 'middle', cls: 'ea-tiny', text: `now ${fmt(st.t)}` });
    // Each link's next attempt (`at`) slides along its row; not_before is a bar.
    st.links.forEach((l) => {
      const y = rows[l.name];
      const nb = Number.isFinite(l.notBefore) && l.notBefore !== null;
      put(svg, `nb-${l.name}`, 'line', { x1: X(80), y1: y, x2: nb ? X(l.notBefore) : X(80), y2: y, cls: nb ? '' : 'ea-hidden', style: 'stroke:var(--ea-c2);stroke-width:3;stroke-dasharray:3 3' });
      const waiting = l.state === 'Waiting';
      const m = put(svg, `at-${l.name}`, 'g', { at: [X(waiting ? l.at : st.t), y], cls: waiting ? '' : 'ea-hidden' });
      put(m, 'd', 'path', { d: 'M0,-8 L8,0 L0,8 L-8,0 Z', cls: 'ea-a' });
      put(m, 't', 'text', { x: 0, y: -11, 'text-anchor': 'middle', cls: 'ea-tiny', text: waiting ? `at ${fmt(l.at)}` : '' });
      put(svg, `cs-${l.name}`, 'text', { x: X(st.t) + (st.t < SPAN / 2 ? 6 : -6), y: y - 5, 'text-anchor': st.t < SPAN / 2 ? 'start' : 'end', cls: waiting ? 'ea-hidden' : 'ea-tiny', text: l.state.toLowerCase() });
    });
    // Ghosts: where the link was due before this step moved it.
    if (st.ghost) {
      const g = put(svg, 'ghost', 'g', { at: [X(625), rows.B] });
      put(g, 'd', 'path', { d: 'M0,-7 L7,0 L0,7 L-7,0 Z', cls: 'ea-ghost' });
    }
    if (st.ghostA) {
      const g = put(svg, 'ghostA', 'g', { at: [X(st.ghostA), rows.A] });
      put(g, 'd', 'path', { d: 'M0,-7 L7,0 L0,7 L-7,0 Z', cls: 'ea-ghost' });
      put(g, 't', 'text', { x: 0, y: 18, 'text-anchor': 'middle', cls: 'ea-tiny', text: `backoff alone: ${fmt(st.ghostA)}` });
    }

    // State cards: each link's Reconnect state, and your pacer's.
    const cy = rows.B + 46;
    const cw = L.small ? L.W - 32 : (L.W - 48) / 3;
    const cards = [
      ['pace', 'your pacer (Pace)', [`starts in window: [${p.starts.map(fmt).join(', ')}]`, `limit ${p.limit} per ${fmt(p.windowMs)} ms`, held ? `held until ${fmt(p.held)}` : 'held: no']],
    ];
    st.links.forEach((l) => {
      const state = l.state === 'Waiting' ? `Waiting { at ${fmt(l.at)}, attempt ${l.attempts} }` : `${l.state}${l.attempts ? ` (attempts ${l.attempts})` : ''}`;
      cards.push([`${l.name}-st`, `link ${l.name}`, [state, Number.isFinite(l.notBefore) && l.notBefore !== null ? `not_before ${fmt(l.notBefore)}` : 'not_before: none', `was open: ${l.wasOpen}`]]);
    });
    cards.forEach(([key, title, lines], n) => {
      const x = L.small ? 16 : 16 + n * (cw + 8);
      const y = L.small ? cy + n * 64 : cy;
      const g = put(svg, `cd-${n}`, 'g', { at: [x, y] });
      put(g, 'bg', 'rect', { width: cw, height: 58, rx: 4, cls: 'ea-box' });
      put(g, 'h', 'text', { x: 8, y: 15, cls: 'ea-small', text: title, style: 'font-weight:bold' });
      lines.forEach((t, k) => put(g, `l${k}`, 'text', { x: 8, y: 29 + k * 12, cls: 'ea-tiny', text: t }));
      const hit = changed.has(key) || (key !== 'pace' && changed.has(`${key[0]}-nb`));
      frame(put, g, 'hl', hit, -3, -3, cw + 6, 64);
    });
  }

  function drawTakeaway(svg, put, L) {
    const wins = [
      'One pacer of yours sees every 429, every refused',
      'handshake and every connect on the IP.',
      'A gate refusal never grows the backoff (B: attempt 1).',
      'The server’s wait beats the backoff (A: 2,000 ms).',
      'Cost: poll is one branch while nothing waits; a',
      'waiting link reads the clock every 250 µs; the gate',
      'is asked only for a due link.',
    ];
    const loses = [
      'One sending process per account and IP: a pacer',
      'in one process cannot see the others.',
      'Sent::Maybe or Unknown: look the order up by its',
      'client order id; never re-send it blindly.',
      'You write the pacer (the test’s is under 50 lines).',
      'No latency is recorded for this; times shown',
      'are illustrative.',
    ];
    const col = (key, title, lines, x, y, cls) => {
      const g = put(svg, key, 'g', { at: [x, y] });
      const w = L.small ? L.W - 32 : (L.W - 40) / 2;
      put(g, 'bg', 'rect', { width: w, height: 30 + lines.length * 15, rx: 6, cls: 'ea-box' });
      put(g, 'bar', 'rect', { width: 6, height: 30 + lines.length * 15, rx: 3, cls });
      put(g, 'h', 'text', { x: 14, y: 20, cls: 'ea-title', text: title });
      lines.forEach((t, k) => put(g, `l${k}`, 'text', { x: 14, y: 40 + k * 15, cls: L.small ? 'ea-tiny' : 'ea-small', text: t }));
    };
    put(svg, 'tk-t', 'text', { x: 16, y: 24, cls: 'ea-title', text: 'Pacing: ergon-io reports, you decide' });
    col('tk-w', 'Why it wins here', wins, 16, 40, 'ea-c');
    col('tk-l', 'When it loses', loses, L.small ? 16 : 24 + (L.W - 40) / 2, L.small ? 40 + 30 + wins.length * 15 + 12 : 40, 'ea-d');
    // B's backoff after its next real failure, from the port above.
    const y = L.small ? 40 + 2 * (30 + 7 * 15) + 40 : 40 + 30 + 7 * 15 + 40;
    put(svg, 'tk-b', 'text', { x: 16, y, cls: 'ea-title', text: L.small ? 'B’s backoff after its next failure' : 'B’s backoff after its next real failure (jitter draw 0.5, illustrative)' });
    const bars = [['ergon-io: gate refusal not counted', 2, 'ea-c'], ['if gate refusals counted', 3, 'ea-d']];
    const scale = (L.small ? 150 : 300) / delayMs(3);
    bars.forEach(([label, attempt, cls], n) => {
      const g = put(svg, `tk-bar${n}`, 'g', { at: [16, y + 16 + n * (L.small ? 40 : 30)] });
      put(g, 'l', 'text', { x: 0, y: 12, cls: 'ea-small', text: L.small ? label.replace('ergon-io: ', '') : label });
      const bx = L.small ? 0 : 230;
      const by = L.small ? 16 : 0;
      put(g, 'r', 'rect', { x: bx, y: by, width: delayMs(attempt) * scale, height: 16, rx: 3, cls });
      put(g, 'v', 'text', { x: bx + delayMs(attempt) * scale + 6, y: by + 12, cls: 'ea-small', text: `attempt ${attempt}: ${delayMs(attempt)} ms` });
    });
    put(svg, 'tk-s', 'text', { x: 16, y: y + (L.small ? 112 : 84), cls: 'ea-small', text: 'Source: Rate Limits and Errors (Limits); reconnect.rs' });
  }

  function legend(svg, put, L, part) {
    const items = [['ea-c', 'may resend / passed'], ['ea-d', 'may have run'], ['ea-b', 'a wait'], ['ea-a', part === 1 ? 'with the kernel' : 'next attempt'], ['ea-stroke-hl', 'changed']];
    items.forEach(([cls, label], n) => {
      const col = L.small ? n % 2 : n;
      const row = L.small ? Math.floor(n / 2) : 0;
      const g = put(svg, `lg-${n}`, 'g', { at: [16 + col * (L.small ? 170 : 140), L.H - (L.small ? 48 : 12) + row * 18] });
      put(g, 'r', 'rect', { x: 1, y: -9, width: 10, height: 10, rx: 2, cls });
      put(g, 't', 'text', { x: 16, y: 0, cls: 'ea-small', text: label });
    });
  }

  // The runtime has no fade-in for new elements, so this module adds one
  // (and turns it off under reduced motion).
  function ensureStyle() {
    if (document.getElementById('cg-style')) return;
    const s = document.createElement('style');
    s.id = 'cg-style';
    s.textContent = '@keyframes cg-in{from{opacity:0}to{opacity:1}}'
      + '.ergon-anim svg .cg-in{animation:cg-in 0.45s ease both}'
      + '@media (prefers-reduced-motion: reduce){.ergon-anim svg .cg-in{animation:none}}';
    document.head.appendChild(s);
  }

  // One step's whole state, drawn with keyed put() into the root group.
  function drawStep(svg, put, L, st, idx) {
    if (st.part === 3) { drawTakeaway(svg, put, L); return; }
    if (L.small) {
      // Narrow: op and math each wrap onto at most two lines.
      wrap(put, svg, 'op', 16, 14, st.op, 54, 11, 'ea-small', 'font-weight:bold');
      wrap(put, svg, 'math', 16, 38, st.math, 66, 9, 'ea-tiny');
    } else {
      put(svg, 'op', 'text', { x: 16, y: 18, cls: 'ea-title', text: st.op });
      put(svg, 'math', 'text', { x: 16, y: 34, cls: 'ea-tiny', text: st.math });
    }
    drawLanes(svg, put, L);
    drawArrows(svg, put, L, st, idx);
    if (st.part === 1) drawRequests(svg, put, L, st);
    else drawTimeline(svg, put, L, st);
    if (st.cost && L.small) wrap(put, svg, 'cost', 16, L.H - 78, st.cost, 62, 10, 'ea-tiny');
    else if (st.cost) put(svg, 'cost', 'text', { x: 16, y: L.H - 34, cls: 'ea-small', text: st.cost });
    legend(svg, put, L, st.part);
  }

  Anim.register('connect-gate', (stage, api) => {
    ensureStyle();
    // The figure (stage's parent) decides the layout: under 560 px wide,
    // padding included, it takes the narrow one.
    const isNarrow = () => stage.parentElement.clientWidth < NARROW;
    let narrow = isNarrow();
    const svg = api.svg(layout(narrow).W, layout(narrow).H, TITLE);
    // Every drawing goes in one monospace, 12-unit group; texts set smaller
    // or larger sizes themselves.
    const root = api.el('g', { class: 'ea-mono', 'font-size': 12 }, svg);
    const k = keyed();
    const draw = (st, i) => {
      k.begin();
      drawStep(root, k.put, layout(narrow), st, i);
      k.sweep(root);
    };
    return {
      steps: buildSteps().map((st, i) => ({ caption: st.caption, ms: st.hold || 2600, show: () => draw(st, i) })),
      onResize() {
        if (isNarrow() === narrow) return false;
        narrow = !narrow;
        const L = layout(narrow);
        svg.setAttribute('viewBox', `0 0 ${L.W} ${L.H}`);
        return true;
      },
    };
  });
})();
