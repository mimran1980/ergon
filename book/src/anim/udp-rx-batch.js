// Generated file: edits here are overwritten by the next export. sha256:f6a11f8d1aa8d3caa2128f261bdded255dc3e179aea1980752e556fe82151713
// UDP batching: one poll drains up to rx_batch datagrams a receive call, and
// the handler replies inline or queues for the send pass.
// Sources: every counter and slot below comes from a port of this code, run in order.
//   ergon-io/src/driver/mod.rs:2047-2105  recv_batch: want = min(left, rx_batch); short call ends the burst
//   ergon-io/src/sys.rs:882-901           arm_rx: slot k at k * max_datagram, at most rx_batch entries
//   ergon-io/src/sys.rs:949-978           recvmmsg: MSG_DONTWAIT, Ok(None) on EAGAIN
//   ergon-io/src/driver/epoll.rs:300-339  recv_index: datagram_burst per ready socket per poll
//   ergon-io/src/driver/epoll.rs:394-400  wait: epoll_wait_calls
//   ergon-io/src/driver/mod.rs:1359-1388  poll: receive pass, then the send pass
//   ergon-io/src/driver/mod.rs:1733-1793  send_datagrams: two or more in one sendmmsg, one alone
//   ergon-io/src/driver/mod.rs:1549-1568  flush: ConnMut::send replies queued after poll leave here
//   ergon-io/src/udp.rs:105-132           UdpTx::send: nothing queued, one send now; else queued behind
//   ergon-io/src/udp.rs:335-343           receive buffer rx_batch x max_datagram, written at build
//   ergon-io/src/udp.rs:516-527           on_datagram_at: slice of slot, rx += 1 after the handler
//   ergon-io/src/backend.rs:279-284       datagram_burst: 16 by default
// Figures: book/src/ergon-io/udp.md:67-97 (32 datagrams queued, rx_batch 1 to 32)
// and :141-156 (queued, inline and mio replies, bursts of 8).
// Illustrative: datagram arrival, burst sizes, 64-byte payloads.
(() => {
  'use strict';
  const MAX_DGRAM = 2048;
  const BURST = 16; // Config::datagram_burst default
  const RING_SHOWN = 8; // of tx_slots 64

  // A port of the driver's datagram path for one connected socket.
  function sock(rx) {
    return {
      rx, queue: [], slots: new Array(rx).fill(null), ring: [], tape: [], next: 1,
      st: { polls: 0, epoll_wait_calls: 0, recvmsg_calls: 0, recvmsg_eagain: 0, completions: 0, send_calls: 0, sends: 0 },
      udp: { rx: 0, tx: 0 },
      handler: null, d1Copied: null, copied: 0, replyAfter: null,
    };
  }
  function arrive(m, n) { for (let k = 0; k < n; k += 1) { m.queue.push(m.next); m.next += 1; } }

  // Driver::poll on one socket: epoll_wait, then recv_batch up to datagram_burst,
  // delivering each datagram as the call returns, then the send pass.
  function* poll(m, reply, burst) {
    m.st.polls += 1;
    m.tape = [];
    m.st.epoll_wait_calls += 1;
    m.tape.push({ k: 'ep', t: 'epoll_wait' });
    if (!m.queue.length) return;
    let left = burst;
    while (left > 0) {
      const want = Math.min(left, m.rx);
      m.st.recvmsg_calls += 1;
      if (!m.queue.length) {
        m.st.recvmsg_eagain += 1;
        m.tape.push({ k: 'again', t: 'recvmmsg→-1' });
        yield { ev: 'eagain', want };
        break;
      }
      const got = Math.min(want, m.queue.length);
      for (let k = 0; k < got; k += 1) m.slots[k] = { id: m.queue.shift(), s: 'new' };
      m.copied += got;
      m.tape.push({ k: 'rx', t: `recvmmsg→${got}` });
      if (got < want) m.st.recvmsg_eagain += 1;
      yield { ev: 'call', want, got };
      for (let k = 0; k < got; k += 1) {
        const d = m.slots[k];
        m.st.completions += 1;
        if (d.id === 1) m.d1Copied = got;
        m.slots.forEach((o) => { if (o && o.s === 'new') o.s = 'wait'; });
        d.s = 'done';
        d.n = m.st.completions;
        m.handler = { id: d.id, slot: k, reply };
        if (reply === 'inline') {
          if (m.ring.length === 0) {
            m.st.send_calls += 1;
            m.st.sends += 1;
            m.udp.tx += 1;
            m.tape.push({ k: 'tx', t: 'send' });
            if (m.replyAfter === null) m.replyAfter = m.st.completions;
            m.handler.sent = 'Now';
          } else {
            m.ring.push(d.id);
            m.handler.sent = 'Queued';
          }
        }
        m.udp.rx += 1;
        yield { ev: 'deliver', id: d.id };
      }
      if (got < want) break;
      left -= got;
    }
    m.handler = null;
    // The pass after the receive pass: only UdpTx replies that came back Queued.
    if (m.ring.length) { sendPass(m); yield { ev: 'sendpass' }; }
  }
  // send_datagrams: two or more queued go in one sendmmsg, one goes alone.
  function sendPass(m) {
    const n = Math.min(m.ring.length, 64);
    m.st.send_calls += 1;
    m.st.sends += n;
    m.udp.tx += n;
    m.tape.push({ k: 'tx', t: n >= 2 ? `sendmmsg×${n}` : 'send' });
    if (m.replyAfter === null) m.replyAfter = m.st.completions;
    m.ring.splice(0, n);
  }
  // Runs `g` until `stop(event)` holds (or to its end).
  function until(g, stop) { for (let r = g.next(); !r.done; r = g.next()) if (stop && stop(r.value)) return r.value; return null; }
  const snap = (m) => JSON.parse(JSON.stringify(m));

  function buildSteps() {
    const steps = [];
    let scene = 0;
    let prev = null;
    const push = (m, op, set, caption, extra) => {
      const s = Object.assign({ scene, op, set, caption, m: snap(m), prevSt: prev }, extra);
      prev = Object.assign({}, m.st, m.udp, { tapeLen: m.tape.length, scene });
      steps.push(s);
    };
    const setTxt = (rx, more) => `rx_batch ${rx} · datagram_burst ${BURST}${more || ''}`;

    // Scene 0: a lone datagram, default rx_batch.
    let m = sock(16);
    push(m, 'UdpConn::connect(…, rx_batch: 16)', setTxt(16, ' · max_datagram 2,048'),
      'Sixteen receive slots of 2,048 bytes: 32 KiB, written at build so the first read takes no page faults. The kernel holds arriving datagrams in the socket’s receive queue until a poll asks for them.');
    arrive(m, 1);
    let g = poll(m, 'inline', BURST);
    until(g, (e) => e.ev === 'call');
    push(m, 'poll: recvmmsg asks for 16, gets 1', setTxt(16),
      'A lone datagram. epoll_wait reports the socket; one recvmmsg asks for up to 16 and gets 1 at once. rx_batch is a cap, not a wait. A short call has already made the read that finds the queue empty, so recvmsg_eagain counts it without a second call.');
    until(g, (e) => e.ev === 'deliver');
    push(m, 'on_datagram(d1) → UdpTx::send', setTxt(16),
      'The handler gets a slice of slot 0 in place: no copy out. It replies on tx: nothing is queued on the socket, so the reply is one send there and then (UdpSent::Now). rx counts after the handler returns.');
    until(g);

    // Scene 1: a burst of 8, rx_batch 1.
    scene += 1; prev = null;
    m = sock(1);
    arrive(m, 8);
    g = poll(m, 'inline', BURST);
    until(g, (e) => e.ev === 'deliver');
    push(m, 'burst of 8 · rx_batch 1: first call', setTxt(1),
      'Eight datagrams queued, one slot. Each recvmmsg takes one, so the handler sees d1 after a single copy and its reply leaves as the poll’s third system call. That is the lowest first-datagram latency.');
    until(g);
    push(m, 'burst of 8 · rx_batch 1: the rest', setTxt(1),
      'The cost comes later: one recvmmsg per datagram, and the eighth call came back full, so a ninth found the queue empty (EAGAIN). Nine receive calls for eight datagrams: d8 is handled after the eighth, and the empty ninth comes after it. Slot 0 was overwritten each time.');

    // Scene 2: the same burst, rx_batch 16.
    scene += 1; prev = null;
    m = sock(16);
    arrive(m, 8);
    g = poll(m, 'inline', BURST);
    until(g, (e) => e.ev === 'call');
    push(m, 'burst of 8 · rx_batch 16: one call', setTxt(16),
      'The same burst with 16 slots. One recvmmsg copies all eight, each to the start of its own 2,048-byte slot with its own timestamp, before the handler sees any. The call came back short (8 of 16), so the queue is known empty: one call in all.');
    until(g, (e) => e.ev === 'deliver');
    push(m, 'on_datagram(d1): after 8 copies', setTxt(16),
      'd1 is handled only now, after the kernel copied the whole batch: a larger batch delays the first datagram. Its inline reply still goes before d2 is delivered.');
    until(g);
    push(m, 'on_datagram(d2…d8), inline replies', setTxt(16),
      'The handler walks the slots in order, reading each in place and replying with one send each. One receive call for the whole burst, so d8 is reached sooner than with one slot.');

    // Scene 3: rx_batch equal to the burst.
    scene += 1; prev = null;
    m = sock(8);
    arrive(m, 8);
    until(poll(m, 'inline', BURST));
    push(m, 'burst of 8 · rx_batch 8: a full call', setTxt(8),
      'A catch: with rx_batch 8 the call comes back full, so the driver cannot know the queue is empty and, with datagram_burst left, reads again and gets EAGAIN. Two receive calls where rx_batch 16 made one.');

    // Scene 4: queued replies: recorded in the handler, sent after poll returns.
    scene += 1; prev = null;
    m = sock(16);
    arrive(m, 8);
    until(poll(m, 'record', BURST));
    m.handler = { text: 'poll returned', sub: 'the handler noted 8 replies to send' };
    m.tape.push({ k: 'ret', t: 'poll returns' }); // not a system call: the boundary before flush
    push(m, 'poll returns: 8 handled, no reply yet', setTxt(16, ' · tx_slots 64'),
      'Queued replies instead. The handler cannot call ConnMut::send during poll (it needs the driver), so it only notes what to answer. The poll made no send call, and no reply has left.');
    for (let id = 1; id <= 8; id += 1) m.ring.push(id);
    m.handler = { text: 'application, after poll', sub: 'ConnMut::send × 8 → send slots' };
    push(m, 'ConnMut::send × 8, after poll', setTxt(16, ' · tx_slots 64'),
      'After poll returns, the application sends each reply with ConnMut::send: each is copied into a preallocated send slot (tx_slots 64). Still no system call.');
    m.handler = { text: 'Driver::flush', sub: 'the queued replies leave in one call' };
    sendPass(m);
    push(m, 'Driver::flush: one sendmmsg', setTxt(16, ' · tx_slots 64'),
      'flush (or the next poll, before its receive pass) hands all eight to the kernel in one sendmmsg: one recvmmsg and one sendmmsg for the burst, against one recvmmsg and eight sends inline. But d1’s reply waited for the whole poll.');

    // Scene 5: datagram_burst splits a larger burst across polls.
    scene += 1; prev = null;
    m = sock(16);
    arrive(m, 20);
    until(poll(m, 'none', BURST));
    push(m, 'poll 1: 20 queued, datagram_burst 16', setTxt(16),
      'Twenty queued. One full call takes 16, which uses up datagram_burst, so this poll leaves the socket with no empty read: other ready sockets get their turn. epoll is level-triggered, so the 4 left are reported again.');
    until(poll(m, 'none', BURST));
    push(m, 'poll 2: the 4 left', setTxt(16),
      'The next poll: epoll_wait reports the socket again and a short call takes the last 4. Keep datagram_burst a multiple of rx_batch (both 16 by default) and raise both together for feeds that burst harder.');

    steps.push({ figures: 'batch', hold: 9000, op: 'Choosing rx_batch', caption: 'The page’s sweep: 32 datagrams of 64 bytes queued, time from the poll to the handler seeing the first and the last (datagram_burst 64, so one poll takes them all). Each extra datagram in a batch costs about 0.35 µs before the handler runs and saves about 0.7 µs against its own call. Act on the first update: 4 to 8. On the whole burst: 16, the default.' });
    steps.push({ figures: 'reply', hold: 9000, op: 'Replying inline', caption: 'Between two Graviton hosts, bursts of 8: inline replies were 1.15 µs lower at p50 than queued ones, because the first reply does not wait for the rest of the burst to be handled. It still waits for recvmmsg to copy the batch, and mio, a call a datagram, stayed lower. For a lone datagram the three were not separated. Every max is unexplained, so none is compared.' });
    return steps;
  }

  const TITLE = 'UDP batching: rx_batch, datagram_burst and replying inline';
  const NARROW = 560; // px of the figure, padding included: under it, the narrow layout
  const SVG_NS = 'http://www.w3.org/2000/svg';

  // Fills and outlines in anim.css's vocabulary. The outline-only kinds need
  // fill:none, which no anim.css class sets (an SVG rect fills black by default).
  // Every entry sets cls and style, so a keyed element that changes kind clears the old one.
  const PAINT = {
    held: { cls: 'ea-k1' },
    written: { cls: 'ea-k2' },
    green: { cls: 'ea-k3' },
    purple: { cls: 'ea-k4' },
    ghost: { style: 'fill:var(--table-header-bg);opacity:0.6' },
    empty: { style: 'fill:none;stroke:var(--table-header-bg)' },
    box: { cls: 'ea-box' },
    hidden: { cls: 'ea-hidden' },
    touched: { style: 'fill:none;stroke:var(--ea-c5);stroke-width:3' },
    scanned: { cls: 'ea-ok', style: 'fill:none' },
  };
  const paint = (kind) => Object.assign({ cls: undefined, style: undefined }, PAINT[kind]);
  // Text sizes in viewBox units; the root group sets 12 and the monospace font.
  const TEXT = {
    title: { cls: 'ea-strong', 'font-size': 14 },
    plain: { cls: undefined, 'font-size': undefined },
    small: { cls: undefined, 'font-size': 10 },
    smallDim: { cls: 'ea-muted', 'font-size': 10 },
    tiny: { cls: undefined, 'font-size': 8 },
    tinyDim: { cls: 'ea-muted', 'font-size': 8 },
  };

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
      for (const [k, v] of Object.entries(attrs || {})) {
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

  function layout(narrow) {
    if (narrow) {
      return { W: 360, H: 690, per: 10, cell: 30, gap: 3, qY: 62, rxY: 140, sY: 160, hY: 238, ringX: 16, ringY: 300, tY: 350, chipW: 78, chipPer: 4, cY: 488, cW: 106, cPer: 3, costY: 616, small: true };
    }
    return { W: 720, H: 492, per: 20, cell: 30, gap: 4, qY: 64, rxY: 116, sY: 138, hY: 188, ringX: 424, ringY: 198, tY: 258, chipW: 80, chipPer: 8, cY: 356, cW: 134, cPer: 5, costY: 444, small: false };
  }
  const cellXY = (L, row0, i) => [16 + (i % L.per) * (L.cell + L.gap), row0 + Math.floor(i / L.per) * (L.cell + 3)];

  const COUNTERS = [
    ['epoll_wait_calls', 'st'], ['recvmsg_calls', 'st'], ['recvmsg_eagain', 'st'], ['completions', 'st'], ['send_calls', 'st'],
    ['sends', 'st'], ['UdpStats rx', 'udp', 'rx'], ['UdpStats tx', 'udp', 'tx'], ['polls', 'st'],
  ];

  function drawSocket(svg, put, L, st) {
    const m = st.m;
    put(svg, 'op', 'text', { x: 16, y: 20, ...TEXT.title, text: st.op });
    put(svg, 'math', 'text', { x: 16, y: 38, ...TEXT.small, text: st.set });

    // The socket's receive queue (kernel memory): datagrams waiting.
    put(svg, 'q-l', 'text', { x: 16, y: L.qY - 8, ...TEXT.small, text: L.small ? 'socket receive queue (kernel)' : 'socket receive queue (kernel): datagrams waiting for a read' });
    // Receive slots: rx_batch x max_datagram, the slots recvmmsg writes into.
    const lastCall = [...m.tape].reverse().find((c) => c.k === 'rx' || c.k === 'again');
    put(svg, 'rx-l', 'text', { x: 16, y: L.rxY, ...TEXT.small, text: lastCall ? `last receive call: ${lastCall.t}${lastCall.k === 'again' ? ' (EAGAIN)' : ''}` : 'no receive call yet' });
    put(svg, 's-l', 'text', { x: 16, y: L.sY - 8, ...TEXT.small, text: L.small ? `slots: ${m.rx} × 2,048 B (${fmtKiB(m.rx)})` : `receive slots: rx_batch ${m.rx} × max_datagram 2,048 B = ${fmtKiB(m.rx)}, written at build` });
    for (let i = 0; i < m.rx; i += 1) {
      const [x, y] = cellXY(L, L.sY, i);
      put(svg, `slot-${i}`, 'rect', { x, y, width: L.cell, height: L.cell, rx: 3, ...paint('empty') });
    }
    // Datagrams keyed by scene and id: one moving from queue to slot slides.
    m.queue.forEach((id, i) => dgram(svg, put, L, st.scene, id, cellXY(L, L.qY, i), 'held', false));
    m.slots.forEach((d, i) => {
      if (!d) return;
      const before = st.prevSt ? st.prevSt.completions : 0;
      let kind = 'ghost';
      if (d.s === 'new') kind = 'written';
      else if (d.s === 'wait') kind = 'held';
      else if (d.n > before) kind = 'green';
      dgram(svg, put, L, st.scene, d.id, cellXY(L, L.sY, i), kind, m.handler && m.handler.id === d.id);
    });
    if (!m.queue.length) put(svg, 'q-empty', 'text', { x: 16, y: L.qY + 19, ...TEXT.smallDim, text: 'empty' });

    // The handler and the send slots.
    put(svg, 'h-l', 'text', { x: 16, y: L.hY, ...TEXT.small, text: 'handler' });
    const h = m.handler;
    let h1 = 'idle';
    let h2 = '';
    if (h && h.text) {
      h1 = h.text;
      h2 = h.sub;
    } else if (h) {
      h1 = `on_datagram(d${h.id}): slot ${h.slot}, read in place`;
      if (h.reply === 'inline') h2 = h.sent === 'Now' ? 'UdpTx::send → Now: one send' : 'UdpTx::send → Queued';
      else if (h.reply === 'record') h2 = 'notes the reply; no ConnMut::send during poll';
      else h2 = 'no reply';
    }
    if (L.small) h1 = h1.replace(', read in place', '');
    put(svg, 'h-1', 'text', { x: 16, y: L.hY + 18, text: h1 });
    put(svg, 'h-2', 'text', { x: 16, y: L.hY + 34, ...TEXT.small, text: h2 });
    put(svg, 'r-l', 'text', { x: L.ringX, y: L.ringY - 8 + (L.small ? 0 : -2), ...TEXT.small, text: `send slots: ${m.ring.length} queued (8 of 64 shown)` });
    for (let i = 0; i < RING_SHOWN; i += 1) {
      const x = L.ringX + i * (L.cell + L.gap);
      const id = m.ring[i];
      put(svg, `ring-${i}`, 'rect', { x, y: L.ringY, width: L.cell, height: L.cell - 6, rx: 3, ...paint(id ? 'held' : 'empty') });
      put(svg, `ring-t${i}`, 'text', { x: x + L.cell / 2, y: L.ringY + 16, 'text-anchor': 'middle', ...TEXT.tiny, text: id ? `r${id}` : '' });
    }

    // System calls this poll, in order: where the replies fall between the reads.
    put(svg, 't-l', 'text', { x: 16, y: L.tY - 8, ...TEXT.small, text: L.small ? 'system calls, in order' : 'system calls since this poll began, in order (left to right, then down)' });
    const fresh = st.prevSt && st.prevSt.scene === st.scene ? st.prevSt.tapeLen : 0;
    const newPoll = st.prevSt && st.m.st.polls !== st.prevSt.polls;
    m.tape.forEach((c, i) => {
      const x = 16 + (i % L.chipPer) * (L.chipW + 2);
      const y = L.tY + Math.floor(i / L.chipPer) * 24;
      const g = put(svg, `chip-${st.scene}-${st.m.st.polls}-${i}`, 'g', { at: [x, y] });
      const kind = c.k === 'ep' ? 'box' : c.k === 'ret' ? 'empty' : c.k === 'rx' ? 'held' : c.k === 'again' ? 'purple' : 'green';
      put(g, 'r', 'rect', { width: L.chipW - 2, height: 20, rx: 3, ...paint(kind) });
      put(g, 't', 'text', { x: (L.chipW - 2) / 2, y: 14, 'text-anchor': 'middle', ...TEXT.small, text: c.t });
      put(g, 'h', 'rect', { x: -2, y: -2, width: L.chipW + 2, height: 24, rx: 4, ...paint(!st.prevSt || i >= fresh || newPoll ? 'touched' : 'hidden') });
    });

    // Counters, as Driver::stats() and UdpConn::stats() would read now.
    put(svg, 'c-l', 'text', { x: 16, y: L.cY - 8, ...TEXT.small, text: 'counters (this scene)' });
    COUNTERS.forEach(([name, src, field], i) => {
      const v = src === 'st' ? m.st[name] : m.udp[field];
      const was = st.prevSt ? (src === 'st' ? st.prevSt[name] : st.prevSt[field]) : 0;
      const x = 16 + (i % L.cPer) * (L.cW + 3);
      const y = L.cY + Math.floor(i / L.cPer) * 36;
      const g = put(svg, `cnt-${i}`, 'g', { at: [x, y] });
      put(g, 'bg', 'rect', { width: L.cW, height: 32, rx: 3, ...paint('box') });
      put(g, 'n', 'text', { x: 6, y: 12, ...TEXT.tiny, text: name });
      put(g, 'v', 'text', { x: 6, y: 27, text: String(v) });
      put(g, 'h', 'rect', { x: -1, y: -1, width: L.cW + 2, height: 34, rx: 4, ...paint(v !== was ? 'touched' : 'hidden') });
    });

    // What it cost so far, from the counters.
    const sys = m.st.epoll_wait_calls + m.st.recvmsg_calls + m.st.send_calls;
    const calls = m.tape.filter((c) => c.k !== 'ret');
    const firstReply = calls.findIndex((c) => c.k === 'tx');
    const l1 = m.d1Copied !== null ? `d1 handled after ${m.d1Copied} cop${m.d1Copied === 1 ? 'y' : 'ies'}` : 'd1 not handled yet';
    const l2 = firstReply >= 0 ? `first reply: system call ${firstReply + 1}, after ${m.replyAfter} handled` : 'no reply has left';
    put(svg, 'cost1', 'text', { x: 16, y: L.costY, text: `${sys} system calls${L.small ? '' : ' so far'} · ${l1}` });
    put(svg, 'cost2', 'text', { x: 16, y: L.costY + 16, ...TEXT.small, text: L.small ? l2 : `${l2} · kernel copy: ${m.copied} × 64 B (illustrative)` });
    legend(svg, put, L);
  }

  const fmtKiB = (n) => (n * MAX_DGRAM >= 1024 ? `${(n * MAX_DGRAM) / 1024} KiB` : `${n * MAX_DGRAM} B`);

  function dgram(svg, put, L, scene, id, [x, y], kind, focus) {
    const g = put(svg, `d-${scene}-${id}`, 'g', { at: [x, y] });
    put(g, 'r', 'rect', { width: L.cell, height: L.cell, rx: 3, ...paint(kind) });
    put(g, 't', 'text', { x: L.cell / 2, y: L.cell / 2 + 4, 'text-anchor': 'middle', ...(kind === 'ghost' ? TEXT.smallDim : TEXT.small), text: `d${id}` });
    put(g, 'f', 'rect', { x: -2, y: -2, width: L.cell + 4, height: L.cell + 4, rx: 4, ...paint(focus ? 'touched' : 'hidden') });
  }

  function legend(svg, put, L) {
    const items = [['held', 'waiting'], ['written', 'copied now'], ['green', 'handled'], ['ghost', 'old bytes'], ['purple', 'EAGAIN'], ['touched', 'changed']];
    items.forEach(([kind, label], n) => {
      const col = L.small ? n % 3 : n;
      const row = L.small ? Math.floor(n / 3) : 0;
      const g = put(svg, `lg-${n}`, 'g', { at: [16 + col * (L.small ? 112 : 112), L.H - (L.small ? 30 : 12) + row * 18] });
      put(g, 'r', 'rect', { x: 1, y: -9, width: 10, height: 10, rx: 2, ...paint(kind) });
      put(g, 't', 'text', { x: 16, y: 0, ...TEXT.small, text: label });
    });
  }

  // book/src/ergon-io/udp.md:74-78, µs: p50 bars, p50 / p99 / max as printed.
  const SWEEP = [
    ['1', 2.0, '2.0 / 2.2 / 7–21', 34.5, '34.5 / 37.6 / 48–77', 33],
    ['4', 3.1, '3.1 / 3.3–3.4 / 8–16', 18.4, '18.4 / 21.1–21.2 / 29–35', 9],
    ['8', 4.6, '4.6 / 4.7–4.9 / 9–12', 15.5, '15.5 / 18.1–18.2 / 20–25', 5],
    ['16', 7.4, '7.4 / 7.7–7.8 / 12–55', 14.0, '14.0 / 16.5–16.7 / 19–106', 3],
    ['32', 13.1, '13.1 / 15.6 / 18–26', 13.2, '13.2–13.3 / 15.8 / 18–26', 2],
  ];

  function drawBatch(svg, put, L) {
    put(svg, 'f-t', 'text', { x: 16, y: 20, ...TEXT.title, text: 'Choosing rx_batch: 32 × 64 B queued' });
    put(svg, 'f-s', 'text', { x: 16, y: 38, ...TEXT.small, text: L.small ? 'µs p50 / p99 / max; ranges over 3 runs' : 'µs from the poll to the handler, p50 / p99 / max; ranges over three runs; bars are p50' });
    const x0 = L.small ? 70 : 90;
    const span = L.small ? 270 : 300;
    const scale = span / 35;
    const rowH = L.small ? 76 : 52;
    SWEEP.forEach(([rx, f, fT, l, lT, calls], i) => {
      const y = 56 + i * rowH;
      const g = put(svg, `sw-${i}`, 'g', { at: [0, y] });
      put(g, 'k', 'text', { x: 16, y: 14, ...(rx === '16' ? TEXT.title : TEXT.plain), text: `rx ${rx}` });
      put(g, 'c', 'text', { x: 16, y: 30, ...TEXT.small, text: `${calls} calls` });
      put(g, 'bf', 'rect', { x: x0, y: 2, width: Math.max(2, f * scale), height: 14, rx: 2, ...paint('held') });
      put(g, 'bl', 'rect', { x: x0, y: 20, width: Math.max(2, l * scale), height: 14, rx: 2, ...paint('written') });
      if (L.small) {
        put(g, 'tf', 'text', { x: x0, y: 48, ...TEXT.tiny, text: `first ${fT}` });
        put(g, 'tl', 'text', { x: x0, y: 60, ...TEXT.tiny, text: `last ${lT}` });
      } else {
        put(g, 'tf', 'text', { x: x0 + span + 12, y: 14, ...TEXT.small, text: `first ${fT}` });
        put(g, 'tl', 'text', { x: x0 + span + 12, y: 32, ...TEXT.small, text: `last ${lT}` });
      }
    });
    const y = 56 + SWEEP.length * rowH + 8;
    const notes = L.small ? [
      'n 50,000 bursts a run; min, p99.99 not recorded',
      'Maxima not attributed (desktop loopback).',
      'Per datagram batched: +0.35 µs before the',
      'handler, −0.7 µs against its own call.',
      'Default 16: last at 14.0, first at 7.4.',
    ] : [
      'n 50,000 bursts a run, three runs; min and p99.99 not recorded; maxima not attributed.',
      'Desktop i5-8500T loopback, no isolated CPUs: the shape holds, not the absolute numbers.',
      'Each datagram added to a batch: +0.35 µs before the handler, −0.7 µs against its own call.',
      'Default 16: the last datagram at 14.0 µs p50; rx_batch 1 gets the first in 2.0 but the last in 34.5.',
    ];
    notes.forEach((t, k) => put(svg, `f-n${k}`, 'text', { x: 16, y: y + k * 16, ...TEXT.small, text: t }));
    const items = [['held', 'first datagram'], ['written', 'last datagram']];
    items.forEach(([kind, label], n) => {
      const g = put(svg, `fl-${n}`, 'g', { at: [16 + n * 140, L.H - 12] });
      put(g, 'r', 'rect', { x: 1, y: -9, width: 10, height: 10, rx: 2, ...paint(kind) });
      put(g, 't', 'text', { x: 16, y: 0, ...TEXT.small, text: label });
    });
  }

  // book/src/ergon-io/udp.md:146, µs p50 / p99 / max, median of 3 reps.
  const REPLY = [
    ['queued (ConnMut::send)', '46.34', '57.95', '191.57'],
    ['inline (UdpTx::send)', '45.18', '56.96', '206.22'],
    ['mio, a call a datagram', '42.56', '55.71', '193.44'],
  ];

  function drawReply(svg, put, L) {
    put(svg, 'f-t', 'text', { x: 16, y: 20, ...TEXT.title, text: 'Replying inline: bursts of 8' });
    put(svg, 'f-s', 'text', { x: 16, y: 38, ...TEXT.small, text: L.small ? 'two Graviton c8g.2xlarge, 50,000 msg/s, µs' : 'two Graviton c8g.2xlarge hosts, 50,000 msg/s, µs, median of 3 reps' });
    const w = L.small ? 328 : 560;
    const g = put(svg, 'card', 'g', { at: [16, 52] });
    put(g, 'bg', 'rect', { width: w, height: 40 + REPLY.length * 26 + 8, rx: 6, ...paint('box') });
    const cols = L.small ? [10, 196, 246, 296] : [10, 300, 380, 470];
    ['echo end', 'p50', 'p99', 'max'].forEach((t, k) => put(g, `h${k}`, 'text', { x: cols[k] + (k ? 30 : 0), y: 24, 'text-anchor': k ? 'end' : 'start', ...TEXT.small, text: t }));
    put(g, 'n', 'text', { x: 10, y: 40, ...TEXT.tiny, text: 'n, min and p99.99 not recorded on the page' });
    REPLY.forEach((r, i) => {
      const y = 62 + i * 26;
      put(g, `r${i}-hl`, 'rect', { x: 4, y: y - 16, width: w - 8, height: 22, rx: 3, ...paint(i === 1 ? 'scanned' : 'hidden') });
      r.forEach((t, k) => put(g, `r${i}-${k}`, 'text', { x: cols[k] + (k ? 30 : 0), y, 'text-anchor': k ? 'end' : 'start', ...(L.small && k === 0 ? TEXT.tiny : TEXT.plain), text: L.small && k === 0 ? t.replace(' (ConnMut::send)', '').replace(' (UdpTx::send)', '') : t }));
    });
    const y0 = 52 + 40 + REPLY.length * 26 + 30;
    const notes = L.small ? [
      'Inline: 1.15 lower p50 than queued; the',
      'first reply skips the rest of the burst',
      'but still waits for the batch copy.',
      'Queued: one sendmmsg for all replies.',
      'Lone datagram: not separated.',
      'Every max unexplained: none compared.',
      'Win: bursts you act on whole (rx_batch',
      '16), or first updates (4 to 8, inline).',
    ] : [
      'Inline is 1.15 lower at p50 than queued: the first reply leaves before the rest of the burst is',
      'handled, but still after recvmmsg copied the batch. mio stays 2.62 lower at 50,000 msg/s.',
      'Queued replies leave in one sendmmsg: fewer calls when many replies share a pass.',
      'A lone datagram: queued, inline and mio are not separated. Every max is unexplained: none compared.',
      'Wins: one call per burst, slots read in place, nothing allocates. Loses: the first datagram of a',
      'large batch waits for the whole copy; pick rx_batch 4 to 8 when the first update is what you act on.',
    ];
    notes.forEach((t, k) => put(svg, `f-n${k}`, 'text', { x: 16, y: y0 + k * 16, ...TEXT.small, text: t }));
  }

  Anim.register('udp-rx-batch', (stage, api) => {
    // The figure (stage's parent) decides the layout, as before: under 560 px
    // wide, padding included, it takes the narrow one.
    const isNarrow = () => stage.parentElement.clientWidth < NARROW;
    let narrow = isNarrow();
    const svg = api.svg(layout(narrow).W, layout(narrow).H, TITLE);
    // Every drawing goes in one monospace, 12-unit group; texts set smaller
    // or larger sizes themselves.
    const root = api.el('g', { class: 'ea-mono', 'font-size': 12 }, svg);
    const k = keyed();
    const draw = (st) => {
      k.begin();
      const L = layout(narrow);
      if (st.figures === 'batch') drawBatch(root, k.put, L);
      else if (st.figures === 'reply') drawReply(root, k.put, L);
      else drawSocket(root, k.put, L, st);
      k.sweep(root);
    };
    return {
      steps: buildSteps().map((st) => ({ caption: st.caption, ms: st.hold || 2600, show: () => draw(st) })),
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
