// Decoder lanes: how random access, memoized and staged decoders find a tail.
//
// Sources:
//   sbe/src/codegen/tail_cache.rs:204   base lane: tail_offset_k = walk_tail_{k-1}(tail_offset_{k-1}), no cache
//   sbe/src/codegen/tail_cache.rs:238   memoized ensure_tail_start: walk from the frontier, publish each end
//   sbe/src/codegen/runtime.rs:28       TailBoundaryCache<N>: known_through + one end slot per tail
//   sbe/src/codegen/runtime.rs:98       publish only at the frontier slot
//   sbe/src/codegen/group_decoder.rs:1804  a level with a nested group is skipped by walking its orders
//   samples/l3-book/schemas/l3-book.xml:30 L3Book: fixed block, bids, asks, symbol
//   samples/l3-book/tests/l3_tests.rs:992  lane fixture: bids 2/1/3 orders, asks 1/2, "BTCUSDT"
//   samples/l3-book/tests/l3_tests.rs:1126 memoized read order: symbol, bids, asks, symbol
//                                          (both lanes are shown in this order; decode_random_access
//                                          at :1026 reads symbol once, then bids and asks)
//   samples/l3-book/tests/l3_tests.rs:1076 staged: into_bids, into_asks, into_symbol_as_str
// Sizes from the generated codec: header 8, fixed block 17, level 18, order 17,
// group header 4, var-data length 4; book_encoded_length(fixture) = 307.
// Counts shown are entries (levels + orders) the code walks, not timings.
Anim.register('decoder-lanes', (stage, api) => {
  const ROW_H = 24;
  const BIDS = [[50800, [101, 102]], [50750, [103]], [50700, [104, 105, 106]]];
  const ASKS = [[50850, [201]], [50900, [202, 203]]];
  const el = api.el;
  // Wide: row labels in a left column. Narrow (phones): labels above each row
  // and a 300-unit viewBox, so text stays legible at 360px.
  const WIDE = { W: 400, X0: 72, S: 1.7, top: 40, pitch: 38, labelAbove: false, callFont: 11 };
  const NARROW = { W: 300, X0: 0, S: 1.7, top: 36, pitch: 44, labelAbove: true, callFont: 9 };

  let g;
  let svg;
  let nodes;
  let frames;
  let laneText;
  let callText;
  let cache;
  let slots;
  let ktText;
  let totals;

  function box(key, x, bytes, y, cls, label, labelCls) {
    const w = bytes * g.S;
    nodes.set(key, el('rect', { x, y, width: w, height: ROW_H, rx: 2, class: cls }, svg));
    if (label !== undefined) {
      el('text', {
        x: x + w / 2, y: y + 15.5, 'text-anchor': 'middle', 'font-size': 8.5,
        class: `ea-mono ${labelCls || ''}`,
      }, svg, String(label));
    }
    return x + w;
  }

  // Returns the y of the row's boxes.
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

  function frame(name, y, bytes) {
    frames[name] = el('rect', {
      x: g.X0 - 3, y: y - 3, width: bytes * g.S + 6, height: ROW_H + 6, rx: 4,
      fill: 'none', stroke: 'none',
    }, svg);
  }

  function side(prefix, levels, y, cls) {
    let x = box(`${prefix}dim`, g.X0, 4, y, cls);
    levels.forEach(([price, orders], i) => {
      x = box(`${prefix}${i}`, x, 18, y, cls, price, 'ea-strong');
      x = box(`${prefix}${i}d`, x, 4, y, cls);
      orders.forEach((id, j) => {
        x = box(`${prefix}${i}o${j}`, x, 17, y, `${cls} ea-sub`, id, 'ea-fine');
      });
    });
    return (x - g.X0) / g.S;
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const X0 = g.X0;
    const S = g.S;
    const yEnd = g.top + 4 * g.pitch;
    const H = yEnd + 70;
    stage.textContent = '';
    svg = api.svg(g.W, H, 'L3Book wire layout and the entries each decoder lane walks');
    nodes = new Map();
    frames = {};

    laneText = el('text', { x: 0, y: 13, 'font-size': 12, class: 'ea-strong' }, svg, '');
    callText = el('text', { x: 0, y: 28, 'font-size': g.callFont, class: 'ea-mono' }, svg, '');

    // Rows, top to bottom in wire order.
    const yF = rowLabel(0, 'fixed', 0);
    box('hdr', X0, 8, yF, 'ea-box');
    box('fixed', X0 + 8 * S, 17, yF, 'ea-k4', 'fixed');
    const yB = rowLabel(1, 'bids', 25);
    frame('bids', yB, side('b', BIDS, yB, 'ea-k1'));
    const yA = rowLabel(2, 'asks', 197);
    frame('asks', yA, side('a', ASKS, yA, 'ea-k2'));
    const yS = rowLabel(3, 'symbol', 296);
    box('slen', X0, 4, yS, 'ea-k5');
    box('sdat', X0 + 4 * S, 7, yS, 'ea-k5');
    el('text', { x: X0 + 11 * S + 5, y: yS + 15.5, 'font-size': 10, class: 'ea-mono' }, svg, '"BTCUSDT"  (ends at byte 307)');
    frame('symbol', yS, 11);

    // Memoized cache panel.
    const yC = yEnd - (g.labelAbove ? 4 : 1);
    cache = el('g', {}, svg);
    if (!g.labelAbove) el('text', { x: 0, y: yC + 15, 'font-size': 11, class: 'ea-strong' }, cache, 'cache');
    // TailBoundaryCache<3>: one slot per tail (bids, asks, symbol).
    slots = [0, 1, 2].map((k) => {
      const x = X0 + k * 72;
      const rect = el('rect', { x, y: yC, width: 68, height: 22, rx: 3, class: 'ea-box' }, cache);
      const text = el('text', { x: x + 34, y: yC + 15, 'font-size': 9, 'text-anchor': 'middle', class: 'ea-mono' }, cache, '');
      return { rect, text, k };
    });
    ktText = el('text', { x: X0 + 218, y: yC + 15, 'font-size': 9, class: 'ea-mono ea-muted' }, cache, '');

    // Running totals.
    el('text', { x: 0, y: yC + 44, 'font-size': 10, class: 'ea-muted' }, svg,
      'Entries walked only to find where a tail starts:');
    totals = [0, 1, 2].map((i) =>
      el('text', { x: i * (g.W / 3), y: yC + 61, 'font-size': 11, class: 'ea-mono' }, svg, ''));
  }

  function walk(prefix, levels) {
    const keys = [`${prefix}dim`];
    let entries = 0;
    levels.forEach(([, orders], i) => {
      keys.push(`${prefix}${i}`, `${prefix}${i}d`);
      orders.forEach((_, j) => keys.push(`${prefix}${i}o${j}`));
      entries += 1 + orders.length;
    });
    return { keys, entries };
  }
  const WB = walk('b', BIDS); // 3 levels + 6 orders = 9 entries
  const WA = walk('a', ASKS); // 2 levels + 3 orders = 5 entries
  const BOTH = WB.keys.concat(WA.keys);
  build(false);

  function render(s) {
    laneText.textContent = s.lane;
    callText.textContent = s.call;
    const order = new Map((s.walk || []).map((k, i) => [k, i]));
    for (const [key, node] of nodes) {
      const i = order.get(key);
      node.classList.toggle('ea-hot', i !== undefined);
      if (i !== undefined) node.style.setProperty('--d', String(i));
    }
    for (const [name, rect] of Object.entries(frames)) {
      rect.classList.toggle('ea-ok', s.target === name);
    }
    cache.classList.toggle('ea-hidden', !s.cache);
    const ends = (s.cache && s.cache.ends) || [];
    for (const slot of slots) {
      const end = ends[slot.k];
      slot.text.textContent = end === undefined ? `slot ${slot.k}: –` : `slot ${slot.k} = ${end}`;
      slot.rect.setAttribute('class', end === undefined ? 'ea-box' : 'ea-k3');
      slot.rect.classList.toggle('ea-ok', s.cache !== undefined && s.cache.hit === slot.k);
    }
    ktText.textContent = `known_through ${ends.length}`;
    const t = s.totals || {};
    totals[0].textContent = `random ${t.ra === undefined ? '–' : t.ra}`;
    totals[1].textContent = `memoized ${t.me === undefined ? '–' : t.me}`;
    totals[2].textContent = `staged ${t.st === undefined ? '–' : t.st}`;
  }

  const RA = 'Random access: L3BookDecoder::try_decode';
  const ME = 'Memoized: decoder.memoized()';
  const ST = 'Staged: into_* stages';
  const states = [
    {
      ms: 6000, lane: 'L3Book lane fixture, 307 bytes', call: 'l3_tests.rs fixture()',
      caption: 'The L3Book test fixture: header and fixed block, then bids (3 levels holding 2, 1 and 3 orders), asks (2 levels holding 1 and 2 orders) and the symbol. A level has no fixed size, so where asks or symbol start is only known by walking everything before them.',
    },
    {
      ms: 5000, lane: RA, call: 'dec.symbol_as_str()', walk: BOTH, target: 'symbol', totals: { ra: 14 },
      caption: 'Random access reads symbol first. tail_offset_2 calls tail_offset_1, which calls tail_offset_0 (the end of the fixed block, byte 25, no walk). It then walks 3 bid levels and their 6 orders to byte 197, then 2 ask levels and 3 orders to byte 296: 14 entries.',
    },
    {
      ms: 3500, lane: RA, call: 'dec.bids()', target: 'bids', totals: { ra: 14 },
      caption: 'bids is tail 0: it starts where the fixed block ends, byte 25, so nothing is walked. The loop then reads each level; reading is not counted here.',
    },
    {
      ms: 4500, lane: RA, call: 'dec.asks()', walk: WB.keys, target: 'asks', totals: { ra: 23 },
      caption: 'asks is tail 1: tail_offset_1 walks all 3 bid levels and 6 orders again to reach byte 197, 9 more entries. The base decoder keeps no state, which is why it stays small and Sync.',
    },
    {
      ms: 5000, lane: RA, call: 'dec.symbol_as_str()  // again', walk: BOTH, target: 'symbol', totals: { ra: 37 },
      caption: 'Reading symbol again repeats the whole walk, 14 more entries. In this read order the base lane walks 37 entries to find tails it had already found.',
    },
    {
      ms: 4000, lane: ME, call: 'let dec = dec.memoized();', cache: { ends: [] }, totals: { ra: 37, me: 0 },
      caption: 'memoized() wraps the same decoder with a TailBoundaryCache: one end-offset slot per tail (three here) and a frontier, known_through. Building it is O(1) and allocates nothing.',
    },
    {
      ms: 5000, lane: ME, call: 'dec.symbol_as_str()', walk: BOTH, target: 'symbol',
      cache: { ends: [197, 296] }, totals: { ra: 37, me: 14 },
      caption: 'A miss: ensure_tail_start(2) walks forward from the frontier. Bids end at byte 197 (published to slot 0), asks end at byte 296 (slot 1). 14 entries, the same as the first random-access read. Slot 2 (where symbol ends) stays empty: only a call that needs the full length, such as encoded_length(), fills it.',
    },
    {
      ms: 3500, lane: ME, call: 'dec.bids()', target: 'bids', cache: { ends: [197, 296] }, totals: { ra: 37, me: 14 },
      caption: 'bids is tail 0 and starts at the end of the fixed block, byte 25, without consulting the cache.',
    },
    {
      ms: 3500, lane: ME, call: 'dec.asks()', target: 'asks', cache: { ends: [197, 296], hit: 0 }, totals: { ra: 37, me: 14 },
      caption: 'asks starts where bids end: slot 0 holds 197, a cache hit. Nothing is walked.',
    },
    {
      ms: 4500, lane: ME, call: 'dec.symbol_as_str()  // again', target: 'symbol', cache: { ends: [197, 296], hit: 1 }, totals: { ra: 37, me: 14 },
      caption: 'symbol again: slot 1 holds 296, another hit. Each boundary was walked once, 14 entries in total. The cost is the Cell cache: the memoized decoder is Send but not Sync.',
    },
    {
      ms: 5500, lane: ST, call: 'into_bids(..)?.into_asks(..)?.into_symbol_as_str()?', walk: BOTH, target: 'symbol',
      totals: { ra: 37, me: 14, st: 0 },
      caption: 'Staged decoding makes one pass in wire order. Each stage carries the cursor, so reading the entries is the walk and nothing is walked a second time. Reading symbol first does not compile: into_symbol_as_str exists only on the stage after asks.',
    },
    {
      ms: 6500, lane: 'Same four reads, three lanes', call: 'symbol, bids, asks, symbol', totals: { ra: 37, me: 14, st: 0 },
      caption: 'Random access walks 37 entries just to find tails, memoized walks 14, and staged walks none beyond its single read, but only in wire order. These are entry counts from the code, not timings: benchmark before choosing memoized.',
    },
  ];

  return {
    steps: states.map((s) => ({ caption: s.caption, ms: s.ms, show: () => render(s) })),
    // Rebuild on crossing the phone breakpoint; true asks the runtime to redraw the step.
    onResize(width) {
      const narrow = width < 520;
      if (narrow === (g === NARROW)) return false;
      build(narrow);
      return true;
    },
  };
});
