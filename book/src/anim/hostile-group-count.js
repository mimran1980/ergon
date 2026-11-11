// Trust boundary: a lying numInGroup in a fixed-stride group is caught before any entry.
//
// Sources:
//   sbe/src/codegen/message_decoder.rs:444  try_decode: header + fixed body proof, Err when short
//   sbe/src/codegen/message_decoder.rs:489  decode: same identity checks, panics when short (wrap)
//   sbe/src/codegen/message_decoder.rs:525  decode_unchecked: unchecked extent (UB if short)
//   sbe/src/codegen/group_decoder.rs:340    group wrap: the only public group constructor, returns Result
//   sbe/src/codegen/group_decoder.rs:228    wrap_with_parent: header fits, dimensions, then extent
//   sbe/src/codegen/tail_stages.rs:524      staged into_<group> reaches the group through wrap_with_parent
//   sbe/src/codegen/runtime.rs:886          checked_group_count: numInGroup <= u32::MAX
//   sbe/src/codegen/runtime.rs:903          checked_group_dimensions: short-block rule (count > 0, block < min_fixed)
//   sbe/src/codegen/group_decoder.rs:92     fixed_extent_validation: count.checked_mul(block) vs bytes left
//   sbe/src/codegen/group_decoder.rs:137    dynamic groups: per-entry acting_block_length proof instead
//   sbe/src/codegen/tail_cache.rs:95        walk_tail: header fits, same short-block rule (:105), then skip per entry
//   sbe/src/codegen/tail_cache.rs:206       tail_offset_k: random access to a later tail walks the earlier ones
//   sbe/src/codegen/message_decoder.rs:1560 verify, fixed-stride group: count.checked_mul, MessageTooShort
//   sbe/src/codegen/group_decoder.rs:393    wrap_trusted: pub(crate) unsafe, internal (nested groups only)
//   sbe/src/codegen/group_decoder.rs:1746   fixed entry skip: block_len vs bytes left, per entry
//   sbe/tests/soundness_hostile_constructors_test.rs:177  count 2 with 0 or 1 entries present: wrap is Err
//   sbe/tests/hostile_input_replay_test.rs:92             blockLength {0,1,MAX} x count {0,1,MAX}: no panic
// No harness times the group checks (checked_unchecked_parity_test.rs times message/encoder constructors only).
// The frame is illustrative: 8-byte header, 16-byte fixed block, then a
// fixed-stride group whose entries need 17 bytes (min_readable_fixed_extent 17).
// Step 10 re-reads the same bytes as a dynamic group: 12-byte block + 5-byte var-data tail per entry.
// The checks and their order follow the code. No timings are shown: the book
// records none for these checks.
Anim.register('hostile-group-count', (stage, api) => {
  const el = api.el;
  const LEN = 79; // 8 header + 16 fixed + 4 group header + 3 x 17 entries
  // Narrow (phones): a 300-unit viewBox and larger text, so labels stay legible at 360px.
  // wrap = characters per line for the cost (sans) and result (mono) lines.
  const WIDE = { W: 400, claim: 96, font: 10, small: 9, row: 17, outX: 124, wrapCost: 84, wrapMono: 64 };
  const NARROW = { W: 300, claim: 82, font: 11, small: 10, row: 19, outX: 112, wrapCost: 54, wrapMono: 44 };

  let g;
  let svg;
  let boxes;
  let claim;
  let claimText;
  let callText;
  let checkRows;
  let tierHead;
  let tierRows;
  let costText;
  let resultText;
  let dimLabel;
  let checkHead;

  function box(key, from, bytes, y, cls, label) {
    const x = from * g.S;
    const w = bytes * g.S;
    boxes.set(key, el('rect', { x, y, width: w, height: 26, rx: 2, class: cls }, svg));
    let t;
    if (label !== undefined) {
      t = el('text', {
        x: x + w / 2, y: y + 16.5, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-mono',
      }, svg, label);
    }
    return t;
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    g.S = (g.W - g.claim - 4) / LEN;
    stage.textContent = '';
    boxes = new Map();
    const yB = 36;
    const yC = yB + 52;
    const yT = yC + 4 * g.row + 14;
    const yK = yT + 4 * g.row + 8;
    svg = api.svg(g.W, yK + 50, 'A hostile frame whose group header claims more entries than the buffer holds, and the checks that reject it');

    callText = el('text', { x: 0, y: 13, 'font-size': narrow ? g.small : g.font + 1, class: 'ea-mono ea-strong' }, svg, '');

    box('hdr', 0, 8, yB, 'ea-box', 'hdr');
    box('fixed', 8, 16, yB, 'ea-k4', 'fixed');
    box('dim', 24, 4, yB, 'ea-k5');
    // The 4-byte group header is too narrow to hold its label, so it sits above.
    dimLabel = el('text', { x: 26 * g.S, y: yB - 8, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-mono ea-strong' }, svg, '');
    ['e0', 'e1', 'e2'].forEach((k, i) => box(k, 28 + i * 17, 17, yB, 'ea-k1', k));
    const xEnd = LEN * g.S;
    claim = el('rect', {
      x: xEnd + 4, y: yB, width: g.claim, height: 26, rx: 2,
      fill: 'none', stroke: 'currentColor', 'stroke-dasharray': '4 3', 'stroke-opacity': 0.7,
    }, svg);
    claimText = el('text', {
      x: xEnd + 4 + g.claim / 2, y: yB + 11, 'text-anchor': 'middle', 'font-size': g.small - 1, class: 'ea-muted',
    }, svg, '');
    claimText.appendChild(el('tspan', { x: xEnd + 4 + g.claim / 2, dy: 0 }));
    claimText.appendChild(el('tspan', { x: xEnd + 4 + g.claim / 2, dy: 10 }));
    // Byte offsets under the boxes.
    // At phone width 24 and 28 would overlap; 28 (where entries start) matters more.
    const marks = narrow ? [[0, 'start'], [28, 'middle'], [LEN, 'end']] : [[0, 'start'], [24, 'middle'], [28, 'middle'], [LEN, 'end']];
    for (const [b, anchor] of marks) {
      el('text', { x: b * g.S, y: yB + 38, 'text-anchor': anchor, 'font-size': g.small - 0.5, class: 'ea-mono ea-muted' }, svg, String(b));
    }
    el('text', { x: 53.5 * g.S, y: yB + 38, 'text-anchor': 'middle', 'font-size': g.small - 0.5, class: 'ea-muted' }, svg, '51 bytes left');
    el('text', { x: xEnd, y: yB - 4, 'text-anchor': 'end', 'font-size': g.small - 0.5, class: 'ea-muted' }, svg, 'buffer ends');

    checkHead = el('text', { x: 0, y: yC, 'font-size': g.font, class: 'ea-strong' }, svg, 'Checks, in code order');
    checkRows = [0, 1, 2, 3].map((i) => {
      const y = yC + 5 + i * g.row;
      const rect = el('rect', { x: 0, y, width: 36, height: g.row - 3, rx: 2, class: 'ea-box' }, svg);
      const status = el('text', { x: 18, y: y + 10.5, 'text-anchor': 'middle', 'font-size': g.small - 0.5, class: 'ea-strong' }, svg, '');
      const label = el('text', { x: 42, y: y + 10.5, 'font-size': g.small, class: 'ea-mono' }, svg, '');
      return { rect, status, label };
    });

    tierHead = el('text', { x: 0, y: yT, 'font-size': g.font, class: 'ea-strong' }, svg, '');
    tierRows = [0, 1, 2].map((i) => {
      const y = yT + 5 + i * g.row;
      const rect = el('rect', { x: 0, y, width: g.W, height: g.row - 3, rx: 2, class: 'ea-box' }, svg);
      const name = el('text', { x: 4, y: y + 10.5, 'font-size': g.small, class: 'ea-mono ea-strong' }, svg, '');
      const out = el('text', { x: g.outX, y: y + 10.5, 'font-size': g.small, class: 'ea-mono' }, svg, '');
      return { rect, name, out };
    });

    costText = el('text', { x: 0, y: yK, 'font-size': g.small, class: 'ea-muted' }, svg, '');
    resultText = el('text', { x: 0, y: yK + 28, 'font-size': narrow ? g.small : g.font, class: 'ea-mono ea-strong' }, svg, '');
  }

  // Word-wrap into at most two tspans; SVG text does not wrap by itself.
  function wrap(node, text, max) {
    node.textContent = '';
    const lines = [''];
    for (const word of (text || '').split(' ')) {
      const cur = lines[lines.length - 1];
      if (cur && (cur + ' ' + word).length > max && lines.length < 2) lines.push(word);
      else lines[lines.length - 1] = cur ? `${cur} ${word}` : word;
    }
    lines.forEach((line, i) => el('tspan', { x: 0, dy: i === 0 ? 0 : '1.25em' }, node, line));
  }

  const STATUS = {
    pass: ['pass', 'ea-k3'],
    fail: ['FAIL', 'ea-k2 ea-hot'],
    skip: ['skip', 'ea-box'],
    todo: ['', 'ea-box'],
    run: ['…', 'ea-k5 ea-hot'],
  };

  function render(s) {
    callText.textContent = s.call;
    const hot = new Map((s.hot || []).map((k, i) => [k, i]));
    for (const [key, rect] of boxes) {
      const i = hot.get(key);
      rect.classList.toggle('ea-hot', i !== undefined);
      if (i !== undefined) rect.style.setProperty('--d', String(i));
      // Entries the caller would see are outlined as "reached".
      rect.classList.toggle('ea-ok', (s.reached || []).includes(key));
    }
    const [bl, n] = s.dim || [17, 1000];
    const [c1, c2] = s.claim || ['997 more', 'claimed, absent'];
    claimText.childNodes[0].textContent = c1;
    claimText.childNodes[1].textContent = c2;
    claim.setAttribute('stroke-width', s.hotClaim ? 2.5 : 1);
    dimLabel.textContent = `bl ${bl} · n ${n}`;

    const checks = s.checks || [];
    checkHead.classList.toggle('ea-hidden', checks.length === 0);
    checkRows.forEach((row, i) => {
      const c = checks[i];
      row.label.textContent = c ? c[0] : '';
      const [word, cls] = STATUS[c ? c[1] : 'todo'];
      row.status.textContent = c ? word : '';
      row.rect.setAttribute('class', c ? cls : 'ea-box ea-hidden');
    });

    tierHead.textContent = s.tierHead || '';
    const tiers = s.tiers || [];
    tierRows.forEach((row, i) => {
      const t = tiers[i];
      row.rect.classList.toggle('ea-hidden', !t);
      row.name.textContent = t ? t[0] : '';
      row.out.textContent = t ? t[1] : '';
    });

    wrap(costText, s.cost, g.wrapCost);
    wrap(resultText, s.result, g.wrapMono);
  }

  build(false);

  const HDR = ['header fits: 24 + 4 ≤ 79', 'pass'];
  const COUNT = ['numInGroup 1000 ≤ u32::MAX', 'pass'];
  const SHORT = ['n > 0 ⇒ blockLength 17 ≥ 17', 'pass'];
  const MUL_FAIL = ['1000 × 17 = 17000 > 51 left', 'fail'];
  const TRY = 'try_decode';
  const BARE = 'decode';
  const UNCH = 'decode_unchecked';
  const SAME = 'group check runs: Err';

  const states = [
    {
      ms: 6500, call: 'hostile frame, 79 bytes (illustrative)', hot: ['dim'], hotClaim: true,
      caption: 'A hostile frame. Its group header says 1000 entries of 17 bytes each, but after the header only 51 bytes remain: room for 3 entries. The other 997 exist only in the header.',
    },
    {
      ms: 6000, call: 'Decoder::try_decode(buf, 0)', hot: ['hdr', 'fixed'],
      tierHead: 'Message tier: header + fixed body (8 + 16 ≤ 79)',
      tiers: [[TRY, 'proves 24 ≤ 79: Ok'], [BARE, 'same proof: Ok'], [UNCH, 'proof skipped (caller vouches)']],
      cost: 'Message constructors read only the 8-byte header and check the fixed block. Tails are not walked.',
      caption: 'Constructing the message proves only the header and fixed block. Here they are real, so all three tiers succeed. The lie sits in the group, which try_decode does not walk up front.',
    },
    {
      ms: 6500, call: 'if the buffer ended inside the fixed block…', hot: ['fixed'],
      tierHead: 'Where the tiers differ: a short fixed block',
      tiers: [[TRY, 'Err(BufferTooShort)'], [BARE, 'panics after the same proof'], [UNCH, 'no proof: UB if short']],
      caption: 'This is the only place the three tiers differ: what happens when the message’s own header or fixed block is short. try_decode returns Err, decode panics after the same proof, and decode_unchecked trusts the caller.',
    },
    {
      ms: 5500, call: 'dec.into_<group>(…)  →  wrap_with_parent', hot: ['dim'],
      checks: [HDR, ['numInGroup ≤ u32::MAX', 'todo'], ['short-block rule', 'todo'], ['count × blockLength ≤ bytes left', 'todo']],
      cost: 'Group check reads so far: 4 bytes (the group header). Entry bytes read: 0.',
      caption: 'Reaching the group calls wrap_with_parent. Check 1: does the 4-byte group header fit? 24 + 4 ≤ 79, so it reads blockLength 17 and numInGroup 1000. Unlike a message, a group has no tiers: its only public constructor, the group decoder’s wrap, returns Result.',
    },
    {
      ms: 5500, call: 'checked_group_dimensions("group", 1000, 17, 17)', hot: ['dim'],
      checks: [HDR, COUNT, SHORT, ['count × blockLength ≤ bytes left', 'todo']],
      cost: 'Group check reads so far: 4 bytes. Entry bytes read: 0. No multiply yet.',
      caption: 'checked_group_dimensions caps the count at u32::MAX and applies the short-block rule: with a positive count, each entry must be at least 17 bytes, the size its getters read. 17 ≥ 17, so both pass. The lie is not visible yet.',
    },
    {
      ms: 7000, call: 'count.checked_mul(block_length)', hot: ['dim'], hotClaim: true,
      checks: [HDR, COUNT, SHORT, MUL_FAIL],
      cost: 'Group check total: 4 bytes read, 1 multiply, the 4 checks above. Entry bytes read: 0.',
      result: 'Err(BufferTooShort { needed: 17000, available: 51 })',
      caption: 'A fixed-stride group proves its whole entry region at once: 1000 × 17 = 17000 bytes needed, 51 available. wrap_with_parent returns BufferTooShort before a single entry decoder is built. checked_mul also turns an overflowing product into an error instead of wrapping.',
    },
    {
      ms: 6500, call: 'same frame, any message tier', hot: ['dim'],
      checks: [HDR, COUNT, SHORT, MUL_FAIL],
      tierHead: 'The group check is the same after every tier',
      tiers: [[TRY, SAME], [BARE, SAME], [UNCH, SAME]],
      result: 'Err(BufferTooShort { needed: 17000, available: 51 })',
      caption: 'decode and decode_unchecked reach the same wrap_with_parent, so the lying count is an Err in every tier: never a panic, never UB. There is no public panicking or unchecked group constructor. The unchecked tier skips only the message’s fixed-body proof, not the group’s.',
    },
    {
      ms: 7000, call: 'second case: blockLength 0, numInGroup 1000', dim: [0, 1000], hot: ['dim'],
      claim: ['1000 × 0', 'bytes claimed'],
      checks: [HDR, COUNT, ['n > 0 ⇒ blockLength 0 ≥ 17', 'fail'], ['1000 × 0 = 0 ≤ 51 (never reached)', 'skip']],
      cost: 'Group check total: 4 bytes read, 0 multiplies. The product alone would pass.',
      result: 'Err(BufferTooShort { needed: 17, available: 0 })',
      caption: 'A zero blockLength with a positive count fools the product: 1000 × 0 = 0 fits anywhere. The short-block rule catches it first: getters still read 17 bytes per entry, and a 0-byte stride would put every entry on the same bytes.',
    },
    {
      ms: 7000, call: 'random access to a later tail: walk_tail', hot: ['e0', 'e1', 'e2'], hotClaim: true,
      checks: [HDR, COUNT, SHORT, ['skip e0, e1, e2; e3: 17 > 0 left', 'fail']],
      cost: 'The walk has no multiply: it skips entry by entry, 3 skips, then fails. Still 0 entry bytes read.',
      result: 'Err(BufferTooShort) at the 4th skip',
      caption: 'Random access to a tail after this group (a getter, encoded_length, the memoized decoder) walks it: the same header and short-block checks, then entry by entry. It fails at the fourth: cost grows with the entries present, not the 1000 claimed. verify and skip_<group> use the product instead.',
    },
    {
      ms: 7000, call: 'a dynamic group (sizes illustrative)', dim: [12, 1000], hot: ['e0', 'e1', 'e2'], reached: ['e0', 'e1', 'e2'],
      checks: [HDR, COUNT, ['n > 0 ⇒ blockLength 12 ≥ 12', 'pass'], ['no up-front product: per-entry proof', 'run']],
      cost: 'Each entry: 12-byte block + 5-byte var-data. Entries 0–2 reach the caller; the 4th fails its proof.',
      result: 'Some(Err(BufferTooShort { needed: 12, available: 0 })) on the 4th next()',
      caption: 'Now the entries hold var-data (here a 12-byte block plus a 5-byte tail), so there is no fixed stride and no product to check. Each entry’s block is proven before it is built and its tail before it is yielded. The 4th finds 0 bytes left; that error is returned once and the group is finished.',
    },
    {
      ms: 8000, call: 'takeaway', hot: ['dim'],
      checks: [HDR, COUNT, SHORT, MUL_FAIL],
      cost: 'Wins: a lying count costs 4 bytes read, 1 multiply and the four checks shown, whatever it claims.',
      result: 'No timing recorded for these checks.',
      caption: 'Why it wins: one header read and one multiply reject a fixed-stride lie of any size, in every tier. Where it loses: dynamic groups pay a check per entry and yield earlier entries first, and random access to a later tail walks even a fixed-stride group entry by entry. Neither the book nor a benchmark records timings for these checks.',
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
