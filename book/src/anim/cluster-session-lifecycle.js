// Cluster client session: connect, keep alive, fail over, close.
//
// Sources:
//   cluster/src/state.rs:5         SessionState: Connected, AwaitingNewLeader, PendingClose, Closed
//   cluster/src/client.rs:32       keep_alive_interval_ms = (message_timeout / 4).clamp(50, 1000)
//   cluster/src/client.rs:128      dispatch_regular only records a NewLeaderEvent / CLOSED during the poll
//   cluster/src/client.rs:254      offer_connect: the connect request lands once
//   cluster/src/client.rs:265      offer_challenge; :285 on_challenge mints a new correlation id
//   cluster/src/client.rs:602      connect; :612 unconnected starts in Closed with session id -1
//   cluster/src/client.rs:793      handshake: bounded by message_timeout, Established -> Connected
//   cluster/src/client.rs:990      send_keep_alive: 24-byte stack buffer, up to SEND_ATTEMPTS (3)
//   cluster/src/client.rs:1028     keep_alive_if_due: only while Connected; poll does not call it
//   cluster/src/client.rs:1126     close: SessionCloseRequest if Connected, then Closed (pubs freed on drop)
//   cluster/src/client.rs:1041     note_ingress_failure: MaxPositionExceeded -> PendingClose outside poll
//   cluster/src/config.rs:90       message_timeout_ms default 5000, so the keep-alive interval is 1000 ms
//   cluster/src/client.rs:1200     apply_polled: CLOSED -> PendingClose first, then NewLeader, then
//                                  poll_state in the same poll() (so CLOSED ends in Closed at once)
//   cluster/src/client.rs:1229     on_new_leader_event: resolve, async add, rebind egress, restart clock
//   cluster/src/client.rs:1269     poll_pending_ingress: adopt term and leader once connected
//   cluster/src/client.rs:1417     poll_state: message_timeout after the event, else new_leader_timeout
//   cluster/src/client.rs:1542     apply_state_transition: egress lost, timeouts, PendingClose -> Closed
//   cluster/src/protocol.rs:29     new_leader_timeout: configured, else 2 x leader heartbeat from the OK
//   cluster/src/protocol.rs:44     NotConnected/Closed -> AwaitingNewLeader, MaxPositionExceeded -> PendingClose
//   cluster/src/poller.rs:61       parse_event; :129 parse_leader_endpoint picks leader_member_id
//   cluster/src/fragment.rs:24     Fragment: SessionEvent, NewLeader, Challenge
//   cluster/src/endpoints.rs:23    parse_ingress_endpoints: "id=host:port,..." ordered by member id
//   cluster/schemas/aeron-cluster-codecs.xml:179  SessionKeepAlive: 8-byte SBE header + two int64 (16-byte block)
// Endpoints (m0:9002 ...) and the S / T / T' field values are illustrative.
// Timeouts are configuration and are shown symbolically; this book records no
// cluster timings, so none are shown.
Anim.register('cluster-session-lifecycle', (stage, api) => {
  const el = api.el;
  const WIDE = { W: 400, CW: 104, MW: 112, f: 1 };
  const NARROW = { W: 300, CW: 84, MW: 96, f: 1 };
  const STATES = [
    ['connected', ['Connected']],
    ['awaiting', ['Awaiting', 'NewLeader']],
    ['pending', ['Pending', 'Close']],
    ['closed', ['Closed']],
  ];
  const H = 292;
  const MY = [44, 90, 136]; // member box tops
  const MH = 38;
  const IN_Y = 84; // client ingress port
  const OUT_Y = 140; // client egress port

  let g;
  let svg;
  let r; // drawn parts

  const px = (n) => Math.round(n * g.f * 10) / 10;

  function text(x, y, size, cls, value, parent, anchor) {
    return el('text', { x, y, 'font-size': px(size), class: cls || '', 'text-anchor': anchor || 'start' }, parent || svg, value);
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    const { W, CW, MW } = g;
    const MX = W - MW;
    stage.textContent = '';
    svg = api.svg(W, H, 'Cluster client session: three members, the client ingress and egress, and its session state');
    r = { members: [], pills: [], MX };

    r.lane = text(0, 13, 12, 'ea-strong', '');
    r.call = text(0, 28, 10, 'ea-mono', '');

    // Client with its two Aeron endpoints.
    el('rect', { x: 0, y: 50, width: CW, height: 116, rx: 4, class: 'ea-box' }, svg);
    text(8, 66, 11, 'ea-strong', 'client');
    r.inPort = el('rect', { x: 6, y: IN_Y - 11, width: CW - 12, height: 22, rx: 3, class: 'ea-k5' }, svg);
    text(CW / 2, IN_Y + 4, 9, 'ea-mono', 'ingress pub', svg, 'middle');
    r.outPort = el('rect', { x: 6, y: OUT_Y - 11, width: CW - 12, height: 22, rx: 3, class: 'ea-k4' }, svg);
    text(CW / 2, OUT_Y + 4, 9, 'ea-mono', 'egress sub', svg, 'middle');
    text(CW / 2, 108, 8, 'ea-muted', 'one Aeron client', svg, 'middle');

    // Wires: ingress (client -> leader), pending ingress, egress (member -> client).
    r.inWire = el('line', { 'stroke-width': 2, stroke: 'var(--ea-c5)' }, svg);
    r.pendWire = el('line', { 'stroke-width': 2, stroke: 'var(--ea-c5)', 'stroke-dasharray': '4 3' }, svg);
    r.outWire = el('line', { 'stroke-width': 2, stroke: 'var(--ea-c4)' }, svg);

    for (let i = 0; i < 3; i += 1) {
      const y = MY[i];
      const m = {};
      m.rect = el('rect', { x: MX, y, width: MW, height: MH, rx: 4, class: 'ea-box' }, svg);
      text(MX + 7, y + 15, 10, 'ea-strong', `member ${i}`);
      m.role = text(MX + 7, y + 30, 9, '', '');
      m.cross = el('path', {
        d: `M${MX + MW - 22} ${y + 10} l12 18 M${MX + MW - 10} ${y + 10} l-12 18`,
        stroke: 'var(--ea-c2)', 'stroke-width': 3, fill: 'none',
      }, svg);
      r.members.push(m);
    }

    // The message in flight.
    r.token = el('g', {}, svg);
    r.tokenRect = el('rect', { x: -31, y: -9, width: 62, height: 18, rx: 9 }, r.token);
    r.tokenText = text(0, 3.5, 8.5, 'ea-mono ea-strong', '', r.token, 'middle');

    r.wire = text(0, 186, 9, 'ea-mono', '');
    r.fields = text(0, 202, 10, 'ea-mono', '');

    // SessionState pills.
    text(0, 222, 9, 'ea-muted', 'SessionState');
    const pw = W / 4 - 4;
    STATES.forEach(([key, lines], i) => {
      const x = i * (W / 4);
      const rect = el('rect', { x, y: 227, width: pw, height: 28, rx: 5, class: 'ea-box' }, svg);
      lines.forEach((line, j) => {
        const y = lines.length === 1 ? 245 : 238.5 + j * 11;
        text(x + pw / 2, y, 9.5, '', line, svg, 'middle');
      });
      r.pills.push({ key, rect });
    });

    // Deadline bar: a clock running against a timeout (length not to scale).
    r.dlLabel = text(0, 270, 9, 'ea-mono', '');
    r.dlTrack = el('rect', { x: 0, y: 276, width: W, height: 8, rx: 3, class: 'ea-box' }, svg);
    r.dlFill = el('rect', { x: 0, y: 276, width: W, height: 8, rx: 3, class: 'ea-k1' }, svg);
    r.dlFill.style.transformOrigin = '0 280px';
    r.ticks = el('g', {}, svg);
    // Default keep-alive interval: 1000 ms ticks over the default 5000 ms message_timeout.
    for (let k = 1; k <= 5; k += 1) {
      el('line', { x1: (W * k) / 5 - 1, x2: (W * k) / 5 - 1, y1: 273, y2: 287, stroke: 'var(--ea-c5)', 'stroke-width': 2 }, r.ticks);
    }
  }

  // Local highlight: ea-hot's 0.6 fill makes labels on these boxes hard to
  // read on coal/navy/ayu, so outline in the text colour with a lighter fill.
  function mark(node, on) {
    node.style.stroke = on ? 'var(--fg)' : '';
    node.style.strokeWidth = on ? '2.5' : '';
    node.style.fillOpacity = on ? '0.38' : '';
  }

  function memberMid(i) {
    return MY[i] + MH / 2;
  }

  function setLine(line, x1, y1, x2, y2) {
    if (x1 === undefined) {
      line.classList.add('ea-hidden');
      return;
    }
    line.classList.remove('ea-hidden');
    line.setAttribute('x1', x1);
    line.setAttribute('y1', y1);
    line.setAttribute('x2', x2);
    line.setAttribute('y2', y2);
  }

  function moveToken(tok) {
    const t = r.token;
    if (!tok) {
      t.classList.add('ea-hidden');
      return;
    }
    t.classList.remove('ea-hidden');
    r.tokenRect.setAttribute('class', tok.dir === 'in' ? 'ea-k5' : 'ea-k4');
    mark(r.tokenRect, true);
    r.tokenText.textContent = tok.label;
    const a = { x: g.CW + 36, y: tok.dir === 'in' ? IN_Y : OUT_Y };
    const b = { x: r.MX - 36, y: memberMid(tok.member) };
    const [from, to] = tok.dir === 'in' ? [a, b] : [b, a];
    // Jump to the sender, then glide to the receiver (reduced motion: the
    // stylesheet's transition:none !important makes this a jump to the end).
    t.style.transition = 'none';
    t.style.transform = `translate(${from.x}px, ${from.y}px)`;
    t.getBoundingClientRect();
    t.style.transition = 'transform 1.1s ease-in-out, opacity 0.35s';
    t.style.transform = `translate(${to.x}px, ${to.y}px)`;
  }

  function runDeadline(dl) {
    const fill = r.dlFill;
    const show = Boolean(dl);
    for (const n of [r.dlLabel, r.dlTrack, fill]) n.classList.toggle('ea-hidden', !show);
    r.ticks.classList.toggle('ea-hidden', !(dl && dl.ticks));
    if (!dl) return;
    r.dlLabel.textContent = dl.label;
    fill.style.transition = 'none';
    fill.style.transform = `scaleX(${dl.from})`;
    fill.getBoundingClientRect();
    fill.style.transition = 'transform 2.4s linear';
    fill.style.transform = `scaleX(${dl.to})`;
  }

  function render(s) {
    r.lane.textContent = s.lane;
    r.call.textContent = s.call;
    r.members.forEach((m, i) => {
      const dead = s.dead === i;
      const leader = s.leader === i;
      m.rect.setAttribute('class', dead ? 'ea-k2' : leader ? 'ea-k1' : 'ea-box');
      mark(m.rect, s.hotMember === i);
      m.role.textContent = dead ? 'down' : leader ? 'leader' : 'follower';
      m.role.setAttribute('class', leader && !dead ? 'ea-strong' : 'ea-muted');
      m.cross.classList.toggle('ea-hidden', !dead);
    });
    const xIn = g.CW;
    const xM = r.MX;
    if (s.ingressTo === null) setLine(r.inWire);
    else setLine(r.inWire, xIn, IN_Y, xM, memberMid(s.ingressTo));
    // The old leader's publication lingers, unconnected, until the swap.
    r.inWire.style.opacity = s.dead !== null && s.ingressTo === s.dead ? '0.3' : '';
    if (s.pendingTo === undefined) setLine(r.pendWire);
    else setLine(r.pendWire, xIn, IN_Y + 4, xM, memberMid(s.pendingTo) + 4);
    if (s.egressFrom === null) setLine(r.outWire);
    else setLine(r.outWire, xM, memberMid(s.egressFrom) + 6, xIn, OUT_Y);
    r.inWire.setAttribute('stroke-width', s.hotWire === 'in' ? 4 : 2);
    mark(r.inPort, s.hotWire === 'in' || (s.token && s.token.dir === 'in'));
    mark(r.outPort, Boolean(s.token && s.token.dir === 'out'));
    r.wire.textContent = s.wire || '';
    r.fields.textContent = s.fields;
    r.fields.setAttribute('class', s.fieldsChanged ? 'ea-mono ea-strong' : 'ea-mono ea-muted');
    for (const p of r.pills) {
      const on = p.key === s.state;
      p.rect.setAttribute('class', on ? STATE_CLS[p.key] : 'ea-box');
      mark(p.rect, on);
    }
    moveToken(s.token);
    runDeadline(s.deadline);
  }

  const STATE_CLS = { connected: 'ea-k3', awaiting: 'ea-k5', pending: 'ea-k4', closed: 'ea-k2' };
  const ENDPOINTS = '0=m0:9002,1=m1:9002,2=m2:9002';
  const F0 = 'session -1 · term -1 · leader -1';
  const F1 = 'session S · term T · leader 0';
  const F2 = "session S · term T' · leader 1";
  const MT = 'message_timeout';
  const NLT = 'new_leader_timeout';

  const states = [
    {
      ms: 6000, lane: 'Three members, one client', call: 'AeronCluster::connect(&builder, aeron_dir)',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'closed', fields: F0,
      caption: 'The client owns one ingress publication, to the leader, and one egress subscription, where the leader\'s replies arrive. Until the handshake succeeds the client is Closed with session id -1. Member 0 leads. (With ingress_endpoints set, connect adds a publication to every member and keeps only the leader\'s.)',
    },
    {
      ms: 5000, lane: '1 · Connect request', call: 'attempt.offer_connect(&ingress)',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'closed', fields: F0, hotMember: 0,
      token: { dir: 'in', member: 0, label: 'Connect' }, wire: 'SessionConnectRequest(correlation C1)',
      deadline: { label: `handshake deadline: ${MT}`, from: 0, to: 0.3 },
      caption: 'connect() resolves the egress response channel, mints correlation id C1 and encodes one SessionConnectRequest. It is offered until it lands once: each request opens a session, so a second copy would leave one behind. message_timeout bounds the whole handshake.',
    },
    {
      ms: 4500, lane: '1 · Challenge (if authenticating)', call: 'HandshakeEvent::Challenge',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'closed', fields: F0,
      token: { dir: 'out', member: 0, label: 'Challenge' }, wire: 'Challenge(correlation C1, session S)',
      deadline: { label: `handshake deadline: ${MT}`, from: 0.3, to: 0.5 },
      caption: 'An authenticator may answer with a Challenge on egress. The client acts only on events that echo its current correlation id; anything else is ignored.',
    },
    {
      ms: 4500, lane: '1 · Challenge response', call: 'attempt.offer_challenge(&ingress)',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'closed', fields: F0, hotMember: 0,
      token: { dir: 'in', member: 0, label: 'Response' }, wire: 'ChallengeResponse(correlation C2, session S)',
      deadline: { label: `handshake deadline: ${MT}`, from: 0.5, to: 0.65 },
      caption: 'on_challenge mints a fresh correlation id, C2, and encodes a ChallengeResponse with the credentials. The original connect request is never offered again.',
    },
    {
      ms: 6000, lane: '1 · Session open', call: 'HandshakeEvent::Established',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'connected', fields: F1, fieldsChanged: true,
      token: { dir: 'out', member: 0, label: 'OK' }, wire: 'SessionEvent(OK, correlation C2)',
      caption: 'SessionEvent(OK) echoing C2: the client stores session id, term and leader, binds the egress image that delivered it, and fixes new_leader_timeout (the configured value, else twice the leader heartbeat in the OK). State: Connected.',
    },
    {
      ms: 6500, lane: '2 · Keep alive', call: 'client.keep_alive_if_due()?',
      leader: 0, ingressTo: 0, egressFrom: 0, state: 'connected', fields: F1, hotMember: 0, hotWire: 'in',
      token: { dir: 'in', member: 0, label: 'KeepAlive' }, wire: 'SessionKeepAlive(term T, session S): 24 bytes',
      deadline: { label: 'default message_timeout 5 s: keep-alive every 1 s', from: 0, to: 1, ticks: true },
      caption: 'poll() never sends keep-alives: call keep_alive_if_due() from the same loop. While Connected and the interval (message_timeout/4, clamped to 50-1000 ms: 1000 ms with the default 5 s, as the ticks show) has passed, it offers a 24-byte SessionKeepAlive from a stack buffer, up to 3 tries if back-pressured. Otherwise it is one clock read and a compare.',
    },
    {
      ms: 6500, lane: '3 · Leader lost', call: 'client.poll(&mut listener, limit)',
      leader: null, dead: 0, ingressTo: 0, egressFrom: null, state: 'awaiting', fields: F1,
      deadline: { label: `awaiting_leader_since -> ${NLT}`, from: 0, to: 0.45 },
      caption: 'Member 0 dies. The next poll sees the bound egress image close and moves to AwaitingNewLeader, starting a clock; an offer refused NotConnected or Closed does the same. The old publication stays, unconnected. Keep-alives stop and offer() returns NotConnected. With no NewLeaderEvent before new_leader_timeout, poll returns Disconnected and the session is Closed.',
    },
    {
      ms: 6500, lane: '4 · NewLeaderEvent', call: 'Fragment::NewLeader { leader_member_id: 1, .. }',
      leader: 1, dead: 0, ingressTo: 0, egressFrom: 1, state: 'awaiting', fields: F1,
      token: { dir: 'out', member: 1, label: 'NewLeader' }, wire: `term T', leader 1, "${ENDPOINTS}"`,
      deadline: { label: `awaiting_leader_since -> ${NLT}`, from: 0.45, to: 0.6 },
      caption: 'The survivors elect member 1, which sends NewLeaderEvent: new term, leader member id 1, and every member\'s ingress endpoint in id order. The fragment handler only records it; apply_polled applies it after the fragments are read, still inside the same poll() call, and the last one in a poll wins.',
    },
    {
      ms: 7000, lane: '4 · Re-add ingress (same poll)', call: 'parse_leader_endpoint(endpoints, 1)',
      leader: 1, dead: 0, ingressTo: 0, pendingTo: 1, egressFrom: 1, state: 'awaiting',
      fields: `${F1}  (pending: T', 1)`, hotMember: 1, wire: 'picks 1=m1:9002, not the first entry',
      deadline: { label: `restarted: new publication -> ${MT}`, from: 0, to: 0.4 },
      caption: 'It takes the entry for member 1, not the first one (that is the dead leader), and async-adds an ingress publication there, keeping every other channel setting. Egress rebinds to the new image and the clock restarts, now bounded by message_timeout. Term, leader and the old publication stay until the new one connects. Nothing blocks.',
    },
    {
      ms: 6000, lane: '4 · Connected again', call: 'poll_pending_ingress()',
      leader: 1, dead: 0, ingressTo: 1, egressFrom: 1, state: 'connected', fields: F2, fieldsChanged: true, hotWire: 'in',
      caption: 'A later poll finds the publication added and connected. It swaps it in, adopts term T\' and leader 1, and returns to Connected. Had it not connected within message_timeout, poll would return Disconnected and the session would be Closed. Had the new leader\'s egress image closed first, the client would drop the pending publication and wait for another NewLeaderEvent, again bounded by new_leader_timeout.',
    },
    {
      ms: 6000, lane: '5 · Cluster closes the session', call: 'client.poll(..)  // apply_polled',
      leader: 1, dead: 0, ingressTo: 1, egressFrom: 1, state: 'pending', fields: F2,
      token: { dir: 'out', member: 1, label: 'CLOSED' }, wire: 'SessionEvent(CLOSED, session S)',
      caption: 'SessionEvent(CLOSED) for this session: once the fragments are read, apply_polled sets PendingClose from Connected or AwaitingNewLeader and drops any pending publication, before a NewLeaderEvent in the same poll is applied, so that cannot undo it.',
    },
    {
      ms: 7000, lane: '5 · Closed', call: 'poll_state()  // same poll',
      leader: 1, dead: 0, ingressTo: 1, egressFrom: 1, state: 'closed', fields: F2,
      caption: 'Still in that poll, poll_state moves PendingClose to Closed, the terminal state, so poll returns with the session Closed. PendingClose stays until the next poll only after an ingress refusal of MaxPositionExceeded outside poll (offer, claim or keep-alive; a full publication cannot carry a close request), or when a decode error ends that poll early. close() while Connected sends a best-effort SessionCloseRequest (up to 3 tries) and goes straight to Closed. Publication and subscription are freed when the client is dropped.',
    },
    {
      ms: 8000, lane: 'Takeaway', call: 'Connected sends; the other states wait',
      leader: 1, dead: 0, ingressTo: 1, egressFrom: 1, state: null, fields: F2,
      wire: 'per message +32 B header · keep-alive 24 B · send path: no heap',
      caption: 'Wins: Connected only adds a 32-byte session header per message and a 24-byte keep-alive per interval, without heap allocation; failover and close are cold paths that never block poll. Loses: from leader loss until the new publication connects, offers fail with NotConnected; the session closes if no NewLeaderEvent comes within new_leader_timeout, or its publication does not connect within message_timeout. This book records no failover timings.',
    },
  ];
  for (const s of states) if (s.dead === undefined) s.dead = null;

  build(false);

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
