// try_claim: encode straight into the publication's term buffer, and the
// offer route it replaces.
//
// Sources:
//   sbe/src/codegen/runtime.rs:577   Claim: frame_mut, commit, abort
//   sbe/src/codegen/runtime.rs:633   AbortOnDrop: an early return or panic aborts the claim
//   sbe/src/codegen/runtime.rs:654   send_frame: encode, length check, header check, commit
//   sbe/src/codegen/runtime.rs:681   OfferClaim: new() zeroes the scratch buffer, commit calls offer
//   sbe/src/codegen/message_encoder.rs:532  generated send: try_wrap_and_apply_header, then the closure
//   sbe/src/codegen/encoded_length.rs:721   compute_length_with_header: the length from the inputs alone
//   sbe/tests/claim_send_test.rs:174  a claim sized for another frame: LengthMismatch, aborted
//   sbe/tests/claim_send_test.rs:179  an encoder error (Order's note, 65 > 64) comes back as Encode, aborted
//   sbe/src/codegen/message_encoder.rs:1470  a var-data setter checks room before writing its prefix,
//     so a 19-byte body in a 34-byte claim leaves header + noteId written: BufferTooShort 23 > 22
//   book/examples/claim-send.rs:18    the Note example: body "rebalance at close", len + 1 mismatch
//   samples/sbe-feature-tour/schemas/feature-tour.xml:127  Note: noteId uint32, body uint32-prefixed
//   cluster/src/client.rs:9           try_claim writes the SessionMessageHeader into the claim
//   cluster/src/client.rs:1490        try_claim: claims 32 + payload_len, header first, payload_mut after
//   cluster/src/client.rs:1364        try_claim is one frame (MTU less header); offer fragments
//   samples/cluster-ha-orderbook/src/publish.rs:144  sized nested encode into the claim
//   samples/cluster-ha-orderbook/src/publish.rs:112  fill payload_mut, commit, or abort (:118) on encode error
// Aeron's side (rusteron wraps the Aeron C client, in the aeron submodule):
//   aeron/aeron-client/src/main/c/aeron_exclusive_publication.c:323  claim: tail += align(len + 32, 32)
//   aeron/aeron-client/src/main/c/aeron_exclusive_publication.c:40   header written with -frame_length
//   aeron/aeron-client/src/main/c/aeron_exclusive_publication.c:100  offer: memcpy after the header
//   aeron/aeron-client/src/main/c/aeron_publication.c:861   commit: release store of +frame_length
//   aeron/aeron-client/src/main/c/aeron_publication.c:874   abort: type PAD, +frame_length
//   aeron/aeron-client/src/main/c/aeron_image.c:286         a reader stops at frame_length <= 0
// Sizes: Note frame 8 + 4 + 4 + 18 = 34 bytes; Aeron data header 32, frame
// alignment 32; SessionMessageHeader 8 + 24 = 32. The log offsets and the
// earlier frame are illustrative. Counts are bytes the code writes, not timings.
Anim.register('try-claim-send', (stage, api) => {
  const el = api.el;
  const ROW_H = 24;
  const WIDE = { W: 400, X0: 76, labelAbove: false, callFont: 10.5, pitch: 66 };
  const NARROW = { W: 300, X0: 0, labelAbove: true, callFont: 8.6, pitch: 70 };
  const OVER_BYTES = 164; // widest log view: earlier frame + a 128-byte cluster frame + tail room
  const BODY = '"rebalance at close"';

  // [key, bytes, short label, colour class]
  const NOTE = [
    ['mh', 8, 'hdr', 'ea-k4'],
    ['id', 4, 'id', 'ea-k1'],
    ['vl', 4, 'len', 'ea-k3'],
    ['body', 18, BODY, 'ea-k3'],
  ];
  const LAYOUTS = {
    note: {
      over: [['prev', 16], ['ahdr', 32], ['claim', 34], ['pad', 30]],
      zoom: NOTE.concat([['x', 1, '', 'ea-k2']]),
      zoomBytes: 35,
      legend: 'hdr 8 + noteId 4 + len 4 + body 18 = 34 bytes',
    },
    cluster: {
      over: [['prev', 16], ['ahdr', 32], ['claim', 66], ['pad', 30]],
      zoom: [['smh', 8, 'hdr', 'ea-k2'], ['sbl', 24, 'SessionMessageHeader', 'ea-k2']].concat(NOTE),
      zoomBytes: 66,
      legend: 'session header 8 + 24, then the 34-byte Note',
    },
  };

  let g;
  let layoutName;
  let svg;
  let parts; // key -> { rect, text, base }
  let over; // key -> { rect, text, x, w }
  let scratch; // key -> { rect, text, base }
  let refs;

  function rowY(i, name) {
    const y = 40 + i * g.pitch;
    if (g.labelAbove) {
      el('text', { x: 0, y: y + 9, 'font-size': 10, class: 'ea-strong' }, svg, name);
      return y + 13;
    }
    el('text', { x: 0, y: y + 15, 'font-size': 10.5, class: 'ea-strong' }, svg, name);
    return y;
  }

  function strip(spec, x0, y, scale, store, fontSize) {
    let x = x0;
    for (const [key, bytes, label, base] of spec) {
      const w = bytes * scale;
      const rect = el('rect', { x, y, width: w, height: ROW_H, rx: 2, class: 'ea-box' }, svg);
      const text = el('text', {
        x: x + w / 2, y: y + 15.5, 'text-anchor': 'middle', 'font-size': fontSize,
        class: 'ea-mono',
      }, svg, label || '');
      store.set(key, { rect, text, base, x, w });
      x += w;
    }
    return x;
  }

  function build(narrow, layout) {
    g = narrow ? NARROW : WIDE;
    layoutName = layout;
    const L = LAYOUTS[layout];
    const avail = g.W - g.X0;
    stage.textContent = '';
    svg = api.svg(g.W, 40 + 2 * g.pitch + ROW_H + 56 + (g.labelAbove ? 13 : 0),
      'A publication term buffer, the claimed region byte by byte, and the offer route scratch buffer');
    parts = new Map();
    over = new Map();
    scratch = new Map();
    refs = {};

    refs.lane = el('text', { x: 0, y: 13, 'font-size': 12, class: 'ea-strong' }, svg, '');
    refs.call = el('text', { x: 0, y: 28, 'font-size': g.callFont, class: 'ea-mono' }, svg, '');

    // Row 0: the log, frame by frame.
    const yA = rowY(0, 'term buffer');
    const sO = avail / OVER_BYTES;
    const overSpec = L.over.map(([k, b]) => [k, b, '', '']);
    const endO = strip(overSpec, g.X0, yA, sO, over, 8);
    over.get('prev').text.textContent = 'prev';
    over.get('prev').rect.setAttribute('class', 'ea-box');
    over.get('pad').text.textContent = '';
    refs.endO = endO;
    refs.tail = el('text', { x: 0, y: yA + 15.5, 'font-size': 8.5, class: 'ea-mono ea-muted' }, svg, 'tail');
    refs.tailMark = el('path', { d: '', stroke: 'currentColor', 'stroke-width': 1.5, fill: 'none' }, svg);
    refs.reader = el('path', { d: '', class: 'ea-k1', 'stroke-width': 2, fill: 'none' }, svg);
    refs.readerText = el('text', { x: 0, y: yA + ROW_H + 13, 'font-size': 8.5, class: 'ea-mono' }, svg, '');
    refs.yA = yA;

    // Row 1: the claimed bytes, magnified.
    const yB = rowY(1, g.labelAbove ? 'the claimed bytes, in the log' : 'claimed');
    if (!g.labelAbove) {
      el('text', { x: 0, y: yB + 26, 'font-size': 9, class: 'ea-muted' }, svg, 'bytes');
    }
    const sZ = avail / L.zoomBytes;
    const ov = over.get('claim');
    // Starts below the subscriber label and ends above the row label, so no text is struck through.
    const fTop = yA + ROW_H + 17;
    const fBot = g.labelAbove ? yB - 15 : yB;
    refs.funnelD = (bytes) => `M${ov.x},${fTop} L${ov.x + ov.w},${fTop} L${g.X0 + bytes * sZ},${fBot} L${g.X0},${fBot} Z`;
    refs.funnel = el('path', { d: '', class: 'ea-k1 ea-sub', 'stroke-dasharray': '2 2' }, svg);
    strip(L.zoom, g.X0, yB, sZ, parts, layout === 'cluster' ? 8 : 9);
    if (layout === 'cluster') {
      parts.get('body').text.textContent = 'body 18';
      if (narrow) parts.get('sbl').text.textContent = 'session hdr';
    }
    refs.legend = el('text', { x: g.X0, y: yB + ROW_H + 12, 'font-size': 8.5, class: 'ea-mono ea-muted' }, svg, L.legend);
    refs.stamp = el('text', { x: g.X0, y: yB + ROW_H + 12, 'font-size': 9.5, class: 'ea-strong' }, svg, '');
    refs.yB = yB;

    // Row 2: the offer route's scratch buffer (same scale as the Note bytes).
    const yC = rowY(2, 'scratch');
    const sS = avail / LAYOUTS.note.zoomBytes;
    strip(NOTE, g.X0, yC, sS, scratch, 8.5);
    refs.noScratch = el('text', { x: g.X0, y: yC + 15.5, 'font-size': 10, class: 'ea-muted' }, svg,
      'none: the encoder writes into the log');
    const copyX = g.X0 + 17 * sS;
    refs.copy = el('g', {}, svg);
    el('path', {
      d: `M${copyX},${yC - 2} L${copyX},${yB + ROW_H + 16} M${copyX - 5},${yB + ROW_H + 22} L${copyX},${yB + ROW_H + 15} L${copyX + 5},${yB + ROW_H + 22}`,
      class: 'ea-k2', 'stroke-width': 2.5, fill: 'none',
    }, refs.copy);
    el('text', { x: copyX + 8, y: yC - 6, 'font-size': 9.5, class: 'ea-strong' }, refs.copy, 'copy 34 B at offer');

    // Check line and running costs.
    const yD = yC + ROW_H + 18;
    refs.check = el('text', { x: 0, y: yD, 'font-size': g.labelAbove ? 8.6 : 10, class: 'ea-mono' }, svg, '');
    refs.cost1 = el('text', { x: 0, y: yD + 17, 'font-size': g.labelAbove ? 9 : 10 }, svg, '');
    refs.cost2 = el('text', { x: 0, y: yD + 31, 'font-size': g.labelAbove ? 9 : 10 }, svg, '');
  }

  // Segment state: 'g' reserved and zeroed, 'w' written, 'h' written this
  // step, 'd' inside an aborted (padding) frame, undefined not drawn.
  function paint(item, st) {
    const { rect, text, base } = item;
    let cls;
    if (st === undefined) cls = 'ea-box ea-hidden';
    else if (st === 'g') cls = 'ea-box';
    else if (st === 'd') cls = `${base} ea-sub`; // written, but inside a padding frame
    else cls = `${base} ${st === 'h' ? 'ea-hot' : ''}`;
    rect.setAttribute('class', cls);
    rect.setAttribute('stroke-dasharray', st === 'g' ? '3 2' : st === 'd' ? '1 2' : 'none');
    text.classList.toggle('ea-hidden', st === undefined);
    text.classList.toggle('ea-muted', st === 'g' || st === 'd');
  }

  function render(s) {
    const narrow = g === NARROW;
    if (s.layout !== layoutName) build(narrow, s.layout);
    refs.lane.textContent = s.lane;
    refs.call.textContent = (narrow && s.callN) || s.call;

    // Log row.
    const o = s.over || {};
    const ah = over.get('ahdr');
    const cl = over.get('claim');
    const pad = over.get('pad');
    ah.base = 'ea-k5';
    cl.base = s.offerFrame ? 'ea-k3' : 'ea-k1';
    paint(ah, o.ahdr);
    paint(cl, o.claim);
    pad.rect.setAttribute('class', o.claim === undefined ? 'ea-box ea-hidden' : 'ea-box');
    pad.rect.setAttribute('fill-opacity', '0.15');
    pad.rect.setAttribute('stroke-dasharray', '1 3');
    ah.text.textContent = o.flen || '';
    cl.text.textContent = o.claimText || '';
    cl.rect.classList.toggle('ea-ok', o.ok === true);
    const prev = over.get('prev');
    const tailX = o.claim === undefined ? prev.x + prev.w : pad.x + pad.w;
    refs.tail.setAttribute('x', tailX + 3);
    refs.tailMark.setAttribute('d', `M${tailX},${refs.yA} L${tailX},${refs.yA + ROW_H}`);
    const readX = s.reader === 'past' ? tailX : ah.x;
    refs.reader.setAttribute('d', `M${readX},${refs.yA - 4} L${readX},${refs.yA + ROW_H + 4}`);
    refs.readerText.setAttribute('x', Math.min(readX, g.W - 150));
    refs.readerText.textContent = s.reader === 'past' ? 'subscriber reads to here' : 'subscriber stops here';

    // Claimed bytes.
    const seg = s.seg || {};
    for (const [key, item] of parts) paint(item, seg[key]);
    refs.funnel.setAttribute('d', refs.funnelD(s.layout === 'cluster' ? 66 : seg.x ? 35 : 34));
    refs.funnel.classList.toggle('ea-hidden', o.claim === undefined);
    refs.legend.textContent = s.legend || LAYOUTS[s.layout].legend;
    refs.legend.classList.toggle('ea-hidden', Object.keys(seg).length === 0 || s.copy === true || !!s.stamp);
    refs.stamp.textContent = s.stamp || '';

    // Scratch.
    const scr = s.scratch;
    for (const [key, item] of scratch) paint(item, scr ? scr[key] : undefined);
    refs.noScratch.classList.toggle('ea-hidden', !s.noScratchNote);
    refs.copy.classList.toggle('ea-hidden', !s.copy);

    refs.check.textContent = (narrow && s.checkN) || s.check || '';
    refs.cost1.textContent = (s.cost || [])[0] || '';
    refs.cost2.textContent = (s.cost || [])[1] || '';
  }

  const ALL_G = { mh: 'g', id: 'g', vl: 'g', body: 'g' };
  const NOTE_W = { mh: 'w', id: 'w', vl: 'w', body: 'w' };
  const CLAIM = 'try_claim: encode in place';
  const NOTHING = ['Nothing written yet.', ''];
  const CLAIM_COST = ['Log: 32 B header (by Aeron) + 34 B frame, each byte written once.', 'Scratch buffer: none. Copies: none.'];
  const steps = [
    {
      ms: 6000, layout: 'note', lane: "The publication's log", call: 'try_claim_owned(len) / offer(frame)',
      reader: 'past', noScratchNote: false, cost: NOTHING,
      caption: "A publication's term buffer holds frames back to back, each a 32-byte Aeron header plus the message, padded to a 32-byte boundary. A subscriber reads frame by frame and stops at the first one whose length field is not yet positive. The earlier frame and the offsets are illustrative.",
    },
    {
      ms: 5500, layout: 'note', lane: '1. Size the frame first', call: 'NoteEncoder::compute_length_with_header(18)',
      callN: 'compute_length_with_header(18)',
      reader: 'past', seg: ALL_G, legend: 'layout only, nothing claimed yet: 8 + 4 + 4 + 18',
      cost: ['Length computed from the input lengths alone: 34.', 'No buffer touched.'],
      caption: 'A claim must be exactly the frame, so the length comes first: message header 8, noteId 4, body length prefix 4, then the 18 body bytes, 34 in all. compute_length_with_header needs only the lengths (EncodedLength does the same for nested groups).',
    },
    {
      ms: 6000, layout: 'note', lane: '2. Claim exactly 34 bytes', call: 'let claim = publication.try_claim_owned(34)?;',
      callN: 'publication.try_claim_owned(34)?',
      over: { ahdr: 'h', claim: 'g', flen: 'len −66' }, reader: 'stop', seg: ALL_G,
      cost: ['Log: Aeron writes its 32 B header; the 34 B region starts zeroed.', 'Tail moves 96 = align(32 + 34, 32).'],
      caption: 'Aeron moves the tail on by the aligned frame and writes the header with a negative length, −66. Until that length turns positive, subscribers stop in front of this frame, even if later frames commit first.',
    },
    {
      ms: 4500, layout: 'note', lane: '3. Encode in place: header', call: 'NoteEncoder::send(claim, |enc| …)',
      over: { ahdr: 'w', claim: 'g', flen: 'len −66' }, reader: 'stop',
      seg: { mh: 'h', id: 'g', vl: 'g', body: 'g' }, noScratchNote: true,
      cost: ['Log: + 8 B message header, written in place.', 'Scratch buffer: none.'],
      caption: "send hands the claim to send_frame, which calls frame_mut() and wraps the encoder directly on the claimed bytes: try_wrap_and_apply_header writes the 8-byte header straight into the log. There is no staging buffer.",
    },
    {
      ms: 4000, layout: 'note', lane: '3. Encode in place: fixed block', call: 'enc.fixed(&NoteFixedFields { note_id: 1 })',
      callN: 'enc.fixed(&NoteFixedFields { note_id: 1 })',
      over: { ahdr: 'w', claim: 'g', flen: 'len −66' }, reader: 'stop',
      seg: { mh: 'w', id: 'h', vl: 'g', body: 'g' }, noScratchNote: true,
      cost: ['Log: + 4 B noteId.', 'Scratch buffer: none.'],
      caption: 'The fixed block, noteId, lands in the next 4 bytes. Each setter writes its field once at its final position.',
    },
    {
      ms: 4500, layout: 'note', lane: '3. Encode in place: var-data', call: '.body(b"rebalance at close")',
      over: { ahdr: 'w', claim: 'w', flen: 'len −66' }, reader: 'stop',
      seg: { mh: 'w', id: 'w', vl: 'h', body: 'h' }, noScratchNote: true,
      cost: ['Log: + 4 B length prefix + 18 B body.', 'Scratch buffer: none.'],
      caption: 'body writes its 4-byte length and the 18 bytes, and returns NoteComplete. The closure must return that terminal stage, so a frame with a tail still unwritten does not compile.',
    },
    {
      ms: 5000, layout: 'note', lane: '4. Check before publishing', call: 'send_frame: frame == claimed, header matches',
      callN: 'send_frame: length and header checks',
      over: { ahdr: 'w', claim: 'w', flen: 'len −66', ok: true }, reader: 'stop', seg: NOTE_W, noScratchNote: true,
      check: 'frame 34 == claimed 34; bytes 0..8 == HEADER_TEMPLATE', checkN: '34 == 34; first 8 B == HEADER_TEMPLATE',
      cost: CLAIM_COST,
      caption: 'send_frame compares the encoded length with the claim (34 against 34) and the first 8 bytes with the message header template. Both are a few integer compares on bytes already in cache.',
    },
    {
      ms: 5500, layout: 'note', lane: '5. Commit: the frame becomes visible', call: 'claim.commit()',
      over: { ahdr: 'h', claim: 'w', flen: 'len 66' }, reader: 'past', seg: NOTE_W, noScratchNote: true,
      cost: CLAIM_COST,
      caption: 'commit stores the positive length, 66, with release ordering. A subscriber that reads it sees the whole frame and moves past it. The message was written once, in the place subscribers read.',
    },
    {
      ms: 6500, layout: 'note', lane: 'Failure: a claim of the wrong size', call: 'try_claim_owned(len + 1) → LengthMismatch',
      callN: 'try_claim_owned(len + 1)',
      over: { ahdr: 'h', claim: 'd', flen: 'PAD 67', claimText: 'padding' }, reader: 'past',
      seg: { mh: 'd', id: 'd', vl: 'd', body: 'd', x: 'g' }, stamp: 'aborted: now a padding frame', noScratchNote: true,
      check: 'frame 34 != claimed 35: SendError::LengthMismatch', checkN: '34 != 35: LengthMismatch',
      cost: ['Encoded, then refused: byte 35 was never written by the encoder.', 'abort() turns the frame into padding.'],
      caption: "A 35-byte claim for a 34-byte frame fails the length check, so send_frame returns LengthMismatch { claimed: 35, frame: 34 } and AbortOnDrop calls abort. Aeron rewrites the header as a padding frame with a positive length; subscribers skip it and the log moves on at once.",
    },
    {
      ms: 6500, layout: 'note', lane: 'Failure: an encoder error', call: '.body(19 bytes) in a claim sized for 18',
      callN: '.body(19 bytes), claim sized for 18',
      over: { ahdr: 'h', claim: 'd', flen: 'PAD 66', claimText: 'padding' }, reader: 'past',
      seg: { mh: 'd', id: 'd', vl: 'g', body: 'g' }, stamp: 'aborted: now a padding frame', noScratchNote: true,
      check: 'BufferTooShort { field: "body", needed: 23, available: 22 }', checkN: 'Encode(BufferTooShort 23 > 22)',
      cost: ['Header and noteId stay in the log but are never read:', 'the frame is padding.'],
      caption: "The claim was sized for an 18-byte body, but body gets 19. The setter checks room before writing, so the header and noteId are in the claim and the length and body are not: it returns BufferTooShort { field: \"body\", needed: 23, available: 22 }, the ? returns early, send_frame returns SendError::Encode, and the guard aborts. claim_send_test covers this path with VarDataTooLong; a panic in the closure unwinds through the same guard.",
    },
    {
      ms: 5500, layout: 'note', lane: 'Offer route: encode into scratch', call: 'OfferClaim::new(&mut scratch, offer)',
      reader: 'past', scratch: ALL_G,
      cost: ['Scratch: 34 B zeroed by OfferClaim::new.', 'Log: nothing yet.'],
      caption: "Aeron's offer copies a finished frame, so OfferClaim adapts it to the same Claim trait over a scratch buffer. new() zeroes the 34 bytes first, so a reused buffer never leaks the previous frame into padding.",
    },
    {
      ms: 4500, layout: 'note', lane: 'Offer route: encode into scratch', call: 'NoteEncoder::send(claim, |enc| …)',
      reader: 'past', scratch: { mh: 'h', id: 'h', vl: 'h', body: 'h' },
      cost: ['Scratch: 34 B zeroed + 34 B encoded.', 'Log: nothing yet.'],
      caption: 'The same send, the same closure and the same checks run, but the bytes land in the scratch buffer. Nothing is in the log yet.',
    },
    {
      ms: 6000, layout: 'note', lane: 'Offer route: commit copies', call: 'commit() → publication.offer(frame)',
      callN: 'commit() → offer(frame)',
      over: { ahdr: 'h', claim: 'h', flen: 'len 66' }, reader: 'past', offerFrame: true,
      seg: { mh: 'h', id: 'h', vl: 'h', body: 'h' }, scratch: NOTE_W, copy: true,
      cost: ['Scratch: 34 B zeroed + 34 B encoded. Log: 32 B header + 34 B copied.', 'The 34 bytes are written three times; try_claim writes them once.'],
      caption: "commit calls offer, which moves the tail the same 96 bytes, writes the header and memcpys the 34 from scratch into the log. The sender writes the frame's bytes three times (zeroed, encoded, copied) against once, and the scratch buffer puts at least one more cache line in play. Aeron's cleaning of the log itself is the same on both routes.",
    },
    {
      ms: 6500, layout: 'cluster', lane: 'Cluster: try_claim adds the session header', call: 'let mut claim = client.try_claim(34)?;',
      callN: 'client.try_claim(34)?',
      over: { ahdr: 'h', claim: 'g', flen: 'len −98' }, reader: 'stop', noScratchNote: true,
      seg: { smh: 'h', sbl: 'h', mh: 'g', id: 'g', vl: 'g', body: 'g' },
      cost: ['Claim: 32 + 34 = 66 B; frame 98, aligned to 128.', 'try_claim writes the 32 B SessionMessageHeader in place.'],
      caption: 'AeronCluster::try_claim(34) claims 32 + 34 bytes and writes the SessionMessageHeader (leadership term, session id, timestamp 0) into the first 32 with ergon, before you see the claim.',
    },
    {
      ms: 6500, layout: 'cluster', lane: 'Cluster: fill payload_mut, then commit', call: 'encode into claim.payload_mut(); claim.commit()?',
      callN: 'payload_mut(); commit()?',
      over: { ahdr: 'h', claim: 'w', flen: 'len 98' }, reader: 'past', noScratchNote: true,
      seg: { smh: 'w', sbl: 'w', mh: 'h', id: 'h', vl: 'h', body: 'h' },
      cost: ['Log: 32 B Aeron header + 32 B session header + 34 B Note, once each.', 'On an encode error the sample calls abort() instead.'],
      caption: 'payload_mut() is the 34 bytes after the session header. Encode the Note there (the cluster-ha-orderbook sample wraps its encoder on it) and commit; on an encode error the sample aborts, so no partial message reaches the cluster.',
    },
    {
      ms: 8000, layout: 'note', lane: 'Why try_claim wins here, and when it loses', call: 'try_claim: 34 B once · offer: 34 B three times',
      callN: 'claim: 34 B once · offer: three times',
      over: { ahdr: 'w', claim: 'w', flen: 'len 66', ok: true }, reader: 'past', seg: NOTE_W, noScratchNote: true,
      cost: ['Wins: no scratch, no scratch zeroing, no copy. Loses: exact length', 'first, one MTU-sized frame only, an open claim stalls subscribers.'],
      caption: 'try_claim removes the scratch buffer, its zeroing and the copy. It needs the exact length before encoding, fits only one frame (offer fragments larger messages), and stalls subscribers while a claim is open, so encode promptly. No timing is shown here: the counts are bytes written.',
    },
  ];

  build(false, 'note');
  return {
    steps: steps.map((s) => ({ caption: s.caption, ms: s.ms, show: () => render(s) })),
    // Rebuild on crossing the phone breakpoint; true asks the runtime to redraw the step.
    onResize(width) {
      const narrow = width < 520;
      if (narrow === (g === NARROW)) return false;
      build(narrow, layoutName);
      return true;
    },
  };
});
