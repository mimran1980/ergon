// Fan-out connect: a publication to every member, one request, keep the leader.
//
// Sources:
//   cluster/src/config.rs:298    ingress_endpoints validates the map at the call site
//   cluster/src/endpoints.rs:23  parse_ingress_endpoints: entries sorted by member id
//   cluster/src/client.rs:196    the request lands once: a second copy at the leader leaves a session
//   cluster/src/client.rs:254    offer_connect: `sent` stops a second copy
//   cluster/src/client.rs:277    on_redirect clears the correlation id, request bytes and `sent`
//   cluster/src/client.rs:285    on_challenge mints a new id for the challenge response
//   cluster/src/client.rs:416    redirect_members keeps only the leader's member
//   cluster/src/client.rs:430    open_member: async add of one exclusive publication
//   cluster/src/client.rs:460    take_connected: first connected member (or the preferred one)
//   cluster/src/client.rs:643    connect_single: one channel, blocking add, reconnect_ingress on redirect
//   cluster/src/client.rs:669    connect_fanout: the loop drawn here
//   cluster/src/client.rs:713    the one request is offered on the held ingress only
//   cluster/src/client.rs:722    classify_handshake on each egress event
//   cluster/src/client.rs:753    drop(members) closes every publication but the leader's
//   cluster/src/client.rs:787    accept only the SessionEvent or Challenge that echoes the id
//   cluster/src/client.rs:1802   AsyncClusterConnect opens the same per-member publications
//   cluster/src/protocol.rs:149  SessionEvent with another correlation id -> Ignore (:185 Challenge)
//   cluster/src/poller.rs:129    parse_leader_endpoint: leader_member_id looked up in the detail map
// Illustrative: which member connects first, that it is a follower, that
// member 2 leads, and the ids 7 and 8. The book records no
// connect timings, so the counts shown come from the code, not measurements.
Anim.register('fanout-connect', (stage, api) => {
  const el = api.el;
  const MEMBERS = [
    { id: 0, host: 'host-a:9010', role: 'follower' },
    { id: 1, host: 'host-b:9010', role: 'follower' },
    { id: 2, host: 'host-c:9010', role: 'leader' },
  ];
  // Wide: client panel and members spread out. Narrow (phones): a 300-unit
  // viewBox so text stays legible at 360px.
  const WIDE = { W: 460, font: 11, small: 10, call: 10, tokW: 96 };
  const NARROW = { W: 300, font: 11, small: 9.5, call: 8.5, tokW: 80 };
  const Y = { client: 36, clientH: 78, laneTop: 120, laneBot: 196, member: 200, memberH: 52, count: 272 };

  let g;
  let svg;
  let heading;
  let call;
  let client;
  let fields;
  let links;
  let linkText;
  let memberRects;
  let tokens;
  let counters;

  const laneX = (i) => (g.W / 3) * i + g.W / 6;

  function token(key, label, cls, lane, up) {
    const node = el('g', { class: 'ea-hidden' }, svg);
    // Opaque backing so the lane line does not show through the tint.
    el('rect', { x: -g.tokW / 2, y: -10, width: g.tokW, height: 20, rx: 4, style: 'fill: var(--bg)' }, node);
    el('rect', { x: -g.tokW / 2, y: -10, width: g.tokW, height: 20, rx: 4, class: `${cls} ea-tok` }, node);
    el('text', { x: 0, y: 4, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-mono ea-strong' }, node, label);
    const x = laneX(lane);
    const near = Y.laneTop + 14;
    const far = Y.laneBot - 14;
    // Requests travel down to a member, replies up to the client.
    tokens[key] = { node, from: [x, up ? far : near], to: [x, up ? near : far] };
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const W = g.W;
    stage.textContent = '';
    svg = api.svg(W, Y.count + 34, 'Fan-out connect: client, three members and the messages between them');
    heading = el('text', { x: 0, y: 12, 'font-size': g.font + 1, class: 'ea-strong' }, svg, '');
    call = el('text', { x: 0, y: 27, 'font-size': g.call, class: 'ea-mono' }, svg, '');

    client = el('rect', { x: 0, y: Y.client, width: W, height: Y.clientH, rx: 5, class: 'ea-box' }, svg);
    el('text', { x: 8, y: Y.client + 15, 'font-size': g.font, class: 'ea-strong' }, svg, 'client');
    const rows = ['preferred', 'ingress', 'correlation id', 'request', 'session'];
    // Three fields in the left column, two in the right; value x per column.
    const valueDx = narrow ? [72, 40] : [90, 56];
    fields = {};
    rows.forEach((name, i) => {
      const col = i < 3 ? 0 : 1;
      const x0 = 8 + col * (W / 2);
      const y = Y.client + 30 + (i % 3) * 14;
      el('text', { x: x0, y, 'font-size': g.small, class: 'ea-muted' }, svg, name);
      fields[name] = el('text', { x: x0 + valueDx[col], y, 'font-size': g.small, class: 'ea-mono' }, svg, '');
    });

    links = [];
    linkText = [];
    memberRects = [];
    const mw = W / 3 - 8;
    MEMBERS.forEach((m, i) => {
      const x = laneX(i);
      links.push(el('line', { x1: x, y1: Y.laneTop, x2: x, y2: Y.laneBot, class: 'ea-k1', 'stroke-width': 2 }, svg));
      linkText.push(el('text', { x: x + 5, y: (Y.laneTop + Y.laneBot) / 2 + 3, 'font-size': g.small - 0.5, class: 'ea-muted' }, svg, ''));
      memberRects.push(el('rect', { x: x - mw / 2, y: Y.member, width: mw, height: Y.memberH, rx: 5, class: 'ea-box' }, svg));
      el('text', { x, y: Y.member + 15, 'text-anchor': 'middle', 'font-size': g.font, class: 'ea-strong' }, svg, `member ${m.id}`);
      el('text', { x, y: Y.member + 29, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-mono' }, svg, m.host);
      el('text', { x, y: Y.member + 43, 'text-anchor': 'middle', 'font-size': g.small, class: 'ea-muted' }, svg, m.role);
    });

    tokens = {};
    token('req7', 'Connect #7', 'ea-k1', 0, false);
    token('redir7', 'REDIRECT #7', 'ea-k2', 0, true);
    token('req8', 'Connect #8', 'ea-k1', 2, false);
    token('ok8', 'OK #8', 'ea-k3', 2, true);

    counters = el('text', { x: 0, y: Y.count, 'font-size': g.small, class: 'ea-mono' }, svg, '');
    el('text', { x: 0, y: Y.count + 15, 'font-size': g.small - 0.5, class: 'ea-muted' }, svg,
      'Illustrative run: member 0 connects first and member 2 leads.');
    // Lines are ingress publications; replies share the one egress subscription.
    el('text', { x: 0, y: Y.count + 28, 'font-size': g.small - 0.5, class: 'ea-muted' }, svg,
      g === NARROW ? 'Requests go down ingress; replies return on egress.'
        : 'Requests go down an ingress publication; replies return on the one egress.');
  }

  // Link states: none | adding | open | ingress | closed.
  function drawLink(i, state) {
    const line = links[i];
    line.classList.toggle('ea-hidden', state === 'none');
    line.classList.toggle('ea-hot', state === 'ingress');
    line.setAttribute('stroke-dasharray', state === 'adding' ? '4 4' : state === 'closed' ? '1 5' : 'none');
    line.setAttribute('stroke-opacity', state === 'closed' ? '0.45' : '1');
    line.setAttribute('stroke-width', state === 'ingress' ? 4 : 2);
    linkText[i].textContent = { none: '', adding: 'adding', open: 'connected', ingress: 'ingress', closed: 'closed' }[state];
  }

  function place(t, pos) {
    const [x, y] = pos === 'from' ? t.from : t.to;
    t.node.style.transform = `translate(${x}px, ${y}px)`;
    t.node.classList.toggle('ea-hidden', pos !== 'at');
  }

  function render(s) {
    heading.textContent = s.heading;
    call.textContent = s.call;
    const f = s.fields;
    for (const [name, node] of Object.entries(fields)) {
      node.textContent = f[name];
      node.classList.toggle('ea-strong', (s.changed || []).includes(name));
    }
    client.setAttribute('class', f.session === 'Connected' ? 'ea-k3' : 'ea-box');
    s.links.forEach((state, i) => drawLink(i, state));
    memberRects.forEach((r, i) => r.classList.toggle('ea-ok', s.focus === i));
    // Tokens before their step sit at their origin, hidden; after it, at their
    // destination, hidden; so each one moves only on the step it is sent.
    for (const [key, t] of Object.entries(tokens)) {
      const at = s.tokens[key];
      place(t, at === undefined ? (s.after.includes(key) ? 'to' : 'from') : 'at');
      // Emphasis by outline only: ea-hot's fuller fill drowns the label on dark themes.
      t.node.querySelector('.ea-tok').setAttribute('stroke-width', at === 'hot' ? 2.5 : 1);
    }
    const c = s.count;
    counters.textContent = `publications ${c[0]} · requests ${c[1]} · matched ${c[2]}`;
  }

  const NONE = ['none', 'none', 'none'];
  const UNSENT = { preferred: '–', ingress: '–', 'correlation id': '–', request: '–', session: 'Closed' };
  const states = [
    {
      ms: 6000, heading: 'Three members, leader unknown', call: '.ingress_endpoints("0=host-a:9010,1=…,2=…")?',
      links: NONE, tokens: {}, after: [], count: [0, 0, 0], fields: UNSENT,
      caption: 'The setter validates the map at the call site; connect parses it again into entries sorted by member id. The client does not know which member leads; in this illustrative run it is member 2.',
    },
    {
      ms: 5500, heading: 'connect_fanout: one publication per member', call: 'open_member(..) × 3  // async add, nothing blocks',
      links: ['adding', 'adding', 'adding'], tokens: {}, after: [], count: [3, 0, 0], fields: UNSENT,
      caption: 'With ingress_endpoints set, connect() takes connect_fanout (connect_async does the same). It starts an async add of the egress subscription and of one exclusive publication per member, then polls them all in one loop under one message-timeout deadline. No request is sent yet.',
    },
    {
      ms: 5500, heading: 'Take the first publication that connects', call: 'take_connected(&mut members, None)',
      links: ['ingress', 'open', 'adding'], tokens: {}, after: [], count: [3, 0, 0], changed: ['ingress'], focus: 0,
      fields: { ...UNSENT, ingress: 'member 0' },
      caption: 'Members 0 and 1 have connected; member 2 is still adding (or added but not yet connected). take_connected takes the first connected one in member-id order. Connected only means the member is listening: it says nothing about who leads.',
    },
    {
      ms: 6500, heading: 'One request, on one publication', call: 'attempt.prepare(..); attempt.offer_connect(member 0)',
      links: ['ingress', 'open', 'adding'], tokens: { req7: 'hot' }, after: [], count: [3, 1, 0], changed: ['correlation id', 'request'], focus: 0,
      fields: { ...UNSENT, ingress: 'member 0', 'correlation id': '7', request: 'sent' },
      caption: 'Once the egress port resolves, prepare mints id 7 and encodes SessionConnectRequest once. It goes to member 0 only, and the sent flag stops a second copy: the request lands once, as Java\'s client sends it, because a second copy reaching the leader would leave a session behind.',
    },
    {
      ms: 6500, heading: 'A follower answers with REDIRECT', call: 'classify_handshake(event, 7, ..) -> Redirect{host-c:9010}',
      links: ['ingress', 'open', 'adding'], tokens: { redir7: 'hot' }, after: ['req7'], count: [3, 1, 1], focus: 0,
      fields: { ...UNSENT, ingress: 'member 0', 'correlation id': '7', request: 'sent' },
      caption: 'Member 0 is a follower: it opens no session. It answers SessionEvent(REDIRECT) echoing id 7, with leader_member_id 2 and the endpoints map in its detail. One i64 comparison accepts it; parse_leader_endpoint looks up member 2: host-c:9010.',
    },
    {
      ms: 6500, heading: 'Keep only the leader\'s member', call: 'redirect_members(..); preferred = host-c; on_redirect()',
      links: ['closed', 'closed', 'adding'], tokens: {}, after: ['req7', 'redir7'], count: [1, 1, 1], changed: ['preferred', 'ingress', 'correlation id', 'request'], focus: 2,
      fields: { ...UNSENT, preferred: 'host-c:9010', ingress: '– (waiting)', 'correlation id': '– (cleared)' },
      caption: 'host-a is not the leader, so the held ingress is dropped. redirect_members keeps only the member whose endpoint matches, closing member 1. on_redirect clears the id, the encoded request and the sent flag; the next poll mints a new id at once, even before member 2 connects.',
    },
    {
      ms: 6500, heading: 'The leader\'s add is already under way', call: 'take_connected(.., Some("host-c:9010")); offer_connect',
      links: ['closed', 'closed', 'ingress'], tokens: { req8: 'hot' }, after: ['req7', 'redir7'], count: [1, 2, 1], changed: ['ingress', 'correlation id', 'request'], focus: 2,
      fields: { ...UNSENT, preferred: 'host-c:9010', ingress: 'member 2', 'correlation id': '8', request: 'sent' },
      caption: 'Member 2\'s add began before the loop, with the others, so no new publication is created. This is what fan-out buys: connect_single would block on a fresh add after the redirect. The request with id 8 waits only for member 2 to connect, then lands on it once.',
    },
    {
      ms: 6500, heading: 'The leader answers OK, echoing 8', call: 'classify_handshake(event, 8, ..) -> Established',
      links: ['closed', 'closed', 'ingress'], tokens: { ok8: 'hot' }, after: ['req7', 'redir7', 'req8'], count: [1, 2, 2], changed: ['session'], focus: 2,
      fields: { ...UNSENT, preferred: 'host-c:9010', ingress: 'member 2', 'correlation id': '8', request: 'sent', session: 'OK accepted' },
      caption: 'Member 2 answers SessionEvent(OK) echoing 8, with the cluster session id and leadership term. An event echoing any other id would be ignored. Had it sent a Challenge echoing 8, the client would answer on the same publication under a new id and wait for the OK that echoes that one.',
    },
    {
      ms: 6000, heading: 'Connected through the leader', call: 'drop(members); on_established(..)',
      links: ['none', 'none', 'ingress'], tokens: {}, after: ['req7', 'redir7', 'req8', 'ok8'], count: [1, 2, 2], changed: ['session'], focus: 2,
      fields: { ...UNSENT, preferred: 'host-c:9010', ingress: 'member 2', 'correlation id': '8', request: 'sent', session: 'Connected' },
      caption: 'The loop ends. drop(members) closes whatever is left, and the client keeps the leader\'s publication and the egress, bound to the OK\'s image. The cluster holds one session: the follower redirected instead of opening one.',
    },
    {
      ms: 8000, heading: 'Why fan-out, and when not', call: 'counts from the code, not timings',
      links: ['none', 'none', 'ingress'], tokens: {}, after: ['req7', 'redir7', 'req8', 'ok8'], count: [1, 2, 2],
      fields: { ...UNSENT, preferred: 'host-c:9010', ingress: 'member 2', 'correlation id': '8', request: 'sent', session: 'Connected' },
      caption: 'Wins when the leader is unknown: 3 publications start at once, the redirect target is already connecting (when the redirect names the configured endpoint string), and it costs 2 round trips (1 if the first member to connect leads). Loses when the leader is known: 2 of the 3 publications and their log buffers exist only to be closed; one ingress_channel is cheaper. The book records no connect timings.',
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
