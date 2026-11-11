// One limit cursor (sbe-tool) and ergo-sbe's split lanes, on one frame.
//
// Sources:
//   sbe/tests/fixtures/schemas/basic-group-schema.xml:20  TestMessage1: block 16, Tag1 u32 at 0,
//                                    group Entries (u16 blockLength + u8 count = 3 B), entry 28 B
//   sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs:581  fixture: tag1 7, "SYM1"/1, "SYM2"/-2, ""/0
//   sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs:623  both generators emit identical bytes
//   sbe/tests/sbe_tool_reference/basic_group/src/test_message_1_codec.rs:293  sbe-tool wrap(offset = body):
//                                    limit = offset + acting_block_length
//   .../test_message_1_codec.rs:316  header(h, 0) adds ENCODED_LENGTH (8) itself
//   .../test_message_1_codec.rs:332  tag_1 reads at self.offset + 0
//   .../test_message_1_codec.rs:337  entries_decoder(self) consumes the message decoder
//   .../test_message_1_codec.rs:383  group wrap reads the dimension at limit, limit += 3
//   .../test_message_1_codec.rs:361  every entry read goes through parent.as_ref().expect()
//   .../test_message_1_codec.rs:411  advance: offset = limit, limit += block_length
//   sbe/tests/sbe_tool_reference/basic_group/src/lib.rs:80  each read slices buf (one bound check)
//   sbe/src/codegen/message_decoder.rs:312   ergo-sbe body_offset = message_offset + HEADER_LENGTH
//   sbe/src/codegen/message_decoder.rs:335   extent proved once at wrap, reads unchecked after
//   sbe/src/codegen/message_decoder.rs:193   base decoder carries no cache: Sync
//   sbe/src/codegen/message_decoder.rs:1192  tail_offset_0 = byte_offset() + acting_block_length
//   sbe/src/codegen/message_decoder.rs:1243  group getter takes &self and calls tail_offset_k()
//   sbe/src/codegen/group_decoder.rs:744     fixed-stride next(): entry at offset, offset += block
//   sbe/src/codegen/tail_stages.rs:700       staged finish() returns the following stage
//   sbe/tests/conformance_test.rs:415        into_entries()?, for e in &mut ents, ents.finish()?
//   sbe/src/codegen/runtime.rs:29            memoized cache is Cell: Send, not Sync
//   book/src/sbe/benchmarks.md:18            the book records no benchmark result
// Every offset and value below is computed from the schema and fixture; the
// wrong reads (0 from padding, 65552 from the header) are what those bytes hold
// in the zeroed buffer the parity test encodes into. Costs are counts read off
// the source, not timings.
Anim.register('single-cursor-vs-lanes', (stage, api) => {
  const el = api.el;
  const HDR = 8; // messageHeader: blockLength, templateId, schemaId, version (u16 each)
  const BLOCK = 16; // TestMessage1 blockLength
  const DIM = 3; // groupSizeEncoding: u16 blockLength + u8 numInGroup
  const ENTRY = 28; // TagGroup1 char[20] + TagGroup2 i64 at 20
  const SYMS = ['"SYM1"', '"SYM2"', '""'];
  const G0 = HDR + BLOCK; // 24: group dimension
  const E0 = G0 + DIM; // 27: entry 0
  const E = (i) => E0 + i * ENTRY; // 27, 55, 83
  const END = E(SYMS.length); // 111
  const TAG1_WRONG = 16 | (1 << 16); // header bytes 0..4: blockLength 16, templateId 1 (LE)

  const WIDE = { W: 400, X0: 66, S: 3, labelAbove: false, font: 10, small: 8 };
  const NARROW = { W: 300, X0: 0, S: 300 / END, labelAbove: true, font: 9, small: 8 };
  const ROW_H = 22;

  let g;
  let svg;
  let rows;
  let laneText;
  let callText;
  let costText;
  const X = (b) => g.X0 + b * g.S;

  // Squeeze a line that would overflow its width rather than clip it.
  function setText(node, text, max) {
    node.textContent = text;
    node.removeAttribute('textLength');
    node.removeAttribute('lengthAdjust');
    if (!text || typeof node.getComputedTextLength !== 'function') return;
    let len = 0;
    try { len = node.getComputedTextLength(); } catch (_) { return; }
    if (len > max) {
      node.setAttribute('textLength', String(max));
      node.setAttribute('lengthAdjust', 'spacingAndGlyphs');
    }
  }

  function seg(row, key, from, to, cls, label, labelCls) {
    const x = X(from);
    const w = (to - from) * g.S;
    row.seg.set(key, el('rect', { x, y: row.y, width: w, height: ROW_H, rx: 2, class: cls }, svg));
    if (label) {
      el('text', {
        x: x + w / 2, y: row.y + 14.5, 'text-anchor': 'middle', 'font-size': g.small,
        class: `ea-mono ${labelCls || ''}`,
      }, svg, label);
    }
  }

  function buildRow(name, top) {
    const row = { seg: new Map() };
    if (g.labelAbove) {
      el('text', { x: 0, y: top + 9, 'font-size': 10, class: 'ea-strong' }, svg, name);
      row.y = top + 44;
    } else {
      el('text', { x: 0, y: top + 40, 'font-size': 11, class: 'ea-strong' }, svg, name);
      row.y = top + 26;
    }
    seg(row, 'hdr', 0, HDR, 'ea-box', 'hdr');
    seg(row, 'block', HDR, G0, 'ea-k4');
    seg(row, 'tag1', HDR, HDR + 4, 'ea-k5 ea-sub');
    el('text', { x: X((HDR + 4 + G0) / 2), y: row.y + 14.5, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-mono ea-fine' }, svg, 'block');
    seg(row, 'dim', G0, E0, 'ea-k3');
    SYMS.forEach((s, i) => {
      seg(row, `e${i}`, E(i), E(i + 1), 'ea-k1', s);
      seg(row, `e${i}t2`, E(i) + 20, E(i + 1), 'ea-k1 ea-sub');
    });
    // Byte offsets under the bar; 24 and 27 sit 3 bytes apart, so anchor them apart.
    const ty = row.y + ROW_H + 9;
    for (const [b, anchor] of [[0, 'start'], [HDR, 'middle'], [G0, 'end'], [E0, 'start'], [E(1), 'middle'], [E(2), 'middle'], [END, 'end']]) {
      el('text', { x: X(b), y: ty, 'font-size': g.small, 'text-anchor': anchor, class: 'ea-mono ea-muted' }, svg, String(b));
    }
    // Bytes this step reads: up to two ranges, outlined.
    row.hl = [0, 1].map(() => el('rect', { x: 0, y: row.y - 2, width: 0, height: ROW_H + 4, rx: 3, class: 'ea-hidden' }, svg));
    // The cursor: an arrow above the bar, moved with a CSS transform.
    row.cur = el('g', { class: 'ea-hidden' }, svg);
    row.cur.style.transform = `translate(${X(0)}px, ${row.y}px)`;
    el('line', { x1: 0, y1: -16, x2: 0, y2: -3, style: 'stroke: var(--fg); stroke-width: 1.5' }, row.cur);
    el('path', { d: 'M -4 -7 L 0 -1 L 4 -7 Z', style: 'fill: var(--fg)' }, row.cur);
    row.curText = el('text', { x: 0, y: -19, 'font-size': g.font, class: 'ea-mono ea-strong' }, row.cur, '');
    row.read = el('text', { x: 0, y: ty + 14, 'font-size': g.font, class: 'ea-mono' }, svg, '');
    row.bottom = ty + 14;
    return row;
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    const topA = 34;
    const pitch = g.labelAbove ? 104 : 92;
    const H = topA + 2 * pitch + 34;
    svg = api.svg(g.W, H, 'TestMessage1 frame decoded by sbe-tool (top) and ergo-sbe (bottom)');
    laneText = el('text', { x: 0, y: 12, 'font-size': 12, class: 'ea-strong' }, svg, '');
    callText = el('text', { x: 0, y: 27, 'font-size': g.font, class: 'ea-mono' }, svg, '');
    rows = { a: buildRow('sbe-tool', topA), b: buildRow('ergo-sbe', topA + pitch) };
    const h = rows.b.bottom + 24;
    svg.setAttribute('viewBox', `0 0 ${g.W} ${h}`);
    costText = el('text', { x: 0, y: h - 4, 'font-size': g.font, class: 'ea-muted' }, svg, '');
  }

  function renderRow(row, s) {
    s = s || {};
    const hot = new Map((s.hot || []).map((k, i) => [k, i]));
    for (const [key, node] of row.seg) {
      const i = hot.get(key);
      node.classList.toggle('ea-hot', i !== undefined);
      if (i !== undefined) node.style.setProperty('--d', String(i));
    }
    row.hl.forEach((rect, i) => {
      const h = (s.hl || [])[i];
      if (!h) { rect.setAttribute('class', 'ea-hidden'); return; }
      const [from, to, ok] = h;
      rect.setAttribute('x', X(from) - 1);
      rect.setAttribute('width', (to - from) * g.S + 2);
      // Correct read: dashed green outline; wrong read: solid heavy vermillion. Text says which.
      rect.setAttribute('class', ok ? 'ea-ok' : '');
      rect.setAttribute('style', ok ? 'fill: none' : 'fill: none; stroke: var(--ea-c2); stroke-width: 3');
    });
    if (s.cur) {
      const [at, label] = s.cur;
      const x = X(at);
      row.cur.classList.remove('ea-hidden');
      row.cur.style.transform = `translate(${x}px, ${row.y}px)`;
      // Keep the label on the page near either edge (the svg clips overflow).
      const anchor = x < g.W * 0.3 ? 'start' : x > g.W * 0.7 ? 'end' : 'middle';
      row.curText.setAttribute('text-anchor', anchor);
      row.curText.setAttribute('x', anchor === 'start' ? -Math.min(4, x) : anchor === 'end' ? Math.min(4, g.W - x) : 0);
      row.curText.textContent = label;
    } else {
      row.cur.classList.add('ea-hidden');
    }
    setText(row.read, s.read || '', g.W);
  }

  function render(st) {
    setText(laneText, st.lane, g.W);
    setText(callText, st.call || '', g.W);
    renderRow(rows.a, st.a);
    renderRow(rows.b, st.b);
    setText(costText, st.cost || '', g.W);
  }

  build(false);

  const ENTRIES = ['e0', 'e1', 'e2'];
  const P1 = 'Part 1: where wrap points';
  const P2 = 'Part 2: who carries the cursor';
  const states = [
    {
      ms: 6500, lane: `TestMessage1 frame, ${END} bytes`, call: 'tag1 = 7; entries: "SYM1" 1, "SYM2" -2, "" 0',
      a: { read: 'sbe-tool encoder: these bytes' },
      b: { read: 'ergo-sbe encoder: the same bytes' },
      cost: `header ${HDR} + block ${BLOCK} + group header ${DIM} + 3 × ${ENTRY}-byte entries`,
      caption: `One frame from the wire-parity test, drawn twice: the 8-byte header, the 16-byte fixed block (tag1 is its first 4 bytes), a 3-byte group header, then three 28-byte entries. Both generators write exactly these ${END} bytes; only the decoder APIs differ.`,
    },
    {
      ms: 5500, lane: P1, call: 'ToolDec::default().wrap(buf, 8, 16, 0).tag_1()',
      a: { cur: [HDR, 'wrap(buf, 8, ..)'], hl: [[HDR, HDR + 4, true]], read: 'tag_1(): offset 8 + field 0 = byte 8 → 7' },
      b: {},
      cost: 'reads 4 bytes; the slice index is one bound check per read',
      caption: 'sbe-tool\'s wrap takes the body offset: for a frame at byte 0 that is 8, already past the header. Each field reads at that offset plus its field offset, so tag1 comes from byte 8.',
    },
    {
      ms: 6000, lane: P1, call: 'TestMessage1Decoder::wrap(buf, 0, 16, 0).tag1()',
      a: { cur: [HDR, 'wrap(buf, 8, ..)'], hl: [[HDR, HDR + 4, true]], read: 'tag_1(): offset 8 + field 0 = byte 8 → 7' },
      b: { cur: [0, 'wrap(buf, 0, ..)'], hot: ['hdr'], hl: [[HDR, HDR + 4, true]], read: 'tag1(): 0 + HEADER_LENGTH 8 + 0 = byte 8 → 7' },
      cost: 'one extent check at wrap (header + 16-byte block fit), then unchecked reads',
      caption: 'ergo-sbe\'s wrap takes the message start, 0, the same offset try_decode and claim buffers use, and adds HEADER_LENGTH itself. Different arguments, same byte: both read tag1 at 8.',
    },
    {
      ms: 6500, lane: P1, call: 'TestMessage1Decoder::wrap(buf, 8, 16, 0)   // sbe-tool habit',
      a: { cur: [HDR, 'wrap(buf, 8, ..)'], hl: [[HDR, HDR + 4, true]], read: 'tag_1(): byte 8 → 7' },
      b: { cur: [HDR, 'wrap(buf, 8, ..)'], hl: [[HDR + 8, HDR + 12, false], [G0 + 8, E0 + 8, false]], read: 'wrong: tag1() reads byte 16, padding → 0' },
      cost: 'extent check passes (16 + 16 ≤ 111): no error, just shifted reads',
      caption: 'The commonest port bug: passing sbe-tool\'s 8 to ergo-sbe. Every field moves 8 bytes: tag1 is read from padding (0 here), and the group header would be read at byte 32, inside the first entry. The buffer is long enough, so nothing fails.',
    },
    {
      ms: 6500, lane: P1, call: 'ToolDec::default().wrap(buf, 0, 16, 0)   // ergo-sbe habit',
      a: { cur: [0, 'wrap(buf, 0, ..)'], hl: [[0, 4, false]], read: `wrong: tag_1() reads header bytes 0..4 → ${TAG1_WRONG}` },
      b: { cur: [0, 'wrap(buf, 0, ..)'], hot: ['hdr'], hl: [[HDR, HDR + 4, true]], read: 'tag1(): byte 8 → 7' },
      cost: 'nothing catches it: blockLength 16 and templateId 1 read as one u32',
      caption: `The reverse mistake reads the header as data: tag1 becomes ${TAG1_WRONG}. Only sbe-tool's bare wrap takes the body offset; its header(h, 0) path takes the message start and adds the 8 itself, like ergo-sbe.`,
    },
    {
      ms: 5500, lane: P2, call: 'let car = ToolDec::default().wrap(buf, 8, 16, 0);',
      a: { cur: [G0, `limit = ${G0}`], hot: ['block'], read: `TestMessage1Decoder { offset: 8, limit: ${G0} }` },
      b: {},
      cost: 'state lives in the decoder: offset plus a limit that only moves forward',
      caption: `sbe-tool's decoder also carries a limit cursor, set by wrap to the end of the fixed block (8 + 16 = ${G0}). Every group and var-data accessor reads at limit and moves it, so the call order is the wire walk.`,
    },
    {
      ms: 5500, lane: P2, call: 'let mut e = car.entries_decoder();   // takes car by value',
      a: { cur: [E0, `limit = ${E0}`], hl: [[G0, E0, true]], read: 'EntriesDecoder { parent: Some(car), count: 3 }' },
      b: {},
      cost: 'reads 3 bytes at limit; the message decoder moves into the group decoder',
      caption: `entries_decoder() consumes the message decoder, reads blockLength 28 and count 3 at limit ${G0}, and moves limit to ${E0}. The message decoder now sits inside the group decoder as its parent.`,
    },
    {
      ms: 5500, lane: P2, call: 'e.advance()?; e.tag_group_2()',
      a: { cur: [E(1), `limit = ${E(1)}`], hot: ['e0'], hl: [[E0 + 20, E(1), true]], read: `advance(): offset = ${E0}, limit = ${E(1)}; read at ${E0} + 20` },
      b: {},
      cost: 'per read, in the source: parent.as_ref().expect(), then a slice bound check',
      caption: `advance() sets the entry offset to limit (${E0}) and moves limit on by one entry (${E(1)}). Each field read fetches the buffer through the parent Option and slices it, so every read carries its own checks in the source.`,
    },
    {
      ms: 6000, lane: P2, call: 'while e.advance()?.is_some() { .. }  let car = e.parent()?;',
      a: { cur: [END, `limit = ${END}`], hot: ENTRIES, read: `parent() hands car back, limit ${END}: entries are behind it` },
      b: {},
      cost: `limit walked ${G0} → ${E0} → ${E(1)} → ${E(2)} → ${END}; going back means wrapping again`,
      caption: `After the last entry, parent() returns the message decoder with limit ${END}. Calling entries_decoder() again would read a group header past the frame, so a second look means a second wrap.`,
    },
    {
      ms: 6000, lane: P2, call: 'let dec = TestMessage1Decoder::try_decode(buf, 0)?;  dec.entries()?',
      a: { cur: [END, `limit = ${END}`], read: 'one &mut cursor, already at the end' },
      b: { cur: [G0, `tail_offset_0() = ${G0}`], hot: ['dim'], read: 'base decoder: &self, no cursor, Sync' },
      cost: 'byte_offset() + acting_block_length = 8 + 16: one add, no walk',
      caption: 'ergo-sbe\'s base decoder holds no cursor. entries() takes &self and recomputes where the group starts, which for the first tail is one addition. Call it again, or from another thread, and the answer is the same.',
    },
    {
      ms: 5500, lane: P2, call: 'for e in dec.entries()? { e.tag_group2(); }',
      a: { cur: [END, `limit = ${END}`], read: 'one &mut cursor, already at the end' },
      b: { cur: [END, `iterator offset = ${END}`], hot: ENTRIES, read: `next(): entry at offset, then offset += ${ENTRY}` },
      cost: `3 + 3 × ${ENTRY} bytes checked when the group opens; next() adds ${ENTRY}`,
      caption: 'A fixed-stride group hands back an iterator that owns its offset, not the decoder\'s. The whole group\'s extent was checked when it opened, so each entry costs an add; the decoder is untouched and can open the group again.',
    },
    {
      ms: 6000, lane: P2, call: 'let mut it = dec.into_entries()?; for e in &mut it {..}; it.finish()?',
      a: { cur: [END, `limit = ${END}`], hot: ENTRIES, read: 'same single pass, order by convention' },
      b: { cur: [END, `finish() → next stage at ${END}`], hot: ENTRIES, read: 'staged: the stage value carries the offset' },
      cost: 'one forward pass, like sbe-tool; no parent() hop, the stage holds the offset',
      caption: 'Staged decoding is sbe-tool\'s single pass made into types. into_entries() consumes the decoder, the stage carries the offset, and finish() returns the next stage. Reading a later tail first does not compile.',
    },
    {
      ms: 6000, lane: P2, call: 'dec.memoized()   // caches where later tails start',
      a: {},
      b: { cur: [G0, `tail_offset_0() = ${G0}, no walk`], hot: ['dim'], read: 'memoized: nothing to save on a one-group message' },
      cost: 'Cell cache: Send, not Sync; pays only when a later tail needs a walk',
      caption: 'memoized() remembers where later variable-length tails start. Here the only tail is entries, which starts after one addition, so there is nothing to cache. The decoder-lanes animation shows a message where it pays.',
    },
    {
      ms: 8000, lane: 'Same bytes, different jobs', call: 'sbe-tool: one cursor · ergo-sbe: base, staged, memoized',
      a: { read: 'one &mut cursor: order is a convention' },
      b: { read: 'any order + Sync, or one typed pass' },
      cost: 'counts come from the source; the book records no benchmark result',
      caption: 'Wins: one wrap offset for decode and claim, extents checked once, reads in any order from a shared &Decoder. Loses: a later dynamic tail is recomputed on every call (37 entries walked against memoized\'s 14 in the decoder-lanes fixture). For time, run just bench: the highest ergon / sbe-tool ratio must stay within its ceiling (1.00 for most pairs).',
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
