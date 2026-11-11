// Generated file: edits here are overwritten by the next export. sha256:4473ad88c4bc8e7aa120d2787af27641ba556e31dd17bb7a275254e1a4eb1ae6
// WebSocket receive: one fragmented message across reads, records and fragments.
// Sources: every buffer state below comes from a port of this code, run in order.
//   ergon-io/src/stream.rs:1-28     buffer layout: leftover plaintext | gap | ciphertext | free tail
//   ergon-io/src/stream.rs:181-198  open_next: a complete record is decrypted in place
//   ergon-io/src/stream.rs:209-231  place: adopt in place when the window is empty, else move the smaller side
//   ergon-io/src/stream.rs:242-281  consume, hold, held_len, held, release
//   ergon-io/src/stream.rs:286-320  compact: reset when empty; move only when the free tail is short
//   ergon-io/src/tls.rs:43-53       record header 5 B, tag 16 B, MAX_RECORD
//   ergon-io/src/tls.rs:543         complete_record_len: a part record waits for the next read
//   ergon-io/src/ws.rs:146-219      parse_header: 2-byte header for payloads up to 125 B
//   ergon-io/src/ws_conn.rs:807-911   parse_frames: stops when the next frame is not all in the window
//   ergon-io/src/ws_conn.rs:914-955   first_fragment: Keep copies, Fragments holds, Stream hands over
//   ergon-io/src/ws_conn.rs:960-1000  continuation: Part { start: held_len + header_len, len }
//   ergon-io/src/ws_conn.rs:1005-1086 stream_fragment: on_fragment borrows the slice in the window
//   ergon-io/src/ws_conn.rs:1090-1176 end_message: on_message(&frag) or on_fragments, then release
//   ergon-io/src/ws_conn.rs:184, 1182-1195, 1973-1975  MAX_FRAGMENTS 32, spill, max_frame
//   ergon-io/src/ws_conn.rs:606     frag: Vec::with_capacity(1 << 20), so Keep's copy allocates nothing here
// (ergon-io/src/frame.rs is the length-prefixed TCP protocol, not this path.)
// Figures: book/src/ergon-io/websocket.md:135-155 (what each case copies),
// :334-350 (Keep, Fragments, Stream, four fragments) and :240-277 (TextCheck::Utf8 rows).
// The frag rows' input: the WebSocket bench's ws_bench.rs:161-205, 917-950
// (Frag4 is preloaded as plaintext, no records, so no frame is cut).
// Illustrative: the message (four 100-byte binary fragments), the record
// sizes and the two reads; only the first 474 bytes of the receive buffer are
// drawn (a real one is far larger, so compact never moves bytes here); the copy
// counts follow from them.
(() => {
  'use strict';
  const HDR = 5; // TLS record header
  const TRAILER = 17; // TLS 1.3 content type (1) + AEAD tag (16)
  const FRAG = 100; // payload per fragment (illustrative)
  const WS_HDR = 2; // payload <= 125: two header bytes, server frames unmasked
  const NFRAG = 4;
  const FRAME = WS_HDR + FRAG;
  const PLAIN = [102, 240, 66]; // plaintext per record: record 1 ends on a frame, record 2 cuts fragment 3
  const READS = [200, 274]; // bytes per TCP read: record 2 straddles them
  const MODES = ['Keep', 'Fragments', 'Stream'];
  const LINE = 64;

  const SLUG = 'ws-receive-paths';
  const TITLE = 'WebSocket receive: one fragmented message under Keep, Fragments and Stream';
  const SVG_NS = 'http://www.w3.org/2000/svg';
  const NARROW = 560; // px of the figure, padding included: under it, the narrow layout

  // The module's drawing calls still name the classes of the old runtime; sty()
  // turns them into anim.css's vocabulary. Every entry's result names cls, style
  // and font-size (undefined when unused), so a keyed element that changes kind
  // loses what the old kind set. Outline-only kinds set fill:none, which no
  // anim.css class does (an SVG rect fills black by default).
  const MAP = {
    'ea-a': { cls: 'ea-k1' },
    'ea-b': { cls: 'ea-k2' },
    'ea-c': { cls: 'ea-k3' },
    'ea-d': { cls: 'ea-k4' },
    'ea-box': { cls: 'ea-box' },
    'ea-hidden': { cls: 'ea-hidden' },
    'ea-ghost': { style: 'fill:var(--table-header-bg);opacity:0.6' },
    'ea-empty': { style: 'fill:none;stroke:var(--table-header-bg)' },
    'ea-stroke-hl': { style: 'fill:none;stroke:var(--ea-c5);stroke-width:3' },
    'ea-stroke-c': { cls: 'ea-ok', style: 'fill:none' },
    'ea-stroke-d': { style: 'fill:none;stroke:var(--ea-c4);stroke-width:2;stroke-dasharray:2 3' },
    'ea-wire': { style: 'fill:none;stroke:var(--fg);stroke-width:1.2;opacity:0.6' },
    'ea-title': { cls: 'ea-strong', 'font-size': 14 },
    'ea-small': { 'font-size': 10 },
    'ea-tiny': { 'font-size': 8 },
    'ea-dim': { cls: 'ea-muted' },
    // Held bytes are fainter than the window's: class fill-opacity beats the
    // attribute the old code set (0.5), so this is a style.
    'ea-held': { cls: 'ea-k1', style: 'fill-opacity:0.11' },
    'ea-on-a': {}, // fg-coloured text on a 0.22-opacity fill is already readable
  };
  function sty(names) {
    const out = { cls: undefined, style: undefined, 'font-size': undefined };
    for (const name of names.split(' ').filter(Boolean)) {
      const m = MAP[name];
      if (!m) throw new Error(`${SLUG}: no mapping for class ${name}`);
      for (const [k, v] of Object.entries(m)) out[k] = k === 'cls' && out.cls ? `${out.cls} ${v}` : v;
    }
    return out;
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

  // The wire: per byte, what it is and which fragment's payload it carries.
  function wire() {
    const out = [];
    let p = 0;
    const recs = [];
    PLAIN.forEach((plen, r) => {
      const start = out.length;
      for (let i = 0; i < HDR; i += 1) out.push({ k: 'hdr', r });
      for (let i = 0; i < plen; i += 1, p += 1) out.push({ k: 'ct', r, f: Math.floor(p / FRAME), o: p % FRAME });
      for (let i = 0; i < TRAILER; i += 1) out.push({ k: 'tag', r });
      recs.push({ start, len: out.length - start, plen });
    });
    return { bytes: out, recs };
  }
  const W = wire();
  const SIZE = W.bytes.length; // 474
  const lines = (a, len) => (len ? Math.floor((a + len - 1) / LINE) - Math.floor(a / LINE) + 1 : 0);
  const fmt = (n) => n.toLocaleString('en-GB');

  // A port of TlsStream's receive side plus WsConn's fragment state, per mode.
  function lane(mode) {
    return {
      mode, buf: new Array(SIZE).fill(null), head: 0, tail: 0, pS: 0, pE: 0, hold: 0, holding: false,
      span: 0, compact: 0, fragCopy: 0, kernel: 0, lineCost: 0,
      frag: [], parts: [], fragOp: false, index: 0, ev: null,
    };
  }
  const liveStart = (L) => (L.holding ? L.hold : L.pS);
  const heldLen = (L) => L.pS - L.hold;
  function copyWithin(L, from, len, to) {
    const tmp = L.buf.slice(from, from + len);
    for (let i = 0; i < len; i += 1) L.buf[to + i] = tmp[i];
    L.lineCost += lines(from, len) + lines(to, len);
  }
  function received(L, n, from) {
    for (let i = 0; i < n; i += 1) L.buf[L.tail + i] = Object.assign({}, W.bytes[from + i]);
    L.tail += n;
    L.kernel += n;
  }
  function place(L, src, plen) {
    const ev = { src, plen };
    if (L.pS === L.pE) {
      // Held bytes stay; the gap up to src joins the held span.
      L.pS = src;
      L.pE = src + plen;
      ev.inPlace = true;
      return ev;
    }
    const live = liveStart(L);
    const left = L.pE - live;
    ev.left = left;
    if (left < plen) {
      const to = src - left;
      copyWithin(L, live, left, to);
      L.pS = to + (L.pS - live);
      L.hold = to;
      L.pE = src + plen;
      L.span += left;
      Object.assign(ev, { move: { from: live, len: left, to }, side: 'leftover' });
    } else {
      copyWithin(L, src, plen, L.pE);
      Object.assign(ev, { move: { from: src, len: plen, to: L.pE }, side: 'record' });
      L.pE += plen;
      L.span += plen;
    }
    return ev;
  }
  function openNext(L) {
    const rec = W.recs.find((r) => r.start === L.head);
    if (!rec || L.head + rec.len > L.tail) return null; // complete_record_len: None
    for (let i = 0; i < rec.plen; i += 1) L.buf[L.head + HDR + i].k = 'pt';
    const start = L.head;
    L.head += rec.len;
    return Object.assign(place(L, start + HDR, rec.plen), { rec: W.recs.indexOf(rec) });
  }
  function consume(L, n) {
    L.pS += n;
    if (L.pS >= L.pE && !L.holding) { L.pS = 0; L.pE = 0; }
  }
  function release(L) {
    L.holding = false;
    if (L.pS >= L.pE) { L.pS = 0; L.pE = 0; }
  }
  function compact(L) {
    const pending = L.holding || L.pS !== L.pE;
    if (L.head === L.tail && !pending) { L.head = 0; L.tail = 0; return 'reset'; }
    return 'kept'; // the free tail of a real buffer is far above 2 x MAX_RECORD here
  }
  // One frame of parse_frames: null when the next frame is not all in the window.
  function parseOne(L) {
    const win = L.pE - L.pS;
    if (win < WS_HDR) return null;
    const c = L.buf[L.pS];
    if (win < FRAME) return { cut: true, have: win, f: c.f };
    const f = c.f;
    const fin = f === NFRAG - 1;
    const at = L.pS + WS_HDR;
    const ev = { f, fin, at };
    if (!L.fragOp) {
      L.fragOp = true; // first_fragment: the handler's Interest decides
      if (L.mode === 'Keep') {
        L.frag = [f];
        L.fragCopy += FRAG;
        L.lineCost += lines(at, FRAG) + lines(0, FRAG);
        ev.copy = { from: at, to: 0 };
      } else if (L.mode === 'Fragments') {
        L.parts = [{ start: WS_HDR, len: FRAG, f }];
        L.hold = L.pS;
        L.holding = true;
        ev.part = 0;
      } else {
        L.index = 0;
        ev.handed = { i: 0, at };
      }
    } else if (L.mode === 'Keep') {
      ev.copy = { from: at, to: L.frag.length * FRAG };
      L.lineCost += lines(at, FRAG) + lines(L.frag.length * FRAG, FRAG);
      L.frag.push(f);
      L.fragCopy += FRAG;
    } else if (L.mode === 'Fragments') {
      L.parts.push({ start: heldLen(L) + WS_HDR, len: FRAG, f });
      ev.part = L.parts.length - 1;
    } else {
      L.index += 1;
      ev.handed = { i: L.index, at };
    }
    consume(L, FRAME);
    if (fin) {
      // end_message
      L.fragOp = false;
      if (L.mode === 'Keep') ev.deliver = 'on_message';
      if (L.mode === 'Fragments') {
        ev.deliver = 'on_fragments';
        ev.slices = L.parts.map((p) => ({ at: L.hold + p.start, start: p.start }));
        ev.base = L.hold;
        release(L);
      }
    }
    return ev;
  }
  const snap = (x) => JSON.parse(JSON.stringify(x));

  function buildSteps() {
    const ls = MODES.map(lane);
    const steps = [];
    const push = (op, math, caption, extra, evs) => steps.push(Object.assign({
      op, math, caption, lanes: snap(ls.map((L, n) => Object.assign({}, L, { ev: evs ? evs[n] : null }))),
    }, extra));
    const all = (fn) => ls.map(fn);
    const [K, F, S] = ls;

    push('the message on the wire', 'four frames, FIN on the last · three TLS 1.3 records · two TCP reads',
      'One binary message in four 100-byte fragments, each with a 2-byte header. TLS seals the 408 plaintext bytes into three records (5-byte header, plaintext, 17-byte type and tag): record 1 ends on a frame, record 2 cuts fragment 3. Two reads carry the 474 bytes. Sizes are illustrative.',
      { wireOnly: true, hold: 5200 });

    all((L) => received(L, READS[0], 0));
    push('read 1: kernel → free tail', `${READS[0]} B written · record 1 complete, record 2 has ${READS[0] - W.recs[1].start} of ${W.recs[1].len} B`,
      `The kernel writes read 1 into the free tail: the one copy every case pays. Each bar is the first ${SIZE} bytes of that case's receive buffer; the rest is free tail. Record 1 is complete; record 2 is not, so it cannot be opened yet.`,
      { read: 0 });

    let evs = all(openNext);
    push('open_next: record 1', 'decrypt in place · window empty → the plaintext becomes the window',
      'Record 1 is decrypted where it lies. The window was empty, so its plaintext simply becomes the window: no byte moves. Its header and tag are now dead bytes.',
      { rec: 0 }, evs);

    evs = all(parseOne);
    push('fragment 0: on_message_start', 'one decision on the first frame, three outcomes',
      `Fragment 0 is whole, so the handler is asked once how to take the message. Keep copies its 100 payload bytes into a separate buffer (about ${lines(evs[0].copy.from, FRAG) + lines(0, FRAG)} cache lines touched). Fragments holds the receive buffer from here and records one slice. Stream hands the slice over now, borrowed.`,
      {}, evs);

    const wait = all((L) => ({ parsed: parseOne(L), opened: openNext(L), compact: compact(L) }));
    push('the window is empty: wait', `record 2: ${READS[0] - W.recs[1].start} of ${W.recs[1].len} B · complete_record_len → None`,
      `Keep and Stream are done with every parsed byte, so their window resets. Fragments still holds ${F.pS - F.hold} bytes (header and payload of fragment 0). Record 2 is incomplete, so the read ends; the free tail is ample, so nothing is compacted.`,
      {}, wait.map(() => null));

    all((L) => received(L, READS[1], READS[0]));
    push('read 2: kernel → free tail', `${READS[1]} B written · records 2 and 3 complete`,
      'Read 2 completes record 2 and brings all of record 3. Still only the kernel has copied anything.',
      { read: 1 });

    evs = all(openNext);
    push('open_next: record 2', 'window empty in every case → adopted in place',
      `Every window is empty, so record 2's plaintext is adopted in place again: no copy. Under Fragments the held bytes stay put and the ${W.recs[1].start + HDR - (W.recs[0].start + HDR + PLAIN[0])}-byte gap (record 1's type and tag, record 2's header) joins the held span; the slice offsets will skip it.`,
      { rec: 1 }, evs);

    evs = all(parseOne);
    push('fragment 1', 'Keep copies · Fragments records a slice · Stream hands over',
      `Keep appends 100 more bytes (${K.fragCopy} so far). Fragments records slice 1 at offset ${F.parts[1].start} from the hold point, past the gap. Stream calls on_fragment(1) and keeps nothing.`,
      {}, evs);

    evs = all(parseOne);
    push('fragment 2', 'the same, one fragment on',
      `Keep: ${K.fragCopy} bytes copied. Fragments: slice 2 at offset ${F.parts[2].start}, ${F.parts.length} of ${32} slots (MAX_FRAGMENTS). Stream: on_fragment(2).`,
      {}, evs);

    evs = all(parseOne);
    const have = evs[0].have;
    push('fragment 3 is cut', `${have} of ${FRAME} B in the window · parse_frames stops`,
      `Record 2 ended inside fragment 3: only ${have} bytes are in the window, so parsing stops and record 3 is opened.`,
      {}, evs);

    evs = all(openNext);
    const kMove = evs[0].move.len;
    const fMove = evs[1].move.len;
    push('open_next: record 3', 'window not empty → move the smaller side (span_copy_bytes)',
      `Now the window is not empty, so the two parts must meet. Keep and Stream: the ${kMove}-byte leftover is smaller than the ${PLAIN[2]}-byte record, so it moves forward. Fragments: its leftover runs from the hold point, ${evs[1].left} bytes with the held fragments, so the record's ${fMove} bytes move back instead.`,
      { rec: 2 }, evs);

    evs = all(parseOne);
    push('fragment 3 (FIN): deliver', 'end_message',
      `Keep copies the last 100 bytes and calls on_message with one contiguous ${K.fragCopy}-byte slice of its own buffer. Fragments calls on_fragments with ${evs[1].slices.length} slices of the receive buffer, then releases the hold. Stream calls on_fragment(3, fin): it already delivered the rest.`,
      { hold: 3600 }, evs);

    all(compact);
    push('compact: buffer empty → reset', 'head = tail = 0 · nothing moved',
      'Every byte is consumed and nothing is held, so compaction just resets the offsets. No step allocated; the next read starts at byte 0.',
      {}, null);

    push('what each case copied', 'beyond the kernel write: span_copy_bytes + frag_copy_bytes',
      `Beyond the kernel's ${SIZE} bytes: Keep moved ${K.span + K.fragCopy} (span ${K.span}, frag ${K.fragCopy}), Fragments ${F.span} (span only), Stream ${S.span} (span only), as the copies table says for frames cut across records. Fragments' join moved more because the held bytes travel with the leftover.`,
      { totals: true, hold: 7000 });

    push('why Fragments and Stream win', '4 fragments per message, TextCheck::None, ns per message',
      'From a few KiB the copy dominates: at 4 KiB in four fragments Keep is 89.8 ns at the p50 against 28.6 for Fragments and 32.5 for Stream; at 16 KiB, 317.7 against 31.2 and 33.8. Timed on plaintext already in the buffer (no socket, no decryption, no cut frames: ws_bench preload_plain): the cost of the connection per message, not of a parser; the gap between Keep and the others is the copy. A parser that reads every byte adds its own pass to each.',
      { figures: 'win', hold: 9000 });

    push('when they lose', 'small messages and the limits',
      'At 300 B the copy is nearly free: Keep 26.0 ns, Fragments 26.6, Stream 29.2. With TextCheck::Utf8 Fragments was slower at 300 B (36.4 against 29.9). Stream pays 2 to 4 ns over Fragments for one call per fragment. Past 32 fragments, or more than buf_size − 33,290 bytes held, Fragments falls back to one copy.',
      { figures: 'lose', hold: 9000 });
    return { steps, final: ls };
  }

  function layout(narrow) {
    if (narrow) {
      return { W: 360, H: 422, bw: 328, wireY: 74, laneY: 124, laneH: [96, 80, 80], ev: 26, bar: 32, bh: 18, small: true };
    }
    return { W: 720, H: 378, bw: 688, wireY: 60, laneY: 108, laneH: [90, 70, 70], ev: 12, bar: 18, bh: 20, small: false };
  }

  // Display class of buffer byte i in lane L (positions follow the port's offsets).
  function kindAt(L, i) {
    const c = L.buf[i];
    if (i >= L.tail || !c) return 'free';
    if (i >= L.head) return 'cipher';
    const pending = L.holding || L.pS < L.pE;
    if (pending && i >= liveStart(L) && i < L.pE) {
      if (c.k !== 'pt') return 'gap';
      return i < L.pS ? 'held' : 'window';
    }
    return 'spent';
  }
  const CLS = { free: 'ea-empty', spent: 'ea-empty', cipher: 'ea-ghost', window: 'ea-a', held: 'ea-held', gap: 'ea-d', given: 'ea-c' };

  // Draws a block that slides from (fx, fy) to (tx, ty) the first time it is drawn
  // (implemented here: the runtime has no motion primitive; reduced motion jumps).
  function slide(api, parent, key, attrs, from, to) {
    let had = false;
    for (const c of parent.children) if (c.dataset && c.dataset.k === key) had = true;
    const e = api.put(parent, key, 'rect', Object.assign({}, attrs, { at: had || api.reduced ? to : from }));
    if (!had && !api.reduced) {
      requestAnimationFrame(() => requestAnimationFrame(() => {
        if (e.isConnected) e.style.transform = `translate(${to[0]}px, ${to[1]}px)`;
      }));
    }
    return e;
  }

  const ROWS = {
    win: {
      title: '4 KiB, four fragments, ns per message',
      cols: ['Keep', 'Fragments', 'Stream'],
      rows: [
        ['n', '20,000', '20,000', '20,000'],
        ['min', 'n/r', 'n/r', 'n/r'],
        ['p50', '89.8', '28.6', '32.5'],
        ['p99', '320.9', '87.2', '229.1'],
        ['p99.99', 'n/r', 'n/r', 'n/r'],
        ['max', '26,444.7', '8,614.3', '11,827.1'],
      ],
      notes: ['16 KiB, p50 / p99 / max:', 'Keep 317.7 / 421.9 / 15,423.9', 'Fragments 31.2 / 41.6 / 6,150.3', 'Stream 33.8 / 44.2 / 3,395.1', 'max: not attributed (macOS cannot pin;', 'load average 27 to 35).', 'n: samples of K messages (64; 16 at 16 KiB).'],
    },
    lose: {
      title: '300 B, four fragments, ns per message',
      cols: ['Keep', 'Fragments', 'Stream'],
      rows: [
        ['n', '20,000', '20,000', '20,000'],
        ['min', 'n/r', 'n/r', 'n/r'],
        ['p50', '26.0', '26.6', '29.2'],
        ['p99', '29.9', '30.5', '33.2'],
        ['p99.99', 'n/r', 'n/r', 'n/r'],
        ['max', '519.1', '713.5', '673.9'],
      ],
      notes: ['Utf8, 300 B kept, p50 / p99 / max:', 'Keep 29.9 / 33.8 / 987.1', 'Fragments 36.4 / 40.3 / 585.5', 'max: not attributed (macOS cannot pin;', 'load average 27 to 35).', 'Over 32 fragments or buf_size − 33,290 B held,', 'Fragments copies once (frag_fallbacks).'],
    },
  };

  function drawFigures(svg, put, L, which) {
    const t = ROWS[which];
    put(svg, 'fig-t', 'text', { x: 16, y: 22, ...sty('ea-title'), text: t.title });
    put(svg, 'fig-s', 'text', { x: 16, y: 40, ...sty('ea-small'), text: 'TextCheck::None · M1 Pro, macOS · n/r: not recorded' });
    const g = put(svg, 'fig-c', 'g', { at: [16, 52] });
    const w = L.small ? 328 : 420;
    put(g, 'bg', 'rect', { width: w, height: 136, rx: 6, ...sty('ea-box') });
    const colX = (c) => (L.small ? 140 + c * 92 : 200 + c * 108);
    t.cols.forEach((c, n) => put(g, `h${n}`, 'text', { x: colX(n), y: 20, 'text-anchor': 'end', ...sty('ea-small'), text: c }));
    t.rows.forEach((r, k) => {
      put(g, `k${k}`, 'text', { x: 10, y: 40 + k * 16, text: r[0] });
      r.slice(1).forEach((v, n) => put(g, `v${k}-${n}`, 'text', { x: colX(n), y: 40 + k * 16, 'text-anchor': 'end', text: v }));
    });
    t.notes.forEach((n, k) => put(svg, `fig-n${k}`, 'text', { x: 16, y: 206 + k * 16, ...sty('ea-small'), text: n }));
  }

  function drawTotals(svg, put, L, final) {
    put(svg, 'tt', 'text', { x: 16, y: 22, ...sty('ea-title'), text: 'Bytes moved beyond the kernel write' });
    put(svg, 'ts', 'text', { x: 16, y: 40, ...sty('ea-small'), text: L.small ? `kernel ${SIZE} B in each · line-aligned buffer assumed` : `kernel: ${SIZE} B in every case · lines assume a line-aligned buffer` });
    const maxB = Math.max(...final.map((x) => x.span + x.fragCopy));
    const bw = L.small ? 200 : 420;
    final.forEach((x, n) => {
      const y = 62 + n * (L.small ? 84 : 64);
      const g = put(svg, `tot${n}`, 'g', { at: [16, y] });
      put(g, 'name', 'text', { x: 0, y: 12, ...sty('ea-title'), text: x.mode });
      const bx = L.small ? 0 : 110;
      const by = L.small ? 20 : 2;
      put(g, 'span', 'rect', { x: bx, y: by, width: Math.max(1, (x.span / maxB) * bw), height: 14, rx: 2, ...sty('ea-d') });
      put(g, 'frag', 'rect', { x: bx + (x.span / maxB) * bw, y: by, width: (x.fragCopy / maxB) * bw, height: 14, rx: 2, ...sty('ea-b') });
      put(g, 'n', 'text', { x: bx + ((x.span + x.fragCopy) / maxB) * bw + 6, y: by + 11, text: `${x.span + x.fragCopy} B` });
      const out = { Keep: 'one 400 B slice of its own buffer', Fragments: '4 slices of the receive buffer', Stream: '4 borrowed slices, one call each' }[x.mode];
      if (L.small) {
        put(g, 'd', 'text', { x: bx, y: by + 28, ...sty('ea-small'), text: `span ${x.span} + frag ${x.fragCopy} · ~${x.lineCost} lines` });
        put(g, 'd2', 'text', { x: bx, y: by + 42, ...sty('ea-small'), text: out });
      } else {
        put(g, 'd', 'text', { x: bx, y: by + 30, ...sty('ea-small'), text: `span ${x.span} + frag ${x.fragCopy} · ~${x.lineCost} lines · ${out}` });
      }
    });
    const ly = L.small ? 330 : 270;
    [['ea-d', 'span_copy_bytes: a frame cut across records'], ['ea-b', 'frag_copy_bytes: reassembly by copy']].forEach(([cls, label], n) => {
      const g = put(svg, `tl${n}`, 'g', { at: [16, ly + n * 18] });
      put(g, 'r', 'rect', { x: 0, y: -9, width: 10, height: 10, rx: 2, ...sty(cls) });
      put(g, 't', 'text', { x: 16, y: 0, ...sty('ea-small'), text: label });
    });
  }

  function drawWire(svg, put, L, st, s) {
    const y = L.wireY;
    put(svg, 'w-l', 'text', { x: 16, y: y - 8, ...sty('ea-small'), text: `on the wire: ${SIZE} B, to scale` });
    const g = put(svg, 'wire', 'g', { at: [16, y] });
    W.recs.forEach((r, n) => {
      put(g, `h${n}`, 'rect', { x: r.start * s, y: 0, width: HDR * s, height: 14, ...sty('ea-d') });
      put(g, `c${n}`, 'rect', { x: (r.start + HDR) * s, y: 0, width: r.plen * s, height: 14, ...sty('ea-ghost') });
      put(g, `t${n}`, 'rect', { x: (r.start + HDR + r.plen) * s, y: 0, width: TRAILER * s, height: 14, ...sty('ea-d') });
      put(g, `rl${n}`, 'text', { x: (r.start + r.len / 2) * s, y: 11, 'text-anchor': 'middle', ...sty('ea-small'), text: L.small ? `rec ${n + 1}` : `record ${n + 1} · ${r.plen} B` });
      put(g, `rh${n}`, 'rect', { x: r.start * s - 1, y: -2, width: r.len * s + 2, height: 18, rx: 2, ...sty(st.rec === n ? 'ea-stroke-hl' : 'ea-hidden') });
    });
    // Frame boundaries inside the ciphertext.
    for (let f = 1; f < NFRAG; f += 1) {
      const idx = W.bytes.findIndex((b) => b.k === 'ct' && b.f === f && b.o === 0);
      put(g, `fb${f}`, 'rect', { x: idx * s - 0.5, y: -4, width: 1.5, height: 22, ...sty('ea-c') });
    }
    let at = 0;
    READS.forEach((n, k) => {
      const x0 = at * s;
      const x1 = (at + n) * s;
      put(g, `rd${k}`, 'rect', { x: x0 + 1, y: 20, width: x1 - x0 - 2, height: 3, rx: 1, ...sty(st.read === k ? 'ea-b' : 'ea-ghost') });
      put(g, `rdl${k}`, 'text', { x: (x0 + x1) / 2, y: 34, 'text-anchor': 'middle', ...sty('ea-small'), text: `read ${k + 1}: ${n} B` });
      at += n;
    });
  }

  function evText(L, mode, small) {
    const ev = L.ev;
    if (!ev) return '';
    if (ev.rec !== undefined) {
      if (ev.inPlace) return 'adopted in place: 0 B moved';
      return `${ev.side === 'leftover' ? 'leftover forward' : 'record back'}: ${ev.move.len} B`;
    }
    if (ev.cut) return `${ev.have} B of fragment ${ev.f}: wait for more`;
    if (ev.f === undefined) return '';
    if (mode === 'Keep') return ev.deliver ? `copy 100 B; on_message(${L.frag.length * FRAG} B)` : `copy 100 B → frag (${L.frag.length * FRAG})`;
    if (mode === 'Fragments') return ev.deliver ? `on_fragments(${ev.slices.length} slices)` : `slice ${ev.part} at +${L.parts[ev.part].start}${small ? '' : ' from hold'}`;
    return `on_fragment(${ev.handed.i}${ev.fin ? ', fin' : ''})${small ? '' : ': borrowed'}`;
  }

  function drawLane(svg, api, Lo, Ln, n, y, s, index) {
    const put = api.put;
    const g = put(svg, `lane${n}`, 'g', { at: [16, y] });
    const sub = { Keep: 'copy into one buffer', Fragments: 'hold slices in place', Stream: 'hand over each fragment' }[Ln.mode];
    put(g, 'name', 'text', { x: 0, y: 12, ...sty('ea-title'), text: Ln.mode });
    put(g, 'sub', 'text', { x: Lo.small ? 86 : 90, y: 12, ...sty('ea-small ea-dim'), text: sub });
    put(g, 'ev', 'text', { x: Lo.small ? 0 : 290, y: Lo.ev, ...sty('ea-small'), text: evText(Ln, Ln.mode, Lo.small) });
    // The buffer as runs of equal display class (and fragment, for plaintext).
    const bar = put(g, 'bar', 'g', { at: [0, Lo.bar] });
    put(bar, 'frame', 'rect', { x: -1, y: -1, width: SIZE * s + 2, height: Lo.bh + 2, rx: 2, ...sty('ea-empty') });
    // Snapshots are taken after consume/release, but the handler read these
    // bytes before that (stream_fragment and on_fragments run first): show them.
    const ev0 = Ln.ev || {};
    const given = ev0.slices ? ev0.slices.map((p) => p.at) : (ev0.handed ? [ev0.handed.at] : []);
    const runs = [];
    for (let i = 0; i < SIZE; i += 1) {
      let k = kindAt(Ln, i);
      if (given.some((a) => i >= a && i < a + FRAG)) k = 'given';
      const c = Ln.buf[i];
      const f = (k === 'window' || k === 'held' || k === 'given') ? c.f : -1;
      const last = runs[runs.length - 1];
      if (last && last.k === k && last.f === f) last.len += 1;
      else runs.push({ k, f, at: i, len: 1 });
    }
    runs.forEach((r, m) => {
      put(bar, `r${m}`, 'rect', { x: r.at * s, y: 0, width: r.len * s, height: Lo.bh, ...sty(CLS[r.k]) });
      if (r.f >= 0 && r.len * s >= 18) {
        put(bar, `rl${m}`, 'text', { x: (r.at + r.len / 2) * s, y: Lo.bh / 2 + 4, 'text-anchor': 'middle', ...sty(r.k === 'held' ? 'ea-small' : 'ea-small ea-on-a'), text: Lo.small ? `${r.f}` : `F${r.f}` });
      }
    });
    const ev = Ln.ev || {};
    // What moved or was handed over in this step.
    if (ev.move) {
      const m = ev.move;
      put(bar, 'dst', 'rect', { x: m.to * s - 1, y: -3, width: m.len * s + 2, height: Lo.bh + 6, rx: 2, ...sty('ea-stroke-hl') });
      slide(api, bar, `mv${index}`, { x: 0, y: 2, width: m.len * s, height: Lo.bh - 4, rx: 2, ...sty('ea-b') }, [m.from * s, 0], [m.to * s, 0]);
    }
    if (ev.handed) {
      put(bar, 'hand', 'rect', { x: ev.handed.at * s, y: Lo.bh + 3, width: FRAG * s, height: 5, rx: 2, ...sty('ea-c') });
    }
    if (ev.at !== undefined && !ev.handed) {
      put(bar, 'read', 'rect', { x: ev.at * s - 1, y: -3, width: FRAG * s + 2, height: Lo.bh + 6, rx: 2, ...sty('ea-stroke-hl') });
    }
    if (Ln.mode === 'Fragments') {
      const base = ev.base !== undefined ? ev.base : Ln.hold;
      const parts = ev.slices ? ev.slices.map((p) => p.at) : (Ln.holding ? Ln.parts.map((p) => base + p.start) : []);
      parts.forEach((x, k) => {
        put(bar, `p${k}`, 'rect', { x: x * s, y: Lo.bh + 3, width: FRAG * s, height: 5, rx: 2, ...sty(ev.slices ? 'ea-c' : (ev.part === k ? 'ea-b' : 'ea-a')) });
      });
    }
    let cy = Lo.bar + Lo.bh + 22;
    if (Ln.mode === 'Keep') {
      const fy = Lo.bar + Lo.bh + 12;
      const fg = put(g, 'frag', 'g', { at: [0, fy] });
      put(fg, 'bg', 'rect', { x: 0, y: 0, width: NFRAG * FRAG * s, height: 10, rx: 2, ...sty('ea-empty') });
      put(fg, 'l', 'text', { x: NFRAG * FRAG * s + 6, y: 9, ...sty('ea-small'), text: Lo.small ? 'frag' : 'frag (Vec)' });
      Ln.frag.forEach((f, k) => {
        const cls = ev.deliver ? 'ea-c' : (ev.copy && k === Ln.frag.length - 1 ? 'ea-b' : 'ea-a');
        put(fg, `f${k}`, 'rect', { x: k * FRAG * s + 0.5, y: 1, width: FRAG * s - 1, height: 8, rx: 1, ...sty(cls) });
      });
      if (ev.copy) {
        slide(api, g, `cp${index}`, { x: 0, y: 0, width: FRAG * s, height: 6, rx: 2, ...sty('ea-b') }, [ev.copy.from * s, Lo.bar + 6], [ev.copy.to * s, fy + 2]);
      }
      cy = fy + 26;
    }
    const tail = Ln.mode === 'Keep' ? ` · frag_copy ${Ln.fragCopy}` : (Ln.mode === 'Fragments' ? ` · held ${Ln.holding ? Ln.pS - Ln.hold : 0}` : '');
    put(g, 'ctr', 'text', { x: 0, y: cy, ...sty('ea-small'), text: `kernel ${Ln.kernel} · span_copy ${Ln.span}${tail}` });
  }

  function legend(svg, put, L) {
    const items = [['ea-ghost', 'ciphertext'], ['ea-a', 'window'], ['ea-d', 'record overhead'], ['ea-b', 'moved now'], ['ea-c', 'to the handler'], ['ea-empty', 'free/spent']];
    items.forEach(([cls, label], n) => {
      const col = L.small ? n % 3 : n;
      const row = L.small ? Math.floor(n / 3) : 0;
      const g = put(svg, `lg-${n}`, 'g', { at: [16 + col * (L.small ? 112 : 114), L.H - (L.small ? 26 : 10) + row * 16] });
      put(g, 'r', 'rect', { x: 1, y: -9, width: 10, height: 10, rx: 2, ...sty(cls) });
      put(g, 't', 'text', { x: 16, y: 0, ...sty('ea-small'), text: label });
    });
  }

  // One step's whole state, drawn with keyed put() into the root group. `pa`
  // carries put and the reduced-motion flag to slide().
  function drawStep(svg, pa, L, st, index, final) {
    const put = pa.put;
    if (st.figures) { drawFigures(svg, put, L, st.figures); return; }
    if (st.totals) { drawTotals(svg, put, L, final); return; }
    const s = L.bw / SIZE;
    put(svg, 'op', 'text', { x: 16, y: 20, ...sty('ea-title'), text: st.op });
    // Narrow: two lines of ea-small rather than one unreadable 8 px line.
    const cut = L.small && st.math.length > 54 ? st.math.lastIndexOf(' ', 54) : -1;
    put(svg, 'math', 'text', { x: 16, y: 36, ...sty('ea-small'), text: cut > 0 ? st.math.slice(0, cut) : st.math });
    put(svg, 'math2', 'text', { x: 16, y: 49, ...sty('ea-small'), text: cut > 0 ? st.math.slice(cut + 1) : '' });
    drawWire(svg, put, L, st, s);
    let y = L.laneY;
    st.lanes.forEach((Ln, n) => {
      drawLane(svg, pa, L, Ln, n, y, s, index);
      y += L.laneH[n];
    });
    legend(svg, put, L);
  }

  Anim.register('ws-receive-paths', (stage, api) => {
    const { steps, final } = buildSteps();
    // The figure (stage's parent) decides the layout: under 560 px wide,
    // padding included, it takes the narrow one.
    const isNarrow = () => stage.parentElement.clientWidth < NARROW;
    let narrow = isNarrow();
    const svg = api.svg(layout(narrow).W, layout(narrow).H, TITLE);
    // Every drawing goes in one monospace, 12-unit group; texts set their own sizes.
    const root = api.el('g', { class: 'ea-mono', 'font-size': 12 }, svg);
    const k = keyed();
    const pa = { put: k.put, get reduced() { return api.reducedMotion; } };
    const draw = (st, index) => {
      k.begin();
      drawStep(root, pa, layout(narrow), st, index, final);
      k.sweep(root);
    };
    return {
      steps: steps.map((st, i) => ({ caption: st.caption, ms: st.hold || 2600, show: () => draw(st, i) })),
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
