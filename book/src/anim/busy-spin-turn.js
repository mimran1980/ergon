// One turn of the busy-spin loop: four bounded steps, then round again.
//
// Sources:
//   book/src/deploy/busy-spin-loop.md:11  1. network I/O: receive, run the handlers, flush sends; read burst
//   book/src/deploy/busy-spin-loop.md:12  2. Aeron in: subscription.poll_fn(handler, FRAGMENT_LIMIT)
//   book/src/deploy/busy-spin-loop.md:13  3. Aeron out: offer what the handlers queued; stop at the first retryable error
//   book/src/deploy/busy-spin-loop.md:14  4. timers: one TSC read (quanta) and a compare
//   book/src/deploy/busy-spin-loop.md:48-56  never in the loop: blocking calls, connect/TLS, allocation, host checks
//   samples/cluster-rfq/examples/rfq_client.rs:86  poll_fn(handler, limit) call shape (that loop is a
//                                          handshake with sleeps, so it is cited for the call only)
//   cluster/src/client.rs:881   offer: fast path try_claim, copy, commit (:903-:906), no heap; a refusal
//                               is returned to the caller, which keeps the message and retries
//   cluster/src/error.rs:97     retryable: NotConnected, BackPressured, AdminAction
//   cluster/src/client.rs:1177  poll(listener, fragment_limit): never blocks, call every turn
//   (not cluster/src/client.rs:1028 keep_alive_if_due: it reads Instant::now() and its send
//    retries inline with spin_loop, so it is not shown as the timer step)
// Recorded numbers: deploy/clocks.md:26-27 (median batch, slowest batch; single-read max not recorded),
// deploy/busy-spin-loop.md:49 (connect/TLS), deploy/shared-cpus.md:7-11 (one Azure Cobalt VM, helpers off its CPU; max at the hypervisor floor),
// findings.md:197 (recv-busy at 64 sockets, Graviton). Packet, fragment and tick counts are illustrative.
// The animation shows ordering, not timings.
Anim.register('busy-spin-turn', (stage, api) => {
  const el = api.el;
  const CHIP_W = 26;
  const CHIP_H = 18;
  const PITCH = 30;
  const WIDE = { W: 400, cx: 114, cy: 126, r: 74, nodeW: 80, px: 238, pw: 160, y0: 48, rowH: 54, side: 40, narrow: false };
  const NARROW = { W: 300, cx: 150, cy: 126, r: 70, nodeW: 76, px: 0, pw: 300, y0: 234, rowH: 54, side: 96, narrow: true };
  // Station k sits at angle k * 90 degrees clockwise from the top.
  const STATIONS = [
    { name: '1 · network I/O', bound: 'read burst', cls: 'ea-k1' },
    { name: '2 · Aeron in', bound: 'FRAGMENT_LIMIT', cls: 'ea-k3' },
    { name: '3 · Aeron out', bound: 'retryable error', cls: 'ea-k5' },
    { name: '4 · timers', bound: 'read + compare', cls: 'ea-k4' },
  ];

  let g;
  let svg;
  let head;
  let call;
  let nodes;
  let cursor;
  let turnText;
  let chips;
  let notes;
  let limitBar;
  let pubBox;
  let pubLabel;
  let lines;
  let side;
  let sideChips;

  function text(x, y, size, cls, str, parent, anchor) {
    const attrs = { x, y, 'font-size': size, class: cls || '' };
    if (anchor) attrs['text-anchor'] = anchor;
    return el('text', attrs, parent || svg, str === undefined ? '' : str);
  }

  // A chip is a group moved with style.transform so CSS transitions slide it.
  function chip(id, cls, label, row) {
    const grp = el('g', {}, svg);
    el('rect', { x: 0, y: 0, width: CHIP_W, height: CHIP_H, rx: 3, class: cls }, grp);
    text(CHIP_W / 2, 12.5, 9, 'ea-mono', label, grp, 'middle');
    grp.style.opacity = '0';
    chips.set(id, { grp, rect: grp.firstChild, row, cls });
  }

  function rowY(i) { return g.y0 + i * g.rowH; }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    const sideY = rowY(4) + g.side;
    const H = sideY + (narrow ? 96 : 60);
    svg = api.svg(g.W, H, 'The busy-spin loop as a ring of four bounded steps, with the data each step touches');
    // Labels sit on highlighted boxes: the shared 0.6 fill gives under 3:1 text contrast on dark themes.
    svg.classList.add('ea-bst');
    el('style', {}, svg, '.ergon-anim svg.ea-bst .ea-hot { fill-opacity: 0.22; }');
    nodes = [];
    chips = new Map();
    notes = {};

    head = text(0, 13, 12, 'ea-strong');
    call = text(0, 28, narrow ? 8.5 : 9.5, 'ea-mono');

    // The ring, with a clockwise arrowhead between each pair of stations.
    el('circle', { cx: g.cx, cy: g.cy, r: g.r, fill: 'none', stroke: 'currentColor', 'stroke-opacity': 0.35, 'stroke-width': 2 }, svg);
    for (let k = 0; k < 4; k += 1) {
      const a = (k * 90 + 45) * Math.PI / 180;
      const x = g.cx + g.r * Math.sin(a);
      const y = g.cy - g.r * Math.cos(a);
      const t = el('polygon', { points: '-4,-4 4,0 -4,4', fill: 'currentColor', 'fill-opacity': 0.5 }, svg);
      t.setAttribute('transform', `translate(${x} ${y}) rotate(${k * 90 + 45})`);
    }
    // The cursor: a bold arc for the quarter just travelled, ending at the top
    // station, rotated round the centre (between the stations, so never hidden).
    cursor = el('g', {}, svg);
    const pt = (deg) => {
      const a = deg * Math.PI / 180;
      return `${(g.cx + g.r * Math.sin(a)).toFixed(1)} ${(g.cy - g.r * Math.cos(a)).toFixed(1)}`;
    };
    el('path', { d: `M${pt(-62)} A${g.r} ${g.r} 0 0 1 ${pt(-40)}`, fill: 'none', stroke: 'currentColor', 'stroke-width': 4, 'stroke-linecap': 'round' }, cursor);
    const tip = el('polygon', { points: '-6,-6 6,0 -6,6', fill: 'currentColor' }, cursor);
    tip.setAttribute('transform', `translate(${pt(-38)}) rotate(-38)`);
    cursor.style.transformBox = 'view-box';
    cursor.style.transformOrigin = `${g.cx}px ${g.cy}px`;
    turnText = text(g.cx, g.cy - 2, 12, 'ea-strong', '', svg, 'middle');
    text(g.cx, g.cy + 12, 8.5, 'ea-muted', 'pinned CPU', svg, 'middle');

    STATIONS.forEach((s, k) => {
      const a = k * 90 * Math.PI / 180;
      const x = g.cx + g.r * Math.sin(a) - g.nodeW / 2;
      const y = g.cy - g.r * Math.cos(a) - 15;
      const rect = el('rect', { x, y, width: g.nodeW, height: 30, rx: 4, class: s.cls }, svg);
      text(x + g.nodeW / 2, y + 12, 9, 'ea-strong', s.name, svg, 'middle');
      text(x + g.nodeW / 2, y + 24, g.narrow ? 8 : 7.5, 'ea-mono', s.bound, svg, 'middle');
      nodes.push(rect);
    });

    // Data each step touches, one row per station.
    const px = g.px;
    const labels = ['socket receive', 'Aeron image (term buffer)', 'outbox (reserved)', 'timer (illustrative ticks)'];
    labels.forEach((l, i) => text(px, rowY(i), 9, 'ea-strong', l));
    ['rx', 'img', 'out', 'tmr'].forEach((k, i) => {
      notes[k] = text(px, rowY(i) + (k === 'tmr' ? 14 : 38), 8.5, k === 'tmr' ? 'ea-mono' : 'ea-muted');
    });
    limitBar = el('g', {}, svg);
    el('path', { d: `M${px} ${rowY(1) + 27} v3 h${3 * PITCH - 4} v-3`, fill: 'none', stroke: 'currentColor', 'stroke-width': 1.5 }, limitBar);
    const pubX = px + g.pw - 60;
    pubBox = el('rect', { x: pubX, y: rowY(2) + 4, width: 60, height: CHIP_H + 2, rx: 3, class: 'ea-box' }, svg);
    pubLabel = text(pubX + 60, rowY(2), 8.5, 'ea-muted', 'publication', svg, 'end');
    g.pubX = pubX;

    ['p1', 'p2'].forEach((id) => chip(id, 'ea-k1', id, 0));
    ['f1', 'f2', 'f3', 'f4', 'f5'].forEach((id) => chip(id, 'ea-k3', id, 1));
    ['m1', 'm2', 'm3'].forEach((id) => chip(id, 'ea-k5', id, 2));

    // Cost lines under the panel, wrapped.
    lines = el('g', {}, svg);
    g.linesY = rowY(4) - 12;

    // Side lane: what never enters the ring.
    side = el('g', {}, svg);
    el('rect', { x: 0, y: sideY, width: g.W, height: H - sideY - 2, rx: 5, fill: 'none', stroke: 'currentColor', 'stroke-opacity': 0.45, 'stroke-dasharray': '4 3' }, side);
    text(6, sideY + 13, 9, 'ea-strong', "Never in the loop · helpers run off the loop's CPU", side);
    const items = [
      ['allocation', 'before the loop'],
      ['blocking call, connect, TLS', 'helper thread'],
      ['host checks, watches', 'housekeeping thread'],
    ];
    sideChips = items.map(([what, where], i) => {
      let x; let y; let w;
      if (narrow) { x = 6; y = sideY + 20 + i * 24; w = g.W - 12; } else { x = 6 + i * 130; y = sideY + 20; w = 124; }
      const rect = el('rect', { x, y, width: w, height: narrow ? 20 : 32, rx: 3, class: 'ea-k2' }, side);
      if (narrow) {
        text(x + 5, y + 14, 8.5, '', `${what} → ${where}`, side);
      } else {
        text(x + 5, y + 13, 8.5, 'ea-strong', what, side);
        text(x + 5, y + 26, 8.5, '', `→ ${where}`, side);
      }
      return rect;
    });
  }

  function place(id, x, y, mode) {
    const c = chips.get(id);
    c.grp.style.transform = `translate(${x}px, ${y}px)`;
    c.grp.style.opacity = mode === 'gone' ? '0' : '1';
    // Back-pressured: the refusal colour of the full publication, dashed so it reads without colour.
    c.rect.setAttribute('class', mode === 'bp' ? 'ea-k2 ea-hot' : `${c.cls}${mode === 'hot' ? ' ea-hot' : ''}`);
    c.rect.style.strokeDasharray = mode === 'bp' ? '4 2' : '';
  }

  function wrap(str, maxChars) {
    const out = [];
    let line = '';
    for (const word of str.split(' ')) {
      if (line && (line.length + 1 + word.length) > maxChars) { out.push(line); line = word; } else line = line ? `${line} ${word}` : word;
    }
    if (line) out.push(line);
    return out;
  }

  // Every chip has a fixed home row; s.rx / s.img / s.out list [id, mode] in slot order.
  // Chips not listed keep their last position and fade out; s.pub lists chips shown in the publication.
  const last = new Map();
  function render(s) {
    head.textContent = s.head;
    call.textContent = s.call || '';
    nodes.forEach((n, k) => n.classList.toggle('ea-hot', s.step === k + 1));
    const ring = s.step ? (s.turn - 1) * 360 + (s.step - 1) * 90 : 0;
    cursor.style.transform = `rotate(${ring}deg)`;
    cursor.style.opacity = s.step ? '1' : '0';
    turnText.textContent = s.turn ? `turn ${s.turn}` : 'turn';
    const seen = new Set();
    const lay = (list, row) => (list || []).forEach(([id, mode], i) => {
      const pos = { x: g.px + i * PITCH, y: rowY(row) + 6 };
      last.set(id, pos);
      seen.add(id);
      place(id, pos.x, pos.y, mode);
    });
    lay(s.rx, 0);
    lay(s.img, 1);
    lay(s.out, 2);
    (s.pub || []).forEach((id, i) => {
      const pos = { x: g.pubX + 3 + i * 28, y: rowY(2) + 5 };
      last.set(id, pos);
      seen.add(id);
      place(id, pos.x, pos.y, 'hot');
    });
    for (const id of chips.keys()) {
      if (seen.has(id)) continue;
      const pos = last.get(id) || { x: g.px, y: rowY(chips.get(id).row) + 6 };
      place(id, pos.x, pos.y, 'gone');
    }
    limitBar.style.opacity = s.limit ? '1' : '0';
    // Full: refusal-colour outline only; a fill under the hot m1 chip would stack and drop its label's contrast.
    pubBox.setAttribute('class', s.bp ? 'ea-box ea-hot' : 'ea-box');
    pubBox.style.stroke = s.bp ? 'var(--ea-c2)' : '';
    pubLabel.textContent = s.bp ? 'publication: full' : 'publication';
    notes.rx.textContent = s.rxNote || '';
    notes.img.textContent = s.imgNote || '';
    notes.out.textContent = s.outNote || '';
    notes.tmr.textContent = s.tmr || 'due at 1,250';

    lines.textContent = '';
    const maxChars = g.narrow ? 62 : 84;
    let y = g.linesY;
    for (const [label, body] of s.cost || []) {
      wrap(`${label}: ${body}`, maxChars).forEach((l, i) => {
        text(0, y, 8.5, '', l, lines);
        y += 11;
      });
    }
    sideChips.forEach((c, i) => {
      c.classList.toggle('ea-hot', !!s.side);
      c.style.setProperty('--d', String(i));
    });
  }

  const POLL = 'subscription.poll_fn(handler, FRAGMENT_LIMIT)';
  const IMG5 = [['f1', 'on'], ['f2', 'on'], ['f3', 'on'], ['f4', 'on'], ['f5', 'on']];
  const states = [
    {
      ms: 7000, head: 'The duty cycle', call: 'loop { io; aeron_in; aeron_out; timers }',
      img: IMG5, imgNote: '5 fragments waiting',
      cost: [['shown', 'order, not timings; packet, fragment and tick counts are illustrative']],
      caption: 'One thread on one pinned CPU runs this ring forever: network I/O, Aeron in, Aeron out, timers, then round again with no sleep. Each step has a bound, so no turn can run long. Packet, fragment and tick counts here are illustrative.',
    },
    {
      ms: 6000, turn: 1, step: 1, head: 'Turn 1 · network I/O', call: 'poll the socket: receive, handlers, flush',
      rx: [['p1', 'hot'], ['p2', 'hot']], rxNote: '2 read, at most the read burst',
      img: IMG5, imgNote: '5 fragments waiting', out: [['m1', 'hot']], outNote: 'handler queued m1',
      cost: [['touches', 'one non-blocking read per socket into buffers reserved before the loop; the handlers run in this turn'], ['bound', "the backend's read burst"]],
      caption: 'Turn 1, network I/O: a non-blocking read takes what has arrived, at most the read burst. The handlers run on both packets now; one queues m1 for Aeron in the outbox, reserved before the loop so nothing allocates. Pending sends are flushed.',
    },
    {
      ms: 6500, turn: 1, step: 2, head: 'Turn 1 · Aeron in', call: POLL, limit: true,
      img: [['f1', 'hot'], ['f2', 'hot'], ['f3', 'hot'], ['f4', 'on'], ['f5', 'on']], imgNote: 'limit 3: read 3, 2 stay for turn 2',
      out: [['m1', 'on'], ['m2', 'hot'], ['m3', 'hot']], outNote: 'handler queued m2, m3',
      cost: [['touches', "the image's term buffer, shared memory mapped from the media driver; the subscriber position moves on"], ['bound', 'FRAGMENT_LIMIT fragments, then poll_fn returns']],
      caption: "Aeron in: poll_fn(handler, FRAGMENT_LIMIT) reads fragments straight from the image's term buffer. Five wait but the limit is 3, so it reads 3 and returns; the other 2 stay in the log. The handler queues m2 and m3.",
    },
    {
      ms: 7000, turn: 1, step: 3, head: 'Turn 1 · Aeron out', call: 'publication.offer(m1) ok; offer(m2) → BackPressured',
      img: [['f4', 'on'], ['f5', 'on']], imgNote: '2 left for turn 2',
      out: [['m2', 'bp'], ['m3', 'on']], pub: ['m1'], bp: true, outNote: 'm2 back-pressured: m2, m3 wait',
      cost: [['touches', "offer's fast path claims space in the publication's term buffer, copies, commits: no heap"], ['bound', 'stops at the first retryable error (NotConnected, BackPressured, AdminAction)']],
      caption: 'Aeron out: offer the outbox in order. m1 is claimed, copied and committed. m2 gets BackPressured, a retryable error, so the loop stops offering at once and keeps m2 and m3 for the next turn rather than wait for the driver.',
    },
    {
      ms: 6500, turn: 1, step: 4, head: 'Turn 1 · timers', call: 'clock.now() >= deadline ?',
      img: [['f4', 'on'], ['f5', 'on']], imgNote: '2 left for turn 2', out: [['m2', 'on'], ['m3', 'on']], outNote: 'carried over to turn 2',
      tmr: 'now 1,000 < 1,250: not due',
      cost: [['touches', 'one TSC read and one compare'], ['recorded', 'c7i.2xlarge, median (slowest) batch: quanta Clock::now() 15.99 (16.95) ns, Instant::now() 24.85 (25.99) ns; single-read max not recorded']],
      caption: 'Timers: one TSC read with quanta and a compare. Nothing is due, so the step costs the read: on c7i.2xlarge the median batch for quanta is 15.99 ns (slowest batch 16.95 ns) against 24.85 ns (25.99 ns) for Instant::now(); the slowest single read is not recorded. The turn ends and the next starts at once.',
    },
    {
      ms: 5000, turn: 2, step: 1, head: 'Turn 2 · network I/O', call: 'poll the socket: would block',
      rxNote: 'nothing arrived: returns at once',
      img: [['f4', 'on'], ['f5', 'on']], imgNote: '2 left from turn 1', out: [['m2', 'on'], ['m3', 'on']], outNote: 'carried over from turn 1',
      cost: [['touches', 'one non-blocking read that finds nothing'], ['bound', "the backend's read burst"]],
      caption: 'Turn 2, network I/O: nothing has arrived. The non-blocking read returns at once and the loop moves on. An empty step is cheap, which keeps the turn short and the wait for the next message short too.',
    },
    {
      ms: 5500, turn: 2, step: 2, head: 'Turn 2 · Aeron in', call: POLL, limit: true,
      img: [['f4', 'hot'], ['f5', 'hot']], imgNote: 'read 2, under the limit',
      out: [['m2', 'on'], ['m3', 'on']], outNote: 'carried over from turn 1',
      cost: [['touches', "the image's term buffer from the saved position"], ['bound', 'FRAGMENT_LIMIT; 2 is under it']],
      caption: 'Aeron in: the 2 fragments left from turn 1 are read now, under the limit. The bound delayed them by one short turn; it did not drop them, because they waited in the log, not in the loop.',
    },
    {
      ms: 6500, turn: 2, step: 3, head: 'Turn 2 · Aeron out', call: 'publication.offer(m2) ok; offer(m3) ok',
      pub: ['m2', 'm3'], outNote: 'outbox empty',
      cost: [['touches', "claim, copy and commit into the publication's term buffer"], ['bound', 'stops at the first retryable error; none this time']],
      caption: 'Aeron out: the carried-over messages go first, in the order they were queued. The publication has room again, so m2 and m3 are committed. Nothing waited inside the loop for this: the other sources were polled meanwhile.',
    },
    {
      ms: 6000, turn: 2, step: 4, head: 'Turn 2 · timers', call: 'clock.now() >= deadline → fire',
      tmr: 'now 1,260 ≥ 1,250: due, fire',
      cost: [['touches', 'one TSC read, one compare, then the timer\'s own work'], ['bound', "the timer's own work"]],
      caption: "Timers: this time the compare finds the deadline passed, so the timer's work runs, bounded by that work. A timer's work must follow the same rules as the rest of the turn: no blocking, and a retryable refusal is kept for the next turn.",
    },
    {
      ms: 7500, head: 'Never in the loop', call: 'start-up, helper thread, housekeeping thread', side: true,
      cost: [['why', 'each of these can stop the thread: a connect or TLS handshake is about 0.7 ms of CPU, an allocation can lock and fault']],
      caption: 'What never enters the ring: allocation (reserve before the loop), blocking calls such as DNS, file I/O, locks and sleep, and connect or TLS handshakes at about 0.7 ms of CPU each (a helper thread), and host checks such as ThrottleWatch and FaultWatch (the housekeeping thread). Helpers run on a CPU other than the loop\'s: the node\'s shared CPU, or the pod\'s second CPU.',
    },
    {
      ms: 9000, head: 'Why it wins here, when it loses', call: 'bounded steps + carry-over = no spikes',
      cost: [
        ['wins', 'one Azure Cobalt VM, helpers off its CPU: p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs (the hypervisor floor)'],
        ['loses', 'it burns a whole CPU even when idle; cpu: 1 puts helpers on the loop\'s CPU; plain recv-busy at 64 sockets (Graviton) costs about 0.6 µs a socket a poll'],
      ],
      caption: 'Why it wins: every step is bounded and back-pressure carries over instead of waiting, so a turn stays short; on one Azure Cobalt VM with helpers off its CPU the loop measured p50 0.02 µs, p99.99 0.03 µs, max 4.81 to 5.52 µs, the max being the 3.5 to 5.5 µs floor of the hypervisor. When it loses: it burns a whole CPU, a cpu: 1 pod puts the helpers on its CPU, and polling many sources every turn adds up (about 0.6 µs a socket a poll at 64 sockets on Graviton).',
    },
  ];

  build(false);

  return {
    steps: states.map((s) => ({ caption: s.caption, ms: s.ms, show: () => render(s) })),
    // Rebuild on crossing the phone breakpoint; true asks the runtime to redraw the step.
    onResize(width) {
      const narrow = width < 520;
      if (narrow === g.narrow) return false;
      last.clear();
      build(narrow);
      return true;
    },
  };
});
