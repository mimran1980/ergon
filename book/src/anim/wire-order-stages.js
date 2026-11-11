// Wire order stages: encoding the feature-tour Car through its type-state chain.
//
// Sources:
//   samples/sbe-feature-tour/src/lib.rs:125            encode_sample_car: the calls and values shown
//   samples/sbe-feature-tour/schemas/feature-tour.xml:99   Car: fixed block, fuelFigures, performanceFigures
//                                                      (nested acceleration), manufacturer, model, activationCode
//   sbe/src/codegen/message_encoder.rs:129             stage names: After{Tail}, the last tail returns {Name}Complete
//   sbe/src/codegen/message_encoder.rs:162             root stage: buf, msg_offset, offset + _header, _fields markers
//   sbe/src/codegen/message_encoder.rs:403             wrap sets offset = header + block length (53), before fixed()
//   sbe/src/codegen/message_encoder.rs:1037            fixed() returns the same CarEncoder with F = FieldsFixed
//   sbe/src/codegen/message_encoder.rs:1309            the first group method exists only on the FieldsFixed root
//   sbe/src/codegen/message_encoder.rs:1324            group: 4-byte header extent check, closure, written == count
//   sbe/src/codegen/message_encoder.rs:1512            var-data: length cap and extent check, then the next stage
//   sbe/src/codegen/message_encoder.rs:1636            as_bytes_with_header exists only on CarComplete
//   sbe/src/codegen/group_encoder.rs:150               add(): count check and entry extent check
//   sbe/src/codegen/group_encoder.rs:202               dynamic add(): closure must return the EntryComplete proof
//   sbe/src/codegen/tail_stages.rs:73                  decoder stages: the same idea, same-layout structs
//   api/generated/car_lean.txt:210                     CarEncoder::fixed / fuel_figures signatures
//   sbe/tests/typestate_budgets_test.rs:11             asks-before-bids must not compile ("no method named `asks`")
//   sbe/tests/typestate_budgets_test.rs:123            size_of CarEncoder == CarAfterFuelFigures == CarComplete
//   sbe/tests/reserved_name_clash_test.rs:176          fixed / as_bytes_with_header are reserved stage method names
// Byte map measured from encode_sample_car (142 bytes): header 0-8, fixed 8-53,
// fuelFigures 53-89 (dim 4, "Urban" entry 15, "Highway" entry 17),
// performanceFigures 89-110, manufacturer 110-119, model 119-132, activationCode 132-142.
// Stage size 32 bytes measured with size_of on a 64-bit target (rustc 1.99), all
// seven stages; needs_drop is false for each (CI pins CarEncoder only).
// Compile errors are rustc 1.99 output with the crate path trimmed.
// Check counts come from the generated code; they are not timings.
Anim.register('wire-order-stages', (stage, api) => {
  const el = api.el;
  const ROWS = [
    { stage: 'CarEncoder<Unfixed>', type: "CarEncoder<'_, HeaderPresent, FieldsUnfixed>",
      m: '.fixed(&fields)' },
    { stage: 'CarEncoder<Fixed>', type: "CarEncoder<'_, HeaderPresent, FieldsFixed>",
      m: '.fuel_figures(2, |g| …)' },
    { stage: 'CarAfterFuelFigures', type: "CarAfterFuelFigures<'_, HeaderPresent>",
      m: '.performance_figures(1, |g| …)' },
    { stage: 'CarAfterPerformanceFigures', type: "CarAfterPerformanceFigures<'_, HeaderPresent>",
      m: '.manufacturer_as_str("Honda")' },
    { stage: 'CarAfterManufacturer', type: "CarAfterManufacturer<'_, HeaderPresent>",
      m: '.model_as_str("Civic VTi")' },
    { stage: 'CarAfterModel', type: "CarAfterModel<'_, HeaderPresent>",
      m: '.activation_code_as_str("abcdef")' },
    { stage: 'CarComplete', type: "CarComplete<'_, HeaderPresent>",
      m: '.as_bytes_with_header()  ·  .encoded_length*()' },
  ];
  // [id, from, to, class, label]; a segment crossing a 64-byte row is split when drawn.
  const SEGS = [
    ['hdr', 0, 8, 'ea-k5', 'hdr'],
    ['fixed', 8, 53, 'ea-k4', 'fixed block, 45 B'],
    ['fdim', 53, 57, 'ea-k1', 'dim'],
    ['f1', 57, 72, 'ea-k1', 'Urban'],
    ['f2', 72, 89, 'ea-k1', 'Highway'],
    ['perf', 89, 110, 'ea-k2', 'performance, 21 B'],
    ['manu', 110, 119, 'ea-k3', 'Honda'],
    ['model', 119, 132, 'ea-k3', 'Civic VTi'],
    ['act', 132, 142, 'ea-k3', 'abcdef'],
  ];
  const LINE = 64;
  // Wide: stage name and method side by side. Narrow (phones): method under
  // the stage name and a 300-unit viewBox, so text stays legible at 360px.
  const WIDE = { W: 440, font: 9, rowH: 16, methodX: 168, methodDy: 0, X0: 28, S: 6.3, narrow: false };
  const NARROW = { W: 300, font: 9, rowH: 25, methodX: 22, methodDy: 11, X0: 20, S: 4.35, narrow: true };
  const CHAR = 0.6; // monospace advance per em; also a safe bound for the body font

  let g;
  let svg;
  let holdText;
  let callText;
  let ladder;
  let rows;
  let hiRow;
  let noteText;
  let segs;
  let cursor;
  let cursorText;
  let rowY;
  let costText;
  let errBox;
  let errText;
  let panels;

  const text = (parent, x, y, size, cls, value, extra) =>
    el('text', Object.assign({ x, y, 'font-size': size, class: cls || '' }, extra || {}), parent, value);

  // Greedy word wrap to `max` characters; returns lines.
  function wrap(s, max) {
    const out = [];
    let line = '';
    for (const word of s.split(' ')) {
      if (line && line.length + 1 + word.length > max) { out.push(line); line = word; } else line = line ? `${line} ${word}` : word;
    }
    if (line) out.push(line);
    return out;
  }

  // Fill a <text> with wrapped tspans; `items` is an array of strings or [cls, string].
  function para(node, items, opts) {
    node.textContent = '';
    const size = Number(node.getAttribute('font-size'));
    const max = Math.floor((opts.width || g.W) / (size * CHAR));
    const x = node.getAttribute('x');
    let first = true;
    for (const item of items) {
      const [cls, s] = Array.isArray(item) ? item : ['', item];
      if (!s) continue;
      for (const l of wrap(s, max)) {
        const t = el('tspan', { x, dy: first ? 0 : size + (opts.gap || 3) }, node, l);
        if (cls) t.setAttribute('class', cls);
        first = false;
      }
    }
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const W = g.W;
    const f = g.font;
    const ladderTop = 46;
    const ladderEnd = ladderTop + ROWS.length * g.rowH;
    const stripTop = ladderEnd + 44;
    const pitch = 32;
    const costTop = stripTop + 3 * pitch + 22;
    const H = costTop + (narrow ? 82 : 52);
    stage.textContent = '';
    svg = api.svg(W, H, 'Car encoder stages and the bytes each call writes');

    holdText = text(svg, 0, 11, f + 1, 'ea-mono ea-strong', '');
    callText = text(svg, 0, 25, f - 0.5, 'ea-mono', '');

    // Stage ladder: one row per type, with the method that moves it to the next
    // type (setters return &mut Self; _unchecked/_with/_unknown_size variants
    // lead to the same next type).
    ladder = el('g', {}, svg);
    hiRow = el('rect', { x: 0, y: 0, width: W, height: g.rowH - 1, rx: 3, class: 'ea-k3' }, ladder);
    hiRow.style.transform = `translate(0px, ${ladderTop}px)`;
    rows = ROWS.map((r, i) => {
      const y = ladderTop + i * g.rowH;
      const ty = y + (narrow ? 10 : g.rowH - 4.5);
      const arrow = text(ladder, 1, ty, f, 'ea-strong', '▶');
      const name = text(ladder, 12, ty, f, 'ea-mono', r.stage);
      const method = text(ladder, g.methodX, ty + g.methodDy, f, 'ea-mono', r.m);
      const mark = text(ladder, W - 3, ty + g.methodDy, f, 'ea-strong', '', { 'text-anchor': 'end' });
      return { arrow, name, method, mark, y };
    });
    noteText = text(svg, 0, ladderEnd + 12, f - 0.5, 'ea-muted', '');

    // Runtime and summary panels share the ladder's space.
    panels = { runtime: buildRuntime(ladderTop), summary: buildSummary(ladderTop) };

    // Byte strip: 142 bytes in 64-byte rows (one row = one cache line if buf is 64-byte aligned).
    text(svg, 0, stripTop - 8, f - 0.5, 'ea-muted',
      narrow ? 'buf: 142 B · one row = 64 B (a cache line if aligned)' : 'buf: 142 B, pre-sized by compute_length · one row = 64 B (a cache line if buf is 64-aligned)');
    rowY = (r) => stripTop + r * pitch;
    for (let r = 0; r < 3; r += 1) {
      text(svg, g.X0 - 3, rowY(r) + 12, f - 1.5, 'ea-mono ea-muted', String(r * LINE), { 'text-anchor': 'end' });
    }
    segs = new Map();
    for (const [id, from, to, cls, label] of SEGS) {
      const parts = [];
      let a = from;
      while (a < to) {
        const r = Math.floor(a / LINE);
        const b = Math.min(to, (r + 1) * LINE);
        const x = g.X0 + (a - r * LINE) * g.S;
        const w = (b - a) * g.S;
        const rect = el('rect', { x, y: rowY(r), width: w, height: 17, rx: 2, class: 'ea-box' }, svg);
        parts.push({ rect, x, w, y: rowY(r) });
        a = b;
      }
      const big = parts.reduce((p, q) => (q.w > p.w ? q : p));
      const lab = text(svg, big.x + big.w / 2, big.y + 12, f - 1.5, 'ea-mono', '', { 'text-anchor': 'middle' });
      segs.set(id, { parts, cls, label, lab, big });
    }
    cursor = el('g', {}, svg);
    el('path', { d: 'M0,0 l-4,7 h8 z', class: 'ea-k2', 'fill-opacity': 1 }, cursor);
    cursorText = text(cursor, 0, 15, f - 1.5, 'ea-mono', '', { 'text-anchor': 'middle' });

    costText = text(svg, 0, costTop, f, '', '');
    errBox = el('rect', { x: 0, y: costTop - f - 3, width: W, height: narrow ? 66 : 46, rx: 4, class: 'ea-k2', 'stroke-width': 2, 'stroke-dasharray': '6 3' }, svg);
    errText = text(svg, 6, costTop, f - 0.5, 'ea-mono', '');
  }

  function buildRuntime(top) {
    const p = el('g', {}, svg);
    const f = g.font;
    text(p, 0, top + 10, f + 1, 'ea-strong', 'Every stage is the same 32 bytes (64-bit target)');
    const fields = [['buf.ptr', g.narrow ? 'ptr' : '&storage'],['buf.len', '142'], ['msg_offset', '0'], ['offset', '142']];
    const bw = g.narrow ? 58 : 82;
    fields.forEach(([name, value], i) => {
      const x = i * (bw + 3);
      el('rect', { x, y: top + 18, width: bw, height: 34, rx: 3, class: i < 2 ? 'ea-k1' : 'ea-k4' }, p);
      text(p, x + bw / 2, top + 31, f - (g.narrow ? 1 : 0), 'ea-mono', name, { 'text-anchor': 'middle' });
      text(p, x + bw / 2, top + 45, f - 1, 'ea-mono', `${value} · 8 B`, { 'text-anchor': 'middle' });
    });
    const zx = 4 * (bw + 3) + 2;
    el('line', { x1: zx, y1: top + 16, x2: zx, y2: top + 54, stroke: 'currentColor', 'stroke-width': 2, 'stroke-dasharray': '3 2' }, p);
    text(p, zx + 4, top + 31, f - 1, 'ea-mono', g.narrow ? 'ZSTs' : '_header, _fields');
    text(p, zx + 4, top + 45, f - 1, 'ea-mono', '0 B');
    const body = text(p, 0, top + 70, f, '', '');
    para(body, [
      'size_of CarEncoder = CarAfterFuelFigures = CarComplete = 32 B, asserted equal in CI.',
      'No enum tag, no "which stage am I in?" branch, no heap, no Drop.',
      'A transition moves these words into a new type; inlining can remove the move.',
    ], {});
    return p;
  }

  function buildSummary(top) {
    const p = el('g', {}, svg);
    const body = text(p, 0, top + 10, g.font, '', '');
    para(body, [
      ['ea-strong', 'Car: 142 bytes in one forward pass.'],
      '7 stage types, 6 moves, 0 tags, 0 "which stage?" branches.',
      'Checks left are the ones the bytes need: extent, group counts, var-data length.',
      ['ea-strong', 'Wins: a wrong-order call cannot compile; no tag or branch at run time.'],
      ['ea-strong', 'Loses: one pass in wire order, no going back to an earlier tail;'],
      'group counts up front (or *_unknown_size); more generated types.',
      ['ea-muted', 'sbe-tool walks the same tree at run time, handing back with .parent().'],
    ], { gap: 5 });
    return p;
  }

  function render(s) {
    const held = ROWS[s.held];
    holdText.textContent = s.panel ? s.title : (g.narrow ? held.stage : `you hold: ${held.type}`);
    para(callText, [g.narrow && s.callN ? s.callN : s.call], {});

    const showLadder = !s.panel;
    ladder.classList.toggle('ea-hidden', !showLadder);
    noteText.classList.toggle('ea-hidden', !showLadder);
    panels.runtime.classList.toggle('ea-hidden', s.panel !== 'runtime');
    panels.summary.classList.toggle('ea-hidden', s.panel !== 'summary');
    hiRow.style.transform = `translate(0px, ${rows[s.held].y}px)`;
    rows.forEach((r, i) => {
      const current = i === s.held;
      const done = i < s.held;
      // On an error step, row `s.error.row` is the type that does have the
      // called method: point at it; the ✗ goes in the note about the held type.
      const owner = s.error && s.error.row === i;
      r.arrow.classList.toggle('ea-hidden', !current);
      r.name.setAttribute('class', `ea-mono${current ? ' ea-strong' : ' ea-muted'}`);
      // The method on row i exists only on type i: lit there, faded elsewhere.
      r.method.setAttribute('class', `ea-mono${current || owner ? ' ea-strong' : ''}`);
      r.method.style.opacity = current || owner ? '1' : (done ? '0.55' : '0.3');
      r.mark.textContent = owner ? '◀ only here' : (done ? '✓' : '');
    });
    noteText.textContent = s.note || '';

    const written = new Set(s.written);
    const hot = new Set(s.hot || []);
    for (const [id, seg] of segs) {
      const on = written.has(id);
      const stale = s.stale && id === 'fixed';
      for (const part of seg.parts) {
        part.rect.setAttribute('class', on ? seg.cls : 'ea-box');
        part.rect.style.opacity = on || stale ? '1' : '0.45';
        part.rect.classList.toggle('ea-hot', hot.has(id));
        if (stale) part.rect.setAttribute('stroke-dasharray', '3 2');
        else part.rect.removeAttribute('stroke-dasharray');
      }
      let label = on ? seg.label : (stale ? 'stale bytes' : '');
      if (label.length * (g.font - 1.5) * CHAR > seg.big.w - 2) label = '';
      seg.lab.textContent = label;
    }
    const off = s.offset;
    const r = Math.min(2, Math.floor(off / LINE));
    const col = off - r * LINE;
    cursor.style.transform = `translate(${g.X0 + col * g.S}px, ${rowY(r) + 19}px)`;
    cursorText.textContent = `offset ${off}`;
    cursorText.setAttribute('text-anchor', col > 52 ? 'end' : (col < 8 ? 'start' : 'middle'));

    const err = s.error;
    errBox.classList.toggle('ea-hidden', !err);
    para(errText, err ? err.lines : [], { width: g.W - 12, gap: 2 });
    para(costText, err ? [] : [['ea-strong', s.cost[0]], s.cost[1], s.cost[2]], {});
  }

  const ALL = SEGS.map((x) => x[0]);
  const upTo = (id) => ALL.slice(0, ALL.indexOf(id) + 1);
  const states = [
    {
      ms: 6000, held: 0, call: 'CarEncoder::try_wrap_and_apply_header(buf, 0)?', written: ['hdr'], hot: ['hdr'],
      stale: true, offset: 53,
      cost: ['wrap: 8 B header copied · 1 extent check (53 B fit)', 'offset already points past the fixed block.', 'The fixed block bytes are still whatever was in buf.'],
      caption: 'Wrapping the 142-byte buffer writes the 8-byte header and checks once that header plus fixed block (53 bytes) fit. offset is set to 53 straight away; the fixed bytes are still stale. The type is CarEncoder with F = FieldsUnfixed. Its field setters return &mut Self and never change the type; the only way forward is fixed().',
    },
    {
      ms: 6500, held: 0, call: 'e.fuel_figures(2, |g| …)   // skipping fixed()', written: ['hdr'], stale: true, offset: 53,
      note: '✗ fuel_figures is not on CarEncoder<Unfixed>.',
      error: {
        row: 1,
        lines: ['error[E0599]: no method named `fuel_figures` found for struct `CarEncoder<\'_>`',
          'note: the method was found for `CarEncoder<\'a, H, FieldsFixed>`', '(rustc 1.99, crate path trimmed)'],
      },
      caption: 'Skipping fixed() does not compile. fuel_figures is defined only on CarEncoder<…, FieldsFixed>, and rustc says so in its note. Stale fixed bytes can never reach the wire this way.',
    },
    {
      ms: 6000, held: 1, call: '.fixed(&CarFixedFields { serial_number: 1234, … })', written: upTo('fixed'), hot: ['fixed'], offset: 53,
      cost: ['fixed(): 45 B written · 0 bounds checks (wrap proved the extent)', 'Same struct, same buf, msg_offset and offset; only F changes.', 'The transition writes no tag and takes no branch.'],
      caption: 'fixed() writes all 45 bytes of the fixed block from one struct, with no bounds checks because wrap already proved they fit. It returns the same CarEncoder with F = FieldsFixed: same fields, same offset 53. Now fuel_figures is the only tail method that exists.',
    },
    {
      ms: 7000, held: 1, call: '.performance_figures(1, |g| …)   // before fuel_figures', written: upTo('fixed'), offset: 53,
      note: '✗ performance_figures is not on CarEncoder<Fixed>.',
      error: {
        row: 2,
        lines: ['error[E0599]: no method named `performance_figures` found', 'method not found in `CarEncoder<\'_, HeaderPresent, FieldsFixed>`', '(rustc 1.99, crate path trimmed)'],
      },
      caption: 'Writing the groups in the wrong order is the asks-before-bids bug: the bytes would still look valid. Here it does not compile, because performance_figures exists only on CarAfterFuelFigures. rustc names the type you hold; the stage list shows which one has the method. CI pins the same rule with an asks-before-bids compile-fail test.',
    },
    {
      ms: 6500, held: 1, call: '.fuel_figures(2, |g| { g.add(|e| … e.usage_description_as_str("Urban"))?; …',
      callN: '.fuel_figures(2, |g| { g.add(|e| … "Urban")?; …',
      written: upTo('f1'), hot: ['fdim', 'f1'], offset: 72,
      note: 'In the closure: add(|e| …) must return CarFuelFiguresEntryComplete.',
      cost: ['4 B group header + 15 B entry (6 B block, 4 B length, "Urban")', 'Checks: header extent, count, entry extent, length cap and extent.', 'The entry is complete only once usageDescription is written.'],
      caption: 'fuel_figures checks the 4-byte group header fits and writes blockLength 6 and count 2. Each add closure gets an entry encoder by value and must return CarFuelFiguresEntryComplete, which only usage_description_as_str produces: you cannot forget the entry\'s string either.',
    },
    {
      ms: 6000, held: 2, call: '  g.add(|e| … e.usage_description_as_str("Highway"))?; Ok(()) })?',
      callN: '  g.add(|e| … "Highway")?; Ok(()) })?',
      written: upTo('f2'), hot: ['f2'], offset: 89,
      cost: ['17 B entry, then written == count (2 == 2)', 'The value moves into CarAfterFuelFigures: the same 32 bytes.', 'The group straddles the first 64-byte line boundary.'],
      caption: 'The second entry fills bytes 72 to 89. After the closure, written must equal the declared count (a runtime check: a mismatch is an error, not a short group). The value moves into CarAfterFuelFigures, and the cursor arrow advances to performance_figures.',
    },
    {
      ms: 6000, held: 3, call: '.performance_figures(1, |g| { g.add(|e| { e.octane_rating(95); e.acceleration(2, …) }) })?',
      callN: '.performance_figures(1, |g| … e.acceleration(2, …))?',
      written: upTo('perf'), hot: ['perf'], offset: 110,
      note: 'Nested: the entry is complete only after acceleration(2, …) writes its group.',
      cost: ['21 B: header 4, octane 1, nested header 4, two 6 B rows', 'The nested group repeats the same header, count and extent checks.', 'Now CarAfterPerformanceFigures.'],
      caption: 'performance_figures has one entry holding a nested group. The same pattern repeats a level down: the entry closure must return its EntryComplete, which only acceleration(2, …) can produce. 21 bytes, ending at 110.',
    },
    {
      ms: 5500, held: 5, call: '.manufacturer_as_str("Honda")?.model_as_str("Civic VTi")?',
      callN: '.manufacturer_as_str("Honda")? .model_as_str("Civic VTi")?', written: upTo('model'), hot: ['manu', 'model'], offset: 132,
      cost: ['9 B + 13 B: a 4 B length prefix, then the bytes', 'Each var-data call: length cap and extent check, then the next type.', 'CarAfterPerformanceFigures → CarAfterManufacturer → CarAfterModel'],
      caption: 'Each var-data call writes a 4-byte length and the bytes, checks the length cap and that it fits, and returns the next stage. Two calls, two moves: CarAfterManufacturer, then CarAfterModel.',
    },
    {
      ms: 6000, held: 6, call: '.activation_code_as_str("abcdef")?.encoded_length_with_header()',
      callN: '.activation_code_as_str("abcdef")? .encoded_length_with_header()', written: ALL, hot: ['act'], offset: 142,
      cost: ['10 B, ending at byte 142 = encoded_length_with_header()', 'That is offset − msg_offset: arithmetic, no walk.', '3 cache lines touched if buf is 64-byte aligned.'],
      caption: 'The last tail returns CarComplete, not an After stage. Only CarComplete has as_bytes_with_header and encoded_length_with_header, which is offset minus msg_offset: 142. A message cannot be sent half-written.',
    },
    {
      ms: 7000, held: 6, panel: 'runtime', title: 'Runtime representation', call: 'struct CarComplete<\'a, H> { buf, msg_offset, offset, _header }',
      written: ALL, offset: 142,
      cost: ['7 types, one layout: the stage is the type, never a field.', 'No stage needs Drop (measured); CI asserts Send + Sync and no Drop for CarEncoder.', ''],
      caption: 'At run time every stage is the same 32 bytes on a 64-bit target: the buffer slice (pointer and length), msg_offset and offset. HeaderPresent and FieldsFixed are zero-sized markers. There is no enum tag to store and no "which stage?" branch to take.',
    },
    {
      ms: 9000, held: 6, panel: 'summary', title: 'Why it wins here, when it loses', call: 'encode_sample_car: 142 B',
      written: ALL, offset: 142,
      cost: ['No recorded timing isolates type-state; the book records none.', 'The sbe-tool comparison gate has a 1.00× ceiling (see Benchmarks).', ''],
      caption: 'Wins: order mistakes are compile errors, and the stages add no tag or branch at run time (their cost is not measured on its own); the checks left (extent, group counts, var-data length) are the ones the bytes need anyway. Loses: one forward pass in wire order, no going back to an earlier tail, counts declared up front (or *_unknown_size back-patching), and seven stage types for Car. sbe-tool walks the same tree at run time with .parent().',
    },
  ];

  build(false);
  return {
    steps: states.map((s) => ({ caption: s.caption, ms: s.ms, show: () => render(s) })),
    // Rebuild on crossing the phone breakpoint; true asks the runtime to redraw the step.
    onResize(width) {
      const narrow = width < 520;
      if (narrow === g.narrow) return false;
      build(narrow);
      return true;
    },
  };
});
