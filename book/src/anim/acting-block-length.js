// A v2 decoder reading old, current and future fixed blocks.
// Sources:
//   sbe/src/codegen/message_decoder.rs:289 min_readable_fixed_extent(acting_version)
//   sbe/src/codegen/message_decoder.rs:320 body_need = max(acting block, readable extent)
//   sbe/src/codegen/message_decoder.rs:466 checked header block length and version
//   sbe/src/codegen/message_decoder.rs:747 optional scalar version/block/null guards
//   sbe/src/codegen/tail_stages.rs:119 first tail starts at body + acting block length
// Illustrative append-only schema: v1 fixed 8 B; optional uint32 added in v2;
// v3 appends another 4 B. The header is 8 B; identity checks are assumed to pass.
Anim.register('acting-block-length', (stage, api) => {
  const el = api.el;
  const svg = api.svg(300, 302, 'Acting version controls field availability; acting block length locates the first tail');
  svg.style.maxWidth = '400px';
  svg.style.marginInline = 'auto';
  const title = el('text', { x: 0, y: 16, 'font-size': 14, class: 'ea-strong' }, svg);
  const header = el('text', { x: 0, y: 42, 'font-size': 12, class: 'ea-mono' }, svg);
  el('text', { x: 0, y: 67, 'font-size': 12 }, svg, 'frame: 8-byte header, then body');
  const blocks = [
    ['header', 0, 96, 'ea-box'],
    ['v1: 8 B', 96, 96, 'ea-k1'],
    ['v2', 192, 48, 'ea-k2'],
    ['v3', 240, 48, 'ea-k4'],
  ].map(([text, x, width, cls]) => {
    const g = el('g', {}, svg);
    g.style.transform = `translate(${x}px, 82px)`;
    const box = el('rect', { width: width - 2, height: 36, rx: 3, class: cls }, g);
    el('text', { x: width / 2, y: 23, 'text-anchor': 'middle', 'font-size': 12 }, g, text);
    return { g, box };
  });
  const tail = el('g', {}, svg);
  el('path', { d: 'M 0 0 L -6 12 L 6 12 Z', class: 'ea-k3 ea-hot' }, tail);
  const tailLabel = el('text', { x: 0, y: 151, 'font-size': 12 }, svg);
  const proof = el('text', { x: 0, y: 184, 'font-size': 12, class: 'ea-mono' }, svg);
  const result = el('text', { x: 0, y: 208, 'font-size': 12, class: 'ea-strong' }, svg);
  const field = el('text', { x: 0, y: 243, 'font-size': 12 }, svg);
  const note = el('text', { x: 0, y: 287, 'font-size': 12 }, svg);
  const states = [
    { title: '1. Keep two lengths separate', ver: 2, bl: 12, tail: 240, visible: 3, proof: 'local BLOCK_LENGTH = 12', result: 'Header blockLength belongs to sender', field: 'Local schema: v1 fields + optional v2', note: 'Illustrative append-only fixed fields.',
      caption: 'This illustrative decoder knows schema v2: an 8-byte original fixed block plus a 4-byte optional uint32 added in v2. Local BLOCK_LENGTH is 12. The frame header carries the sender’s acting version and acting block length; do not substitute the local constant for that length.' },
    { title: '2. Accept an older fixed block', ver: 1, bl: 8, tail: 192, visible: 2, proof: 'need = max(8, extent(v1)=8)', result: '8 body bytes fit → construct view', field: 'Optional v2 field → None', note: 'Old data does not need v2 storage.',
      caption: 'A v1 sender declares blockLength 8. The minimum readable fixed extent at acting version 1 is also 8, so the checked constructor accepts a body of 8 bytes. The optional v2 getter returns None without reading the absent field. A full local 12-byte fixed block is not required.' },
    { title: '3. Locate the old frame’s tail', ver: 1, bl: 8, tail: 192, visible: 2, proof: 'tail = 8 + acting blockLength', result: 'First tail starts at frame byte 16', field: 'Using local 12 would start 4 B late', note: 'Tail contents are checked on consume.',
      caption: 'The first dynamic tail begins at header length plus acting block length: 8 + 8 = frame byte 16. The green marker moves with this boundary. Using the decoder’s local BLOCK_LENGTH would skip four real tail bytes. Tail contents still need validation when consumed.' },
    { title: '4. Read the current version', ver: 2, bl: 12, tail: 240, visible: 3, proof: 'need = max(12, extent(v2)=8)', result: 'First tail starts at frame byte 20', field: 'Optional v2 field → decoded value', note: 'Version and field extent permit a read.',
      caption: 'A v2 sender includes the appended field and declares blockLength 12. The version and block-length guards allow its optional getter to read the wire value, and the first tail moves to frame byte 20. An encoded null still returns None.' },
    { title: '5. Skip a future fixed suffix', ver: 3, bl: 16, tail: 288, visible: 4, proof: 'need = max(16, known extent=8)', result: 'First tail starts at frame byte 24', field: 'Known v2 fields remain readable', note: 'Unknown 4-byte suffix stays opaque.',
      caption: 'For this compatible append-only evolution, a v3 sender adds four more fixed bytes and declares blockLength 16. The v2 decoder knows only 12 bytes of fields, but proves the entire declared 16-byte block fits. It skips the unknown suffix and starts the existing first tail at frame byte 24.' },
    { title: '6. Reject a truncated fixed block', ver: 2, bl: 12, tail: 240, visible: 3, bad: true, proof: 'need 12 body bytes; only 8 remain', result: 'try_decode → Err(BufferTooShort)', field: 'No decoder or field read is returned', note: 'A header claim does not prove bytes exist.',
      caption: 'Now the header claims a v2 12-byte fixed block, but the frame ends after only 8 body bytes. try_decode rejects it with BufferTooShort before returning a decoder. Acting metadata guides the proof; it is not trusted evidence that the bytes are present.' },
    { title: '7. Bounds proof has a scope', ver: 2, bl: 12, tail: 240, visible: 3, proof: 'header + fixed extent first', result: 'Dynamic tails: checked as reached', field: 'Value validation is a separate opt-in', note: 'No eager proof of every tail or value.',
      caption: 'Successful checked construction proves the header and version-aware fixed extent, not all dynamic tails or schema value ranges. Tail checks run as the decoder advances; an explicit verify adds a structural pass. with_validate(true) generates the separate validate walk for supported value constraints.' },
  ];
  function render(s) {
    title.textContent = s.title;
    header.textContent = `actingVersion ${s.ver} · blockLength ${s.bl}`;
    proof.textContent = s.proof;
    result.textContent = s.result;
    field.textContent = s.field;
    note.textContent = s.note;
    tailLabel.textContent = s.bad ? 'Frame ends at byte 16 (truncated)' : `First tail: frame byte ${8 + s.bl}`;
    tail.style.transform = `translate(${s.bad ? 192 : s.tail}px, 121px)`;
    blocks.forEach((block, i) => {
      block.g.style.opacity = i < s.visible ? '1' : '0.15';
      block.box.setAttribute('stroke-dasharray', s.bad && i === 2 ? '4 3' : 'none');
      if (s.bad && i === 2) block.g.style.opacity = '0.3';
    });
  }
  return { steps: states.map((s) => ({ ms: 6500, caption: s.caption, show: () => render(s) })) };
});
