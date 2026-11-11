// Multi-message framing: each decode tells you where the next message starts.
//
// Sources:
//   samples/sbe-feature-tour/src/lib.rs:436   demo_any_message: Heartbeat then Note, one buffer
//   samples/sbe-feature-tour/src/lib.rs:472   Heartbeat: offset += d.encoded_length_with_header()?
//   samples/sbe-feature-tour/src/lib.rs:479   Note: offset += complete.encoded_length() + HEADER_LENGTH
//   samples/sbe-feature-tour/schemas/feature-tour.xml:20  8-byte header: blockLength, templateId, schemaId, version (u16 each)
//   samples/sbe-feature-tour/schemas/feature-tour.xml:93  Heartbeat id 1: sequence u64, timestamp u64 (block 16)
//   samples/sbe-feature-tour/schemas/feature-tour.xml:127 Note id 3: noteId u32 (block 4), body u32 length + UTF-8
//   sbe/src/codegen/runtime.rs:3282   AnyMessage::try_decode: header length check, read header, schemaId, match templateId
//   sbe/src/codegen/runtime.rs:3314   unknown template: Err(UnknownTemplateLength), no skip
//   sbe/src/codegen/message_decoder.rs:306   try_wrap: one fixed-extent check, no copy
//   sbe/src/codegen/message_decoder.rs:1401  base encoded_length_with_header: walks the tails (none for Heartbeat)
//   sbe/src/codegen/message_decoder.rs:1768  Metadata::remaining(): after the acting fixed block only
//   sbe/src/codegen/tail_stages.rs:332       into_*_as_str: UTF-8 check over the var-data bytes
//   sbe/src/codegen/tail_stages.rs:948       complete stage encoded_length_with_header: tail_start - offset + 8
//   sbe/src/codegen/tail_stages.rs:954       complete stage remaining(): &buf[tail_start..]
//   sbe/src/codegen/runtime.rs:3170   FrameCursor, LengthPrefixU32Le: 4-byte LE prefix per frame
//   sbe/src/codegen/runtime.rs:3443   decode_frame: unknown template -> Unknown { header, frame }
//   sbe/src/codegen/runtime.rs:3363   decode_frame, known template: <Msg>Decoder::try_decode reads the header again
//   sbe/src/codegen/message_decoder.rs:444  per-message try_decode: header read + checks, then try_wrap
//   sbe/src/codegen/runtime.rs:3239   FrameCursor: offset = frame_end; any error parks offset at buf.len()
//   sbe/src/codegen/schema_dispatch.rs:224  AnySchemaMessage::decode keeps the whole slice as one frame
//   sbe/tests/schema_dispatch_test.rs:113   unknown schema -> Other { template_id: 77, .. }
//   cluster/schemas/aeron-cluster-codecs.xml:132  SessionMessageHeader: 3 x int64 (block 24), fixed-only
//   cluster/schemas/aeron-cluster-codecs.xml:179  SessionKeepAlive: 2 x int64 (block 16)
//   sbe/src/codegen/runtime.rs:3268   try_decode arm calls try_wrap itself: the extent check happens before it returns
//   sbe/src/codegen/tail_stages.rs:229 into_*: prefix-fits check, maxValue check (feature-tour.xml:31), checked_var_data_bounds
//   sbe/src/codegen/tail_cache.rs:8   base decoder re-walks tails on every access; memoized() walks each boundary once
//   book/src/cluster/chained-decoding.md:149     headers carry no complete length; unknown template stops try_decode
//   book/src/cluster/chained-decoding.md:167     remaining() is only the next message for a fixed-only message
// Byte offsets follow from those schemas: Heartbeat 0..24, Note 24..56.
// The third frame (templateId 9, 12-byte block, 20-byte frame) is illustrative.
// Counts are reads and checks in the generated code, not timings; the book
// records no benchmark result (book/src/sbe/benchmarks.md:18).
Anim.register('multi-message-framing', (stage, api) => {
  const el = api.el;
  // Narrow (phones): a 260-unit viewBox so text stays legible at 360px, and
  // the call, cost and total lines wrap onto two lines each.
  const BASE_Y = { lane: 14, call: 30, rowLabel: 50, curLabel: 63, curTop: 67, box: 77, boxH: 28, tick: 117, ghost: 135, hdrLabel: 157, hdr: 163, hdrH: 32, armLabel: 213, arm: 219, armH: 24, cost: 261, total: 277, H: 286 };
  const WIDE = { W: 400, f: 1, lines: 1, dy: 0, Y: BASE_Y };
  const NARROW = { W: 260, f: 1.15, lines: 2, dy: 13, Y: null };
  NARROW.Y = Object.fromEntries(Object.entries(BASE_Y).map(([k, v]) =>
    [k, ['lane', 'call', 'boxH', 'hdrH', 'armH'].includes(k) ? v : v + NARROW.dy]));
  NARROW.Y.total += 13;
  NARROW.Y.H += 2 * 13;
  let Y = BASE_Y;

  // Rows: segments [key, from, to, label, short, cls].
  const ROWS = {
    A: {
      bytes: 76, label: 'stream: Heartbeat + Note', size: '56 bytes',
      segs: [
        ['h1', 0, 8, 'hdr', 'h', 'ea-k1 ea-sub'],
        ['seq', 8, 16, 'sequence', 'seq', 'ea-k1'],
        ['ts', 16, 24, 'timestamp', 'ts', 'ea-k1'],
        ['h2', 24, 32, 'hdr', 'h', 'ea-k3 ea-sub'],
        ['id', 32, 36, 'noteId', 'id', 'ea-k3'],
        ['len', 36, 40, 'len 16', 'len', 'ea-k3'],
        ['body', 40, 56, '"hello AnyMessage"', 'body', 'ea-k3'],
        ['h3', 56, 64, 'hdr', 'h', 'ea-k5 ea-sub'],
        ['unk', 64, 76, 'block 12', 'blk', 'ea-k5'],
      ],
      ticks: [0, 8, 24, 32, 36, 40, 56, 64, 76],
      extra: ['h3', 'unk', 64, 76], // illustrative third frame, shown only when unknown
    },
    B: {
      bytes: 88, label: 'length-prefixed: LengthPrefixU32Le', size: '88 bytes',
      segs: [
        ['p1', 0, 4, '24', '24', 'ea-k4'],
        ['bh', 4, 12, 'hdr', 'h', 'ea-k1 ea-sub'],
        ['bb', 12, 28, 'Heartbeat', 'HB', 'ea-k1'],
        ['p2', 28, 32, '32', '32', 'ea-k4'],
        ['nh', 32, 40, 'hdr', 'h', 'ea-k3 ea-sub'],
        ['nb', 40, 64, 'Note', 'Note', 'ea-k3'],
        ['p3', 64, 68, '20', '20', 'ea-k4'],
        ['uh', 68, 76, 'hdr', 'h', 'ea-k5 ea-sub'],
        ['ub', 76, 88, 'id 9', '9', 'ea-k5'],
      ],
      ticks: [0, 4, 28, 32, 64, 68, 88],
    },
    C: {
      bytes: 56, label: 'SessionMessageHeader + KeepAlive', size: '56 bytes',
      segs: [
        ['sh', 0, 8, 'hdr', 'h', 'ea-k2 ea-sub'],
        ['sb', 8, 32, 'SessionMessageHeader', 'SMH', 'ea-k2'],
        ['kh', 32, 40, 'hdr', 'h', 'ea-k3 ea-sub'],
        ['kb', 40, 56, 'KeepAlive', 'KA', 'ea-k3'],
      ],
      ticks: [0, 8, 32, 40, 56],
    },
  };
  const HDR_FIELDS = ['blockLength', 'templateId', 'schemaId', 'version'];
  const ARMS = [['1', 'Heartbeat'], ['2', 'Car'], ['3', 'Note'], ['4', 'Quote'], ['_', '']];

  let g;
  let svg;
  let laneText;
  let callText;
  let rows;
  let hdrPanel;
  let hdrLabel;
  let hdrBoxes;
  let armPanel;
  let armBoxes;
  let costText;
  let totalText;

  const fits = (text, w, size) => text.length * size * 0.58 <= w - 3;

  function cursor(parent, ghost) {
    const c = el('g', { class: ghost ? 'ea-hidden' : '' }, parent);
    const stroke = ghost ? 'var(--ea-c2)' : 'var(--fg)';
    const yTop = ghost ? Y.box : Y.curTop;
    const yBot = Y.box + Y.boxH + (ghost ? 4 : 3);
    el('line', { x1: 0, x2: 0, y1: yTop, y2: yBot, stroke, 'stroke-width': ghost ? 2 : 2.5, 'stroke-dasharray': ghost ? '3 2' : 'none' }, c);
    // The ghost's arrow sits under the tick numbers so it never covers them.
    if (ghost) el('polygon', { points: `-4,${Y.tick + 9} 4,${Y.tick + 9} 0,${Y.tick + 3}`, fill: stroke }, c);
    else el('polygon', { points: `-5,${yTop - 1} 5,${yTop - 1} 0,${yTop + 6}`, fill: stroke }, c);
    const label = el('text', { x: 0, y: ghost ? Y.ghost + 3 : Y.curLabel, 'font-size': 9.5 * g.f, class: 'ea-mono ea-strong' }, c, '');
    return { c, label };
  }

  function buildRow(name) {
    const spec = ROWS[name];
    const S = g.W / spec.bytes;
    const grp = el('g', { class: 'ea-hidden' }, svg);
    el('text', { x: 0, y: Y.rowLabel, 'font-size': 10 * g.f, class: 'ea-strong' }, grp, spec.label);
    const size = el('text', { x: g.W, y: Y.rowLabel, 'font-size': 9 * g.f, 'text-anchor': 'end', class: 'ea-muted' }, grp, spec.size);
    const segs = new Map();
    const extra = el('g', {}, grp);
    for (const [key, from, to, label, short, cls] of spec.segs) {
      const parent = spec.extra && spec.extra.includes(key) ? extra : grp;
      const x = from * S;
      const w = (to - from) * S;
      const rect = el('rect', { x: x + 0.5, y: Y.box, width: w - 1, height: Y.boxH, rx: 2, class: cls }, parent);
      const fs = 8.5 * g.f;
      const text = fits(label, w, fs) ? label : (fits(short, w, fs) ? short : '');
      el('text', { x: x + w / 2, y: Y.box + 17.5, 'font-size': fs, 'text-anchor': 'middle', class: 'ea-mono' }, parent, text);
      segs.set(key, rect);
    }
    let lastLabel = -Infinity;
    for (const t of spec.ticks) {
      const parent = spec.extra && spec.extra.slice(2).includes(t) ? extra : grp;
      el('line', { x1: t * S, x2: t * S, y1: Y.box + Y.boxH, y2: Y.box + Y.boxH + 4, stroke: 'var(--fg)', 'stroke-opacity': 0.5 }, parent);
      // Drop a number that would touch its neighbour (row B on phones); the tick stays.
      if (t * S - lastLabel < 13 && t !== spec.bytes) continue;
      lastLabel = t * S;
      const anchor = t === 0 ? 'start' : (t === spec.bytes ? 'end' : 'middle');
      el('text', { x: t * S, y: Y.tick, 'font-size': 8 * g.f, 'text-anchor': anchor, class: 'ea-mono ea-muted' }, parent, String(t));
    }
    const ghost = cursor(grp, true);
    const main = cursor(grp, false);
    return { grp, segs, extra, size, S, spec, main, ghost };
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    Y = g.Y;
    svg = api.svg(g.W, Y.H, 'Byte layout of back-to-back SBE messages with a moving decode cursor');
    laneText = el('text', { x: 0, y: Y.lane, 'font-size': 12 * g.f, class: 'ea-strong' }, svg, '');
    callText = el('text', { x: 0, y: Y.call, 'font-size': 9.5 * g.f, class: 'ea-mono' }, svg, '');
    rows = { A: buildRow('A'), B: buildRow('B'), C: buildRow('C') };

    hdrPanel = el('g', {}, svg);
    hdrLabel = el('text', { x: 0, y: Y.hdrLabel, 'font-size': 10 * g.f, class: 'ea-strong' }, hdrPanel, '');
    const hw = g.W / 4;
    hdrBoxes = HDR_FIELDS.map((name, i) => {
      const x = i * hw;
      const rect = el('rect', { x: x + 1, y: Y.hdr, width: hw - 4, height: Y.hdrH, rx: 3, class: 'ea-box' }, hdrPanel);
      const label = el('text', { x: x + hw / 2 - 1, y: Y.hdr + 12, 'font-size': 8 * g.f, 'text-anchor': 'middle', class: 'ea-muted' }, hdrPanel, name);
      const value = el('text', { x: x + hw / 2 - 1, y: Y.hdr + 26, 'font-size': 11 * g.f, 'text-anchor': 'middle', class: 'ea-mono ea-strong' }, hdrPanel, '');
      return { rect, value, label };
    });

    armPanel = el('g', {}, svg);
    el('text', { x: 0, y: Y.armLabel, 'font-size': 10 * g.f, class: 'ea-strong' }, armPanel, 'match templateId');
    const aw = g.W / ARMS.length;
    armBoxes = ARMS.map(([id, name], i) => {
      const x = i * aw;
      const rect = el('rect', { x: x + 1, y: Y.arm, width: aw - 4, height: Y.armH, rx: 3, class: 'ea-box' }, armPanel);
      // Narrow boxes have room for the variant name only.
      const label = g === NARROW ? name : `${id} ${name}`;
      const text = el('text', { x: x + aw / 2 - 1, y: Y.arm + 16, 'font-size': 8.5, 'text-anchor': 'middle', class: 'ea-mono' }, armPanel, name ? label : id);
      return { rect, text, id };
    });

    costText = el('text', { x: 0, y: Y.cost, 'font-size': 9.5 * g.f, class: 'ea-mono' }, svg, '');
    totalText = el('text', { x: 0, y: Y.total, 'font-size': 9.5 * g.f, class: 'ea-mono ea-muted' }, svg, '');
  }

  // Fill a text element, breaking it onto a second line at a space when it
  // would run past the viewBox (narrow layout only).
  function wrap(node, str) {
    node.textContent = '';
    const fs = Number(node.getAttribute('font-size'));
    const max = Math.floor(g.W / (fs * 0.6));
    let lines = [str];
    if (g.lines > 1 && str.length > max) {
      const cut = str.lastIndexOf(' ', max);
      const at = cut > max / 3 ? cut : max;
      lines = [str.slice(0, at), str.slice(at).trimStart()];
    }
    lines.forEach((line, i) => {
      el('tspan', { x: 0, dy: i === 0 ? 0 : 13 }, node, line);
    });
  }

  function place(row, cur, at, text) {
    if (at === undefined) {
      cur.c.classList.add('ea-hidden');
      return;
    }
    cur.c.classList.remove('ea-hidden');
    const x = at * row.S;
    cur.c.style.transform = `translate(${x}px, 0px)`;
    cur.label.textContent = text;
    const room = text.length * 9.5 * g.f * 0.6;
    const anchor = x < room / 2 ? 'start' : (x > g.W - room / 2 ? 'end' : 'middle');
    cur.label.setAttribute('text-anchor', anchor);
  }

  function render(s) {
    laneText.textContent = s.lane;
    wrap(callText, s.call);
    const hot = new Map((s.hot || []).map((k, i) => [k, i]));
    for (const [name, row] of Object.entries(rows)) {
      const visible = name === s.row;
      row.grp.classList.toggle('ea-hidden', !visible);
      row.extra.classList.toggle('ea-hidden', !s.unknown);
      if (name === 'A') row.size.textContent = s.unknown ? '56 bytes + illustrative' : '56 bytes';
      for (const [key, rect] of row.segs) {
        const i = visible ? hot.get(key) : undefined;
        rect.classList.toggle('ea-hot', i !== undefined);
        if (i !== undefined) rect.style.setProperty('--d', String(i));
        rect.classList.toggle('ea-ok', visible && s.ok !== undefined && s.ok.includes(key));
      }
      place(row, row.main, visible ? s.at : undefined, s.atLabel || `offset ${s.at}`);
      place(row, row.ghost, visible ? s.ghost : undefined, s.ghostLabel || '');
    }
    hdrPanel.classList.toggle('ea-hidden', !s.hdr);
    if (s.hdr) {
      hdrLabel.textContent = s.hdrLabel;
      hdrBoxes.forEach((b, i) => {
        b.value.textContent = String(s.hdr[i]);
        const isHot = Boolean(s.hdrHot && s.hdrHot.includes(i));
        b.rect.setAttribute('class', isHot ? 'ea-k5 ea-hot' : 'ea-box');
        // Full-strength name on a highlighted box: the muted grey is unreadable on the fill.
        b.label.setAttribute('class', isHot ? '' : 'ea-muted');
      });
    }
    armPanel.classList.toggle('ea-hidden', s.arm === undefined);
    armBoxes.forEach((b) => {
      const chosen = s.arm === b.id;
      // An error arm is marked as a failure (vermillion, no dashed "ok").
      const failed = chosen && b.id === '_' && s.fallback === 'Err';
      b.rect.setAttribute('class', chosen ? (failed ? 'ea-k2 ea-hot' : 'ea-k3 ea-hot') : 'ea-box');
      if (b.id === '_') b.text.textContent = s.fallback ? `_ ${s.fallback}` : '_ other';
      b.rect.classList.toggle('ea-ok', chosen && !failed);
    });
    wrap(costText, s.cost || '');
    wrap(totalText, s.total || '');
  }

  const HB = [16, 1, 77, 0];
  const NOTE = [4, 3, 77, 0];
  const UNK = [12, 9, 77, 0];
  const L1 = 'Chained: AnyMessage::try_decode';
  const L2 = 'Alternate: FrameCursor';
  const states = [
    {
      ms: 6500, lane: 'One buffer, two framed messages', call: 'demo_any_message(): stream = Heartbeat ++ Note',
      row: 'A', at: 0, total: 'nothing read yet',
      caption: 'The feature-tour sample builds one 56-byte buffer: a Heartbeat (8-byte header + 16-byte block) then a Note (header, 4-byte noteId, then a 4-byte length and 16 bytes of text). Nothing in the buffer says where the Note starts; the decoder must work it out.',
    },
    {
      ms: 6000, lane: L1, call: 'AnyMessage::try_decode(&stream, 0)?',
      row: 'A', at: 0, hot: ['h1'], hdr: HB, hdrHot: [1, 2], hdrLabel: 'header at offset 0', arm: '1', fallback: 'Err',
      cost: 'this step: read 8 B header, 4 checks', total: 'so far: 8 B read, 4 checks',
      caption: 'try_decode checks 8 bytes remain, copies the header, checks schemaId is 77 and matches templateId 1: Heartbeat. In that arm try_wrap checks the 16-byte block fits (0 + 8 + 16 ≤ 56) and returns a view over the buffer, copying nothing.',
    },
    {
      ms: 5000, lane: L1, call: 'Heartbeat(d) => d.sequence()',
      row: 'A', at: 0, hot: ['seq'], hdr: HB, hdrLabel: 'header at offset 0', arm: '1', fallback: 'Err',
      cost: 'this step: read 8 B, 0 checks', total: 'so far: 16 B read, 4 checks',
      caption: 'The extent was proved before try_decode returned, so d.sequence() reads 8 bytes in place with no further check.',
    },
    {
      ms: 5500, lane: L1, call: 'offset += d.encoded_length_with_header()?  // 24',
      row: 'A', at: 24, ok: ['h1', 'seq', 'ts'], hdr: HB, hdrHot: [0], hdrLabel: 'header at offset 0',
      cost: 'this step: 0 B read; 8 + blockLength 16 = 24', total: 'so far: 16 B read, 4 checks',
      caption: 'Heartbeat has no groups or var-data, so its length is the 8-byte header plus the 16-byte blockLength from the header. The cursor moves to 24 without reading anything new.',
    },
    {
      ms: 5500, lane: L1, call: 'AnyMessage::try_decode(&stream, 24)?',
      row: 'A', at: 24, hot: ['h2', 'id'], hdr: NOTE, hdrHot: [1, 2], hdrLabel: 'header at offset 24', arm: '3', fallback: 'Err',
      cost: 'this step: read 8 B header + 4 B noteId, 4 checks', total: 'so far: 28 B read, 8 checks',
      caption: 'The next header, at 24, says templateId 3: Note, with a 4-byte block. try_wrap checks 24 + 8 + 4 ≤ 56; note_id() reads the block. The Note still has a var-data tail after byte 36.',
    },
    {
      ms: 7000, lane: 'The trap: Metadata::remaining()', call: 'd.get_metadata().remaining()  // starts at limit() = 36',
      row: 'A', at: 24, ghost: 36, ghostLabel: 'remaining(): 36, wrong', hot: ['len'], hdr: NOTE, hdrHot: [0], hdrLabel: 'header at offset 24',
      cost: 'this step: 0 B read; 24 + 8 + blockLength 4 = 36', total: 'so far: 28 B read, 8 checks',
      caption: 'Metadata::remaining() starts where the fixed block ends, byte 36. For a message with a tail, byte 36 is not the next message: it is the Note\'s own length prefix. A header alone never gives the frame end.',
    },
    {
      ms: 7000, lane: L1, call: 'let (body, complete) = d.into_body_as_str()?;',
      row: 'A', at: 56, atLabel: 'offset 56 = len', hot: ['len', 'body'], ok: ['h2', 'id', 'len', 'body'], hdr: NOTE, hdrLabel: 'header at offset 24',
      cost: 'this step: read 4 B length, 3 checks, UTF-8 over 16 B', total: 'so far: 48 B read, 11 checks, no byte twice',
      caption: 'The staged walk checks the 4-byte length fits, reads it (16), checks it is within the schema maxValue and that 16 bytes remain, validates them as UTF-8 and returns the complete stage at tail_start 56. complete.encoded_length() + 8 = 24 + 8 = 32, so the offset moves from 24 to 56: arithmetic, no rescan.',
    },
    {
      ms: 6000, lane: 'Loop ends at stream.len()', call: 'while offset < stream.len() { .. }',
      row: 'A', at: 56, atLabel: 'offset 56 = len', ok: ['h1', 'seq', 'ts', 'h2', 'id', 'len', 'body'],
      cost: '2 header reads, 2 dispatches, 0 rescans', total: '56 B: 1 cache line if 64-aligned, else 2',
      caption: 'Each decode left the cursor exactly at the next header, so the loop stops at 56 and no byte was read twice (timestamp was never read at all). The whole chain touches one or two 64-byte cache lines, depending on where the buffer starts.',
    },
    {
      ms: 7500, lane: 'Unknown template: stop', call: 'try_decode(&stream, 56) → Err(UnknownTemplateLength)',
      row: 'A', unknown: true, at: 56, ghost: 76, ghostLabel: '56 + 8 + 12 = 76? a guess', hot: ['h3'], hdr: UNK, hdrHot: [0, 1], hdrLabel: 'header at offset 56 (illustrative)', arm: '_', fallback: 'Err',
      cost: 'this step: read 8 B header, 3 checks, then stop', total: 'cursor stays at 56; the ? returns the error',
      caption: 'Add an illustrative third frame with templateId 9, which this schema does not know. try_decode returns UnknownTemplateLength and the chain stops. Skipping by blockLength would only clear the fixed block: unknown tails may follow, so the next offset would be a guess.',
    },
    {
      ms: 6500, lane: L2, call: 'FrameCursor::new(&buf, FramingPolicy::LengthPrefixU32Le)',
      row: 'B', at: 0, hot: ['p1', 'p2', 'p3'],
      cost: '+4 B per frame, written by the producer', total: '',
      caption: 'The alternative puts the length outside SBE: the producer writes a 4-byte little-endian frame length before each message, and FrameCursor reads it. This is what decode_frame needs to handle unknown templates (the third length, 20, is illustrative).',
    },
    {
      ms: 5500, lane: L2, call: 'next() → decode_frame(&buf, 4, 24)',
      row: 'B', at: 28, hot: ['p1', 'bh'], ok: ['bh', 'bb'], hdr: HB, hdrHot: [1, 2], hdrLabel: 'header at offset 4', arm: '1', fallback: 'Unknown',
      cost: 'this step: 4 B prefix + 8 B header, read twice', total: 'offset = frame_end = 4 + 24 = 28',
      caption: 'next() reads the prefix 24, checks the frame fits, then decode_frame reads the header to dispatch and HeartbeatDecoder::try_decode reads and checks it again. The cursor moves to frame_end, 28, from the prefix alone, not from the message.',
    },
    {
      ms: 5000, lane: L2, call: 'next() → decode_frame(&buf, 32, 32)',
      row: 'B', at: 64, hot: ['p2', 'nh'], ok: ['nh', 'nb'], hdr: NOTE, hdrHot: [1, 2], hdrLabel: 'header at offset 32', arm: '3', fallback: 'Unknown',
      cost: 'this step: 4 B prefix + 8 B header, read twice', total: 'offset = 32 + 32 = 64; the Note tail is unread',
      caption: 'Prefix 32: the Note frame. The cursor is already past its tail, so the caller may read the body or ignore it; the next frame starts at 64 either way.',
    },
    {
      ms: 7000, lane: L2, call: 'next() → Ok(Unknown { header, frame })',
      row: 'B', at: 88, atLabel: 'offset 88 = len', hot: ['p3', 'uh'], ok: ['uh', 'ub'], hdr: UNK, hdrHot: [1], hdrLabel: 'header at offset 68 (illustrative)', arm: '_', fallback: 'Unknown',
      cost: 'this step: read 4 B prefix + 8 B header', total: 'unknown frame kept whole; next() then returns None',
      caption: 'Prefix 20, templateId 9: decode_frame returns Unknown with the header and all 20 frame bytes, so the caller can skip or forward it. If a prefix claims more bytes than remain, next() returns one error and then None.',
    },
    {
      ms: 7000, lane: 'Approach 2: remaining() slot', call: 'smh.get_metadata().remaining()  // smh = decode(&buf, 0)?',
      row: 'C', at: 32, atLabel: 'remaining(): 32', hot: ['sh', 'sb'], ok: ['kh', 'kb'],
      cost: 'fixed-only: limit() = 8 + 24 = 32', total: 'tail.len() == 24 == SessionKeepAlive frame',
      caption: 'Metadata::remaining() is the right call when the first message is fixed-only. SessionMessageHeader has no tail, so the end of its 24-byte block, byte 32, is exactly where the SessionKeepAlive starts. For the Note it was wrong.',
    },
    {
      ms: 9000, lane: 'Why it wins, when it loses', call: 'offset = complete stage end, or an external length',
      row: 'A', at: 56, atLabel: 'offset 56 = len', ok: ['h1', 'seq', 'ts', 'h2', 'id', 'len', 'body'],
      cost: 'wins: known templates, walk finished, 0 rescans', total: 'loses: base-decoder rescans; unknown id stops',
      caption: 'Wins: when every template is known and each walk is finished, the next offset is arithmetic on a stage you already hold. Loses: stop at random access and the base decoder\'s encoded_length_with_header() walks the tails again (memoized() walks each boundary once); meet an unknown template and the chain stops unless an external length (FrameCursor: 4 B per frame and a second header check) is there. Counts from the code, not timings.',
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
