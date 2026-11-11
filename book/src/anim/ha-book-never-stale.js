// HA order book: the LeadershipAwareBook never serves a stale price.
//
// Sources:
//   samples/cluster-ha-orderbook/src/ha_book.rs:14   StaleReason: NotReady, LeadershipRelease, SequenceGap, TermMismatch
//   samples/cluster-ha-orderbook/src/ha_book.rs:81   stale_reason() is None while serving
//   samples/cluster-ha-orderbook/src/ha_book.rs:97   live_image() is None unless serving; frozen_image() at :107 always
//   samples/cluster-ha-orderbook/src/ha_book.rs:112  on_leadership_release: serving = false, stale = LeadershipRelease,
//                                                    term, last_seq and image kept
//   samples/cluster-ha-orderbook/src/ha_book.rs:120  apply_snapshot: no checks, installs image, term, seq, serving = true
//   samples/cluster-ha-orderbook/src/ha_book.rs:143  apply_increment: serving? then term, then seq == last_seq + 1
//   samples/cluster-ha-orderbook/src/ha_book.rs:174  SequenceGap { expected, got }
//   samples/cluster-ha-orderbook/src/ha_book.rs:178  an applied increment replaces the bids and asks Vecs whole
//   samples/cluster-ha-orderbook/src/follower.rs:42  release counts a resync; :48 snapshot; :67 increment counters
//   samples/cluster-ha-orderbook/src/follower.rs:123 on the wire path a payload is a snapshot whenever not serving
//   samples/cluster-ha-orderbook/tests/ha_offline_pipeline.rs:49  release, dropped old-term increment, term-2 snapshot
//   samples/cluster-ha-orderbook/tests/ha_kill_leader.rs:94       the wire path passes the listener's current term
//   samples/cluster-ha-orderbook/tests/ha_kill_leader.rs:186      prices 5.00/5.01 before, 5.10/5.09/5.11 after
//   samples/cluster-ha-orderbook/src/publish.rs:144  publish_l2_snapshot: the leader sends whole L2 books
// Terms, sequences and the increments' levels are illustrative; every outcome
// follows ha_book.rs. An applied increment moves the caller's Vecs in and
// frees the old ones: no per-level copy, but a heap free per side.
// Costs are compares and field writes read from the code, not timings: the
// book records no timings for this sample.
Anim.register('ha-book-never-stale', (stage, api) => {
  const el = api.el;
  const WIDE = {
    W: 420, callFont: 11, outFont: 10, bw: 196, sx: 210, sy: 40, sw: 210,
    py: 40, bh: 134, sh: 134, cy: 190, cw: 101, cg: 5.3, fy: 242, H: 274, foot2: false,
  };
  const NARROW = {
    W: 300, callFont: 9.5, outFont: 8, bw: 300, sx: 0, sy: 164, sw: 300,
    py: 40, bh: 116, sh: 134, cy: 316, cw: 72, cg: 4, fy: 368, H: 416, foot2: true,
  };

  let g;
  let svg;
  let callText;
  let outText;
  let image;
  let rows;
  let stamp;
  let fields;
  let checks;
  let checkHead;
  let counterText;
  let costText;

  const price = (m) => (m / 100).toFixed(2);
  // Local, not in anim.js: ea-hot's 0.6 fill hides light text on dark themes,
  // so highlights keep its heavy outline with a lighter fill.
  function mark(rect, cls) {
    rect.setAttribute('class', cls);
    rect.style.fillOpacity = /\bea-hot\b/.test(cls) ? '0.32' : '';
  }

  function panel(x, y, w, h, title) {
    el('rect', { x, y, width: w, height: h, rx: 4, class: 'ea-box' }, svg);
    el('text', { x: x + 6, y: y + 13, 'font-size': 10, class: 'ea-strong' }, svg, title);
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    svg = api.svg(g.W, g.H, 'HA order book image beside its LeadershipAwareBook state and apply checks');
    callText = el('text', { x: 0, y: 13, 'font-size': g.callFont, class: 'ea-mono ea-strong' }, svg, '');
    outText = el('text', { x: 0, y: 28, 'font-size': g.outFont, class: 'ea-mono' }, svg, '');

    // Book image: asks above bids, two rows each, as a price ladder.
    panel(0, g.py, g.bw, g.bh, 'BookImage (L2)');
    el('text', { x: g.bw - 6, y: g.py + 13, 'font-size': 8.5, 'text-anchor': 'end', class: 'ea-muted ea-mono' },
      svg, 'size ×10⁻⁴');
    image = el('g', {}, svg);
    rows = [];
    for (let i = 0; i < 4; i += 1) {
      const y = g.py + 20 + i * 20;
      const side = i < 2 ? 'ask' : 'bid';
      const rect = el('rect', { x: 6, y, width: g.bw - 12, height: 17, rx: 2, class: side === 'ask' ? 'ea-k2' : 'ea-k1' }, image);
      const label = el('text', { x: 12, y: y + 12, 'font-size': 9.5, class: 'ea-strong' }, image, side);
      const px = el('text', { x: g.bw / 2, y: y + 12, 'font-size': 10, 'text-anchor': 'middle', class: 'ea-mono' }, image, '');
      const sz = el('text', { x: g.bw - 12, y: y + 12, 'font-size': 10, 'text-anchor': 'end', class: 'ea-mono' }, image, '');
      rows.push({ rect, label, px, sz, side });
    }
    // Outside the dimmed group so it stays readable.
    stamp = el('text', { x: g.bw / 2, y: g.py + 116 - 8, 'font-size': 9, 'text-anchor': 'middle', class: 'ea-strong' }, svg, '');

    // LeadershipAwareBook state.
    panel(g.sx, g.sy, g.sw, g.sh, 'LeadershipAwareBook');
    fields = {};
    const names = [['term', 'leadership_term_id'], ['seq', 'last_seq'], ['serving', 'is_serving()'], ['live', 'live_image()'], ['stale', 'stale_reason()']];
    names.forEach(([key, label], i) => {
      const y = g.sy + 19 + i * 19;
      const tall = key === 'stale';
      const rect = el('rect', { x: g.sx + 4, y, width: g.sw - 8, height: tall ? 33 : 17, rx: 2, fill: 'none', stroke: 'none' }, svg);
      el('text', { x: g.sx + 9, y: y + 12, 'font-size': 9, class: 'ea-muted ea-mono' }, svg, label);
      const value = tall
        ? el('text', { x: g.sx + 9, y: y + 27, 'font-size': g.foot2 ? 8.5 : 7.5, class: 'ea-mono ea-strong' }, svg, '')
        : el('text', { x: g.sx + g.sw - 9, y: y + 12, 'font-size': 9.5, 'text-anchor': 'end', class: 'ea-mono ea-strong' }, svg, '');
      fields[key] = { rect, value };
    });

    // apply_increment's checks, in code order, then the result.
    checkHead = el('text', { x: 0, y: g.cy - 4, 'font-size': 9, class: 'ea-muted' }, svg, '');
    checks = [0, 1, 2, 3].map((i) => {
      const x = i * (g.cw + g.cg);
      const rect = el('rect', { x, y: g.cy, width: g.cw, height: 36, rx: 3, class: 'ea-box' }, svg);
      const top = el('text', { x: x + g.cw / 2, y: g.cy + 14, 'font-size': g.foot2 ? 8.5 : 9.5, 'text-anchor': 'middle', class: 'ea-mono' }, svg, '');
      const bottom = el('text', { x: x + g.cw / 2, y: g.cy + 29, 'font-size': 9.5, 'text-anchor': 'middle', class: 'ea-strong' }, svg, '');
      return { rect, top, bottom };
    });

    counterText = [el('text', { x: 0, y: g.fy, 'font-size': 9.5, class: 'ea-mono' }, svg, '')];
    if (g.foot2) counterText.push(el('text', { x: 0, y: g.fy + 14, 'font-size': 9.5, class: 'ea-mono' }, svg, ''));
    costText = el('text', { x: 0, y: g.fy + (g.foot2 ? 32 : 18), 'font-size': g.foot2 ? 8.5 : 9.5, class: 'ea-muted' }, svg, '');
  }

  function render(s) {
    callText.textContent = s.call;
    outText.textContent = s.out ? `→ ${s.out}` : '';
    const ladder = [s.asks[1], s.asks[0], s.bids[0], s.bids[1]];
    rows.forEach((r, i) => {
      const lvl = ladder[i];
      r.label.classList.toggle('ea-hidden', !lvl);
      r.px.textContent = lvl ? price(lvl[0]) : '';
      r.sz.textContent = lvl ? String(lvl[1]) : '';
      const hot = s.replaced && lvl !== undefined;
      mark(r.rect, `${r.side === 'ask' ? 'ea-k2' : 'ea-k1'}${hot ? ' ea-hot' : ''}${lvl ? '' : ' ea-hidden'}`);
      if (hot) r.rect.style.setProperty('--d', String(i));
    });
    const empty = s.asks.length === 0 && s.bids.length === 0;
    image.style.opacity = s.serving ? '1' : '0.3';
    stamp.textContent = s.serving || empty ? '' : 'frozen_image() only: not live';

    const values = {
      term: s.term === null ? 'None' : `Some(${s.term})`,
      seq: String(s.seq),
      serving: s.serving ? 'true' : 'false',
      live: s.serving ? 'Some(&image)' : 'None',
      stale: s.serving ? 'None' : `Some(${s.stale})`,
    };
    for (const [key, f] of Object.entries(fields)) {
      f.value.textContent = values[key];
      const hot = (s.changed || []).includes(key);
      mark(f.rect, hot ? 'ea-k5 ea-hot' : '');
      if (!hot) { f.rect.setAttribute('fill', 'none'); f.rect.setAttribute('stroke', 'none'); }
      else { f.rect.removeAttribute('fill'); f.rect.removeAttribute('stroke'); }
    }

    const snap = s.call.includes('apply_snapshot');
    const isInc = s.call.includes('apply_increment');
    checkHead.textContent = snap ? 'apply_snapshot runs none of the increment checks:'
      : isInc ? 'apply_increment checks, in order:' : 'No apply checks run on this step:';
    const labels = ['serving?', `term == ${s.checkTerm ?? 'book'}?`, `seq == ${s.checkSeq ?? 'next'}?`, 'result'];
    checks.forEach((c, i) => {
      c.top.textContent = labels[i];
      const r = (s.checks || [])[i];
      // pass = green dashed outline, fail = purple fill + ✗; the text carries it too.
      let cls = 'ea-box';
      let text = '–';
      if (r === 'pass') { cls = 'ea-box ea-ok'; text = '✓ yes'; }
      else if (r === 'fail') { cls = 'ea-k4 ea-hot'; text = '✗ no'; }
      else if (typeof r === 'string') { cls = r.startsWith('✗') ? 'ea-k4 ea-hot' : 'ea-k3 ea-hot'; text = r; }
      mark(c.rect, cls);
      c.bottom.textContent = text;
    });

    const [sn, inc, dr, rs] = s.counters;
    if (g.foot2) {
      counterText[0].textContent = `snapshots ${sn} · increments ${inc}`;
      counterText[1].textContent = `dropped ${dr} · resyncs ${rs}`;
    } else {
      counterText[0].textContent = `BookFollower: snapshots ${sn} · increments ${inc} · dropped ${dr} · resyncs ${rs}`;
    }
    costText.textContent = s.cost;
  }

  const S1 = { bids: [[500, 1]], asks: [[501, 2]] }; // ha_kill_leader.rs:186
  const I11 = { bids: [[500, 1], [499, 4]], asks: [[501, 2]] };
  const I12 = { bids: [[500, 2], [499, 4]], asks: [[501, 2], [502, 6]] };
  const S2 = { bids: [[510, 3], [509, 4]], asks: [[511, 5]] }; // ha_kill_leader.rs:237
  const states = [
    {
      ms: 4500, call: 'LeadershipAwareBook::new()', out: '', term: null, seq: 0, serving: false, stale: 'NotReady',
      bids: [], asks: [], counters: [0, 0, 0, 0], cost: 'No image yet.',
      caption: 'A follower starts not serving: no term, last_seq 0, stale reason NotReady. live_image() is None, so a reader gets no price at all rather than a made-up one.',
    },
    {
      ms: 5000, call: 'follower.apply_snapshot(term 1, seq 10, ..)', out: 'SnapshotApplied',
      term: 1, seq: 10, serving: true, stale: 'NotReady', ...S1, replaced: true, checks: [null, null, null, '✓ install'],
      changed: ['term', 'seq', 'serving', 'live', 'stale'], counters: [1, 0, 0, 0],
      cost: 'No checks; 4 field writes, both level Vecs moved in.',
      caption: 'A snapshot is a whole L2 book for a term. apply_snapshot checks nothing: it installs the image, the term and the sequence and starts serving. Prices are the kill-leader test\'s pre-failover book, bid 5.00 and ask 5.01.',
    },
    {
      ms: 4500, call: 'follower.apply_increment(term 1, seq 11, ..)', out: 'IncrementApplied',
      term: 1, seq: 11, serving: true, stale: 'NotReady', ...I11, replaced: true,
      checkTerm: 1, checkSeq: 11, checks: ['pass', 'pass', 'pass', '✓ apply'],
      changed: ['seq'], counters: [1, 1, 0, 0],
      cost: '3 compares; Vecs moved in, old ones freed; seq, ts written.',
      caption: 'Seq 11 passes all three checks: serving, same term, and exactly last_seq + 1. The sample then replaces the bids and asks Vecs whole, rather than editing single levels, and records seq 11.',
    },
    {
      ms: 4000, call: 'follower.apply_increment(term 1, seq 12, ..)', out: 'IncrementApplied',
      term: 1, seq: 12, serving: true, stale: 'NotReady', ...I12, replaced: true,
      checkTerm: 1, checkSeq: 12, checks: ['pass', 'pass', 'pass', '✓ apply'],
      changed: ['seq'], counters: [1, 2, 0, 0],
      cost: '3 compares; Vecs moved in, old ones freed; seq, ts written.',
      caption: 'Seq 12 is the next in line, so it applies the same way. Every applied increment costs the same three compares before it touches the image.',
    },
    {
      ms: 5500, call: 'follower.apply_increment(term 1, seq 14, ..)', out: 'ResyncRequired(SequenceGap { expected: 13, got: 14 })',
      term: 1, seq: 12, serving: false, stale: 'SequenceGap{expected:13, got:14}', ...I12,
      checkTerm: 1, checkSeq: 13, checks: ['pass', 'pass', 'fail', '✗ resync'],
      changed: ['serving', 'live', 'stale'], counters: [1, 2, 0, 1],
      cost: '3 compares; 2 field writes. The image and last_seq are left as they were.',
      caption: 'Seq 13 never arrived, so seq 14 fails the sequence check. The book stops serving at once: live_image() returns None. The last image is kept, greyed here, for frozen_image(), which is for debugging only.',
    },
    {
      ms: 5500, call: 'NewLeaderEvent → follower.on_leadership_release()', out: 'stale = LeadershipRelease',
      term: 1, seq: 12, serving: false, stale: 'LeadershipRelease', ...I12,
      changed: ['stale'], counters: [1, 2, 0, 2],
      cost: 'No compares; 2 field writes; term 1 kept to name late messages.',
      caption: 'The leader dies and a new one is elected. on_leadership_release sets serving false (already false here) and the reason to LeadershipRelease, replacing the gap. It keeps the term, last_seq and image. The follower counts it as a resync.',
    },
    {
      ms: 5000, call: 'follower.apply_increment(term 1, seq 13, ..)', out: 'DroppedNotServing',
      term: 1, seq: 12, serving: false, stale: 'LeadershipRelease', ...I12,
      checkTerm: 1, checkSeq: 13, checks: ['fail', null, null, '✗ drop'],
      changed: [], counters: [1, 2, 1, 2],
      cost: '1 compare; nothing written to the book.',
      caption: 'Even the missing seq 13 is dropped: the book has not served since the gap, and while not serving an increment is refused after one compare. It cannot patch a frozen image back to life. Only a snapshot can.',
    },
    {
      ms: 5500, call: 'follower.apply_snapshot(term 2, seq 1, ..)', out: 'SnapshotApplied',
      term: 2, seq: 1, serving: true, stale: 'LeadershipRelease', ...S2, replaced: true, checks: [null, null, null, '✓ install'],
      changed: ['term', 'seq', 'serving', 'live', 'stale'], counters: [2, 2, 1, 2],
      cost: 'No checks; 4 field writes, both level Vecs moved in.',
      caption: 'The new leader sends a full term-2 book: bids 5.10 and 5.09, ask 5.11, as in the kill-leader test. It replaces the frozen image whole, so the old 5.00 bid cannot survive the failover. The book serves again.',
    },
    {
      ms: 6000, call: 'follower.apply_increment(term 1, seq 13, ..)  // late', out: 'ResyncRequired(TermMismatch { book_term: 2, msg_term: 1 })',
      term: 2, seq: 1, serving: false, stale: 'TermMismatch{book_term:2, msg_term:1}', ...S2,
      checkTerm: 2, checkSeq: 2, checks: ['pass', 'fail', null, '✗ resync'],
      changed: ['serving', 'live', 'stale'], counters: [2, 2, 1, 3],
      cost: '2 compares; 2 field writes. The term-2 image is kept but not served.',
      caption: 'A term-1 increment from the old leader arrives late. It fails the term check, so it is never merged into the term-2 book. The book stops serving again and waits for the next snapshot: a stale message costs a resync, never a wrong price.',
    },
    {
      ms: 7000, call: 'The rule the kill-leader test proves', out: 'live_image() is Some only while serving',
      term: 2, seq: 1, serving: false, stale: 'TermMismatch{book_term:2, msg_term:1}', ...S2,
      counters: [2, 2, 1, 3],
      cost: 'Counts above come from the code; the book has no timings for this sample.',
      caption: 'Why this wins here: one to three integer compares per increment buy a hard rule, no silent last-good price across a gap or a leader change. When it loses: any gap or late message stops quoting until a full snapshot lands, and each increment swaps in caller-built Vecs and frees the old ones, which suits a sample, not a hot path.',
    },
  ];

  build(false);
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
