// CPU isolation: what each boot flag removes from the busy loop's CPU.
//
// Sources:
//   book/src/deploy/hft.md:78    Isolation table: gaps >5/>20/>100 µs a minute and the longest gap,
//                                runs flags-aws-20261014T163114Z and flags-azure-20261014T144146Z,
//                                one 120 s sample per boot line (so .5 values are two-minute averages)
//   book/src/deploy/hft.md:31    what the boot line keeps off the busy CPUs: tasks, tick, RCU, new IRQs
//   tune/src/cpu.rs:151          T1_EVIDENCE (the same AWS run)
//   tune/src/cpu.rs:195          T1 "is not isolated": the scheduler balances other tasks onto it
//   tune/src/cpu.rs:230          T1 "is not nohz_full": 1,002 local-timer interrupts a second, 0 with it
//   tune/src/cpu.rs:250          T1 ok line once isolated, nohz_full and outside default_smp_affinity
//   tune/src/cpu.rs:289          T1 default_smp_affinity (what irqaffinity sets)
//   tune/src/cpu.rs:326          the fix's CPUs: the spinning ones plus those already isolated
//   tune/src/cpu.rs:331          the boot line T1 recommends
//   tune/src/noise.rs:322        T15: other processes' tasks on a busy CPU (per-CPU kernel threads are not)
//   tune/src/finding.rs:130      how a finding prints: `T1 warn: <message> (evidence: ...)`
//   tune/src/bin/ergon-tune.rs:178  ok lines print only with --all
// Counts and maxima are the page's; the cause of each max, the kworker and
// timer-softirq stalls and the IPIs come from the runs' own traces. The T1
// lines are ergon-tune's real output for the AWS run's isolcpus boot (CPUs
// 1-3,5-7 isolated, irqaffinity=0,4), cut short with "…". Where a gap falls
// along the strip is illustrative. The runtime has no redraw hook for an
// in-stage control, so the host toggle redraws and rewrites the caption here.
Anim.register('isolation-noise', (stage, api) => {
  const el = api.el;
  const BELOW = 'no kernel event (hypervisor or in-kernel time)';
  const HOSTS = {
    aws: {
      name: 'AWS c7i', other: 'Azure', run: 'flags-aws-20261014T163114Z', scale: 150,
      rows: [
        { g5: '60,120', g20: '120.0', g100: '34.5', max: '146.95', cause: 'softirq TIMER' },
        { g5: '60,063.5', g20: '4.0', g100: '0.0', max: '52.30', cause: BELOW },
        { g5: '50.0', g20: '4.5', g100: '1.0', max: '133.90', cause: BELOW },
      ],
    },
    azure: {
      name: 'Azure D8ds_v6', other: 'AWS c7i', run: 'flags-azure-20261014T144146Z', scale: 620,
      rows: [
        { g5: '17,189', g20: '120.0', g100: '3.0', max: '612.94', cause: BELOW },
        { g5: '10,656', g20: '47.0', g100: '0.0', max: '58.94', cause: BELOW },
        { g5: '239.5', g20: '37.0', g100: '0.0', max: '67.82', cause: BELOW },
      ],
    },
  };
  const BOOTS = [
    ['stock boot line: no isolation flags', ''],
    ['isolcpus=managed_irq,domain,<busy>', 'irqaffinity=<housekeeping>'],
    ['isolcpus=managed_irq,domain,<busy> irqaffinity=<hk>', '+ nohz_full=<busy> rcu_nocbs=<busy>'],
  ];
  const SOURCES = ['other processes\' tasks', 'device interrupts', 'scheduler tick', 'RCU callbacks', 'IPIs, unexplained stalls'];
  const MOVED = ['isolcpus', 'isolcpus, irqaffinity', 'nohz_full', 'rcu_nocbs', null];
  const LANES = [['g5', '>5 µs'], ['g20', '>20 µs'], ['g100', '>100 µs']];
  const POOL = { g5: 240, g20: 120, g100: 35 };
  // One scale per lane per host, from its stock row, so fewer gaps always means fewer lines.
  const nice = (x) => { const p = 10 ** Math.floor(Math.log10(x)); return [1, 2, 5, 10].map((m) => m * p).find((v) => v >= x); };
  const perLine = (key) => { const stock = num(HOSTS[host].rows[0][key]); return stock <= POOL[key] ? 1 : nice(stock / POOL[key]); };
  const WIDE = { W: 400, X0: 74, pitch: 30, top: 50, above: false, chars: 74, mono: 70, chipCols: 2 };
  const NARROW = { W: 300, X0: 0, pitch: 40, top: 50, above: true, chars: 56, mono: 52, chipCols: 2 };

  const num = (s) => Number(s.replace(/,/g, ''));
  // Fixed illustrative positions, so a mark stays put from step to step.
  let seed = 7;
  const rand = () => { seed = (seed * 16807) % 2147483647; return (seed - 1) / 2147483646; };
  const SPOT = {};
  for (const [key] of LANES) SPOT[key] = Array.from({ length: POOL[key] }, rand);

  let host = 'aws';
  let current = null;
  let g;
  let svg;
  let parts;

  function text(x, y, size, cls, s, parent) {
    return el('text', { x, y, 'font-size': size, class: cls || '' }, parent || svg, s);
  }

  function wrap(s, chars) {
    const out = [];
    let line = '';
    for (const word of s.split(' ')) {
      if (line && line.length + 1 + word.length > chars) { out.push(line); line = word; }
      else line = line ? `${line} ${word}` : word;
    }
    if (line) out.push(line);
    return out;
  }

  function build(narrow) {
    g = narrow ? NARROW : WIDE;
    stage.textContent = '';
    const toggle = document.createElement('button');
    toggle.type = 'button';
    toggle.className = 'ea-btn';
    toggle.style.marginBottom = '0.4em';
    toggle.setAttribute('aria-controls', `${api.id}-caption`);
    toggle.addEventListener('click', () => {
      host = host === 'aws' ? 'azure' : 'aws';
      if (current) render(current);
      const cap = stage.parentNode && stage.parentNode.querySelector('.ea-caption');
      if (cap && current) cap.textContent = current.caption();
    });
    stage.appendChild(toggle);

    const SW = g.W - g.X0;
    const laneY = (i) => g.top + i * g.pitch + (g.above ? 12 : 0);
    const yAxis = laneY(2) + 18 + 11;
    const yMax = yAxis + (g.above ? 24 : 20);
    const yChips = yMax + 56;
    const chipW = g.W / g.chipCols;
    const chipRows = Math.ceil(SOURCES.length / g.chipCols);
    const yPanel = yChips + 14 + chipRows * 34 + 10;
    const H = yPanel + (g.above ? 140 : 116);
    svg = api.svg(g.W, H, 'Gaps on the busy loop CPU for one minute, per boot line');    parts = { toggle, lanes: {}, chips: [], laneY, SW, yPanel };

    parts.title = text(0, 13, 12, 'ea-strong', '');
    parts.boot1 = text(0, 27, 9, 'ea-mono', '');
    parts.boot2 = text(0, 39, 9, 'ea-mono', '');

    for (const [i, [key, name]] of LANES.entries()) {
      const y = laneY(i);
      const lane = {};
      if (g.above) {
        text(0, y - 4, 10, 'ea-strong', name);
        lane.count = text(48, y - 4, 9, 'ea-mono', '');
      } else {
        text(0, y + 8, 10.5, 'ea-strong', name);
        lane.count = text(0, y + 20, 8.5, 'ea-mono', '');
      }
      lane.note = text(g.W, y - 3, 8, 'ea-muted', '');
      lane.note.setAttribute('text-anchor', 'end');
      lane.bg = el('rect', { x: g.X0, y, width: SW, height: 18, rx: 2, class: 'ea-box' }, svg);
      lane.marks = [];
      for (let j = 0; j < POOL[key]; j += 1) {
        const m = el('line', {
          x1: g.X0 + 0.5, x2: g.X0 + 0.5, y1: y + 2, y2: y + 16,
          'stroke-width': 0.9, class: 'ea-hidden',
        }, svg);
        m.style.stroke = 'var(--ea-c2)';
        lane.marks.push(m);
      }
      parts.lanes[key] = lane;
    }
    for (const [t, label] of [[0, '0 s'], [0.5, '30 s'], [1, '60 s']]) {
      text(g.X0 + t * SW, yAxis, 8.5, 'ea-muted', label).setAttribute('text-anchor', t === 0 ? 'start' : t === 1 ? 'end' : 'middle');
    }

    // Longest gap: a bar scaled to the host's stock maximum.
    text(0, yMax + (g.above ? -3 : 8), 10, 'ea-strong', 'longest');
    parts.maxBar = el('rect', { x: g.X0, y: yMax, width: SW, height: 9, rx: 2, class: 'ea-k2' }, svg);
    parts.maxBar.style.transformBox = 'fill-box';
    parts.maxBar.style.transformOrigin = 'left center';
    parts.max1 = text(g.X0, yMax + 21, 9, 'ea-mono', '');
    parts.max2 = text(g.X0, yMax + 32, 8.5, 'ea-mono', '');
    parts.max3 = text(g.X0, yMax + 43, 8.5, 'ea-muted ea-mono', '');

    text(0, yChips + 4, 10, 'ea-strong', 'What runs on the loop\'s CPU');
    for (const [i, name] of SOURCES.entries()) {
      const x = (i % g.chipCols) * chipW;
      const y = yChips + 12 + Math.floor(i / g.chipCols) * 34;
      const chip = {};
      chip.rect = el('rect', { x: x + 1, y, width: chipW - 6, height: 28, rx: 4, class: 'ea-k2' }, svg);
      chip.name = text(x + 7, y + 12, 9.5, 'ea-strong', name);
      chip.state = text(x + 7, y + 23, 8.5, 'ea-muted', '');
      chip.strike = el('line', { x1: x + 5, x2: x + chipW - 9, y1: y + 9, y2: y + 9, 'stroke-width': 1.5, class: 'ea-hidden' }, svg);
      chip.strike.style.stroke = 'var(--fg)';
      parts.chips.push(chip);
    }
    parts.panel = el('g', {}, svg);
  }

  function lanes(row, prev) {
    for (const [key] of LANES) {
      const lane = parts.lanes[key];
      const value = row ? num(row[key]) : 0;
      const per = perLine(key);
      const shown = Math.min(POOL[key], per === 1 ? Math.round(value) : Math.ceil(value / per));
      lane.marks.forEach((m, j) => {
        const t = per === 1 ? SPOT[key][j] : (j + 0.5) / shown;
        if (j < shown) m.style.transform = `translateX(${(t * (parts.SW - 1)).toFixed(1)}px)`;
        m.classList.toggle('ea-hidden', j >= shown);
      });
      // Highlight only a real move, not 60,120 → 60,063.5.
      const a = row ? num(row[key]) : 0;
      const b = prev ? num(prev[key]) : 0;
      const changed = Boolean(row && prev && Math.abs(a - b) >= 0.2 * Math.max(a, b));
      lane.bg.classList.toggle('ea-hot', changed);
      lane.count.classList.toggle('ea-strong', changed);
      lane.count.textContent = row ? `${row[key]}/min` : '–';
      lane.note.textContent = per > 1 ? `1 line ≈ ${per.toLocaleString('en-GB')} gaps` : '';
    }
  }

  function panelText(lines, y, mono) {
    for (const [cls, s] of lines) {
      for (const line of wrap(s, mono ? g.mono : g.chars)) {
        text(0, y, mono ? 8.2 : 9.5, cls, line, parts.panel);
        y += mono ? 10.5 : 12;
      }
    }
    return y;
  }

  function panel(kind) {
    parts.panel.textContent = '';
    const y0 = parts.yPanel;
    if (kind === 'zoom') {
      if (host === 'aws') {
        text(0, y0, 10, 'ea-strong', 'Zoom: 10 ms of the strip, isolcpus boot', parts.panel);
        const w = g.W - 20;
        el('rect', { x: 0, y: y0 + 8, width: w, height: 18, rx: 2, class: 'ea-box' }, parts.panel);
        for (let i = 0; i < 10; i += 1) {
          const x = ((i + 0.5) / 10) * w;
          const m = el('line', { x1: x, x2: x, y1: y0 + 10, y2: y0 + 24, 'stroke-width': 2 }, parts.panel);
          m.style.stroke = 'var(--ea-c2)';
        }
        text(0, y0 + 38, 8.5, 'ea-muted', '0 ms', parts.panel);
        text(w, y0 + 38, 8.5, 'ea-muted', '10 ms', parts.panel).setAttribute('text-anchor', 'end');
        panelText([['', '1,002 local-timer interrupts a second without nohz_full (ergon-tune T1 evidence): about one a millisecond, and 60,063.5 gaps over 5 µs a minute is the same rate.']], y0 + 52);
      } else {
        panelText([
          ['ea-strong', 'Zoom: the tick on Azure'],
          ['', 'This host\'s tick rate is not in the table. Adding nohz_full later cut its gaps over 5 µs from 10,656 to 239.5 a minute (97.7 %), and the run\'s trace puts most of them on the tick handler.'],
        ], y0);
      }
    } else if (kind === 't1') {
      const y = panelText([['ea-strong', '$ ergon-tune --hot 1 --all   (T1 lines; AWS run\'s isolcpus boot: CPUs 1-3,5-7 isolated)']], y0, true);
      const y2 = panelText([['ea-mono', 'WARN T1 warn: CPU 1 (loop) is not nohz_full: its scheduler tick keeps firing, CONFIG_HZ times a second. On AWS c7i … Fix: boot with `isolcpus=nohz,domain,managed_irq,1-3,5-7 nohz_full=1-3,5-7 rcu_nocbs=1-3,5-7 irqaffinity=0,4` (evidence: lab runs flags-aws-20261014T163114Z …)']], y + 2, true);
      panelText([
        ['ea-strong', 'after the full boot line:'],
        ['ea-mono', 'ok T1 ok: CPU 1 (loop): isolated, nohz_full and outside default_smp_affinity'],
      ], y2 + 4, true);
    } else if (kind === 'take') {
      const r = HOSTS[host].rows;
      const other = host === 'aws' ? 'Azure 97.7 %' : 'AWS 99.9 %';
      const cut = host === 'aws' ? '99.9 %' : '97.7 %';
      panelText([
        ['ea-strong', `Wins: nohz_full takes gaps over 5 µs from ${r[1].g5} to ${r[2].g5} a minute (${cut} fewer; ${other}). isolcpus + irqaffinity took gaps over 20 µs from ${r[0].g20} to ${r[1].g20}.`],
        ['', `Does not fix: the longest gap, ${r[2].max} µs, for which the trace shows ${r[2].cause}; none of these flags removed such stalls. IPIs still arrive. With one 120 s sample, >20 µs, >100 µs and the max do not rank the two isolated lines.`],
        ['', 'Costs: housekeeping CPUs take all other work, each busy thread must be pinned, a change needs a reboot, and kernel entry and exit cost more on a nohz_full CPU (kernel NO_HZ docs; not measured here).'],
      ], y0);
    }
  }

  function render(s) {
    current = s;
    const h = HOSTS[host];
    parts.toggle.textContent = `Host: ${h.name} · show ${h.other}`;
    parts.toggle.setAttribute('aria-label', `Showing ${h.name} rows; switch to ${h.other} rows`);
    const row = s.row === undefined ? null : h.rows[s.row];
    const prev = s.delta && s.row > 0 ? h.rows[s.row - 1] : null;
    parts.title.textContent = `${h.name}: one minute on the loop's CPU`;
    const boot = s.row === undefined ? ['', ''] : BOOTS[s.row];
    parts.boot1.textContent = boot[0];
    parts.boot2.textContent = boot[1];
    lanes(row, prev);
    parts.maxBar.style.transform = `scaleX(${row ? Math.min(1, num(row.max) / h.scale).toFixed(3) : 0})`;
    parts.maxBar.classList.toggle('ea-hot', Boolean(prev));
    parts.max1.textContent = row ? `max ${row.max} µs` : 'max –';
    parts.max2.textContent = row ? `cause: ${row.cause}` : '';
    parts.max3.textContent = `run ${h.run}, 120 s sample`;
    const off = s.off || [];
    parts.chips.forEach((c, i) => {
      const gone = off.includes(i);
      c.rect.setAttribute('class', gone ? 'ea-box' : 'ea-k2');
      c.rect.style.strokeDasharray = gone ? '4 3' : '';
      c.rect.classList.toggle('ea-hot', (s.hot || []).includes(i));
      c.strike.classList.toggle('ea-hidden', !gone);
      c.name.setAttribute('class', gone ? 'ea-muted' : 'ea-strong');
      c.state.textContent = gone ? `off: ${MOVED[i]}` : MOVED[i] ? 'on this CPU' : 'on: no boot flag moves it';
    });
    panel(s.panel);
  }

  const cap = (aws, azure) => () => (host === 'aws' ? aws : azure);
  const states = [
    {
      ms: 6500, off: [],
      caption: cap(
        'One minute on the busy loop\'s CPU, AWS c7i. A loop that only reads the CPU counter records every gap. Each lane draws gaps over 5, 20 and 100 µs, the bar the longest gap. Counts are per minute from one 120 s sample per boot line; where a mark falls is illustrative.',
        'One minute on the busy loop\'s CPU, Azure D8ds_v6. A loop that only reads the CPU counter records every gap. Each lane draws gaps over 5, 20 and 100 µs, the bar the longest gap. Counts are per minute from one 120 s sample per boot line; where a mark falls is illustrative.'),
    },
    {
      ms: 6500, row: 0, off: [],
      caption: cap(
        'Stock boot: 60,120 gaps over 5 µs a minute, about 1,000 a second, too many to draw one by one. 120 over 20 µs and 34.5 over 100 µs. Other tasks, interrupts, the scheduler tick and RCU callbacks all share the CPU. Longest gap 146.95 µs, a softirq TIMER.',
        'Stock boot: 17,189 gaps over 5 µs a minute, too many to draw one by one. 120 over 20 µs and 3.0 over 100 µs. Other tasks, interrupts, the scheduler tick and RCU callbacks all share the CPU. Longest gap 612.94 µs, for which the trace shows no kernel event.'),
    },
    {
      ms: 7500, row: 1, delta: true, off: [0, 1], hot: [0, 1],
      caption: cap(
        'isolcpus=managed_irq,domain takes the CPU out of scheduler load balancing, so other processes\' tasks stay off, and keeps managed interrupts off; irqaffinity sends new interrupts to housekeeping CPUs. In the trace, the stock boot\'s gaps over 20 µs were mostly timer softirqs and kworker preemptions; here neither shows. Gaps over 20 µs fall from 120 to 4.0 a minute, over 100 µs to 0.0, longest 52.30 µs. Over 5 µs barely moves: 60,063.5.',
        'isolcpus=managed_irq,domain takes the CPU out of scheduler load balancing, so other processes\' tasks stay off, and keeps managed interrupts off; irqaffinity sends new interrupts to housekeeping CPUs. Gaps over 20 µs fall from 120 to 47.0 a minute, over 100 µs to 0.0, longest 58.94 µs. Over 5 µs only falls to 10,656. Per-CPU kworkers and timer softirqs still show in this host\'s trace until nohz_full.'),
    },
    {
      ms: 7000, row: 1, off: [0, 1], hot: [2], panel: 'zoom',
      caption: cap(
        'Why over 5 µs barely moved: the scheduler tick still fires. isolcpus alone leaves it on: about one interrupt a millisecond, each a gap over 5 µs. The table only implies this; ergon-tune\'s T1 check reports it on its own.',
        'Why over 5 µs fell so little: the scheduler tick still fires. isolcpus alone leaves it on. The table only implies this; ergon-tune\'s T1 check reports it on its own.'),
    },
    {
      ms: 8000, row: 2, delta: true, off: [0, 1, 2, 3], hot: [2, 3],
      caption: cap(
        'nohz_full stops the tick while one task runs (0 local-timer interrupts a second) and rcu_nocbs moves RCU callbacks to housekeeping CPUs; the two were measured together. Over 5 µs: 60,063.5 to 50.0 a minute, 99.9 % fewer. Over 20 µs 4.5, over 100 µs 1.0, longest 133.90 µs with no kernel event in the trace: single events in one sample. Short IPIs still arrive.',
        'nohz_full stops the tick while one task runs and rcu_nocbs moves RCU callbacks to housekeeping CPUs; the two were measured together. Over 5 µs: 10,656 to 239.5 a minute, 97.7 % fewer. Over 20 µs 37.0, over 100 µs 0.0, longest 67.82 µs with no kernel event in the trace. Short IPIs still arrive.'),
    },
    {
      ms: 8000, row: 2, off: [0, 1, 2, 3], panel: 't1',
      caption: cap(
        'ergon-tune T1 checks each busy CPU for all three: isolated, nohz_full, outside the default IRQ affinity. On the isolcpus boot it warns that the tick still fires and prints the full boot line as the fix, for every CPU already isolated (cut short here); after it, with --all, the ok line. Its evidence is this AWS run.',
        'ergon-tune T1 checks each busy CPU for all three: isolated, nohz_full, outside the default IRQ affinity. On the isolcpus boot it warns that the tick still fires and prints the full boot line as the fix, for every CPU already isolated (cut short here); after it, with --all, the ok line. Its evidence is the AWS run.'),
    },
    {
      ms: 9000, row: 2, off: [0, 1, 2, 3], panel: 'take',
      caption: cap(
        'Takeaway: the tick was nearly every gap over 5 µs, and only nohz_full removes it. The longest gap and the few left over 20 µs stay: the trace shows no kernel event for them, and no flag tested removed them.',
        'Takeaway: most gaps over 5 µs were the tick, and only nohz_full removes it. The longest gap and 37.0 a minute over 20 µs stay: the trace shows no kernel event for them, and no flag tested removed them.'),
    },
  ];

  build(false);
  return {
    steps: states.map((s) => ({
      get caption() { return s.caption(); },
      ms: s.ms,
      show: () => render(s),
    })),
    onResize(width) {
      const narrow = width < 520;
      if (narrow === (g === NARROW)) return false;
      build(narrow);
      return true;
    },
  };
});
