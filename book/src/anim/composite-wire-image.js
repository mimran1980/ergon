// Composites are their wire bytes: the feature-tour Engine three ways.
//
// Sources:
//   samples/sbe-feature-tour/schemas/feature-tour.xml:54   Engine: capacity u16, numCylinders u8,
//                                                           maxRpm constant, manufacturerCode char[3],
//                                                           fuel constant, efficiency, boosterEnabled, booster
//   samples/sbe-feature-tour/schemas/feature-tour.xml:45   Booster: boostType char + horsePower u8 (2 bytes)
//   samples/sbe-feature-tour/schemas/feature-tour.xml:108  Car.engine follows extras in the fixed block
//   sbe/src/codegen/field_type.rs:14       uint8/char -> u8, uint16 -> u16 (member Rust types)
//   sbe/src/codegen/runtime.rs:2230        byteOrder picks from_le_bytes or from_be_bytes
//   sbe/src/codegen/runtime.rs:2499        #[repr(transparent)] pub struct Engine(pub [u8; N])
//   sbe/src/codegen/runtime.rs:2516        const assert size_of::<Engine>() == N
//   sbe/src/codegen/runtime.rs:2748        EngineDecoder { buf, offset } flyweight
//   sbe/src/codegen/message_encoder.rs:798 encoder copies val.0 into the frame
//   sbe/src/codegen/message_decoder.rs:884 engine() flyweight at self.offset + 35
//   sbe/src/codegen/message_decoder.rs:908 engine_value(): read_bytes_unchecked::<N> copy
//   sbe/tests/composite_layout_test.rs:97  10-byte fixture d0 07 04 '1' '2' '3' 35 1 'N' 200
//     (the test uses sbe/tests/fixtures/schemas/example-schema.xml: same Engine and Car layout)
//   sbe/tests/composite_layout_test.rs:192 engine at frame byte 8 + 35 = 43
//   sbe/tests/sbe_tool_reference/all_types_be/src/lib.rs:115  u16::from_be_bytes in a BE schema
//
// Native layout, measured with offset_of! on rustc 1.99 (64-bit):
//   #[repr(C)] Engine { u16, u8, [u8; 3], i8, u8, Booster { u8, u8 } }: size 10, align 2,
//   same offsets as the wire. #[repr(C)] Car fixed fields: extras at 34, engine at 36,
//   size 48, align 8; the wire block is 45 bytes with engine at 35.
// The big-endian row is illustrative: feature-tour is little-endian.
// No timings are shown: the page asks readers to use live Criterion output.
Anim.register('composite-wire-image', (stage, api) => {
  const el = api.el;
  const ROW_H = 24;
  const WIDE = { W: 400, X0: 112, rows: [62, 132], above: false, H: 222, code: 10, note: 10 };
  const NARROW = { W: 300, X0: 0, rows: [76, 156], above: true, H: 250, code: 8.5, note: 9 };

  // [first byte, length, label, colour class] for the Engine wire image.
  const FIELDS = [
    [0, 2, 'capacity', 'ea-k1'],
    [2, 1, 'cyl', 'ea-k2'],
    [3, 3, 'mfr code', 'ea-k3'],
    [6, 1, 'eff', 'ea-k4'],
    [7, 1, 'on', 'ea-k5'],
    [8, 2, 'booster', 'ea-k1 ea-sub'],
  ];
  const fieldCls = (b) => FIELDS.find(([o, n]) => b >= o && b < o + n)[3];
  const LE = ['d0', '07', '04', '31', '32', '33', '23', '01', '4e', 'c8'];
  const BE = ['07', 'd0'].concat(LE.slice(2));
  const range = (a, n) => Array.from({ length: n }, (_, i) => a + i);

  let g;
  let svg;
  let layer;
  let laneText;
  let codeText;
  let resultText;
  let costText;
  let items;

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    svg = api.svg(g.W, g.H, 'Engine composite bytes: wire image, accessors and a native repr(C) layout');
    laneText = el('text', { x: 0, y: 13, 'font-size': 12, class: 'ea-strong' }, svg, '');
    codeText = el('text', { x: 0, y: 30, 'font-size': g.code, class: 'ea-mono' }, svg, '');
    layer = el('g', {}, svg);
    resultText = el('text', { x: 0, y: g.H - 22, 'font-size': g.note, class: 'ea-mono' }, svg, '');
    costText = el('text', { x: 0, y: g.H - 6, 'font-size': g.note, class: 'ea-muted' }, svg, '');
    items = new Map();
  }

  // Keyed items persist across steps, so a byte that moves slides (CSS transform).
  function item(key, make) {
    let it = items.get(key);
    if (!it) {
      const node = el('g', {}, layer);
      it = { node, rect: null, text: null };
      if (make === 'cell') it.rect = el('rect', { x: 0, y: 0, height: ROW_H, rx: 2 }, node);
      it.text = el('text', { 'text-anchor': 'middle', class: 'ea-mono' }, node, '');
      items.set(key, it);
    }
    it.used = true;
    return it;
  }

  function place(it, x, y) {
    it.node.style.transform = `translate(${x}px, ${y}px)`;
  }

  // One byte cell: c = { key, x, y, w, cls, text, hot, ghost }.
  function cell(c) {
    const it = item(c.key, 'cell');
    place(it, c.x, c.y);
    it.rect.setAttribute('width', String(Math.max(1, c.w - 1.5)));
    it.rect.setAttribute('class', `${c.cls}${c.hot ? ' ea-hot' : ''}`);
    if (c.ghost) it.rect.setAttribute('stroke-dasharray', '3 2');
    else it.rect.removeAttribute('stroke-dasharray');
    it.text.setAttribute('x', String(c.w / 2));
    it.text.setAttribute('y', '15.5');
    it.text.setAttribute('font-size', String(c.font || 9.5));
    it.text.setAttribute('class', `ea-mono${c.muted ? ' ea-muted' : ''}${c.hot ? ' ea-strong' : ''}`);
    it.text.textContent = c.text || '';
  }

  function label(key, x, y, text, opts) {
    const o = opts || {};
    const it = item(key, 'label');
    place(it, x, y);
    it.text.setAttribute('x', '0');
    it.text.setAttribute('y', '0');
    it.text.setAttribute('text-anchor', o.anchor || 'middle');
    it.text.setAttribute('font-size', String(o.font || 9));
    it.text.setAttribute('class', o.cls || 'ea-mono');
    it.text.textContent = text;
  }

  // Row heading: left column (wide) or above the row (narrow).
  function heading(row, main, sub) {
    const y = g.rows[row];
    if (g.above) {
      label(`h${row}`, 0, y - 22, main, { anchor: 'start', font: 10, cls: 'ea-strong' });
      label(`s${row}`, g.W, y - 22, sub || '', { anchor: 'end', font: 8.5, cls: 'ea-muted' });
    } else {
      label(`h${row}`, 0, y + 10, main, { anchor: 'start', font: 10, cls: 'ea-strong' });
      label(`s${row}`, 0, y + 23, sub || '', { anchor: 'start', font: 8.5, cls: 'ea-muted' });
    }
  }

  // A window of `count` byte slots starting at byte `first`, drawn in row `row`.
  function grid(row, first, count, numbers) {
    const w = (g.W - g.X0) / count;
    const y = g.rows[row];
    for (let i = 0; numbers !== false && i < count; i++) {
      label(`o${row}_${i}`, g.X0 + i * w + w / 2, y - 4, String(first + i), { font: count > 10 ? 7 : 8, cls: 'ea-muted ea-mono' });
    }
    return { w, y, x: (b) => g.X0 + (b - first) * w };
  }

  // The ten Engine bytes at window byte `at`; `bytes` null draws empty cells.
  function engine(prefix, gr, at, bytes, opts) {
    const o = opts || {};
    const hot = new Set(o.hot || []);
    for (let b = 0; b < 10; b++) {
      cell({
        key: `${prefix}${b}`, x: gr.x(at + b), y: gr.y, w: gr.w, cls: o.cls || fieldCls(b),
        text: bytes ? bytes[b] : '', hot: hot.has(b), ghost: o.ghost, font: gr.w < 24 ? 8 : 9.5,
      });
    }
    if (o.labels === false) return;
    for (const [off, n, name] of FIELDS) {
      const text = gr.w < 24 && name !== 'capacity' && name !== 'booster' ? '' : name;
      label(`${prefix}L${off}`, gr.x(at + off) + (n * gr.w) / 2, gr.y + ROW_H + 11, text, { font: 8.5 });
    }
  }

  function other(key, gr, b, n, text, opts) {
    const o = opts || {};
    for (let i = 0; i < n; i++) {
      cell({
        key: `${key}${i}`, x: gr.x(b + i), y: gr.y, w: gr.w, cls: o.cls || 'ea-box',
        text: o.cellText || '', muted: true, ghost: o.ghost, font: 7.5,
      });
    }
    if (text) label(`${key}L`, gr.x(b) + (n * gr.w) / 2, gr.y + ROW_H + 11, text, { font: 8 });
  }

  // Scene drawers, one per state.
  const SCENES = {
    schema() {
      heading(0, 'Engine', 'schema, 10 bytes');
      engine('e', grid(0, 0, 10), 0, null);
      label('const', g.above ? 0 : g.X0, g.rows[0] + ROW_H + 30,
        'maxRpm = 9000, fuel = "Petrol": constant, 0 bytes', { anchor: 'start', font: 9, cls: 'ea-muted' });
    },
    image(hot) {
      heading(0, 'Engine.0', 'wire image, LE');
      engine('e', grid(0, 0, 10), 0, LE, { hot });
    },
    bigEndian() {
      heading(0, 'Engine.0', 'littleEndian schema');
      engine('e', grid(0, 0, 10), 0, LE, { hot: [0, 1] });
      heading(1, 'Engine.0', 'bigEndian (illustrative)');
      engine('b', grid(1, 0, 10), 0, BE, { hot: [0, 1] });
    },
    ghostEngine() {
      heading(0, 'Engine.0', 'wire image, align 1');
      engine('e', grid(0, 0, 10), 0, LE);
      heading(1, '#[repr(C)]', 'native, align 2');
      engine('g', grid(1, 0, 10), 0, LE, { ghost: true });
    },
    block() {
      const w = grid(0, 32, 16);
      heading(0, 'Car block', 'wire, bytes 32–47');
      other('wv', w, 32, 2, 'vc');
      other('wx', w, 34, 1, 'ext');
      engine('e', w, 35, LE, { labels: false, hot: [0, 1] });
      label('wE', w.x(35) + 5 * w.w, w.y + ROW_H + 11, 'engine 35–44', { font: 8 });
      other('wn', w, 45, 3, 'group');
      const n = grid(1, 32, 16);
      heading(1, '#[repr(C)]', 'native, bytes 32–47');
      other('gv', n, 32, 2, 'vc', { ghost: true });
      other('gx', n, 34, 1, 'ext', { ghost: true });
      other('gp', n, 35, 1, '', { ghost: true, cellText: 'pad' });
      // What an overlay at 36 would read from the wire bytes: everything one byte late.
      engine('g', n, 36, LE.slice(1).concat('?'), { ghost: true, labels: false, hot: [0, 1] });
      label('gE', n.x(36) + 5 * n.w, n.y + ROW_H + 11, 'engine 36–45', { font: 8 });
      other('gt', n, 46, 2, '', { ghost: true, cellText: 'pad' });
    },
    frame(step) {
      heading(0, step === 'fly' ? 'EngineDecoder' : 'Engine', step === 'fly' ? '{ buf, offset: 43 }' : 'value, 10 bytes');
      if (step !== 'fly') engine('e', grid(0, 40, 16, false), 43, LE, { labels: false, hot: step === 'value' ? range(0, 10) : [] });
      const f = grid(1, 40, 16);
      heading(1, 'frame', 'bytes 40–55');
      other('fv', f, 40, 2, 'vc');
      other('fx', f, 42, 1, 'ext');
      const hot = step === 'fly' ? [0, 1] : range(0, 10);
      engine('f', f, 43, LE, { labels: false, hot });
      label('fE', f.x(43) + 5 * f.w, f.y + ROW_H + 11, 'engine 43–52', { font: 8 });
      other('fg', f, 53, 3, 'group');
      const arrow = { enc: '↓ copy_from_slice(&val.0)', value: '↑ read_bytes_unchecked::<10>', fly: '' }[step];
      label('arrow', f.x(43) + 5 * f.w, g.rows[0] + ROW_H + 24, arrow, { font: 9 });
    },
    takeaway() {
      const rows = [
        ['capacity()', '2 bytes read, in place'],
        ['engine_value()', '10-byte copy'],
        ['encode', '10-byte copy'],
        ['Car block', 'wire 45 B vs repr(C) 48 B'],
      ];
      rows.forEach(([a, b], i) => {
        const y = g.rows[0] - 10 + i * 22;
        label(`t${i}a`, 0, y, a, { anchor: 'start', font: 10, cls: 'ea-mono ea-strong' });
        label(`t${i}b`, g.above ? 110 : 140, y, b, { anchor: 'start', font: 10, cls: 'ea-mono' });
      });
    },
  };

  function render(s) {
    for (const it of items.values()) it.used = false;
    s.draw();
    for (const [key, it] of items) {
      if (!it.used) { it.node.remove(); items.delete(key); }
    }
    laneText.textContent = s.lane;
    codeText.textContent = g.above && s.codeShort ? s.codeShort : s.code;
    resultText.textContent = s.result || '';
    costText.textContent = s.cost || '';
  }

  const states = [
    {
      ms: 6500, lane: 'The schema decides the bytes', code: '<composite name="Engine">  feature-tour.xml:54',
      draw: () => SCENES.schema(), result: '2 + 1 + 3 + 1 + 1 + 2 = 10 wire bytes',
      cost: 'Constants live in the code, not the frame.',
      caption: 'Engine in the feature-tour schema: capacity (uint16), numCylinders, a 3-char manufacturer code, efficiency, boosterEnabled and a nested 2-byte Booster. maxRpm and fuel are constants, so they take no wire bytes; their accessors just return 9000 and "Petrol". SBE packs members back to back: 10 bytes, no gaps.',
    },
    {
      ms: 6000, lane: 'View 1: the value is the wire image', code: 'pub struct Engine(pub [u8; 10]);',
      draw: () => SCENES.image([]), result: 'size_of::<Engine>() == 10  // const assert',
      cost: 'A byte array: align 1, so it can sit at any address.',
      caption: 'The generated type is #[repr(transparent)] over [u8; 10]. These are the test fixture bytes: d0 07 is 2000 little-endian, then 4, "123", 35, 1 (true) and the Booster bytes "N" and 200. A compile-time assert locks the Rust size to the wire size.',
    },
    {
      ms: 5500, lane: 'View 2: accessors read at schema offsets', code: 'u16::from_le_bytes(read_bytes::<2>(&self.0, 0))',
      codeShort: 'u16::from_le_bytes(read_bytes::<2>(&self.0, 0))',
      draw: () => SCENES.image([0, 1]), result: 'capacity() = 0x07d0 = 2000',
      cost: 'Touches 2 bytes. On a little-endian CPU: one 16-bit load.',
      caption: 'capacity() reads bytes 0 and 1 and assembles them little-endian. Nothing else in the image is touched, and on a little-endian CPU the compiler lowers this to a single unaligned-safe load.',
    },
    {
      ms: 5000, lane: 'Nested composites are wire images too', code: 'Booster(read_bytes::<2>(&self.0, 8))',
      draw: () => SCENES.image([8, 9]), result: 'booster() = Booster([0x4e, 0xc8]) = NITROUS, 200 hp',
      cost: 'A 2-byte copy; no re-packing of members.',
      caption: 'booster() returns the nested Booster composite (feature-tour.xml:45) by copying its 2 bytes from offset 8. Booster is itself a [u8; 2] wire image with its own accessors, so composites nest without any layout translation.',
    },
    {
      ms: 6500, lane: 'Endianness is explicit, per schema', code: 'u16::from_be_bytes(read_bytes::<2>(&self.0, 0))',
      draw: () => SCENES.bigEndian(), result: 'both rows: capacity() = 2000',
      cost: 'BE schema on an LE CPU: one byte swap per multi-byte field.',
      caption: 'The schema\'s byteOrder picks the accessor. In a bigEndian schema the same 2000 is stored 07 d0 and read with from_be_bytes; single-byte members do not change. A native overlay would silently read 0xd007 = 53255. The lower row is illustrative: feature-tour is little-endian.',
    },
    {
      ms: 6500, lane: 'View 3: a native #[repr(C)] Engine', code: '#[repr(C)] struct Engine { capacity: u16, .. }',
      draw: () => SCENES.ghostEngine(), result: 'repr(C) Engine: size 10, align 2, same offsets',
      cost: 'No padding inside Engine. The catch is align 2.',
      caption: 'A hand-written #[repr(C)] Engine happens to match here: every member already falls on its natural boundary, so it is 10 bytes with the same offsets. But it needs 2-byte alignment because of capacity, and that is where a native layout stops matching the wire.',
    },
    {
      ms: 8000, lane: 'Where native layout breaks: inside Car', code: 'wire: engine @ 35 · repr(C): engine @ 36',
      draw: () => SCENES.block(), result: 'Car block: wire 45 B; repr(C) 48 B (3 pad bytes)',
      cost: 'An overlay would read capacity from 07 04: 0x0407 = 1031, not 2000.',
      caption: 'In the Car fixed block, engine follows extras at byte 34, so on the wire it starts at 35. A #[repr(C)] Car struct must put the 2-aligned Engine at 36, inserting a pad byte, and rounds the struct up to 48. Overlaid on the frame, it reads every member one byte late: capacity from 07 04, which is 1031.',
    },
    {
      ms: 6500, lane: 'Encode: copy the image into the frame', code: 'self.buf[offset..offset + 10].copy_from_slice(&val.0)',
      codeShort: 'buf[43..53].copy_from_slice(&val.0)',
      draw: () => SCENES.frame('enc'), result: 'frame bytes 43–52 = Engine.0',
      cost: 'One 10-byte copy. Odd offset: fine for bytes, misaligned for &repr(C).',
      caption: 'The encoder writes the whole value with one 10-byte copy to frame byte 43 (8-byte header + offset 35). No per-member encoding happens here. Byte 43 is an odd offset, which a [u8; 10] does not care about; in a buffer that starts 2-aligned (the usual case), a reference to a 2-aligned repr(C) Engine there would be misaligned, which is undefined behaviour.',
    },
    {
      ms: 6000, lane: 'Decode: the flyweight reads in place', code: 'dec.engine().capacity()  // offset 43',
      draw: () => SCENES.frame('fly'), result: 'capacity() = 2000 from frame bytes 43–44',
      cost: 'Nothing copied: 2 bytes read where they lie.',
      caption: 'dec.engine() returns EngineDecoder { buf, offset }, a borrowed view at byte 43 of the same frame. Its capacity() reads frame bytes 43 and 44 with the same from_le_bytes as the value. This is the default decode path.',
    },
    {
      ms: 5500, lane: 'Decode: or take an owned value', code: 'dec.engine_value()  // read_bytes_unchecked::<10>(buf, 43)',
      codeShort: 'dec.engine_value()',
      draw: () => SCENES.frame('value'), result: 'engine_value().0 == frame[43..53]',
      cost: 'One 10-byte copy, then the same accessors as View 1.',
      caption: 'engine_value() copies the 10 bytes once into an owned Engine. It is the same image the encoder wrote, with no field-by-field unpacking, but you pay the copy even if you then read one field.',
    },
    {
      ms: 9000, lane: 'Why the wire image wins, and what still costs', code: 'flyweight ≈ [u8; N] ≈ repr(C, packed): layout_access_bench',
      codeShort: 'flyweight ≈ [u8; N] ≈ repr(C, packed)',
      draw: () => SCENES.takeaway(), result: 'Read live Criterion output for timings.',
      cost: 'Costs: copying a large composite to read one field.',
      caption: 'Wins: one portable form that is the wire bytes, any alignment, either byte order, and on little-endian a field read is the same single load a packed overlay gives; layout_access_bench measures the three as equivalent for one field. Still costs: materialising a big composite to read one field (the bench\'s 256-byte copy; use the flyweight instead) and, for big-endian schemas on little-endian CPUs, a swap per multi-byte field, which any correct reader pays.',
    },
  ];

  build(false);
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
