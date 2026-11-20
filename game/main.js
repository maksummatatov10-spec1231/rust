(() => {
  'use strict';

  const W = 960, H = 600, STRIDE = 32;
  const canvas = document.getElementById('game');
  const ctx = canvas.getContext('2d');
  const overlay = document.getElementById('overlay');
  const startBtn = document.getElementById('startBtn');
  const hud = {
    score: document.getElementById('score'),
    lives: document.getElementById('lives'),
    level: document.getElementById('level'),
    time: document.getElementById('time'),
    kills: document.getElementById('kills'),
  };

  const imgs = {};
  const tinted = {};
  const keys = new Set();
  const pointer = { x: W / 2 + 120, y: H / 2, down: false };
  let ex = null;
  let memoryView = null;
  let started = false;
  let over = false;
  let paused = false;
  let last = 0;
  let ready = false;

  const imageFiles = {
    grass: 'assets/grass.png',
    flowers: 'assets/flowers.png',
    mushrooms: 'assets/mushrooms.png',
    leaves: 'assets/leaves.png',
    rocks: 'assets/rocks.png',
    leaf: 'assets/leaf.png',
    weapons: 'assets/weapons.png',
    halloween: 'assets/halloween.png',
    tools: 'assets/tools.png',
  };

  const audioPaths = {
    start: 'assets/sfx_start.ogg',
    shoot: 'assets/sfx_shoot.ogg',
    collect: 'assets/sfx_select.ogg',
    hit: 'assets/sfx_hit.ogg',
    hurt: 'assets/sfx_error.ogg',
    power: 'assets/sfx_confirm.ogg',
    over: 'assets/sfx_error.ogg',
  };

  function loadImage(src) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = reject;
      img.src = src;
    });
  }

  function tint(img, color, key) {
    const c = document.createElement('canvas');
    c.width = img.width;
    c.height = img.height;
    const g = c.getContext('2d');
    g.drawImage(img, 0, 0);
    g.globalCompositeOperation = 'source-in';
    g.fillStyle = color;
    g.fillRect(0, 0, c.width, c.height);
    tinted[key] = c;
    return c;
  }

  async function boot() {
    startBtn.disabled = true;
    startBtn.textContent = 'Загрузка…';
    const entries = Object.entries(imageFiles);
    const loaded = await Promise.all(entries.map(([, src]) => loadImage(src)));
    entries.forEach(([name], i) => { imgs[name] = loaded[i]; });

    tint(imgs.leaves, '#1b4732', 'leavesDark');
    tint(imgs.leaves, '#306b46', 'leavesMid');
    tint(imgs.leaf, '#a6f36c', 'leafLime');
    tint(imgs.leaf, '#8ff3ff', 'leafCyan');

    const bytes = await fetch('core.wasm').then((r) => {
      if (!r.ok) throw new Error('core.wasm not found: ' + r.status);
      return r.arrayBuffer();
    });
    const { instance } = await WebAssembly.instantiate(bytes, {});
    ex = instance.exports;
    memoryView = new DataView(ex.memory.buffer);
    ex.init_game();
    ready = true;
    startBtn.disabled = false;
    startBtn.textContent = 'Начать игру';
    requestAnimationFrame(frame);
  }

  function play(name) {
    const src = audioPaths[name];
    if (!src) return;
    try {
      const a = new Audio(src);
      a.volume = name === 'shoot' ? 0.22 : 0.38;
      a.play().catch(() => {});
    } catch (_) { /* audio is optional */ }
  }

  function start() {
    if (!ready) return;
    ex.start_game();
    started = true;
    over = false;
    paused = false;
    overlay.classList.add('hidden');
  }

  startBtn.addEventListener('click', start);
  window.addEventListener('keydown', (e) => {
    const k = e.key.toLowerCase();
    if (k === 'p') { paused = !paused; return; }
    if (!started && (k === 'enter' || k === ' ')) { start(); return; }
    if (started && over && (k === 'enter' || k === ' ')) { start(); return; }
    keys.add(k);
    if ([' ', 'arrowup', 'arrowdown', 'arrowleft', 'arrowright'].includes(k)) e.preventDefault();
  });
  window.addEventListener('keyup', (e) => keys.delete(e.key.toLowerCase()));

  function updatePointer(e) {
    const r = canvas.getBoundingClientRect();
    pointer.x = (e.clientX - r.left) / r.width * W;
    pointer.y = (e.clientY - r.top) / r.height * H;
  }
  canvas.addEventListener('pointermove', updatePointer);
  canvas.addEventListener('pointerdown', (e) => {
    updatePointer(e);
    pointer.down = true;
    if (!started) start();
  });
  window.addEventListener('pointerup', () => { pointer.down = false; });

  function inputFlags() {
    let f = 0;
    if (keys.has('w') || keys.has('arrowup')) f |= 1;
    if (keys.has('s') || keys.has('arrowdown')) f |= 2;
    if (keys.has('a') || keys.has('arrowleft')) f |= 4;
    if (keys.has('d') || keys.has('arrowright')) f |= 8;
    if (pointer.down || keys.has(' ') || keys.has('j')) f |= 16;
    return f;
  }

  function handleEvents() {
    if (!ex) return;
    const n = ex.event_count();
    if (!n) return;
    const p = ex.events_ptr();
    for (let i = 0; i < n; i++) {
      const id = memoryView.getInt32(p + i * 4, true);
      switch (id) {
        case 0: play('start'); break;
        case 1: play('collect'); break;
        case 2: play('shoot'); break;
        case 3: play('hit'); break;
        case 4: play('hurt'); break;
        case 5:
          play('over');
          over = true;
          showGameOver();
          break;
        case 6: play('power'); break;
      }
    }
    ex.clear_events();
  }

  function showGameOver() {
    overlay.querySelector('h2').innerHTML = 'Лес померк…<br><em> но семена остались</em>';
    overlay.querySelector('p').textContent = `Вы набрали ${ex.state(0)} очков и пережили ${ex.state(3)} сек. Попробуйте снова.`;
    startBtn.textContent = 'Играть ещё';
    overlay.classList.remove('hidden');
  }

  function drawFlower(type, x, y, size, alpha = 1) {
    const crop = [
      [0, 0, 470, 450],
      [520, 0, 480, 430],
      [320, 310, 390, 380],
      [0, 540, 450, 440],
      [610, 560, 400, 420],
    ][type % 5];
    ctx.save();
    ctx.globalAlpha = alpha;
    ctx.drawImage(imgs.flowers, crop[0], crop[1], crop[2], crop[3], x - size / 2, y - size / 2, size, size);
    ctx.restore();
  }

  const decor = [
    { x: 70, y: 90, s: 74, t: 0 }, { x: 890, y: 130, s: 66, t: 1 },
    { x: 140, y: 520, s: 70, t: 3 }, { x: 820, y: 510, s: 64, t: 4 },
    { x: 470, y: 70, s: 52, t: 2 }, { x: 530, y: 550, s: 58, t: 2 },
  ];
  const leafSpots = [
    [110, 160, 0.5, 1.2], [260, 80, -0.3, 0.9], [410, 210, 0.9, 1.1],
    [620, 110, -0.8, 0.8], [790, 240, 0.25, 1.3], [90, 380, -0.5, 1.0],
    [300, 470, 0.6, 0.9], [510, 400, -0.2, 1.2], [700, 520, 0.8, 1.0],
    [880, 420, -0.6, 0.7], [190, 280, 0.2, 0.6], [620, 320, 1.1, 0.7],
  ];

  function drawBackground(t) {
    const g = ctx.createLinearGradient(0, 0, W, H);
    g.addColorStop(0, '#123a29');
    g.addColorStop(0.55, '#0d2820');
    g.addColorStop(1, '#07170f');
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, W, H);

    ctx.save();
    ctx.globalAlpha = 0.13;
    const pattern = ctx.createPattern(imgs.grass, 'repeat');
    ctx.fillStyle = pattern;
    ctx.fillRect(0, 0, W, H);
    ctx.restore();

    // Soft moonlight.
    const glow = ctx.createRadialGradient(W * 0.72, H * 0.2, 10, W * 0.72, H * 0.2, 380);
    glow.addColorStop(0, '#bff5c033');
    glow.addColorStop(1, '#bff5c000');
    ctx.fillStyle = glow;
    ctx.fillRect(0, 0, W, H);

    // Leaf shadows with a slow parallax sway.
    tinted.leavesDark && leafSpots.forEach(([x, y, rot, scale], i) => {
      ctx.save();
      ctx.translate(x + Math.sin(t * 0.4 + i) * 8, y + Math.cos(t * 0.3 + i) * 6);
      ctx.rotate(rot + Math.sin(t * 0.2 + i) * 0.08);
      ctx.globalAlpha = 0.16;
      const size = 150 * scale;
      ctx.drawImage(tinted.leavesDark, -size / 2, -size / 2, size, size);
      ctx.restore();
    });

    decor.forEach((d, i) => drawFlower(d.t, d.x + Math.sin(t * 0.5 + i) * 3, d.y, d.s, 0.22));
  }

  function drawPlayer(e, t) {
    const invuln = ex.state(6) > 0 && Math.floor(t * 12) % 2 === 0;
    ctx.save();
    ctx.translate(e.x, e.y);
    if (invuln) ctx.globalAlpha = 0.45;

    // Ground shadow.
    ctx.fillStyle = '#0008';
    ctx.beginPath(); ctx.ellipse(0, 10, 22, 8, 0, 0, 6.283); ctx.fill();

    // Body.
    const body = ctx.createRadialGradient(-6, -8, 3, 0, 0, 22);
    body.addColorStop(0, '#e8ffc9');
    body.addColorStop(0.45, '#93e07a');
    body.addColorStop(1, '#3c8c53');
    ctx.fillStyle = body;
    ctx.beginPath(); ctx.arc(0, 0, e.r, 0, 6.283); ctx.fill();
    ctx.strokeStyle = '#d6ffb0'; ctx.lineWidth = 2; ctx.stroke();

    // Leaf crown from the packed leaf sprite.
    if (tinted.leafLime) {
      ctx.save();
      ctx.translate(0, -e.r - 2);
      ctx.rotate(Math.sin(t * 2) * 0.18);
      ctx.drawImage(tinted.leafLime, -14, -20, 28, 28);
      ctx.restore();
    }

    // Eyes and aim direction.
    const ax = e.vx, ay = e.vy;
    const a = Math.atan2(ay, ax);
    ctx.rotate(a);
    ctx.fillStyle = '#17351f';
    ctx.beginPath(); ctx.arc(7, -5, 2.4, 0, 6.283); ctx.arc(7, 5, 2.4, 0, 6.283); ctx.fill();
    ctx.restore();
  }

  function drawEnemy(e) {
    const pulse = 1 + Math.sin(e.data) * 0.09;
    ctx.save();
    ctx.translate(e.x, e.y);
    ctx.scale(pulse, 2 - pulse);
    const g = ctx.createRadialGradient(-4, -6, 2, 0, 0, e.r * 1.3);
    g.addColorStop(0, '#5b3d7a');
    g.addColorStop(0.55, '#2b1e43');
    g.addColorStop(1, '#100c1c00');
    ctx.fillStyle = g;
    ctx.beginPath(); ctx.arc(0, 0, e.r * 1.2, 0, 6.283); ctx.fill();
    ctx.fillStyle = '#ff7de0';
    ctx.shadowColor = '#ff5bd0'; ctx.shadowBlur = 12;
    ctx.beginPath(); ctx.arc(-5, -3, 2.6, 0, 6.283); ctx.arc(5, -3, 2.6, 0, 6.283); ctx.fill();
    ctx.restore();
  }

  function drawShot(e) {
    ctx.save();
    ctx.translate(e.x, e.y);
    ctx.rotate(Math.atan2(e.vy, e.vx));
    ctx.shadowColor = '#8ff3ff'; ctx.shadowBlur = 14;
    if (tinted.leafCyan) ctx.drawImage(tinted.leafCyan, -11, -9, 22, 18);
    else { ctx.fillStyle = '#8ff3ff'; ctx.beginPath(); ctx.arc(0, 0, 7, 0, 6.283); ctx.fill(); }
    ctx.restore();
  }

  function drawMushroom(e) {
    ctx.save();
    ctx.translate(e.x, e.y + Math.sin(e.data * 3) * 3);
    ctx.shadowColor = '#f7c46b'; ctx.shadowBlur = 18;
    ctx.drawImage(imgs.mushrooms, 0, 60, 260, 500, -20, -28, 40, 56);
    ctx.restore();
  }

  function drawParticle(e) {
    const colors = ['#8ff3ff', '#ff6b7a', '#ffe27a', '#c78bff'];
    ctx.save();
    ctx.globalAlpha = Math.min(1, e.life * 2);
    ctx.fillStyle = colors[Math.floor(e.data) % colors.length];
    ctx.shadowColor = ctx.fillStyle; ctx.shadowBlur = 10;
    ctx.beginPath(); ctx.arc(e.x, e.y, e.r, 0, 6.283); ctx.fill();
    ctx.restore();
  }

  function drawEntities(t) {
    const p = ex.entities_ptr();
    const n = ex.entity_count();
    // Decorative pickups first, shadows last so the player stays readable.
    for (let i = 1; i < n; i++) {
      const b = p + i * STRIDE;
      const kind = memoryView.getInt32(b, true);
      if (kind === 4) {
        const type = memoryView.getFloat32(b + 28, true);
        const e = readEntity(b);
        drawFlower(type, e.x, e.y + Math.sin(t * 3 + type) * 4, 40 + e.r);
      }
    }
    for (let i = 1; i < n; i++) {
      const e = readEntity(p + i * STRIDE);
      if (e.kind === 5) drawMushroom(e);
      else if (e.kind === 6) drawParticle(e);
    }
    for (let i = 1; i < n; i++) {
      const e = readEntity(p + i * STRIDE);
      if (e.kind === 2) drawEnemy(e);
    }
    for (let i = 1; i < n; i++) {
      const e = readEntity(p + i * STRIDE);
      if (e.kind === 3) drawShot(e);
    }
    if (n > 0) drawPlayer(readEntity(p), t);
  }

  function readEntity(b) {
    return {
      kind: memoryView.getInt32(b, true),
      x: memoryView.getFloat32(b + 4, true),
      y: memoryView.getFloat32(b + 8, true),
      vx: memoryView.getFloat32(b + 12, true),
      vy: memoryView.getFloat32(b + 16, true),
      r: memoryView.getFloat32(b + 20, true),
      life: memoryView.getFloat32(b + 24, true),
      data: memoryView.getFloat32(b + 28, true),
    };
  }

  function updateHud() {
    hud.score.textContent = ex.state(0);
    hud.lives.textContent = '♥'.repeat(Math.max(0, ex.state(1))) || '—';
    hud.level.textContent = ex.state(2);
    const s = ex.state(3);
    hud.time.textContent = `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
    hud.kills.textContent = ex.state(4);
  }

  function frame(ts) {
    const dt = Math.min((ts - last) / 1000, 0.05);
    last = ts;
    const t = ts / 1000;
    if (!ex) return;
    if (started && !paused && !over) ex.step(dt, inputFlags(), pointer.x, pointer.y);
    handleEvents();

    ctx.save();
    const shake = ex.state(5) / 1000;
    if (shake > 0) ctx.translate((Math.random() - 0.5) * shake * 40, (Math.random() - 0.5) * shake * 40);
    drawBackground(t);
    drawEntities(t);
    ctx.restore();

    if (paused && started) {
      ctx.fillStyle = '#05120cbb';
      ctx.fillRect(0, 0, W, H);
      ctx.fillStyle = '#d9ffd0';
      ctx.font = '700 42px system-ui';
      ctx.textAlign = 'center';
      ctx.fillText('Пауза', W / 2, H / 2);
    }
    updateHud();
    requestAnimationFrame(frame);
  }

  boot().catch((err) => {
    startBtn.textContent = 'Ошибка загрузки';
    overlay.querySelector('p').textContent = String(err && err.message ? err.message : err);
    console.error(err);
  });
})();
