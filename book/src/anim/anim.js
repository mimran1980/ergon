// ergon book animation runtime.
//
// A page embeds an animation with
//   <div class="ergon-anim" data-anim="SLUG" data-title="Short title"></div>
// followed by a blank line and a text alternative. When the embed nears the
// viewport this runtime loads anim/SLUG.js (book/src/anim/SLUG.js), which calls
//   Anim.register('SLUG', (stage, api) => ({ steps: [{ caption, show, ms }] }))
// Each step's show() draws the whole state for that step, so stepping back is
// a redraw, never an undo. Movement comes from CSS transitions (anim.css),
// which reduced motion turns off; colours come from mdbook's theme variables.
// Plain ES2020, no network, no build step.
(() => {
  'use strict';
  if (window.Anim) return;

  const SVG_NS = 'http://www.w3.org/2000/svg';
  const SLUG = /^[a-z0-9]+(?:-[a-z0-9]+)*$/;
  const SPEEDS = [0.5, 1, 2];
  const DEFAULT_MS = 3000;
  const factories = new Map();
  const pending = new Map(); // slug -> roots waiting for the module
  let uid = 0;

  const reducedMotion = () =>
    typeof window.matchMedia === 'function' &&
    window.matchMedia('(prefers-reduced-motion: reduce)').matches;

  function moduleUrl(slug) {
    // mdbook declares a global `const path_to_root` on every page (also
    // print.html); a global const is not a window property, so read it by name.
    const base = typeof path_to_root === 'string' ? path_to_root : '';
    return `${base}anim/${slug}.js`;
  }

  function fail(root, message) {
    root.classList.add('ea-failed');
    root.textContent = `Animation unavailable (${message}); the description below covers it.`;
  }

  function el(tag, attrs, parent, text) {
    const node = document.createElementNS(SVG_NS, tag);
    for (const [k, v] of Object.entries(attrs || {})) node.setAttribute(k, String(v));
    if (text !== undefined) node.textContent = text;
    if (parent) parent.appendChild(node);
    return node;
  }

  function html(tag, cls, parent, text) {
    const node = document.createElement(tag);
    if (cls) node.className = cls;
    if (text !== undefined) node.textContent = text;
    if (parent) parent.appendChild(node);
    return node;
  }

  function mount(root, factory) {
    const id = `ea${++uid}`;
    const title = root.dataset.title || root.dataset.anim;
    root.textContent = '';
    root.classList.add('ea-ready');
    root.setAttribute('role', 'group');
    root.setAttribute('aria-roledescription', 'animation');
    root.setAttribute('aria-label', `${title}. Keys: space plays or pauses, arrows step, Home restarts.`);
    root.tabIndex = 0;

    const header = html('div', 'ea-header', root);
    const heading = html('div', '', header);
    html('span', 'ea-eyebrow', heading, 'Interactive walkthrough');
    html('p', 'ea-title', heading, title);
    const status = html('span', 'ea-status', header, 'Paused');
    status.setAttribute('aria-hidden', 'true');
    const stage = html('div', 'ea-stage', root);
    const caption = html('p', 'ea-caption', root);
    caption.id = `${id}-caption`;
    caption.setAttribute('aria-live', 'polite');
    const timeline = html('div', 'ea-timeline', root);
    const scrubber = html('input', 'ea-scrubber', timeline);
    scrubber.type = 'range';
    scrubber.min = '1';
    scrubber.step = '1';
    scrubber.setAttribute('aria-label', 'Animation step');
    scrubber.setAttribute('aria-controls', caption.id);
    const progress = html('span', 'ea-progress', timeline);
    progress.setAttribute('aria-hidden', 'true');
    const controls = html('div', 'ea-controls', root);

    const api = {
      id,
      reducedMotion: reducedMotion(),
      el,
      /** Create a scalable SVG in the stage; `label` becomes its accessible name. */
      svg(width, height, label) {
        const svg = el('svg', {
          viewBox: `0 0 ${width} ${height}`,
          role: 'img',
          'aria-labelledby': `${id}-svgtitle ${id}-caption`,
          preserveAspectRatio: 'xMidYMin meet',
        }, stage);
        el('title', { id: `${id}-svgtitle` }, svg, label || title);
        return svg;
      },
      /** Current value of a theme CSS variable (for canvas users; SVG uses CSS). */
      css(name) {
        return getComputedStyle(root).getPropertyValue(name).trim();
      },
    };

    let spec;
    try {
      spec = factory(stage, api);
    } catch (err) {
      fail(root, err && err.message ? err.message : 'module error');
      return;
    }
    const steps = (spec && spec.steps) || [];
    if (steps.length === 0) {
      fail(root, 'no steps');
      return;
    }
    scrubber.max = String(steps.length);

    let index = 0;
    let playing = false;
    let userPaused = api.reducedMotion;
    let speed = 1;
    let timer = 0;
    let inViewport = false;
    let finished = false;

    const button = (label, aria, onClick) => {
      const b = html('button', 'ea-btn', controls, label);
      b.type = 'button';
      b.setAttribute('aria-label', aria);
      b.setAttribute('aria-controls', caption.id);
      // aria-disabled, not `disabled`: a disabled button drops focus to <body>,
      // which sends the next arrow key to mdbook's chapter navigation.
      b.addEventListener('click', () => { if (b.getAttribute('aria-disabled') !== 'true') onClick(); });
      return b;
    };
    const restartBtn = button('Restart', 'Restart animation', () => { pause(true); show(0); });
    const backBtn = button('Back', 'Previous step', () => { pause(true); show(index - 1); });
    const playBtn = button('Play', 'Play animation', () => (playing ? pause(true) : play()));
    const nextBtn = button('Next', 'Next step', () => { pause(true); show(index + 1); });
    const speedLabel = html('label', 'ea-speed', controls);
    html('span', '', speedLabel, 'Speed ');
    const speedSel = html('select', '', speedLabel);
    speedSel.setAttribute('aria-label', 'Playback speed');
    for (const s of SPEEDS) {
      const o = html('option', '', speedSel, `${s}×`);
      o.value = String(s);
      if (s === speed) o.selected = true;
    }
    speedSel.addEventListener('change', () => {
      speed = Number(speedSel.value) || 1;
      if (playing) schedule();
    });
    scrubber.addEventListener('input', () => {
      pause(true);
      show(Number(scrubber.value) - 1);
    });

    const setDisabled = (b, off) => b.setAttribute('aria-disabled', String(off));

    function show(i) {
      index = Math.max(0, Math.min(steps.length - 1, i));
      const step = steps[index];
      step.show();
      caption.textContent = step.caption;
      progress.textContent = `Step ${index + 1} of ${steps.length}`;
      scrubber.value = String(index + 1);
      scrubber.setAttribute('aria-valuetext', `${progress.textContent}: ${step.caption}`);
      scrubber.style.setProperty('--ea-progress', `${steps.length > 1 ? index / (steps.length - 1) * 100 : 100}%`);
      setDisabled(backBtn, index === 0);
      setDisabled(nextBtn, index === steps.length - 1);
      setDisabled(restartBtn, index === 0);
      finished = index === steps.length - 1;
      updateStatus();
    }

    function updateStatus() {
      status.textContent = playing ? 'Playing' : finished ? 'Complete' : 'Paused';
      root.classList.toggle('ea-playing', playing);
    }

    function schedule() {
      clearTimeout(timer);
      const ms = (steps[index].ms || DEFAULT_MS) / speed;
      timer = setTimeout(() => {
        if (index >= steps.length - 1) { pause(false); return; }
        show(index + 1);
        schedule();
      }, ms);
    }

    function play() {
      userPaused = false;
      if (index >= steps.length - 1) show(0);
      playing = true;
      playBtn.textContent = 'Pause';
      playBtn.setAttribute('aria-label', 'Pause animation');
      updateStatus();
      schedule();
    }

    function pause(byUser) {
      if (byUser) userPaused = true;
      playing = false;
      clearTimeout(timer);
      playBtn.textContent = 'Play';
      playBtn.setAttribute('aria-label', 'Play animation');
      updateStatus();
    }

    root.addEventListener('keydown', (e) => {
      if (e.altKey || e.ctrlKey || e.metaKey) return;
      // Native range keys scrub the timeline; don't turn them into chapter
      // navigation or move two steps by also handling them here.
      if (e.target instanceof HTMLSelectElement || e.target instanceof HTMLInputElement) {
        e.stopPropagation();
        return;
      }
      const onButton = e.target instanceof HTMLButtonElement;
      let handled = true;
      switch (e.key) {
        case ' ':
        case 'k':
          if (onButton && e.key === ' ') { handled = false; break; }
          if (playing) pause(true); else play();
          break;
        case 'ArrowRight':
        case 'l':
          pause(true); show(index + 1);
          break;
        case 'ArrowLeft':
        case 'j':
          pause(true); show(index - 1);
          break;
        case 'Home':
          pause(true); show(0);
          break;
        case 'End':
          pause(true); show(steps.length - 1);
          break;
        default:
          handled = false;
      }
      // Stop mdbook's arrow-key chapter navigation while the animation has focus.
      if (handled) { e.preventDefault(); e.stopPropagation(); }
    });

    if (typeof ResizeObserver === 'function') {
      new ResizeObserver((entries) => {
        const width = entries[0].contentRect.width;
        root.classList.toggle('ea-narrow', width < 520);
        // A module returns true when it rebuilt its drawing for the new width.
        if (typeof spec.onResize === 'function' && spec.onResize(width) === true) show(index);
      }).observe(root);
    }

    if (typeof IntersectionObserver === 'function') {
      new IntersectionObserver((entries) => {
        inViewport = entries[0].isIntersecting;
        if (inViewport && !document.hidden && !playing && !userPaused && !finished) play();
        else if (!inViewport && playing) pause(false);
      }, { threshold: 0.25 }).observe(root); // 0.5 never fires for a tall figure at the end of a page
    }

    // A background tab should not consume the reader's remaining steps.
    document.addEventListener('visibilitychange', () => {
      if (document.hidden) pause(false);
      else if (inViewport && !userPaused && !finished) play();
    });
    if (typeof window.matchMedia === 'function') {
      const preference = window.matchMedia('(prefers-reduced-motion: reduce)');
      preference.addEventListener('change', (event) => {
        if (event.matches) pause(true);
      });
    }

    show(0);
  }

  function load(slug) {
    if (factories.has(slug) || pending.has(slug)) return;
    pending.set(slug, []);
    const script = document.createElement('script');
    script.src = moduleUrl(slug);
    script.async = true;
    script.onerror = () => {
      for (const root of pending.get(slug) || []) fail(root, `could not load ${slug}.js`);
      pending.delete(slug);
    };
    document.head.appendChild(script);
  }

  function request(root) {
    const slug = root.dataset.anim || '';
    if (!SLUG.test(slug)) { fail(root, 'bad data-anim'); return; }
    if (factories.has(slug)) { mount(root, factories.get(slug)); return; }
    load(slug);
    const queue = pending.get(slug);
    if (queue) queue.push(root);
  }

  window.Anim = Object.freeze({
    /** Register an animation module; called by anim/<slug>.js. */
    register(slug, factory) {
      if (!SLUG.test(slug) || typeof factory !== 'function') return;
      factories.set(slug, factory);
      const queue = pending.get(slug) || [];
      pending.delete(slug);
      for (const root of queue) mount(root, factory);
    },
  });

  function start() {
    const roots = Array.from(document.querySelectorAll('.ergon-anim[data-anim]'));
    if (typeof IntersectionObserver !== 'function') { roots.forEach(request); return; }
    const near = new IntersectionObserver((entries, observer) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        observer.unobserve(entry.target);
        request(entry.target);
      }
    }, { rootMargin: '300px 0px' });
    roots.forEach((r) => near.observe(r));
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
