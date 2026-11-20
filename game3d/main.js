/* Forest Bloom 3D — rendering, UI and input.
   Gameplay simulation lives in Rust (core.wasm). */
(() => {
  'use strict';

  const $ = (id) => document.getElementById(id);
  const view = $('view');
  const hud = $('hud');
  const loading = $('loading');
  const overlays = {
    main: $('mainMenu'), settings: $('settingsMenu'), pause: $('pauseMenu'), over: $('gameOver'),
  };
  const ui = {
    hp: $('hpBar').firstElementChild, st: $('stBar').firstElementChild,
    hpText: $('hpText'), stText: $('stText'), score: $('score'),
    wave: $('waveLine'), kills: $('killLine'), hit: $('hit'), vignette: $('vignette'),
    overText: $('overText'),
  };

  const S = {
    sens: 0.0022, fov: 78, volume: 0.5, quality: 1,
    shadows: true, fog: true, invert: false,
  };
  const EMBEDDED = window.__EMBEDDED_FILES__ || null;
  const WASM_B64 = window.__WASM_B64__ || null;
  const assetUrl = (p) => (EMBEDDED && EMBEDDED[p]) ? EMBEDDED[p] : p;

  let renderer, scene, camera, controls, clock, sun, sky, fog;
  let ex = null, dv = null;
  let ready = false, mode = 0, settingsReturn = 'main';
  let models = {}, images = {}, swordRig = null, slash = null;
  let enemyViews = new Map(), pickupViews = new Map();
  let animClips = [];
  let mouseDX = 0, mouseDY = 0, mouseDown = false;
  const keys = new Set();
  let ambientTime = 0;
  let audioVolume = S.volume;
  const sounds = {};

  // ---------- helpers ----------
  function base64Bytes(b64) {
    const bin = atob(b64);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out.buffer;
  }

  function resolvePath(base, rel) {
    const parts = (base + rel).split('/');
    const stack = [];
    for (const part of parts) {
      if (part === '' || part === '.') continue;
      if (part === '..') stack.pop();
      else stack.push(part);
    }
    return stack.join('/');
  }

  function loadImage(path) {
    return new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error('image: ' + path));
      img.src = assetUrl(path);
    });
  }

  function loadSound(name, path) {
    const a = new Audio(assetUrl(path));
    a.preload = 'auto';
    sounds[name] = a;
  }

  function playSound(name) {
    const a = sounds[name];
    if (!a || audioVolume <= 0) return;
    try {
      const c = a.cloneNode();
      c.volume = Math.min(audioVolume * (name === 'swing' ? 0.45 : 0.8), 1);
      c.play().catch(() => {});
    } catch (_) { /* audio is best-effort */ }
  }

  const gltfLoader = typeof THREE !== 'undefined' ? new THREE.GLTFLoader() : null;

  function loadModel(path) {
    return new Promise((resolve, reject) => {
      if (EMBEDDED) {
        fetch(assetUrl(path)).then((r) => r.arrayBuffer()).then((buf) => {
          if (path.endsWith('.glb')) {
            gltfLoader.parse(buf, '', resolve, reject);
            return;
          }
          const json = JSON.parse(new TextDecoder().decode(buf));
          const dir = path.slice(0, path.lastIndexOf('/') + 1);
          for (const section of ['buffers', 'images']) {
            for (const item of json[section] || []) {
              const uri = item.uri;
              if (uri && !uri.startsWith('data:')) {
                item.uri = assetUrl(resolvePath(dir, uri));
              }
            }
          }
          gltfLoader.parse(JSON.stringify(json), '', resolve, reject);
        }).catch(reject);
      } else {
        gltfLoader.load(path, resolve, undefined, reject);
      }
    });
  }

  function show(which) {
    for (const [k, el] of Object.entries(overlays)) el.classList.toggle('hidden', k !== which);
  }
  function hideAll() {
    for (const el of Object.values(overlays)) el.classList.add('hidden');
  }

  // ---------- procedural textures ----------
  function groundTexture() {
    const c = document.createElement('canvas');
    c.width = c.height = 512;
    const g = c.getContext('2d');
    const grad = g.createLinearGradient(0, 0, 512, 512);
    grad.addColorStop(0, '#2f5d35');
    grad.addColorStop(0.5, '#264c30');
    grad.addColorStop(1, '#1c3c29');
    g.fillStyle = grad;
    g.fillRect(0, 0, 512, 512);
    if (images.grass) {
      g.globalAlpha = 0.22;
      for (let i = 0; i < 16; i++) {
        g.drawImage(images.grass, (i * 131) % 512, (i * 197) % 512, 128, 128);
      }
      g.globalAlpha = 1;
    }
    for (let i = 0; i < 9000; i++) {
      const x = Math.random() * 512, y = Math.random() * 512;
      g.fillStyle = Math.random() > 0.5 ? '#ffffff0b' : '#00000012';
      g.fillRect(x, y, 1 + Math.random() * 2, 1 + Math.random() * 2);
    }
    const tex = new THREE.CanvasTexture(c);
    tex.wrapS = tex.wrapT = THREE.RepeatWrapping;
    tex.repeat.set(26, 26);
    tex.anisotropy = Math.min(8, renderer.capabilities.getMaxAnisotropy());
    tex.encoding = THREE.sRGBEncoding;
    return tex;
  }

  function glowTexture(color) {
    const c = document.createElement('canvas');
    c.width = c.height = 128;
    const g = c.getContext('2d');
    const grd = g.createRadialGradient(64, 64, 2, 64, 64, 64);
    grd.addColorStop(0, color);
    grd.addColorStop(0.25, color.replace('1)', '0.65)'));
    grd.addColorStop(1, color.replace('1)', '0)'));
    g.fillStyle = grd;
    g.fillRect(0, 0, 128, 128);
    return new THREE.CanvasTexture(c);
  }

  // ---------- scene ----------
  function initRenderer() {
    renderer = new THREE.WebGLRenderer({ antialias: true, powerPreference: 'high-performance' });
    renderer.setSize(window.innerWidth, window.innerHeight);
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, 1.5));
    renderer.shadowMap.enabled = S.shadows;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    renderer.outputEncoding = THREE.sRGBEncoding;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.15;
    view.appendChild(renderer.domElement);

    scene = new THREE.Scene();
    fog = new THREE.FogExp2(0x9fc2ad, 0.0125);
    scene.fog = fog;
    camera = new THREE.PerspectiveCamera(S.fov, window.innerWidth / window.innerHeight, 0.08, 500);
    camera.position.set(0, 4, 12);
    clock = new THREE.Clock();

    controls = new THREE.PointerLockControls(camera, renderer.domElement);
    controls.addEventListener('unlock', () => {
      if (!ready || mode !== 1) return;
      mode = 2;
      ex.set_mode(2);
      show('pause');
    });

    // Sky dome with sun disk and warm horizon.
    const skyMat = new THREE.ShaderMaterial({
      side: THREE.BackSide,
      depthWrite: false,
      uniforms: {
        sunDir: { value: new THREE.Vector3(0.42, 0.55, -0.72).normalize() },
        topColor: { value: new THREE.Color('#3e86c8') },
        horizon: { value: new THREE.Color('#e5d9b5') },
        groundColor: { value: new THREE.Color('#78957b') },
      },
      vertexShader: `varying vec3 vDir;
        void main(){ vDir = position; gl_Position = projectionMatrix * modelViewMatrix * vec4(position,1.0); }`,
      fragmentShader: `varying vec3 vDir; uniform vec3 sunDir; uniform vec3 topColor; uniform vec3 horizon; uniform vec3 groundColor;
        void main(){
          vec3 d = normalize(vDir);
          float h = d.y;
          vec3 col = mix(horizon, topColor, smoothstep(-0.02, 0.65, h));
          col = mix(groundColor, col, smoothstep(-0.28, 0.02, h));
          float s = max(dot(d, normalize(sunDir)), 0.0);
          col += vec3(1.0,0.93,0.72) * pow(s, 480.0) * 2.2;
          col += vec3(1.0,0.72,0.42) * pow(s, 18.0) * 0.45;
          col += vec3(1.0,0.83,0.55) * pow(s, 4.0) * 0.10;
          gl_FragColor = vec4(col,1.0);
        }`,
    });
    sky = new THREE.Mesh(new THREE.SphereGeometry(300, 32, 20), skyMat);
    sky.frustumCulled = false;
    scene.add(sky);

    const hemi = new THREE.HemisphereLight(0xcfe8ff, 0x3a5b32, 0.85);
    scene.add(hemi);
    sun = new THREE.DirectionalLight(0xffe6b8, 2.0);
    sun.position.set(38, 62, -52);
    sun.castShadow = true;
    sun.shadow.mapSize.set(2048, 2048);
    sun.shadow.camera.left = -55; sun.shadow.camera.right = 55;
    sun.shadow.camera.top = 55; sun.shadow.camera.bottom = -55;
    sun.shadow.camera.near = 1; sun.shadow.camera.far = 180;
    sun.shadow.bias = -0.0006;
    scene.add(sun);
    scene.add(sun.target);

    const ground = new THREE.Mesh(
      new THREE.PlaneGeometry(120, 120),
      new THREE.MeshStandardMaterial({ map: groundTexture(), roughness: 0.96, metalness: 0.0 })
    );
    ground.rotation.x = -Math.PI / 2;
    ground.receiveShadow = true;
    scene.add(ground);

    window.addEventListener('resize', () => {
      camera.aspect = window.innerWidth / window.innerHeight;
      camera.updateProjectionMatrix();
      renderer.setSize(window.innerWidth, window.innerHeight);
    });
  }

  function prepModel(gltf, targetHeight) {
    const root = gltf.scene;
    root.traverse((o) => {
      if (o.isMesh) {
        o.castShadow = true;
        o.receiveShadow = true;
        if (o.material) {
          o.material.envMapIntensity = 0.5;
        }
      }
    });
    if (targetHeight) {
      const box = new THREE.Box3().setFromObject(root);
      const size = new THREE.Vector3();
      box.getSize(size);
      const s = targetHeight / Math.max(size.y, 0.01);
      root.scale.setScalar(s);
    }
    return root;
  }

  function seeded(n) {
    let x = n * 374761393 + 668265263;
    x = (x ^ (x >> 13)) * 1274126177;
    return ((x ^ (x >> 16)) >>> 0) / 4294967296;
  }

  function groupModels(list, pattern) {
    return list.filter((p) => p.includes(pattern));
  }

  function buildWorld() {
    const manifest = models.manifest.models;
    const nature = groupModels(manifest, '/nature/');
    const ruins = groupModels(manifest, '/ruins/');
    const trees = nature.filter((p) => /CommonTree|Pine_|TwistedTree|DeadTree/.test(p));
    const small = nature.filter((p) => /Bush|Fern|Grass_|Mushroom|Rock_|Pebble|Flower|Plant|Clover/.test(p));

    const pick = (arr, k) => arr[Math.floor(seeded(k * 7 + 11) * arr.length) % arr.length];
    const count = S.quality === 0 ? 55 : S.quality === 1 ? 105 : 175;
    for (let i = 0; i < count && trees.length; i++) {
      const a = seeded(i * 3 + 1) * Math.PI * 2;
      const r = 9 + Math.sqrt(seeded(i * 3 + 2)) * 38;
      const obj = prepModel(models[pick(trees, i)].clone(true), 5 + seeded(i * 9) * 4);
      obj.position.set(Math.cos(a) * r, 0, Math.sin(a) * r);
      obj.rotation.y = seeded(i * 13) * 6.28;
      obj.updateMatrix();
      obj.matrixAutoUpdate = false;
      scene.add(obj);
    }
    const smallCount = S.quality === 0 ? 60 : S.quality === 1 ? 130 : 220;
    for (let i = 0; i < smallCount && small.length; i++) {
      const a = seeded(i * 5 + 41) * Math.PI * 2;
      const r = 3 + Math.sqrt(seeded(i * 5 + 42)) * 40;
      const path = pick(small, i + 100);
      const obj = prepModel(models[path].clone(true), /Mushroom/.test(path) ? 0.7 : 0.45 + seeded(i * 17) * 0.6);
      obj.position.set(Math.cos(a) * r, 0, Math.sin(a) * r);
      obj.rotation.y = seeded(i * 23) * 6.28;
      obj.updateMatrix();
      obj.matrixAutoUpdate = false;
      scene.add(obj);
    }
    for (let i = 0; i < Math.min(5, ruins.length); i++) {
      const a = (i / 5) * Math.PI * 2 + 0.6;
      const obj = prepModel(models[ruins[i]].clone(true), 2.4);
      obj.position.set(Math.cos(a) * 15, 0, Math.sin(a) * 15);
      obj.rotation.y = a;
      scene.add(obj);
    }

    // Instanced grass blades for cheap ground detail.
    const blade = new THREE.ConeGeometry(0.055, 0.45, 4, 1);
    blade.translate(0, 0.22, 0);
    const grassMat = new THREE.MeshStandardMaterial({ color: 0x69a851, roughness: 1, side: THREE.DoubleSide });
    const grassCount = S.quality === 0 ? 700 : S.quality === 1 ? 1600 : 3200;
    const inst = new THREE.InstancedMesh(blade, grassMat, grassCount);
    const dummy = new THREE.Object3D();
    const color = new THREE.Color();
    for (let i = 0; i < grassCount; i++) {
      const a = seeded(i * 11 + 3) * Math.PI * 2;
      const r = Math.sqrt(seeded(i * 13 + 5)) * 45;
      dummy.position.set(Math.cos(a) * r, 0, Math.sin(a) * r);
      dummy.rotation.y = seeded(i * 19) * 6.28;
      const s = 0.6 + seeded(i * 29) * 1.2;
      dummy.scale.set(s, s, s);
      dummy.updateMatrix();
      inst.setMatrixAt(i, dummy.matrix);
      color.setHSL(0.26 + seeded(i * 31) * 0.06, 0.55, 0.32 + seeded(i * 37) * 0.16);
      inst.setColorAt(i, color);
    }
    inst.instanceMatrix.needsUpdate = true;
    inst.castShadow = false;
    inst.receiveShadow = true;
    scene.add(inst);

    // Fireflies.
    const ffGeo = new THREE.BufferGeometry();
    const n = 140;
    const pos = new Float32Array(n * 3);
    for (let i = 0; i < n; i++) {
      const a = seeded(i * 41) * 6.283, r = Math.sqrt(seeded(i * 43)) * 40;
      pos[i * 3] = Math.cos(a) * r;
      pos[i * 3 + 1] = 0.6 + seeded(i * 47) * 3.4;
      pos[i * 3 + 2] = Math.sin(a) * r;
    }
    ffGeo.setAttribute('position', new THREE.BufferAttribute(pos, 3));
    const ff = new THREE.Points(ffGeo, new THREE.PointsMaterial({
      color: 0xd8ff9a, size: 0.09, transparent: true, opacity: 0.85,
      blending: THREE.AdditiveBlending, depthWrite: false,
    }));
    scene.add(ff);
  }

  function buildSword() {
    const path = (models.manifest.models.find((p) => p.includes('sword_A.gltf')));
    let blade;
    if (path && models[path]) {
      blade = prepModel(models[path].clone(true), 1.05);
    } else {
      blade = new THREE.Group();
      const b = new THREE.Mesh(new THREE.BoxGeometry(0.07, 0.95, 0.035),
        new THREE.MeshStandardMaterial({ color: 0xd9e6ef, metalness: 0.85, roughness: 0.22 }));
      b.position.y = 0.55; blade.add(b);
      const h = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.28, 0.06),
        new THREE.MeshStandardMaterial({ color: 0x6b4426, roughness: 0.8 }));
      h.position.y = 0; blade.add(h);
    }
    const rig = new THREE.Group();
    blade.rotation.z = Math.PI / 2;
    blade.position.set(0, 0, -0.12);
    rig.add(blade);
    rig.position.set(0.34, -0.32, -0.58);
    rig.rotation.set(-0.25, -0.15, -0.28);
    camera.add(rig);
    scene.add(camera);

    // Slash arc.
    slash = new THREE.Mesh(
      new THREE.TorusGeometry(0.85, 0.035, 8, 40, Math.PI * 1.15),
      new THREE.MeshBasicMaterial({ color: 0xbdf6ff, transparent: true, opacity: 0.8, blending: THREE.AdditiveBlending, depthWrite: false })
    );
    slash.position.set(0.15, -0.1, -1.1);
    slash.rotation.z = -0.6;
    slash.visible = false;
    camera.add(slash);
    return rig;
  }

  function makeEnemyView(i) {
    const paths = models.manifest.models.filter((p) => p.includes('/characters/'));
    let g;
    if (paths.length && models[paths[i % paths.length]]) {
      g = prepModel(models[paths[i % paths.length]].clone(true), 1.85);
      g.traverse((o) => {
        if (o.isMesh && o.material) {
          o.material = o.material.clone();
          if (o.material.emissive) o.material.emissive.setHex(0x120a1c);
        }
      });
    } else {
      g = new THREE.Mesh(new THREE.CapsuleGeometry(0.34, 1.0, 4, 10),
        new THREE.MeshStandardMaterial({ color: 0x40304f, roughness: 0.9 }));
      g.position.y = 0.9;
    }
    const eyes = new THREE.Mesh(new THREE.SphereGeometry(0.075, 8, 8),
      new THREE.MeshBasicMaterial({ color: 0xff65d2 }));
    eyes.position.set(0, 1.62, 0.18);
    g.add(eyes);
    const shadow = new THREE.Mesh(new THREE.CircleGeometry(0.45, 16),
      new THREE.MeshBasicMaterial({ color: 0x000000, transparent: true, opacity: 0.3, depthWrite: false }));
    shadow.rotation.x = -Math.PI / 2;
    shadow.position.y = 0.015;
    g.add(shadow);

    let mixer = null;
    if (animClips.length && paths.length) {
      try {
        mixer = new THREE.AnimationMixer(g);
        mixer.clipAction(animClips[i % animClips.length]).play();
      } catch (_) { mixer = null; }
    }
    scene.add(g);
    return { group: g, mixer, eye: eyes };
  }

  function makePickupView() {
    const g = new THREE.Group();
    const core = new THREE.Mesh(new THREE.IcosahedronGeometry(0.28, 1),
      new THREE.MeshStandardMaterial({ color: 0x8dff9c, emissive: 0x2faf55, emissiveIntensity: 1.4, roughness: 0.25 }));
    const halo = new THREE.Sprite(new THREE.SpriteMaterial({
      map: glowTexture('rgba(140,255,170,1)'), transparent: true, blending: THREE.AdditiveBlending, depthWrite: false,
    }));
    halo.scale.set(1.6, 1.6, 1);
    g.add(core, halo);
    scene.add(g);
    return { group: g, core };
  }

  // ---------- wasm / gameplay ----------
  function readPlayer() {
    const p = ex.player_ptr();
    return {
      x: dv.getFloat32(p + 0, true), z: dv.getFloat32(p + 4, true), y: dv.getFloat32(p + 8, true),
      yaw: dv.getFloat32(p + 12, true), pitch: dv.getFloat32(p + 16, true),
      attack: dv.getFloat32(p + 40, true),
    };
  }

  function readFoes() {
    const base = ex.foes_ptr();
    const out = [];
    for (let i = 0; i < ex.foes_count(); i++) {
      const b = base + i * 36;
      out.push({
        kind: dv.getInt32(b, true), x: dv.getFloat32(b + 4, true), z: dv.getFloat32(b + 8, true),
        hp: dv.getFloat32(b + 16, true), phase: dv.getFloat32(b + 24, true),
        flash: dv.getFloat32(b + 28, true), yaw: dv.getFloat32(b + 32, true),
      });
    }
    return out;
  }

  function handleEvents() {
    const n = ex.event_count();
    if (!n) return;
    const p = ex.events_ptr();
    for (let i = 0; i < n; i++) {
      switch (dv.getInt32(p + i * 4, true)) {
        case 0: playSound('start'); break;
        case 2: playSound('swing'); break;
        case 3: playSound('hit'); break;
        case 4: playSound('kill'); break;
        case 5: playSound('hurt'); break;
        case 6: playSound('pickup'); break;
        case 7:
          playSound('hurt');
          mode = 3;
          ui.overText.textContent = `Вы набрали ${Math.round(ex.state(2))} очков, убили ${Math.round(ex.state(4))} теней и продержались ${Math.round(ex.state(5))} сек.`;
          show('over');
          if (document.pointerLockElement) document.exitPointerLock();
          break;
      }
    }
    ex.clear_events();
  }

  function inputFlags() {
    let f = 0;
    if (keys.has('w') || keys.has('arrowup')) f |= 1;
    if (keys.has('s') || keys.has('arrowdown')) f |= 2;
    if (keys.has('a') || keys.has('arrowleft')) f |= 4;
    if (keys.has('d') || keys.has('arrowright')) f |= 8;
    if (keys.has('shift')) f |= 16;
    if (mouseDown || keys.has(' ')) f |= 32;
    if (jumpQueued) { f |= 64; jumpQueued = false; }
    return f;
  }
  let jumpQueued = false;

  function syncWorld(dt) {
    const p = readPlayer();
    if (mode === 1) {
      camera.position.set(p.x, p.y, p.z);
      camera.rotation.order = 'YXZ';
      camera.rotation.set(p.pitch, p.yaw, 0);
      const shake = ex.state(8);
      if (shake > 0) {
        camera.position.x += (Math.random() - 0.5) * shake * 0.7;
        camera.position.y += (Math.random() - 0.5) * shake * 0.5;
      }
    } else {
      ambientTime += dt * 0.08;
      camera.position.set(Math.sin(ambientTime) * 13, 3.4 + Math.sin(ambientTime * 1.7) * 0.5, Math.cos(ambientTime) * 13);
      camera.lookAt(0, 2.2, 0);
    }

    if (swordRig) {
      const a = p.attack;
      swordRig.rotation.z = -0.28 - Math.sin(a * Math.PI) * 1.35;
      swordRig.rotation.x = -0.25 + Math.sin(a * Math.PI) * 0.45;
      swordRig.position.z = -0.58 - Math.sin(a * Math.PI) * 0.35;
      if (slash) {
        slash.visible = a > 0.12;
        slash.rotation.z = -0.6 + (1 - a) * 1.7;
        slash.material.opacity = Math.min(0.85, a * 1.2);
      }
    }

    // Foes.
    const foes = readFoes();
    const active = new Set();
    for (let i = 0; i < foes.length; i++) {
      const f = foes[i];
      active.add(i);
      if (f.kind === 3) {
        if (!pickupViews.has(i)) pickupViews.set(i, makePickupView());
        const v = pickupViews.get(i);
        v.group.position.set(f.x, f.phase ? f.hp + 0.55 + Math.sin(f.phase * 2) * 0.2 : 0.7, f.z);
        v.group.rotation.y += dt * 1.5;
        continue;
      }
      if (!enemyViews.has(i)) enemyViews.set(i, makeEnemyView(i));
      const v = enemyViews.get(i);
      v.group.position.set(f.x, Math.abs(Math.sin(f.phase * 2.4)) * 0.06, f.z);
      v.group.rotation.y = f.yaw + Math.PI / 2;
      if (v.mixer) v.mixer.update(dt);
      else v.group.rotation.z = Math.sin(f.phase * 3) * 0.06;
      if (v.eye) v.eye.scale.setScalar(1 + Math.sin(f.phase * 6) * 0.25);
      v.group.traverse((o) => {
        if (o.isMesh && o.material && o.material.emissive) {
          o.material.emissive.setHex(f.flash > 0 ? 0xff3355 : 0x120a1c);
        }
      });
    }
    for (const [k, v] of enemyViews) if (!active.has(k)) { scene.remove(v.group); enemyViews.delete(k); }
    for (const [k, v] of pickupViews) if (!active.has(k)) { scene.remove(v.group); pickupViews.delete(k); }

    if (sky) sky.position.copy(camera.position);
    sun.target.position.set(p.x, 0, p.z);
    sun.position.set(p.x + 38, 62, p.z - 52);

    const hit = ex.state(9) > 0;
    ui.hit.style.opacity = hit ? '1' : '0';
    ui.vignette.style.opacity = String(Math.min(1, ex.state(10) * 1.3));
  }

  function updateHud() {
    ui.hp.style.width = `${Math.max(0, ex.state(0))}%`;
    ui.st.style.width = `${Math.max(0, ex.state(1))}%`;
    ui.hpText.textContent = Math.round(ex.state(0));
    ui.stText.textContent = Math.round(ex.state(1));
    ui.score.textContent = Math.round(ex.state(2));
    ui.wave.textContent = `Волна ${Math.round(ex.state(3))} · врагов ${Math.round(ex.state(12))}`;
    const t = Math.floor(ex.state(5));
    ui.kills.textContent = `Убито ${Math.round(ex.state(4))} · ${String(Math.floor(t / 60)).padStart(2, '0')}:${String(t % 60).padStart(2, '0')}`;
  }

  function frame() {
    requestAnimationFrame(frame);
    const dt = Math.min(clock.getDelta(), 0.05);
    if (!ex) return;
    if (mode === 1) {
      const mdy = S.invert ? -mouseDY : mouseDY;
      ex.step(dt, inputFlags(), mouseDX, mdy);
      mouseDX = 0; mouseDY = 0;
    } else { mouseDX = 0; mouseDY = 0; }
    handleEvents();
    syncWorld(dt);
    if (mode !== 0) updateHud();
    renderer.render(scene, camera);
  }

  // ---------- UI actions ----------
  function startGame() {
    hideAll();
    hud.classList.add('on');
    ex.set_mode(1);
    ex.reset_game();
    mode = 1;
    controls.lock();
  }

  function toMenu() {
    mode = 0;
    ex.set_mode(0);
    hud.classList.remove('on');
    show('main');
    if (document.pointerLockElement) document.exitPointerLock();
  }

  function applySettings() {
    ex.set_sensitivity(S.sens);
    camera.fov = S.fov;
    camera.updateProjectionMatrix();
    audioVolume = S.volume;
    renderer.shadowMap.enabled = S.shadows;
    scene.fog = S.fog ? fog : null;
    renderer.setPixelRatio(Math.min(window.devicePixelRatio || 1, S.quality === 2 ? 2 : S.quality === 1 ? 1.5 : 1));
    scene.traverse((o) => { if (o.isMesh && o.material) o.material.needsUpdate = true; });
  }

  document.addEventListener('click', (e) => {
    const btn = e.target.closest('[data-action]');
    if (!btn) return;
    const a = btn.dataset.action;
    if (a === 'play') startGame();
    else if (a === 'resume') {
      hideAll(); mode = 1; ex.set_mode(1); controls.lock();
    } else if (a === 'menu') toMenu();
    else if (a === 'settings') {
      settingsReturn = !overlays.pause.classList.contains('hidden') ? 'pause' : 'main';
      show('settings');
    } else if (a === 'settingsBack') {
      applySettings();
      show(settingsReturn);
    } else if (a === 'about') {
      document.querySelector('#mainMenu .sub').textContent =
        'Полноценный лес из 3D-ассетов архива: деревья, валуны, грибы, руины, меч и анимированные персонажи. Логика боя и волн написана на Rust и выполняется в WebAssembly.';
    }
  });

  const bind = (id, key, parse = parseFloat) => {
    const el = $(id);
    const handler = () => { S[key] = parse(el.type === 'checkbox' ? el.checked : el.value); applySettings(); };
    el.addEventListener('input', handler);
    el.addEventListener('change', handler);
  };
  bind('sens', 'sens'); bind('fov', 'fov'); bind('volume', 'volume');
  bind('quality', 'quality', (v) => parseInt(v, 10));
  bind('shadows', 'shadows', (v) => !!v);
  bind('fog', 'fog', (v) => !!v);
  bind('invert', 'invert', (v) => !!v);

  window.addEventListener('keydown', (e) => {
    const k = e.key.toLowerCase();
    keys.add(k);
    if (k === ' ') { e.preventDefault(); jumpQueued = true; }
    if (k === 'escape' && mode === 1) {
      mode = 2; ex.set_mode(2); show('pause');
    }
  });
  window.addEventListener('keyup', (e) => keys.delete(e.key.toLowerCase()));
  document.addEventListener('mousemove', (e) => {
    if (mode !== 1) return;
    mouseDX += e.movementX || 0;
    mouseDY += e.movementY || 0;
  });
  document.addEventListener('mousedown', (e) => { if (e.button === 0 && mode === 1) mouseDown = true; });
  document.addEventListener('mouseup', (e) => { if (e.button === 0) mouseDown = false; });

  // ---------- boot ----------
  async function boot() {
    initRenderer();
    const manifestPath = 'models/manifest.json';
    const manifest = await fetch(assetUrl(manifestPath)).then((r) => r.json());
    models.manifest = manifest;

    const imagePaths = ['textures/grass.png', 'textures/leaves.png', 'textures/rocks.png'];
    const loadedImages = await Promise.all(imagePaths.map(loadImage));
    imagePaths.forEach((p, i) => { images[p.split('/').pop()] = loadedImages[i]; });

    ['start', 'swing', 'hit', 'kill', 'hurt', 'pickup'].forEach((n, i) => {
      loadSound(n, ['sfx_start.ogg', 'sfx_shoot.ogg', 'sfx_hit.ogg', 'sfx_confirm.ogg', 'sfx_error.ogg', 'sfx_select.ogg'][i]);
    });

    const wasmBuffer = WASM_B64
      ? base64Bytes(WASM_B64)
      : await fetch('core.wasm').then((r) => r.arrayBuffer());
    const { instance } = await WebAssembly.instantiate(wasmBuffer, {});
    ex = instance.exports;
    dv = new DataView(ex.memory.buffer);
    ex.init_game();
    ex.set_sensitivity(S.sens);

    const gltfPaths = manifest.models;
    const loaded = await Promise.all(gltfPaths.map((p) => loadModel(p).catch((err) => {
      console.warn('model failed:', p, err);
      return null;
    })));
    gltfPaths.forEach((p, i) => { if (loaded[i]) models[p] = loaded[i]; });
    const animPath = gltfPaths.find((p) => p.endsWith('.glb'));
    if (animPath && models[animPath]) animClips = models[animPath].animations || [];

    buildWorld();
    swordRig = buildSword();
    ready = true;
    loading.classList.add('hidden');
    clock.start();
    requestAnimationFrame(frame);
  }

  boot().catch((err) => {
    loading.textContent = 'Ошибка загрузки: ' + (err && err.message ? err.message : err);
    console.error(err);
  });
})();
