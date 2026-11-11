// NRI shared CPUs: how helper threads get a CPU without touching the exclusive ones.
//
// Sources:
//   k8s/nri-shared-cpus/internal/assign/assign.go:41   Need: 1 up to 1000 m, else (m + 999) / 1000
//   k8s/nri-shared-cpus/internal/assign/assign.go:50   Split: m / n each, the lowest m % n CPUs get one more
//   k8s/nri-shared-cpus/internal/assign/assign.go:67   Load: sum of Split over every holder
//   k8s/nri-shared-cpus/internal/assign/assign.go:86   Choose: sticky set if same size and still shared,
//                                                      else sort by load, ties to the lower CPU, take k
//   k8s/nri-shared-cpus/internal/assign/assign.go:106  Hold: byID[id] and sticky[key]
//   k8s/nri-shared-cpus/internal/assign/assign.go:112  Release: deletes byID only, sticky stays
//   k8s/nri-shared-cpus/internal/assign/assign.go:119  ForgetPod: deletes the pod's sticky sets
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:330  CreateContainer: Choose then Hold under p.mu
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:340  cpuset = exclusive union chosen shared
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:363  createRuntime hook gets the chosen set
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:812  StopContainer / RemoveContainer: release
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:835  RemovePodSandbox: ForgetPod
//   k8s/nri-shared-cpus/internal/nriplugin/plugin.go:668  Synchronize: new table from live containers only,
//                                                         so a plugin restart also drops a stopped container's set
//   k8s/nri-shared-cpus/internal/hook/hook.go:165      Run: partition, then timer slack, then pin; runs on the
//                                                      container's init before exec, so app threads do not exist yet
//   k8s/nri-shared-cpus/internal/hook/hook.go:843      SetTimerSlack: one task scan, one write per tid, one read-back
//   k8s/nri-shared-cpus/internal/hook/hook.go:896      PinProcess: MaxPasses = 5 passes; each pass re-pins every
//                                                      live tid (newTIDs counts all of them, so the early break
//                                                      only fires when every thread has gone), then one getaffinity
//   k8s/nri-shared-cpus/examples/pin.c:34              pin_isolated(i): index i of ERGON_ISOLATED_CPUS
//   book/src/deploy/shared-cpus.md:194                 timer slack table, EKS c8g.xlarge, idle rows
// The 8-CPU node, the pods and their millicpus are illustrative; the selection
// follows assign.go exactly. Costs are operation counts from the code, not
// timings. Latency figures are the page's recorded EKS c8g.xlarge results.
Anim.register('shared-cpus-allocation', (stage, api) => {
  const el = api.el;
  const SHARED = [1, 2];
  const POD_CLS = { A: 'ea-k1', B: 'ea-k2', C: 'ea-k4' };
  const MAX_M = 1300; // bar scale; CPU 1 peaks at 1,250 m
  const WIDE = {
    W: 560, cols: 8, cellW: 40, gap: 4, cellH: 40, gridTop: 54, brackets: true,
    left: 352, callFont: 12, subFont: 9, subChars: 100, stickyChars: 64, costChars: 72, panelX: 372, panelY: 54, panelW: 188,
  };
  const NARROW = {
    W: 300, cols: 4, cellW: 71, gap: 5, cellH: 38, gridTop: 56, brackets: false,
    left: 300, callFont: 11, subFont: 8.5, subChars: 56, stickyChars: 54, costChars: 56, panelX: 0, panelW: 300,
  };

  let g;
  let svg;
  let cells;
  let chips;
  let segs;
  let sums;
  let txt;
  let rows;
  let barX;
  let barS;

  // Every highlighted rect here carries a label, and the runtime's .ea-hot fill
  // (0.6) puts that label under 3:1 on coal, navy and ayu; keep the outline only.
  function hot(rect, on) {
    rect.classList.toggle('ea-hot', on);
    rect.style.fillOpacity = on ? '0.22' : '';
  }

  // shortcut: wraps by character count, not measured width; fine for these short lines.
  function wrap(node, text, maxChars, lineH) {
    node.textContent = '';
    const x = node.getAttribute('x');
    let line = '';
    let first = true;
    const flush = () => {
      el('tspan', { x, dy: first ? 0 : lineH }, node, line);
      first = false;
    };
    for (const word of text.split(' ')) {
      if (line && (line + ' ' + word).length > maxChars) { flush(); line = word; } else line = line ? `${line} ${word}` : word;
    }
    if (line) flush();
  }

  function cellXY(cpu) {
    const col = cpu % g.cols;
    const row = Math.floor(cpu / g.cols);
    return { x: col * (g.cellW + g.gap), y: g.gridTop + row * (g.cellH + g.gap) };
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    const gridRows = 8 / g.cols;
    const gridBottom = g.gridTop + gridRows * (g.cellH + g.gap) - g.gap;
    const threadTop = gridBottom + 6;
    const loadTop = threadTop + 3 * 16 + 14;
    const stickyY = loadTop + 76;
    const costY = stickyY + 28;
    const panelY = narrow ? costY + 34 : g.panelY;
    const H = narrow ? panelY + 194 : Math.max(costY + 26, panelY + 196);
    svg = api.svg(g.W, H, 'Node CPUs, the shared-CPU load table and the timer-slack results');
    txt = {};
    txt.call = el('text', { x: 0, y: 14, 'font-size': g.callFont, class: 'ea-strong' }, svg, '');
    txt.sub = el('text', { x: 0, y: 29, 'font-size': g.subFont, class: 'ea-mono' }, svg, '');

    if (g.brackets) {
      const bracket = (from, to, label) => {
        const x0 = cellXY(from).x;
        const x1 = cellXY(to).x + g.cellW;
        el('path', { d: `M${x0} ${g.gridTop - 3}v-4H${x1}v4`, fill: 'none', stroke: 'currentColor', 'stroke-opacity': 0.5 }, svg);
        el('text', { x: (x0 + x1) / 2, y: g.gridTop - 10, 'font-size': 8.5, 'text-anchor': 'middle', class: 'ea-muted' }, svg, label);
      };
      bracket(0, 2, 'reservedSystemCPUs 0-2');
      bracket(3, 7, 'static CPU manager pool');
    }

    cells = new Map();
    for (let cpu = 0; cpu < 8; cpu++) {
      const { x, y } = cellXY(cpu);
      const rect = el('rect', { x, y, width: g.cellW, height: g.cellH, rx: 3, class: 'ea-box' }, svg);
      el('text', { x: x + g.cellW / 2, y: y + 15, 'font-size': 9.5, 'text-anchor': 'middle', class: 'ea-strong' }, svg, `CPU ${cpu}`);
      const role = el('text', { x: x + g.cellW / 2, y: y + 30, 'font-size': 8.5, 'text-anchor': 'middle' }, svg, '');
      cells.set(cpu, { rect, role });
    }

    // Pod A's threads; positioned by CSS transform so a move animates.
    chips = new Map();
    for (const name of ['main', 'conductor', 'hot']) {
      const grp = el('g', { class: 'ea-hidden' }, svg);
      hot(el('rect', { x: 0, y: 0, width: g.cellW - 2, height: 14, rx: 7, class: 'ea-k1' }, grp), name === 'hot');
      const label = el('text', { x: (g.cellW - 2) / 2, y: 10, 'font-size': 8, 'text-anchor': 'middle', class: 'ea-mono' }, grp, '');
      chips.set(name, { grp, label, threadTop });
    }

    // Load table: one bar per shared CPU, stacked by holder.
    el('text', { x: 0, y: loadTop, 'font-size': 10, class: 'ea-strong' }, svg, 'Load table (millicpus, accounting only)');
    barX = 44;
    barS = (g.left - barX - 48) / MAX_M;
    rows = new Map();
    SHARED.forEach((cpu, i) => {
      const y = loadTop + 10 + i * 22;
      el('text', { x: 0, y: y + 12, 'font-size': 9.5, class: 'ea-mono' }, svg, `CPU ${cpu}`);
      el('rect', { x: barX, y, width: MAX_M * barS, height: 16, rx: 2, class: 'ea-box', 'fill-opacity': 0.4 }, svg);
      rows.set(cpu, y);
    });
    const tickX = barX + 1000 * barS;
    el('path', { d: `M${tickX} ${loadTop + 6}V${loadTop + 52}`, stroke: 'currentColor', 'stroke-dasharray': '3 2', 'stroke-opacity': 0.7 }, svg);
    el('text', { x: tickX, y: loadTop + 62 - 1, 'font-size': 8, 'text-anchor': 'middle', class: 'ea-muted' }, svg, '1,000 m = 1 CPU of accounting');
    segs = new Map();
    for (const pod of ['A', 'B', 'C']) {
      for (const cpu of SHARED) {
        const grp = el('g', { class: 'ea-hidden' }, svg);
        const rect = el('rect', { x: 0, y: rows.get(cpu), width: 1, height: 16, class: POD_CLS[pod], 'vector-effect': 'non-scaling-stroke' }, grp);
        const label = el('text', { x: 0, y: rows.get(cpu) + 11.5, 'font-size': 8, 'text-anchor': 'middle', class: 'ea-mono' }, grp, '');
        segs.set(`${pod}${cpu}`, { grp, rect, label });
      }
    }
    sums = new Map(SHARED.map((cpu) => [cpu, el('text', { x: 0, y: rows.get(cpu) + 12, 'font-size': 9, class: 'ea-mono ea-strong' }, svg, '')]));

    txt.sticky = el('text', { x: 0, y: stickyY, 'font-size': 9, class: 'ea-mono' }, svg, '');
    txt.cost = el('text', { x: 0, y: costY, 'font-size': 9, class: 'ea-muted' }, svg, '');

    buildPanel(g.panelX, panelY, g.panelW);
  }

  function buildPanel(x, y, w) {
    el('rect', { x, y, width: w, height: 190, rx: 4, class: 'ea-box', 'fill-opacity': 0.5 }, svg);
    const px = x + 8;
    el('text', { x: px, y: y + 15, 'font-size': 10, class: 'ea-strong' }, svg, 'Timer slack, idle shared CPU');
    el('text', { x: px, y: y + 28, 'font-size': 8.5, class: 'ea-muted' }, svg, 'EKS c8g.xlarge, 1 ms sleeps, µs late');
    el('text', { x: px, y: y + 39, 'font-size': 8.5, class: 'ea-muted' }, svg, 'n 15,000 a cell; min not recorded');
    const barMax = w - 70;
    const scale = barMax / 60; // 60 µs full width
    const block = (top, title, cls, p50, p99, p9999, max) => {
      const frame = el('rect', { x: x + 3, y: top - 11, width: w - 6, height: 55, rx: 3, fill: 'none', stroke: 'none' }, svg);
      el('text', { x: px, y: top, 'font-size': 9, class: 'ea-strong' }, svg, title);
      el('rect', { x: px, y: top + 5, width: Math.max(2, p50 * scale), height: 10, rx: 2, class: cls }, svg);
      el('text', { x: px + Math.max(2, p50 * scale) + 4, y: top + 13.5, 'font-size': 8.5, class: 'ea-mono' }, svg, `p50 ${p50}`);
      el('text', { x: px, y: top + 27, 'font-size': 8.5, class: 'ea-mono' }, svg, `p99 ${p99}  p99.99 ${p9999}`);
      el('text', { x: px, y: top + 39, 'font-size': 8.5, class: 'ea-mono ea-strong' }, svg, `max ${max}`);
      return frame;
    };
    txt.slow = block(y + 60, '50 µs slack (kernel default)', 'ea-k5', 52.6, '56.8-57.8', '1,390-2,907', '2,447-3,265');
    txt.fast = block(y + 118, '1 ns slack (the hook)', 'ea-k3', 2.6, '8.5', '1,343', '3,369');
    el('text', { x: px, y: y + 175, 'font-size': 8, class: 'ea-muted' }, svg, 'Maxima unexplained: slack and CPU ruled out,');
    el('text', { x: px, y: y + 184, 'font-size': 8, class: 'ea-muted' }, svg, 'node daemons on CPU 0 not.');
  }

  function render(s) {
    txt.call.textContent = s.call;
    wrap(txt.sub, s.sub, g.subChars, 11);
    wrap(txt.sticky, `held: ${Object.keys(s.load).join(', ') || 'none'} · sticky: ${s.sticky || 'none'}`, g.stickyChars, 11);
    wrap(txt.cost, `Cost: ${s.cost}`, g.costChars, 11);

    for (const [cpu, c] of cells) {
      let cls = 'ea-box';
      let role = 'free';
      if (cpu === 0) role = 'IRQ';
      else if (SHARED.includes(cpu)) { cls = 'ea-k3'; role = 'shared'; } else if (s.excl[cpu]) { cls = POD_CLS[s.excl[cpu]]; role = `${s.excl[cpu]} excl`; }
      c.rect.setAttribute('class', cls);
      hot(c.rect, (s.hot || []).includes(cpu));
      c.rect.classList.toggle('ea-ok', (s.pick || []).includes(cpu));
      c.role.textContent = role;
    }

    const slots = {};
    for (const [name, chip] of chips) {
      const cpu = s.threads ? s.threads[name] : undefined;
      chip.grp.classList.toggle('ea-hidden', cpu === undefined);
      if (cpu === undefined) continue;
      const slot = slots[cpu] || 0;
      slots[cpu] = slot + 1;
      const { x } = cellXY(cpu);
      chip.grp.style.transform = `translate(${x + 1}px, ${chip.threadTop + slot * 16}px)`;
      chip.label.textContent = g === NARROW ? `${name === 'conductor' ? 'cond' : name} @${cpu}` : (name === 'conductor' ? 'cond' : name);
    }

    for (const cpu of SHARED) {
      let off = 0;
      for (const pod of ['A', 'B', 'C']) {
        const m = (s.load[pod] && s.load[pod][cpu]) || 0;
        const seg = segs.get(`${pod}${cpu}`);
        seg.grp.classList.toggle('ea-hidden', m === 0);
        hot(seg.rect, (s.grow || []).includes(`${pod}${cpu}`));
        if (m > 0) {
          seg.rect.style.transform = `translateX(${barX + off * barS}px) scaleX(${m * barS})`;
          seg.label.style.transform = `translateX(${barX + (off + m / 2) * barS}px)`;
          seg.label.textContent = `${pod} ${m}`;
        }
        off += m;
      }
      const sum = sums.get(cpu);
      sum.style.transform = `translateX(${barX + off * barS + 4}px)`;
      sum.textContent = `${off.toLocaleString('en-GB')} m`;
    }
    txt.slow.classList.toggle('ea-ok', s.panel === 2);
    txt.fast.classList.toggle('ea-ok', s.panel >= 1);
  }

  const EXCL_A = { 3: 'A', 4: 'A' };
  const EXCL_AB = { 3: 'A', 4: 'A', 5: 'B', 6: 'B' };
  const EXCL_ABC = { ...EXCL_AB, 7: 'C' };
  const EXCL_BC = { 5: 'B', 6: 'B', 7: 'C' };
  const L_A = { A: { 1: 250 } };
  const L_AB = { A: { 1: 250 }, B: { 1: 750, 2: 750 } };
  const L_B = { B: { 1: 750, 2: 750 } };
  const L_BC = { B: { 1: 750, 2: 750 }, C: { 1: 250 } };
  const L_ABC = { A: { 1: 250 }, B: { 1: 750, 2: 750 }, C: { 1: 250 } };
  const INIT_ONLY = { main: 1 };
  const HOT_PINNED = { main: 1, conductor: 1, hot: 3 };

  const states = [
    {
      ms: 6500, call: 'An illustrative 8-CPU node', excl: {}, load: {},
      sub: 'sharedCpus "1,2" · oversubscription 1 · capacity 2 × 1000 = 2,000 m',
      cost: 'none yet. The plugin acts only on container and pod events; the busy loop never calls it.',
      caption: 'CPU 0 takes NIC interrupts; CPUs 1 and 2 are the shared CPUs. All three are in reservedSystemCPUs. CPUs 3 to 7 are the static CPU manager\'s pool: the kubelet hands those out whole, never the plugin.',
    },
    {
      ms: 5000, call: 'Pod A: cpu: 2, shared-millicpus: 250', excl: EXCL_A, load: {}, hot: [3, 4],
      sub: 'Need(250) = 1, since 250 ≤ 1000',
      cost: 'Need: one comparison. No table access yet.',
      caption: 'Pod A asks for 2 whole CPUs and 250 m of shared time. The kubelet gives it exclusive CPUs 3 and 4. The plugin works out how many shared CPUs that is: up to 1,000 m is one.',
    },
    {
      ms: 5500, call: 'Choose(A, 1)', excl: EXCL_A, load: {}, pick: [1],
      sub: 'sticky[A]: miss → Load = {1: 0, 2: 0} → sort → [1, 2] → take 1',
      cost: 'one sticky lookup, Load over 0 holders, a sort of 2 CPUs: 1 comparison.',
      caption: 'A has no remembered set, so Choose sums the load table and sorts the shared CPUs by load, ties to the lower number. Both carry 0, so CPU 1 wins.',
    },
    {
      ms: 6000, call: 'Hold(A, {1}, 250)', excl: EXCL_A, load: L_A, sticky: 'A→{1}', grow: ['A1'],
      sub: 'cpuset.cpus = {3,4} ∪ {1} = 1,3-4 · ERGON_SHARED_CPUS=1 · ERGON_ISOLATED_CPUS=3,4',
      cost: 'two map writes (byID, sticky) under the plugin\'s lock.',
      caption: 'Hold books A\'s 250 m on CPU 1 and remembers {1} for A. The container\'s cgroup gets exclusive plus shared, 1,3-4, and the env names the two lists separately.',
    },
    {
      ms: 6000, call: 'Pod B: cpu: 2, shared-millicpus: 1,500', excl: EXCL_AB, load: L_A, sticky: 'A→{1}', hot: [5, 6], pick: [1, 2],
      sub: 'Need(1500) = (1500 + 999) / 1000 = 2 → sort → [2 (0 m), 1 (250 m)] → take 2',
      cost: 'sticky miss, Load over 1 holder, 1 comparison. Need 3 here would be refused (too-many).',
      caption: 'Pod B gets exclusive CPUs 5 and 6. Its 1,500 m needs 2 shared CPUs. The sort puts the idle CPU 2 first, but B needs both, so it gets {1, 2}.',
    },
    {
      ms: 7000, call: 'Hold(B, {1,2}, 1500)', excl: EXCL_AB, load: L_AB, sticky: 'A→{1}  B→{1,2}', grow: ['B1', 'B2'],
      sub: 'Split(1500, {1,2}) = {1: 750, 2: 750}',
      cost: 'two map writes. Load sums Split per holder.',
      caption: 'Split gives each of B\'s CPUs 750 m, so CPU 1 reads 1,000 m. That is accounting, not a cap: A and B may each use all of CPU 1, and when both are busy cgroup cpu.weight (from cpu:) divides it, not M.',
    },
    {
      ms: 7000, call: 'OCI hook for A (createRuntime)', excl: EXCL_AB, load: L_AB, sticky: 'A→{1}  B→{1,2}', threads: INIT_ONLY, pick: [1], panel: 1,
      sub: 'SetTimerSlack(pid, 1 ns) then PinProcess(pid, {1})',
      cost: 'per tid: 1 slack write, then 1 sched_setaffinity on each of 5 passes; 6 task scans, 2 read-backs.',
      caption: 'Before A\'s program starts, the hook works on the container\'s init, which later execs into main. It sets 1 ns timer slack, pins every thread to A\'s shared set {1} over five passes, and checks the mask. Any failure stops the create.',
    },
    {
      ms: 6500, call: 'pin_isolated(0) in the app', excl: EXCL_AB, load: L_AB, sticky: 'A→{1}  B→{1,2}', threads: HOT_PINNED, hot: [3], panel: 1,
      sub: 'ERGON_ISOLATED_CPUS = 3,4 → index 0 → CPU 3',
      cost: 'one sched_setaffinity by the app; the plugin takes no part.',
      caption: 'Main\'s new threads copy its mask {1} and its 1 ns slack. The hot thread then pins itself by index, so it never needs real CPU numbers; the conductor stays on CPU 1. Create helpers before pinning: a new thread copies its creator\'s mask.',
    },
    {
      ms: 5500, call: 'A\'s container stops: Release(A)', excl: EXCL_AB, load: L_B, sticky: 'A→{1}  B→{1,2}',
      sub: 'byID[A] deleted · sticky[A] kept',
      cost: 'one map delete.',
      caption: 'Release drops A\'s 250 m, so CPU 1 falls to 750 m. A\'s remembered set stays. CPUs 3 and 4 stay drawn as A\'s: the kubelet\'s choice, not the plugin\'s.',
    },
    {
      ms: 6000, call: 'Pod C: cpu: 1, shared-millicpus: 250', excl: EXCL_ABC, load: L_BC, sticky: 'A→{1}  B→{1,2}  C→{1}', pick: [1], grow: ['C1'],
      sub: 'Need = 1 → Load = {1: 750, 2: 750} → tie → CPU 1',
      cost: 'sticky miss, Load over 1 holder, 1 comparison, two map writes.',
      caption: 'Pod C takes exclusive CPU 7. Both shared CPUs carry 750 m and the tie goes to the lower number, so C lands on CPU 1: now 1,000 m against 750 m on CPU 2.',
    },
    {
      ms: 7500, call: 'A restarts: Choose(A, 1)', excl: EXCL_ABC, load: L_ABC, sticky: 'A→{1}  B→{1,2}  C→{1}', pick: [1], grow: ['A1'],
      sub: 'sticky[A] = {1}: size 1 = Need, ⊆ shared → return {1}, no sort',
      cost: 'one lookup and a subset check; no Load, no sort. Then two map writes.',
      caption: 'The restart has a new container ID but the same key: pod UID and container name. Choose\'s first branch returns the remembered {1}, although CPU 2 is lighter. No thread exists yet; the hook will start the new init on CPU 1 again.',
    },
    {
      ms: 6000, call: 'Pod A deleted: ForgetPod(A)', excl: EXCL_BC, load: L_BC, sticky: 'B→{1,2}  C→{1}',
      sub: 'StopContainer → Release · RemovePodSandbox → ForgetPod',
      cost: 'one delete, then one pass over the remembered sets.',
      caption: 'Deleting the pod releases its load and ForgetPod drops its remembered set. Only this, or a plugin restart (Synchronize rebuilds the table from live containers), clears it. The kubelet returns CPUs 3 and 4 to the pool.',
    },
    {
      ms: 9000, call: 'Why it wins here, and when it loses', excl: EXCL_BC, load: L_BC, sticky: 'B→{1,2}  C→{1}', panel: 2,
      sub: 'Wins: isolated loop CPUs, helpers on shared ones. Loses: M is no cap.',
      cost: 'counts are from the code, not timings; latency figures are the page\'s EKS run.',
      caption: 'The loop keeps whole isolated CPUs while helpers share the housekeeping ones, and 1 ns slack cut a sleeping helper\'s p50 lateness from 52.6 to 2.6 µs. It loses on fairness: M is no cap (100 m used 0.99 of a core), and the 3.4 ms max stayed.',
    },
  ];

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
