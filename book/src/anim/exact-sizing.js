// Exact sizing: the staged length builder counts the same shape encode_book writes.
//
// Sources:
//   sbe/src/codegen/message_encoder.rs:314  compute_length() -> {Msg}EncodedLength::new()
//   sbe/src/codegen/encoded_length.rs:91    {Msg}EncodedLength::new: Accumulator::new(BLOCK_LENGTH)
//   sbe/src/codegen/encoded_length.rs:865   Accumulator::new: len starts at the fixed block
//   sbe/src/codegen/encoded_length.rs:911   enter_group: + dim, then + count x entry block in one multiply
//   sbe/src/codegen/encoded_length.rs:510   *_ragged: entry blocks pre-counted, so add() adds 0 bytes
//   sbe/src/codegen/encoded_length.rs:516   add() count != declared count -> GroupCountMismatch
//   sbe/src/codegen/encoded_length.rs:1026  group_ragged (orders(..)): + nested dim
//   sbe/src/codegen/encoded_length.rs:1092  uniform(n): n x 17 in one multiply
//   sbe/src/codegen/encoded_length.rs:621   symbol(len): + length prefix + bytes
//   sbe/src/codegen/encoded_length.rs:652   encoded_length_with_header: + header, added last
//   samples/l3-book/src/lib.rs:59           book_encoded_length
//   samples/l3-book/src/lib.rs:133          encode_book: header, fixed, bids, asks, symbol in wire order
//   samples/l3-book/tests/l3_tests.rs:996   fixture: bids 2/1/3 orders, asks 1/2, "BTCUSDT"
//   samples/l3-book/tests/l3_tests.rs:1015  assert_eq!(len, actual)
//   samples/l3-book/tests/l3_tests.rs:1016  a buffer one byte short is an error, not a truncation
//   samples/l3-book/schemas/l3-book.xml:30  L3Book: fixed block, bids, asks, symbol
// Sizes from the generated codec: header 8, fixed block 17, level 18, order 17,
// group header 4, var-data length 4; book_encoded_length(fixture) = 307.
// Costs shown are counts from the code (levels, orders, bytes); cache-line
// counts are illustrative and assume a 64-byte aligned buffer. No timings.
Anim.register('exact-sizing', (stage, api) => {
  const ROW_H = 26;
  const BAND = 9; // top band: counted by the builder; bottom band: written by the encoder
  const BIDS = [[50800, [101, 102]], [50750, [103]], [50700, [104, 105, 106]]];
  const ASKS = [[50850, [201]], [50900, [202, 203]]];
  const TOTAL = 307;
  const el = api.el;
  const WIDE = { W: 400, X0: 72, S: 1.7, top: 40, pitch: 38, labelAbove: false, font: 11 };
  const NARROW = { W: 300, X0: 0, S: 1.7, top: 36, pitch: 44, labelAbove: true, font: 9.5 };

  // ── Builder terms, in the order the generated code adds them ──────────────
  // Each inner array is one checked add or multiply, so its boxes light together.
  const orderKeys = (p, i, n) => Array.from({ length: n }, (_, j) => `${p}${i}o${j}`);
  const levelGroups = (p, i, n) => [[`${p}${i}d`], orderKeys(p, i, n)];
  const levelKeys = (p, i, n) => [`${p}${i}d`, ...orderKeys(p, i, n)];
  const B = [
    { call: 'L3BookEncoder::compute_length()', groups: [['fixed']], terms: [17], levels: 0 },
    { call: '.bids_ragged(3, |g| …)', groups: [['bdim'], ['b0', 'b1', 'b2']], terms: [4, 3 * 18], levels: 0 },
    { call: 'g.add()?.orders(|og| og.uniform(2))', groups: levelGroups('b', 0, 2), terms: [0, 4, 2 * 17], levels: 1 },
    { call: 'g.add()?.orders(|og| og.uniform(1))', groups: levelGroups('b', 1, 1), terms: [0, 4, 17], levels: 2 },
    { call: 'g.add()?.orders(|og| og.uniform(3))', groups: levelGroups('b', 2, 3), terms: [0, 4, 3 * 17], levels: 3 },
    {
      call: '.asks_ragged(2, |g| …)',
      groups: [['adim'], ['a0', 'a1'], ...levelGroups('a', 0, 1), ...levelGroups('a', 1, 2)],
      terms: [4, 2 * 18, 0, 4, 17, 0, 4, 2 * 17], levels: 5,
    },
    { call: '.symbol(7)', groups: [['slen'], ['sdat']], terms: [4, 7], levels: 5 },
    { call: '.encoded_length_with_header()', groups: [['hdr']], terms: [8], levels: 5 },
  ];
  for (const b of B) {
    b.keys = b.groups.flat();
    b.phase = new Map(b.groups.flatMap((ks, i) => ks.map((k) => [k, i])));
  }
  let len = 0;
  for (const b of B) { len += b.terms.reduce((a, t) => a + t, 0); b.len = len; }
  // Faithfulness guard: section ends must be the verified offsets minus the header.
  if ([B[4].len, B[5].len, B[6].len, B[7].len].join() !== '189,288,299,307') throw new Error('builder sums');

  // ── Encoder writes, in wire order ─────────────────────────────────────────
  const wireKeys = (p, levels) => {
    const keys = [`${p}dim`];
    levels.forEach(([, orders], i) => keys.push(`${p}${i}`, ...levelKeys(p, i, orders.length)));
    return keys;
  };
  const E = [
    { call: 'try_wrap_and_apply_header(buf, 0)?.fixed(..)', keys: ['hdr', 'fixed'], cursor: 25 },
    { call: '.bids(3, |g| …)', keys: wireKeys('b', BIDS), cursor: 197 },
    { call: '.asks(2, |g| …)', keys: wireKeys('a', ASKS), cursor: 296 },
    { call: '.symbol_as_str("BTCUSDT")', keys: ['slen', 'sdat'], cursor: TOTAL },
  ];

  let g;
  let svg;
  let boxes; // key -> { top, bot, bytes }
  let laneText;
  let callText;
  let lenText;
  let curText;
  let costText;
  let cost2Text;
  let endMark;

  // Bands leave a gap at y+9.5..y+16.5 where the label sits, so no edge crosses the text.
  function box(key, x, bytes, y, cls, label, labelCls) {
    const w = bytes * g.S;
    el('rect', { x, y, width: w, height: ROW_H, rx: 2, class: 'ea-box', 'fill-opacity': 0.35 }, svg);
    const top = el('rect', { x: x + 0.5, y: y + 0.5, width: Math.max(w - 1, 0.5), height: BAND, class: cls }, svg);
    const bot = el('rect', { x: x + 0.5, y: y + ROW_H - BAND - 0.5, width: Math.max(w - 1, 0.5), height: BAND, class: cls }, svg);
    boxes.set(key, { top, bot, cls, bytes });
    if (label !== undefined) {
      el('text', {
        x: x + w / 2, y: y + 15.5, 'text-anchor': 'middle', 'font-size': 8.5,
        class: `ea-mono ${labelCls || ''}`,
      }, svg, String(label));
    }
    return x + w;
  }

  function rowLabel(i, name, offset) {
    const y = g.top + i * g.pitch;
    if (g.labelAbove) {
      el('text', { x: 0, y: y + 9, 'font-size': 10, class: 'ea-strong' }, svg, name);
      el('text', { x: 46, y: y + 9, 'font-size': 9, class: 'ea-muted ea-mono' }, svg, `byte ${offset}`);
      return y + 13;
    }
    el('text', { x: 0, y: y + 11, 'font-size': 11, class: 'ea-strong' }, svg, name);
    el('text', { x: 0, y: y + 22, 'font-size': 9, class: 'ea-muted ea-mono' }, svg, `byte ${offset}`);
    return y;
  }

  function side(p, levels, y, cls) {
    let x = box(`${p}dim`, g.X0, 4, y, cls);
    levels.forEach(([price, orders], i) => {
      x = box(`${p}${i}`, x, 18, y, cls, price, 'ea-strong');
      x = box(`${p}${i}d`, x, 4, y, cls);
      orders.forEach((id, j) => { x = box(`${p}${i}o${j}`, x, 17, y, `${cls} ea-sub`, id, 'ea-fine'); });
    });
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const { X0, S } = g;
    stage.textContent = '';
    svg = api.svg(g.W, g.top + 4 * g.pitch + 100, 'L3Book wire layout: bytes counted by the length builder and bytes written by the encoder');
    boxes = new Map();
    laneText = el('text', { x: 0, y: 13, 'font-size': 12, class: 'ea-strong' }, svg, '');
    callText = el('text', { x: 0, y: 28, 'font-size': g.font, class: 'ea-mono' }, svg, '');

    const yF = rowLabel(0, 'fixed', 0);
    box('hdr', X0, 8, yF, 'ea-k5');
    box('fixed', X0 + 8 * S, 17, yF, 'ea-k4', 'fixed');
    const yB = rowLabel(1, 'bids', 25);
    side('b', BIDS, yB, 'ea-k1');
    const yA = rowLabel(2, 'asks', 197);
    side('a', ASKS, yA, 'ea-k2');
    const yS = rowLabel(3, 'symbol', 296);
    box('slen', X0, 4, yS, 'ea-k5');
    box('sdat', X0 + 4 * S, 7, yS, 'ea-k5');
    el('text', { x: X0 + 11 * S + 8, y: yS + 15.5, 'font-size': 10, class: 'ea-mono' }, svg, '"BTCUSDT"  (ends at byte 307)');
    // End-of-buffer marker, shown once both tracks land on byte 307.
    const pad = g.labelAbove ? 1.5 : 3; // narrow: keep clear of the row label above
    endMark = el('rect', { x: X0 - pad, y: yS - pad, width: 11 * S + 2 * pad, height: ROW_H + 2 * pad, rx: 4, fill: 'none', class: 'ea-ok' }, svg);

    // Legend: a mini box per track with its band filled. Narrow: one line each.
    const yL = yS + ROW_H + 14;
    const legend = (x, y, topBand, label) => {
      el('rect', { x, y: y - 9, width: 16, height: 12, rx: 1, class: 'ea-box' }, svg);
      el('rect', { x: x + 0.5, y: topBand ? y - 8.5 : y - 2.5, width: 15, height: 5, class: 'ea-k3' }, svg);
      el('text', { x: x + 21, y: y + 1, 'font-size': 9, class: 'ea-muted' }, svg, label);
    };
    legend(0, yL, true, 'top band: counted by the length builder');
    const y2 = g.labelAbove ? yL + 15 : yL;
    legend(g.labelAbove ? 0 : g.W / 2 + 20, y2, false, 'bottom band: written by encode_book');
    const yC = y2 + 22;
    lenText = el('text', { x: 0, y: yC, 'font-size': g.font, class: 'ea-mono' }, svg, '');
    curText = el('text', { x: 0, y: yC + 16, 'font-size': g.font, class: 'ea-mono' }, svg, '');
    costText = el('text', { x: 0, y: yC + 34, 'font-size': 9.5, class: 'ea-muted' }, svg, '');
    cost2Text = el('text', { x: 0, y: yC + 48, 'font-size': 9.5, class: 'ea-muted' }, svg, '');
    svg.setAttribute('viewBox', `0 0 ${g.W} ${yC + 54}`);
  }
  build(false);

  // A state: how many builder ops are done (nb), whether the buffer exists,
  // how many encoder ops are done (ne), and which band is hot this step.
  function render(s) {
    laneText.textContent = s.lane;
    const op = s.hot === 'b' ? B[s.nb - 1] : s.hot === 'e' ? E[s.ne - 1] : undefined;
    callText.textContent = s.call || (op ? op.call : '');
    const counted = new Set(B.slice(0, s.nb).flatMap((b) => b.keys));
    const written = new Set(E.slice(0, s.ne || 0).flatMap((e) => e.keys));
    const hotTop = s.hot === 'b' ? B[s.nb - 1].phase : new Map();
    const hotBot = s.ne > 0 && s.hot === 'e' ? E[s.ne - 1].keys : [];
    for (const [key, b] of boxes) {
      b.top.classList.toggle('ea-hidden', !counted.has(key));
      const ti = hotTop.get(key);
      b.top.classList.toggle('ea-hot', ti !== undefined);
      if (ti !== undefined) b.top.style.setProperty('--d', String(ti));
      // Bottom band: absent before allocation, a plain zeroed slot after, the section colour once written.
      b.bot.setAttribute('class', written.has(key) ? b.cls : 'ea-box');
      b.bot.classList.toggle('ea-hidden', !s.alloc);
      const bi = hotBot.indexOf(key);
      b.bot.classList.toggle('ea-hot', bi >= 0);
      if (bi >= 0) b.bot.style.setProperty('--d', String(bi));
    }
    endMark.classList.toggle('ea-hidden', !s.done);
    const bl = s.nb > 0 ? B[s.nb - 1].len : undefined;
    if (bl === undefined) lenText.textContent = 'builder len  –';
    else if (s.nb < B.length) lenText.textContent = `builder len  ${bl}   (+ 8 header = ${bl + 8})`;
    else lenText.textContent = `builder len  ${bl}   (header included)`;
    const cur = s.ne > 0 ? E[s.ne - 1].cursor : 0;
    curText.textContent = s.alloc ? `encoder cursor  ${cur} / ${TOTAL}` : 'encoder buffer  not allocated';
    const levels = s.nb > 0 ? B[s.nb - 1].levels : 0;
    if (s.cost) {
      [costText.textContent, cost2Text.textContent] = s.cost;
    } else if (s.ne > 0) {
      costText.textContent = `bytes written ${cur} / ${TOTAL}, in wire order`;
      cost2Text.textContent = `cache lines ≈ ${Math.ceil(cur / 64)} (illustrative, 64-byte aligned)`;
    } else {
      costText.textContent = `levels visited ${levels} / 5 · orders visited 0 / 9`;
      cost2Text.textContent = 'buffer bytes touched 0';
    }
  }

  const LB = 'Length builder: L3BookEncodedLength';
  const LE = 'Encoder: encode_book';
  const states = [
    {
      ms: 6500, lane: 'L3Book lane fixture', call: 'l3_tests.rs fixture()', nb: 0,
      cost: ['Each box has two bands:', 'top = counted by the builder, bottom = written'],
      caption: 'The L3 book fixture: bids with 3 levels holding 2, 1 and 3 orders, asks with 2 levels holding 1 and 2, then "BTCUSDT". Each box has two bands: the top fills as the length builder counts it, the bottom as encode_book writes it.',
    },
    {
      ms: 5000, lane: LB, nb: 1, hot: 'b',
      caption: 'compute_length() starts the length at the fixed block: 17 bytes (timestamp 8, sequence 8, isActive 1). The 8-byte message header is not counted yet; encoded_length_with_header() adds it at the very end.',
    },
    {
      ms: 5000, lane: LB, nb: 2, hot: 'b',
      caption: 'bids_ragged(3, …) adds the group header (4) and all three level blocks at once: 3 × 18 = 54 in one checked multiply, because every level block has the same width. len = 75.',
    },
    {
      ms: 5000, lane: LB, nb: 3, hot: 'b',
      caption: 'First level: g.add() adds 0 bytes (its block is already counted) and only counts the entry. orders(… uniform(2)) adds the nested header (4) and 2 × 17 for the orders. len = 113.',
    },
    {
      ms: 4500, lane: LB, nb: 4, hot: 'b',
      caption: 'The second level holds 1 order: 4 + 17, len = 134. Order counts differ per level, so the side cannot be multiplied out: one uniform(n) per level.',
    },
    {
      ms: 5500, lane: LB, nb: 5, hot: 'b',
      caption: 'The third level holds 3 orders: 4 + 3 × 17 = 55. len = 189, and 189 + 8 = 197, exactly where bids end on the wire. bids_ragged checks that add() ran 3 times; any other count is GroupCountMismatch.',
    },
    {
      ms: 5500, lane: LB, nb: 6, hot: 'b',
      caption: 'asks_ragged(2, …) repeats the pattern: 4 + 2 × 18 for header and level blocks, then 4 + 17 and 4 + 2 × 17 for the orders. len = 288; with the header, 296, where asks end.',
    },
    {
      ms: 4000, lane: LB, nb: 7, hot: 'b',
      caption: 'symbol(7) adds the 4-byte length prefix and the 7 bytes of "BTCUSDT". len = 299.',
    },
    {
      ms: 5500, lane: LB, nb: 8, hot: 'b',
      caption: 'encoded_length_with_header() adds the 8-byte header: 307. The builder visited 5 levels, never an order one by one, and read or wrote no buffer memory, only checked adds and multiplies.',
    },
    {
      ms: 5500, lane: 'Allocate exactly once', call: 'let mut storage = vec![0u8; 307];', nb: 8, alloc: true,
      cost: ['307 bytes reserved, 0 slack', 'MAX_ENCODED_LENGTH counts one group entry only'],
      caption: 'Allocate, or claim from an Aeron try_claim slot, exactly 307 bytes. No guess such as vec![0u8; 4096] and no copy out later; MAX_ENCODED_LENGTH would not do either, since it counts one group entry only.',
    },
    {
      ms: 4500, lane: LE, nb: 8, alloc: true, ne: 1, hot: 'e',
      caption: 'encode_book: try_wrap_and_apply_header writes the 8-byte header first, then fixed(…) the 17-byte block. Cursor 25. Unlike the builder, the encoder goes strictly in wire order.',
    },
    {
      ms: 5500, lane: LE, nb: 8, alloc: true, ne: 2, hot: 'e',
      caption: 'bids(3, …) writes the group header, then for each level its 18-byte block, the nested header and every order: 3 level blocks and 6 orders, one by one. Cursor 197, the builder\'s 189 + 8.',
    },
    {
      ms: 4500, lane: LE, nb: 8, alloc: true, ne: 3, hot: 'e',
      caption: 'asks(2, …) writes 2 level blocks and 3 orders. Cursor 296, the builder\'s 288 + 8.',
    },
    {
      ms: 5000, lane: LE, nb: 8, alloc: true, ne: 4, hot: 'e',
      caption: 'symbol_as_str("BTCUSDT") writes the length prefix and 7 bytes. The cursor stops at the end of the buffer, and encode_book returns encoded_length_with_header() = 307.',
    },
    {
      ms: 6000, lane: 'assert_eq!(len, actual)', call: '307 == 307', nb: 8, alloc: true, ne: 4, done: true,
      cost: ['no byte left over: no oversizing', 'no write past the end: no truncation'],
      caption: 'The fixture test asserts len == actual: 307 = 307. Both tracks end on the same byte. Given one byte less, encode_book stops with BufferTooShort when the symbol does not fit, rather than truncate; the same test checks that.',
    },
    {
      ms: 8000, lane: 'Why it wins here, when it loses', call: 'size exactly, then write', nb: 8, alloc: true, ne: 4, done: true,
      cost: ['builder: 5 levels, 0 orders, 0 buffer bytes', 'encoder: 5 levels, 9 orders, 307 bytes'],
      caption: 'Wins: a slot that must be claimed before writing gets its exact size, and the builder\'s work grows with levels, not orders. Loses: it is a second pass over the levels; with an oversized scratch buffer you skip it but copy instead. Flat shapes get a direct helper instead. The book records no timing for this builder.',
    },
  ];

  return {
    steps: states.map((s) => ({ caption: s.caption, ms: s.ms, show: () => render(s) })),
    onResize(width) {
      const narrow = width < 520;
      if (narrow === (g === NARROW)) return false;
      build(narrow);
      return true;
    },
  };
});
