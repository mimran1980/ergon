// A borrowed view and an owned DTO have different work and lifetimes.
// Sources:
//   book/examples/flyweight-access.rs:39 per-field CarDecoder access
//   samples/sbe-feature-tour/src/lib.rs:412 CarDomain::try_from_decoder round trip
//   sbe/src/codegen/domain_cluster.rs:750 materialise group entries
//   sbe/src/codegen/domain_cluster.rs:894 Strings mode copies text with to_owned
//   sbe/src/codegen/domain_cluster.rs:914 Bytes mode copies data with to_vec
// The three tiles summarise a Car; widths are conceptual, not byte offsets.
Anim.register('flyweight-or-owned', (stage, api) => {
  const el = api.el;
  const svg = api.svg(300, 294, 'Read selected fields from borrowed bytes, or materialise an owned CarDomain');
  svg.style.maxWidth = '400px';
  svg.style.marginInline = 'auto';
  const title = el('text', { x: 0, y: 16, 'font-size': 14, class: 'ea-strong' }, svg);
  el('text', { x: 0, y: 43, 'font-size': 12 }, svg, 'Input frame (schematic)');
  const names = ['1234', 'groups', 'Honda'];
  const xs = [0, 102, 204];
  const source = [];
  const copies = [];
  names.forEach((name, i) => {
    const group = el('g', {}, svg);
    el('rect', { width: 96, height: 38, rx: 4, class: `ea-k${i + 1}` }, group);
    source.push(el('text', { x: 48, y: 24, 'text-anchor': 'middle', 'font-size': 13 }, group, name));
    group.style.transform = `translate(${xs[i]}px, 56px)`;
    const copy = el('g', {}, svg);
    el('rect', { width: 96, height: 38, rx: 4, class: `ea-k${i + 1}` }, copy);
    el('text', { x: 48, y: 24, 'text-anchor': 'middle', 'font-size': 13 }, copy, name);
    copies.push(copy);
  });
  const marker = el('g', {}, svg);
  el('path', { d: 'M 0 0 L 8 10 L -8 10 Z', class: 'ea-k1 ea-hot' }, marker);
  const view = el('text', { x: 0, y: 143, 'font-size': 12, class: 'ea-mono' }, svg);
  const work = el('text', { x: 0, y: 166, 'font-size': 12 }, svg);
  const dto = el('text', { x: 0, y: 201, 'font-size': 12, class: 'ea-strong' }, svg);
  const note = el('text', { x: 0, y: 282, 'font-size': 12 }, svg);
  const states = [
    { title: '1. Borrow the frame', at: 0, view: 'CarDecoder { borrowed bytes }', work: 'No whole-message materialisation', note: 'The view depends on its input buffer.',
      caption: 'CarDecoder borrows the input bytes. These tiles summarise fixed fields, repeating groups and text; they are not a byte layout. Constructing a view does not build a CarDomain or copy the whole message.' },
    { title: '2. Read one fixed field', at: 0, view: 'serial_number() → 1234', work: 'Read the requested field in place', note: 'Groups and text stay in the frame.',
      caption: 'serial_number() reads the selected field at its schema offset. A scalar result is returned by value; zero-copy means no whole-message materialisation, not that the CPU reads no bytes. Unused tails need not be decoded.' },
    { title: '3. Borrow a tail when needed', at: 2, view: 'borrowed var-data → Honda', work: 'Walk / skip preceding tails first', note: 'Borrowed text still needs the frame.',
      caption: 'To reach var-data, the decoder walks or skips preceding tails and checks their structure. A borrowed slice or text view refers to the original frame. Borrowing avoids an owned String, but tail navigation and text checks still do work.' },
    { title: '4. Choose to own the message', at: 2, count: 1, view: 'CarDomain::try_from_decoder(dec)', work: 'Materialisation can return an error', note: 'Copy fields; collect groups and text.',
      caption: 'CarDomain::try_from_decoder consumes a decoder and materialises an owned message. This is a fallible operation: malformed tails or invalid text can still be discovered while it walks the message. Fixed values are copied and groups are collected.' },
    { title: '5. Owned values leave the frame', at: 2, count: 3, view: 'CarDomain owns its values', work: 'Strings mode: Vec groups + String text', note: 'More work, independent storage.',
      caption: 'With DomainVarData::Strings, group collections and String text own storage; Bytes mode owns Vec<u8> data instead. This copies and can allocate. Use it for tooling or processing that needs the whole message beyond the input buffer lifetime.' },
    { title: '6. Release the borrow, reuse input', at: 0, count: 3, reused: true, view: 'Borrow ended before buffer reuse', work: 'The DTO still contains the old values', note: 'Rust prevents reuse while borrowed.',
      caption: 'Once all borrowed views have ended, the input buffer can be reused for another frame. Rust enforces that lifetime. The owned DTO retains 1234 and Honda because its values no longer borrow this frame; no stale decoder is used after reuse.' },
    { title: '7. Decide at the ownership boundary', at: 0, count: 3, reused: true, view: 'Few fields → borrowed decoder', work: 'Keep whole message → owned DTO', note: 'FixedFields is an encode input, too.',
      caption: 'Choose by the work and lifetime you need: read selected fields with a borrowed decoder, or pay for owned materialisation when the whole message must outlive the frame. FixedFields is a separate convenience for writing a complete fixed block; it is not an owned decode of the dynamic tails.' },
  ];
  function render(s) {
    title.textContent = s.title;
    view.textContent = s.view;
    work.textContent = s.work;
    note.textContent = s.note;
    dto.textContent = s.count ? 'Owned CarDomain (independent storage)' : '';
    marker.style.transform = `translate(${xs[s.at] + 48}px, 100px)`;
    marker.style.opacity = s.reused ? '0' : '1';
    source.forEach((text, i) => { text.textContent = s.reused ? 'next frame' : names[i]; });
    copies.forEach((copy, i) => {
      const copied = i < (s.count || 0);
      copy.style.transform = `translate(${xs[i]}px, ${copied ? 216 : 56}px)`;
      copy.style.opacity = copied ? '1' : '0';
    });
  }
  return { steps: states.map((s) => ({ ms: 6000, caption: s.caption, show: () => render(s) })) };
});
