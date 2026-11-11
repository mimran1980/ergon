// Codegen pipeline: from schema XML to the generated Rust module.
//
// Sources:
//   sbe/src/build.rs:121            generate() = parse, Schema::from_ir, Generator::generate
//   sbe/src/xml/entry.rs:87         with_xsd_validation(): opt-in, root document only
//   sbe/src/xml/entry.rs:170        xsd_check runs on the raw text before roxmltree
//   sbe/src/xsd.rs:112              validate_against_sbe_xsd
//   sbe/src/xml/entry.rs:263        roxmltree Document, then parse_schema (xml/schema.rs:61)
//   sbe/src/ir.rs:195               Token { signal, name, id, encoding }; ir.rs:211 Ir
//   sbe/src/xml/entry.rs:299        resolve_schema runs inside parse, on the token Ir in place
//   sbe/src/resolve.rs:165          resolve_schema: duplicate ids, sinceVersion, defaults, offsets
//   sbe/src/resolve.rs:206          null/min/max defaults per primitive
//   sbe/src/resolve.rs:425          message field offsets; :450 message blockLength; :524 group's
//   sbe/src/schema.rs:73            Schema::from_ir only wraps the Ir
//   sbe/src/codegen/mod.rs:578      config validated before anything is emitted
//   sbe/src/codegen/mod.rs:1176     partition_tokens (structured_ir.rs:18)
//   sbe/src/structured_ir.rs:377    parse_message_structure
//   sbe/src/codegen/message_decoder.rs:38    decoder; staged at :1850 (tail_stages.rs:966),
//                                            memoized at :1896 (memoized_decoder.rs:36)
//   sbe/src/codegen/tail_stages.rs:1039      ordered lane (ordered_lane.rs:37)
//   sbe/src/codegen/message_encoder.rs:26    encoder; encoded_length::generate at :64
//   sbe/src/codegen/mod.rs:1511     syn parse, then prettyplease::unparse
//   sbe/src/build.rs:136            write_to_out_dir; all-or-none write at build.rs:205
//   sbe/src/build.rs:438            sbe_mod!: include!(concat!(env!("OUT_DIR"), ...))
//   sbe/src/xml/message.rs:394      undefined field type -> ParseError::Invalid
//   sbe/src/xml/error.rs:73         code ergo_sbe::schema_parse::invalid
//   sbe/src/config.rs:717, :1087    with_external_sbe_rt, with_shared_module
//
// The schema is illustrative. The token count (46), offsets, block lengths
// and generated names below were read from ergo-sbe's output for exactly
// this XML (verbatim below, saved as schema.xml) with
// GenerationConfig::new("quote"); the error text is its miette rendering
// with price's type changed to "Price". No timings are shown:
// the book records none.
//
//   <?xml version="1.0" encoding="UTF-8"?>
//   <messageSchema package="demo" id="7" version="0" byteOrder="littleEndian">
//     <types>
//       <composite name="messageHeader">
//         <type name="blockLength" primitiveType="uint16"/>
//         <type name="templateId" primitiveType="uint16"/>
//         <type name="schemaId" primitiveType="uint16"/>
//         <type name="version" primitiveType="uint16"/>
//       </composite>
//       <composite name="groupSizeEncoding">
//         <type name="blockLength" primitiveType="uint16"/>
//         <type name="numInGroup" primitiveType="uint16"/>
//       </composite>
//       <composite name="varStringEncoding">
//         <type name="length" primitiveType="uint32"/>
//         <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
//       </composite>
//     </types>
//     <message name="Quote" id="1">
//       <field name="price" id="1" type="int64"/>
//       <field name="qty" id="2" type="uint32"/>
//       <group name="legs" id="3" dimensionType="groupSizeEncoding">
//         <field name="px" id="4" type="int64"/>
//       </group>
//       <data name="venue" id="5" type="varStringEncoding"/>
//     </message>
//   </messageSchema>
Anim.register('codegen-pipeline', (stage, api) => {
  const el = api.el;
  const CHIPS = [
    ['schema.xml', 'SchemaSource'],
    ['XSD check', 'opt-in'],
    ['parse', 'token Ir'],
    ['resolve', 'inside parse'],
    ['Generator', '+ config'],
    ['OUT_DIR', 'sbe_mod!'],
  ];
  const XSD = 1;
  // Wide: one row of six stages. Narrow (phones): two rows of three and a
  // 300-unit viewBox, so text stays legible at 360px.
  const WIDE = { W: 420, perRow: 6, gap: 10, chipH: 32, rowGap: 0, font: 9, pitch: 12.5, title: 11 };
  const NARROW = { W: 300, perRow: 3, gap: 14, chipH: 30, rowGap: 18, font: 8.4, pitch: 12, title: 10.5 };
  const MAX_LINES = 12;

  let g;
  let svg;
  let chips;
  let packet;
  let panel;
  let panelTop;
  let H;
  let cost;

  function chipPos(i) {
    const w = (g.W - (g.perRow - 1) * g.gap) / g.perRow;
    const row = Math.floor(i / g.perRow);
    const col = i % g.perRow;
    return { x: col * (w + g.gap), y: 2 + row * (g.chipH + g.rowGap), w };
  }

  function arrow(x1, y1, x2, y2) {
    el('line', { x1, y1, x2: x2 - 4, y2, stroke: 'currentColor', 'stroke-width': 1.2, class: 'ea-muted' }, svg);
    el('path', { d: `M${x2} ${y2} l-5 -3 v6 z`, fill: 'currentColor', 'fill-opacity': 0.68 }, svg);
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const rows = Math.ceil(CHIPS.length / g.perRow);
    const chipsH = 2 + rows * g.chipH + (rows - 1) * g.rowGap;
    panelTop = chipsH + (narrow ? 22 : 28);
    H = panelTop + 18 + MAX_LINES * g.pitch + 34;
    stage.textContent = '';
    svg = api.svg(g.W, H, 'The ergo-sbe pipeline from schema XML to a generated Rust module');
    svg.style.color = 'var(--fg)';

    chips = CHIPS.map(([name, sub], i) => {
      const p = chipPos(i);
      const rect = el('rect', { x: p.x, y: p.y, width: p.w, height: g.chipH, rx: 4, class: 'ea-box' }, svg);
      if (i === XSD) rect.setAttribute('stroke-dasharray', '4 3');
      el('text', { x: p.x + p.w / 2, y: p.y + 13, 'text-anchor': 'middle', 'font-size': 9, class: 'ea-strong' }, svg, name);
      const tag = el('text', { x: p.x + p.w / 2, y: p.y + 25, 'text-anchor': 'middle', 'font-size': 7.5, class: 'ea-muted ea-mono' }, svg, sub);
      return { rect, tag, sub, p };
    });
    for (let i = 0; i + 1 < CHIPS.length; i += 1) {
      const a = chipPos(i);
      const b = chipPos(i + 1);
      const y = a.y + g.chipH / 2;
      if (a.y === b.y) {
        arrow(a.x + a.w + 1, y, b.x - 1, y);
      } else {
        // Wrap to the next row: down from the last chip, back to the first.
        const yMid = a.y + g.chipH + g.rowGap / 2;
        el('path', {
          d: `M${a.x + a.w / 2} ${a.y + g.chipH} V${yMid} H${b.x + b.w / 2} V${b.y - 4}`,
          fill: 'none', stroke: 'currentColor', 'stroke-width': 1.2, class: 'ea-muted',
        }, svg);
        el('path', { d: `M${b.x + b.w / 2} ${b.y} l-3 -5 h6 z`, fill: 'currentColor', 'fill-opacity': 0.68 }, svg);
      }
    }
    // The packet marks where the build currently is; it slides between stages.
    packet = el('circle', { cx: 0, cy: 0, r: 4, class: 'ea-k1 ea-hot' }, svg);

    el('line', {
      x1: 0, y1: panelTop - 10, x2: g.W, y2: panelTop - 10,
      stroke: 'currentColor', 'stroke-width': 0.6, class: 'ea-muted',
    }, svg);
    panel = el('g', {}, svg);
    cost = el('text', { x: 0, y: H - 6, 'font-size': g.font, class: 'ea-muted' }, svg, '');
  }

  // A line is a string, or { t, hot, muted, k } where k is an ea-k* colour
  // key drawn as a swatch (the text names the concern, so colour is not
  // the only cue). Hot lines get a marker and bold text.
  function drawPanel(s) {
    panel.textContent = '';
    // One monospace advance, so every character can sit on a fixed grid:
    // box-drawing glyphs fall back to a wider font and would push miette's
    // underline off its field otherwise. Measured per draw, after web fonts load.
    const probe = el('text', { 'font-size': g.font, class: 'ea-mono' }, panel, '0000000000');
    const adv = probe.getComputedTextLength() / 10 || 0.6 * g.font;
    probe.remove();
    const grid = (x0, n) => Array.from({ length: n }, (_, i) => (x0 + i * adv).toFixed(2)).join(' ');
    el('text', { x: 0, y: panelTop + 2, 'font-size': g.title, class: 'ea-strong' }, panel, s.title);
    el('text', {
      x: g.W, y: H - 6 - g.pitch, 'text-anchor': 'end', 'font-size': 7.5, class: 'ea-muted',
    }, panel, 'Illustrative Quote schema; values from ergo-sbe’s output');
    s.lines.forEach((raw, i) => {
      const line = typeof raw === 'string' ? { t: raw } : raw;
      const y = panelTop + 18 + i * g.pitch;
      let x = 10;
      if (line.k) {
        el('rect', { x: 10, y: y - 7.5, width: 7, height: 9, rx: 1.5, class: `ea-${line.k}${line.hot ? ' ea-hot' : ''}` }, panel);
        x = 21;
      }
      if (line.hot) el('text', { x: 0, y, 'font-size': g.font, class: 'ea-mono ea-strong' }, panel, '▸');
      const cls = ['ea-mono'];
      if (line.hot) cls.push('ea-strong');
      if (line.muted) cls.push('ea-muted');
      // No-break spaces stop SVG collapsing runs of spaces, so there is one
      // grid position per character (all of them are single UTF-16 units).
      const text = line.t.replace(/ /g, '\u00a0');
      el('text', { x: grid(x, text.length), y, 'font-size': g.font, class: cls.join(' ') }, panel, text);
    });
    if (s.strip) drawStrip(panelTop + 18 + s.lines.length * g.pitch + 2);
    // Fade the new panel in; reduced motion turns the transition off.
    panel.style.opacity = '0';
    panel.getBoundingClientRect(); // flush, so the change to 1 transitions
    panel.style.opacity = '1';
  }

  // Wire image of one Quote with 2 legs and a 4-byte venue (illustrative
  // values; sizes from the resolved tokens). Field labels are text, never
  // colour alone.
  function drawStrip(y) {
    const parts = [
      [8, 'header', 'ea-box'], [8, 'price', 'ea-k4'], [4, 'qty', 'ea-k4'],
      [4, 'dim', 'ea-k1'], [8, 'px', 'ea-k1 ea-sub'], [8, 'px', 'ea-k1 ea-sub'],
      [4, 'len', 'ea-k2'], [4, 'XLON', 'ea-k2 ea-sub'],
    ];
    const total = parts.reduce((n, [b]) => n + b, 0); // 48 bytes
    const S = g.W / total;
    let x = 0;
    const H = 16;
    for (const [bytes, label, cls] of parts) {
      const w = bytes * S;
      el('rect', { x, y, width: w - 0.5, height: H, rx: 1.5, class: cls }, panel);
      el('text', { x: x + w / 2, y: y + 11, 'text-anchor': 'middle', 'font-size': 7.5, class: 'ea-mono' }, panel, label);
      x += w;
    }
    const block = (x0, x1, text) => {
      el('path', { d: `M${x0} ${y + H + 3} v3 H${x1} v-3`, fill: 'none', stroke: 'currentColor', 'stroke-width': 0.8, class: 'ea-muted' }, panel);
      el('text', { x: (x0 + x1) / 2, y: y + H + 14, 'text-anchor': 'middle', 'font-size': 7.5, class: 'ea-muted' }, panel, text);
    };
    block(8 * S, 20 * S, 'block 12');
    block(24 * S, 40 * S, '2 × 8');
  }

  function render(s) {
    chips.forEach((c, i) => {
      let cls = 'ea-box';
      if (s.dead && s.dead.includes(i)) cls = 'ea-box';
      else if (i === s.at) cls = s.error ? 'ea-k2 ea-hot' : 'ea-k1 ea-hot';
      else if (s.done && s.done.includes(i)) cls = 'ea-k3';
      c.rect.setAttribute('class', cls);
      c.rect.style.opacity = s.dead && s.dead.includes(i) ? '0.35' : '1';
      // Lighter fill than ea-hot's so the label keeps its contrast on dark themes.
      c.rect.style.fillOpacity = i === s.at ? '0.32' : '';
      c.tag.textContent = i === s.at && s.error ? 'error' : (s.dead && s.dead.includes(i) ? 'not run' : c.sub);
    });
    const p = chips[s.at].p;
    // Wide: under the stage. Narrow: in its corner, clear of the row-wrap arrow.
    const at = g === NARROW ? [p.x + p.w - 7, p.y + 7] : [p.x + p.w / 2, p.y + g.chipH + 8];
    packet.style.transform = `translate(${at[0]}px, ${at[1]}px)`;
    packet.setAttribute('class', s.error ? 'ea-k2 ea-hot' : 'ea-k1 ea-hot');
    drawPanel(s);
    cost.textContent = s.cost || '';
  }

  const DEC = 'k1';
  const ENC = 'k2';
  const TYP = 'k4';
  const LEN = 'k5';
  const states = [
    {
      ms: 6500, at: 0, title: 'build.rs: generate(source, config)',
      lines: [
        { t: '<message name="Quote" id="1">' },
        { t: '  <field name="price" id="1" type="int64"/>', hot: true },
        { t: '  <field name="qty" id="2" type="uint32"/>', hot: true },
        { t: '  <group name="legs" id="3"', hot: true },
        { t: '         dimensionType="groupSizeEncoding">' },
        { t: '    <field name="px" id="4" type="int64"/>' },
        '  </group>',
        { t: '  <data name="venue" id="5"', hot: true },
        { t: '        type="varStringEncoding"/>' },
        '</message>',
        { t: '(illustrative; the <types> block is not shown)', muted: true },
      ],
      cost: 'Runs once per build, before your crate compiles.',
      caption: 'A build script passes generate() a SchemaSource and a GenerationConfig. generate() is three calls in a row: parse, Schema::from_ir and Generator::generate. This illustrative Quote message has two fixed fields, a repeating group and a var-data field.',
    },
    {
      ms: 6500, at: 1, done: [0], title: 'Optional: the XSD shape check',
      lines: [
        'SchemaSource::file("schema.xml")',
        { t: '    .with_xsd_validation()', hot: true },
        'validate_against_sbe_xsd(xml)      // xsd.rs',
        '  root element is <messageSchema>',
        '  required attributes are present',
        '  no unexpected elements or attributes',
        { t: 'fails as ergo_sbe::schema_parse::invalid', muted: true },
        { t: '  (unreadable XML: ...::malformed_xml)', muted: true },
        { t: 'off by default; included files are not checked', muted: true },
      ],
      cost: 'An extra parse of the root document, before the real one.',
      caption: 'with_xsd_validation() checks the root document against the embedded SBE XSD before anything is parsed. It is off by default and skips included files; parse alone already rejects malformed XML, unexpected elements and unknown attributes.',
    },
    {
      ms: 7000, at: 2, done: [0], title: 'parse: XML to a flat token stream',
      lines: [
        { t: 'BeginMessage   Quote   id=1', hot: true },
        '  BeginField   price   int64      / EndField',
        '  BeginField   qty     uint32     / EndField',
        { t: '  BeginGroup   legs    id=3', hot: true },
        { t: '    groupSizeEncoding  6 tokens, inlined', muted: true },
        '    BeginField px      int64      / EndField',
        '  EndGroup     legs',
        { t: '  BeginVarData venue   id=5', hot: true },
        { t: '    varStringEncoding  6 tokens, inlined', muted: true },
        '  EndVarData   venue',
        'EndMessage     Quote',
        { t: '46 tokens: 22 for the three composites, 24 Quote', muted: true },
      ],
      cost: 'Offsets are still blank: nothing is laid out yet.',
      caption: 'roxmltree reads the text, then parse_schema walks the elements into one flat Vec<Token>, sbe-tool style: Begin/End brackets around messages, fields, groups and var-data. Type references are inlined, so the group and the var-data each carry their own copy of their composite.',
    },
    {
      ms: 7500, at: 3, done: [0, 2], title: 'resolve: offsets, block lengths, defaults',
      strip: true,
      lines: [
        { t: 'BeginMessage   Quote   blockLength 12', hot: true },
        { t: '  BeginField   price   offset 0   null i64::MIN', hot: true },
        { t: '  BeginField   qty     offset 8   null u32::MAX', hot: true },
        { t: '  BeginGroup   legs    blockLength 8', hot: true },
        '    BeginField px      offset 0',
        '  BeginVarData venue   u32 length, then bytes',
        { t: 'checked: unique template ids, sinceVersion', muted: true },
      ],
      cost: 'Illustrative: 2 legs, venue "XLON", 48 bytes.',
      caption: 'Still inside parse(), resolve_schema fills the same tokens in place: each fixed field gets an offset (price 0, qty 8), the message its block length (12), the group its entry length (8), and each primitive its null, min and max. It also rejects duplicate template ids and a sinceVersion above the schema version.',
    },
    {
      ms: 7500, at: 4, done: [0, 2, 3], title: 'Schema::from_ir, then the Generator',
      lines: [
        'Schema { ir }            // wraps the Ir, no copy',
        'Generator::new(GenerationConfig::new("quote"))',
        { t: 'validate config: conversions, type mappings,', hot: true },
        { t: '  header values, encode version, paths', hot: true },
        'partition_tokens(&ir.tokens)',
        '  composites 3  enums 0  sets 0  messages 1',
        'parse_message_structure(Quote)',
        '  fields  price, qty',
        '  groups  legs { px }',
        '  data    venue',
      ],
      cost: 'A bad config fails here, before any Rust is emitted.',
      caption: 'Schema::from_ir only wraps the resolved Ir. Generator::generate first checks the GenerationConfig against the schema, then partition_tokens and parse_message_structure turn the flat tokens into typed messages, groups and fields: the structured IR every emitter reads.',
    },
    {
      ms: 8000, at: 4, done: [0, 2, 3], title: 'Types, then one decoder call per message',
      lines: [
        { t: 'types     sbe_rt, MessageHeader,', k: TYP, hot: true },
        { t: '          GroupSizeEncoding, VarStringEncoding', k: TYP, hot: true },
        { t: 'decoder   QuoteDecoder, LegsDecoder', k: DEC, hot: true },
        { t: '          message_decoder.rs', muted: true },
        { t: 'staged    into_legs, then into_venue', k: DEC, hot: true },
        { t: '          tail_stages.rs', muted: true },
        { t: 'ordered   ordered() -> sbe_rt::Ordered', k: DEC, hot: true },
        { t: '          ordered_lane.rs', muted: true },
        { t: 'memoized  memoized() -> QuoteMemoizedDecoder', k: DEC, hot: true },
        { t: '          memoized_decoder.rs', muted: true },
        { t: 'encoder   (next)', k: ENC, muted: true },
      ],
      cost: 'One decoder call emits every read lane.',
      caption: 'gen_schema emits the sbe_rt runtime and the composites first, then calls generate_message_decoder for Quote. That call produces every read lane: random-access getters, the staged into_* chain, ordered() over those stages, and the opt-in memoized() wrapper.',
    },
    {
      ms: 8000, at: 4, done: [0, 2, 3], title: 'The encoder and its length helpers',
      lines: [
        { t: 'types     sbe_rt, MessageHeader, ...', k: TYP, muted: true },
        { t: 'decoder   QuoteDecoder, stages, ordered, memoized', k: DEC, muted: true },
        { t: 'encoder   QuoteEncoder -> QuoteAfterLegs', k: ENC, hot: true },
        { t: '          -> QuoteComplete', k: ENC, hot: true },
        { t: '          message_encoder.rs', muted: true },
        { t: 'length    compute_encoded_length(', k: LEN, hot: true },
        { t: '              legs_count, venue_len)', k: LEN, hot: true },
        { t: '          encoded_length.rs', muted: true },
        '          = 12 + (4 + legs_count × 8)',
        '               + (4 + venue_len)',
        '          2 legs, "XLON": 40, plus 8 header = 48',
      ],
      cost: 'A const fn: size the buffer exactly, then encode.',
      caption: 'generate_message_encoder emits the type-state QuoteEncoder: each stage (QuoteAfterLegs, QuoteComplete) offers only the next tail, so writing venue before legs does not compile. It calls encoded_length::generate for the sizing helpers.',
    },
    {
      ms: 6500, at: 4, done: [0, 2, 3], title: 'One formatted Rust module, still in memory',
      lines: [
        'syn::parse_str::<syn::File>(&src)',
        { t: '  failure: GenerateError::InvalidGeneratedSource', muted: true },
        'annotate_missing_public_docs(&mut file)',
        'prettyplease::unparse(&file)',
        { t: 'GeneratedModuleSet { quote.rs }', hot: true },
        '',
        { t: 'set.modules() -> path "quote.rs", source', hot: true },
        { t: 'the examples on this page stop here', muted: true },
      ],
      cost: 'Nothing has touched the disk yet.',
      caption: 'The emitted code is parsed with syn, so a generator bug is an error rather than a broken file, then missing public docs are filled in and prettyplease formats it. The examples on this page read set.modules() at this point instead of writing files.',
    },
    {
      ms: 7500, at: 5, done: [0, 2, 3, 4], title: 'write_to_out_dir, then sbe_mod!',
      lines: [
        { t: '// build.rs', muted: true },
        'generate(SchemaSource::file("schema.xml"),',
        '         GenerationConfig::new("quote"))?',
        { t: '    .write_to_out_dir()?;', hot: true },
        '  -> $OUT_DIR/quote.rs        (all or none)',
        '  -> cargo::rerun-if-changed=.../schema.xml',
        { t: '// lib.rs', muted: true },
        { t: 'ergo_sbe::sbe_mod!(quote);', hot: true },
        '  -> mod quote {',
        '       include!(concat!(env!("OUT_DIR"),',
        '                "/quote.rs")); }',
      ],
      cost: 'sbe_mod! needs ergo-sbe as a normal dependency.',
      caption: 'write_to_out_dir stages every module to a temporary file and promotes them all or none, then prints rerun-if-changed for the schema and each include, so editing the XML reruns the build script. sbe_mod!(quote) includes the file as module quote.',
    },
    {
      ms: 8000, at: 2, error: true, done: [0], dead: [3, 4, 5], title: 'A schema error stops the build at parse',
      lines: [
        { t: 'ergo_sbe::schema_parse::invalid', hot: true },
        '',
        '  × invalid type for field \'price\': Price',
        '    ╭─[schema.xml:20:5]',
        ' 19 │   <message name="Quote" id="1">',
        { t: ' 20 │     <field name="price" id="1" type="Price"/>', hot: true },
        '    ·     ────────────────────┬────────────────────',
        '    ·                         ╰── invalid here',
        ' 21 │     <field name="qty" id="2" type="uint32"/>',
        '    ╰────',
      ],
      cost: 'resolve and Generator never run; nothing is written.',
      caption: 'A mistake: price now names type="Price", which the schema never defines. parse_schema stops at that field with ParseError::Invalid. A build.rs returning miette::Result prints the code ergo_sbe::schema_parse::invalid with the line underlined.',
    },
    {
      ms: 9000, at: 5, done: [0, 2, 3, 4, 5], title: 'Build time, not run time',
      lines: [
        '  stage            runs in     leaves behind',
        { t: 'parse, resolve   build.rs    46 tokens, offsets', k: 'k3' },
        { t: 'Generator        build.rs    QuoteDecoder, ...', k: 'k3' },
        { t: 'write            build.rs    $OUT_DIR/quote.rs', k: 'k3' },
        { t: 'your program     run time    the generated code', k: DEC, hot: true },
        '',
        { t: 'loses: each schema change is a rebuild', muted: true },
        { t: 'loses: each module embeds its own sbe_rt unless', muted: true },
        { t: '  with_shared_module or with_external_sbe_rt', muted: true },
      ],
      cost: 'The book records no timings for this pipeline.',
      caption: 'Why this wins: every step runs once in the build script, so a running program never parses or resolves a schema, and a schema mistake is a build error with a code. When it loses: every schema change means a rebuild, and each module carries its own sbe_rt unless you share one with with_shared_module or with_external_sbe_rt.',
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
