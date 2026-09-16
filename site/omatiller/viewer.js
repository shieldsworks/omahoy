import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { RoomEnvironment } from 'three/addons/environments/RoomEnvironment.js';
import { mergeGeometries } from 'three/addons/utils/BufferGeometryUtils.js';

const stage = document.getElementById('stage');
const status = document.getElementById('load-state');
const travel = document.getElementById('travel');
const travelValue = document.getElementById('travel-value');
const runButton = document.getElementById('run');
const orbitButton = document.getElementById('orbit');
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const descriptions = {
  housing: ['01 / THE OUTER SHELL', 'Made to come apart.', 'A rounded housing, a separate service cover, and an accessible interior. The concept puts the drive, bearings, and electronics within reach, with a gasket at the cover and a wiper around the sliding rod.', 'Pale shell · graphite cover · replaceable seals'],
  drive: ['02 / THE LINEAR DRIVE', 'A turn becomes a correction.', 'A rotating screw moves a captive nut and the hollow pushrod. Two guide rails keep the carriage aligned. The front bushing supports the rod; a wiper sits at the point where it meets the weather.', 'Screw + traveling nut · twin guide rails · front bushing'],
  motor: ['03 / THE TRANSMISSION', 'Power, tucked alongside.', 'A motor sits beneath the screw, connected by a compact belt drive. The parallel layout keeps the unit short and leaves the transmission accessible. Motor size and reduction are provisional in this first study.', 'Parallel motor · flanged pulleys · timing belt'],
  electronics: ['04 / THE CONTROL LOOP', 'The intelligence stays aboard.', 'The planned local controller combines heading and rate of turn with drive feedback. It will handle motor output, travel limits, and faults without depending on a running desktop app. The board shown here is a layout concept.', 'Original Rust firmware planned · local control · physical standby'],
  ram: ['05 / THE CONNECTION', 'The tiller is still yours.', 'A polished sliding tube ends in a removable tiller attachment. The opposite end pivots in a cockpit socket. The orange release detail marks the manual connection; its mechanism and release under load still need prototyping.', '250 mm concept travel · removable attachment · pivoting base'],
};

let renderer, scene, camera, controls, root, data;
let running = false, orbiting = false, visible = true, frame = 0, previous = 0;
let view = 'assembled', explosion = 0, desiredExplosion = 0, selected = null;
let phase = Math.PI / 2, travelMm = 125;
const components = [];
const movingPivots = [];

function fail(message) {
  stage.classList.remove('ready');
  status.textContent = message;
  if (renderer) renderer.domElement.hidden = true;
  document.getElementById('gesture').hidden = true;
  document.querySelectorAll('.controls button, .controls input, .view-tools button').forEach(el => el.disabled = true);
  running = orbiting = false;
  if (frame) cancelAnimationFrame(frame);
  frame = 0;
}

function requestDraw() {
  if (!frame && visible && !document.hidden && renderer) frame = requestAnimationFrame(draw);
}

function setTravel(mm) {
  travelMm = Math.max(0, Math.min(data.stroke, mm));
  travel.value = Math.round(travelMm);
  travelValue.textContent = `${Math.round(travelMm)} mm`;
  travel.setAttribute('aria-valuetext', `${Math.round(travelMm)} millimeters`);
  requestDraw();
}

function setRunning(value) {
  running = value;
  runButton.setAttribute('aria-pressed', String(value));
  runButton.setAttribute('aria-label', value ? 'Pause ram travel' : 'Animate ram travel');
  runButton.textContent = value ? 'Ⅱ' : '▶';
  phase = Math.asin(Math.max(-1, Math.min(1, (travelMm - 125) / 125)));
  requestDraw();
}

function setOrbit(value) {
  orbiting = value;
  orbitButton.setAttribute('aria-pressed', String(value));
  orbitButton.textContent = value ? 'Ⅱ Stop rotation' : '↻ Rotate';
  requestDraw();
}

function setView(name) {
  view = name;
  stage.dataset.view = name;
  desiredExplosion = name === 'exploded' ? 1 : 0;
  if (reducedMotion.matches) explosion = desiredExplosion;
  document.querySelectorAll('[data-view]').forEach(button => {
    if (button.tagName === 'BUTTON') button.setAttribute('aria-pressed', String(button.dataset.view === name));
  });
  updateMaterials();
  requestDraw();
}

function updateMaterials() {
  for (const component of components) {
    const { group, material: materialName } = component.meta;
    const ghost = view === 'inside' && ['housing', 'cover'].includes(group);
    const chosen = selected && (group === selected || (selected === 'motor' && group === 'transmission') || (selected === 'drive' && group === 'guide'));
    component.node.traverse(mesh => {
      if (!mesh.isMesh) return;
      mesh.material.transparent = ghost || materialName === 'label';
      mesh.material.opacity = ghost ? .075 : 1;
      mesh.material.depthWrite = !ghost && materialName !== 'label';
      mesh.castShadow = !ghost;
      if (mesh.material.emissive) {
        mesh.material.emissive.set(chosen ? '#704022' : '#000000');
        mesh.material.emissiveIntensity = chosen ? .16 : 0;
        if (materialName === 'orange') mesh.material.emissiveIntensity = chosen ? .2 : 0;
      }
      mesh.material.needsUpdate = true;
    });
  }
}

function selectPart(name, changeView = true) {
  if (!descriptions[name]) name = ({ cover: 'housing', guide: 'drive', transmission: 'motor', mount: 'ram' })[name] || 'housing';
  selected = name;
  const text = descriptions[name];
  ['part-kicker', 'part-title', 'part-description', 'part-note'].forEach((id, i) => document.getElementById(id).textContent = text[i]);
  document.querySelectorAll('[data-part]').forEach(button => button.setAttribute('aria-pressed', String(button.dataset.part === name)));
  if (root && changeView) setView(name === 'housing' || name === 'ram' ? 'assembled' : 'exploded');
  if (root) updateMaterials();
  requestDraw();
}
document.querySelectorAll('[data-part]').forEach(button => button.addEventListener('click', () => selectPart(button.dataset.part)));

function home() {
  const portrait = stage.clientWidth < 600;
  camera.position.set(portrait ? 450 : 370, portrait ? 700 : 330, portrait ? 550 : 630);
  controls.target.set(10, 0, 0);
  camera.zoom = 1;
  camera.updateProjectionMatrix();
  controls.update();
  setOrbit(false);
  requestDraw();
}

function size() {
  if (!renderer) return;
  const width = stage.clientWidth, height = stage.clientHeight;
  renderer.setSize(width, height, false);
  // Keep the full assembly in frame on a narrow screen without wasting desktop space.
  const halfWidth = Math.max(width < 600 ? 350 : 400, 235 * width / height);
  camera.left = -halfWidth; camera.right = halfWidth;
  camera.top = halfWidth * height / width; camera.bottom = -camera.top;
  camera.updateProjectionMatrix();
  requestDraw();
}

function label(text, subtext, position, width, height) {
  const canvas = document.createElement('canvas');
  canvas.width = 1024; canvas.height = 256;
  const context = canvas.getContext('2d');
  context.fillStyle = '#e6e9e0';
  context.font = 'bold 104px monospace';
  context.textAlign = 'center';
  context.fillText(text, 512, 122);
  context.fillStyle = '#91a6b2';
  context.font = '30px monospace';
  context.fillText(subtext, 512, 191);
  const texture = new THREE.CanvasTexture(canvas);
  texture.colorSpace = THREE.SRGBColorSpace;
  texture.anisotropy = Math.min(4, renderer.capabilities.getMaxAnisotropy());
  const mesh = new THREE.Mesh(new THREE.PlaneGeometry(width / 1000, height / 1000), new THREE.MeshBasicMaterial({ map: texture, transparent: true, depthWrite: false, polygonOffset: true, polygonOffsetFactor: -1 }));
  mesh.position.set(...position.map(v => v / 1000));
  root.add(mesh);
  components.push({ node: mesh, base: mesh.position.clone(), meta: { group: 'cover', material: 'label', explode: [0, 0, 95] } });
}

function draw(now) {
  frame = 0;
  const dt = Math.min((now - (previous || now)) / 1000, .05);
  previous = now;
  if (running) { phase += dt * .75; setTravel(125 + Math.sin(phase) * 125); }
  const delta = desiredExplosion - explosion;
  if (Math.abs(delta) > .001) explosion += delta * Math.min(1, dt * 7);
  else explosion = desiredExplosion;
  for (const component of components) {
    component.node.position.copy(component.base);
    const e = component.meta.explode || [0, 0, 0];
    component.node.position.addScaledVector(new THREE.Vector3(...e), explosion / 1000);
    if (component.meta.moving) component.node.position.x += (travelMm - 125) / 1000;
  }
  for (const pivot of movingPivots) pivot.rotation.x = -(travelMm - 125) / data.screwLead * Math.PI * 2;
  if (orbiting) {
    const offset = camera.position.clone().sub(controls.target);
    offset.applyAxisAngle(new THREE.Vector3(0, 1, 0), dt * .2);
    camera.position.copy(controls.target).add(offset);
  }
  controls.update();
  renderer.render(scene, camera);
  if (running || orbiting || Math.abs(desiredExplosion - explosion) > .001) requestDraw();
}

async function start() {
  renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'low-power' });
  renderer.setPixelRatio(Math.min(devicePixelRatio, 1.6));
  renderer.setClearColor(0x000000, 0);
  renderer.outputColorSpace = THREE.SRGBColorSpace;
  renderer.toneMapping = THREE.ACESFilmicToneMapping;
  renderer.toneMappingExposure = 1.1;
  renderer.shadowMap.enabled = true;
  renderer.shadowMap.type = THREE.PCFSoftShadowMap;
  const canvas = renderer.domElement;
  canvas.tabIndex = 0;
  canvas.setAttribute('role', 'img');
  canvas.setAttribute('aria-label', 'Interactive Omatiller assembly. Drag to orbit, scroll to zoom. With the model focused, use arrow keys to orbit, plus or minus to zoom, and Home to reset.');
  canvas.addEventListener('webglcontextlost', event => { event.preventDefault(); fail('3D paused · reload the page to restore the interactive view.'); });
  stage.appendChild(canvas);
  scene = new THREE.Scene();
  camera = new THREE.OrthographicCamera(-400, 400, 240, -240, 1, 5000);
  controls = new OrbitControls(camera, canvas);
  controls.enableDamping = false;
  controls.enablePan = false;
  controls.minZoom = .65; controls.maxZoom = 2.5;
  controls.minPolarAngle = .1; controls.maxPolarAngle = Math.PI * .85;
  controls.addEventListener('change', requestDraw);
  controls.addEventListener('start', () => setOrbit(false));

  const pmrem = new THREE.PMREMGenerator(renderer);
  const room = new RoomEnvironment();
  const environment = pmrem.fromScene(room, .04);
  scene.environment = environment.texture;
  room.dispose(); pmrem.dispose();
  scene.add(new THREE.HemisphereLight(0xc8e4fa, 0x293b4c, 1.7));
  const key = new THREE.DirectionalLight(0xffefdc, 3);
  key.position.set(-200, 500, 300);
  key.castShadow = true;
  key.shadow.mapSize.set(1024, 1024);
  Object.assign(key.shadow.camera, { left: -650, right: 650, top: 450, bottom: -450, near: 1, far: 1500 });
  key.shadow.bias = -.0005; key.shadow.normalBias = 1;
  scene.add(key);
  const rim = new THREE.DirectionalLight(0xb6ddff, 2);
  rim.position.set(150, 100, -400); scene.add(rim);
  const floor = new THREE.Mesh(new THREE.PlaneGeometry(2500, 1600), new THREE.ShadowMaterial({ opacity: .08 }));
  floor.rotation.x = -Math.PI / 2;
  floor.position.y = -170; floor.receiveShadow = true; scene.add(floor);
  const shadowCanvas = document.createElement('canvas');
  shadowCanvas.width = shadowCanvas.height = 128;
  const shadowContext = shadowCanvas.getContext('2d');
  const shadowGradient = shadowContext.createRadialGradient(64, 64, 4, 64, 64, 64);
  shadowGradient.addColorStop(0, '#00000066'); shadowGradient.addColorStop(1, '#00000000');
  shadowContext.fillStyle = shadowGradient; shadowContext.fillRect(0, 0, 128, 128);
  const softShadow = new THREE.Mesh(new THREE.PlaneGeometry(860, 300), new THREE.MeshBasicMaterial({ map: new THREE.CanvasTexture(shadowCanvas), transparent: true, depthWrite: false }));
  softShadow.rotation.x = -Math.PI / 2; softShadow.position.y = -169; scene.add(softShadow);

  const [gltf, response] = await Promise.all([
    new GLTFLoader().loadAsync('./model/omatiller-01.glb'),
    fetch('./model/omatiller-01.json').then(r => { if (!r.ok) throw new Error('Missing model metadata'); return r.json(); }),
  ]);
  data = response;
  scene.add(gltf.scene);
  gltf.scene.scale.setScalar(1000);
  root = gltf.scene.getObjectByName('omatiller_01');
  if (!root) throw new Error('Missing assembly root');
  for (const [name, meta] of Object.entries(data.parts)) {
    const node = root.getObjectByName(name);
    if (!node) throw new Error(`Missing CAD component: ${name}`);
    // OCCT exports separate primitives for each face. Combine faces within each
    // named solid: 716 face draws become 69 component draws, preserving selection.
    if (!node.isMesh) {
      node.updateWorldMatrix(true, true);
      const inverse = node.matrixWorld.clone().invert();
      const faces = [];
      node.traverse(mesh => { if (mesh.isMesh) faces.push(mesh); });
      if (faces.length > 1) {
        const geometries = faces.map(mesh => {
          const geometry = mesh.geometry.clone();
          geometry.applyMatrix4(new THREE.Matrix4().multiplyMatrices(inverse, mesh.matrixWorld));
          return geometry;
        });
        const merged = mergeGeometries(geometries);
        geometries.forEach(geometry => geometry.dispose());
        if (!merged) throw new Error(`Cannot combine CAD faces for ${name}`);
        const material = faces[0].material;
        faces.forEach(mesh => mesh.removeFromParent());
        node.add(new THREE.Mesh(merged, material));
      }
    }
    node.traverse(mesh => {
      if (!mesh.isMesh) return;
      const color = mesh.material.color.clone();
      const metal = ['steel', 'bronze', 'silver'].includes(meta.material);
      mesh.material = new THREE.MeshStandardMaterial({ color, metalness: metal ? .82 : .12,
        roughness: meta.material === 'steel' ? .23 : metal ? .34 : .43,
        envMapIntensity: metal ? 1.4 : .6 });
      mesh.castShadow = true; mesh.receiveShadow = true;
      mesh.userData.part = meta.group;
    });
    if (name === 'screw_shaft' || name === 'screw_helix') {
      const pivot = new THREE.Group();
      pivot.position.z = .01;
      root.add(pivot); pivot.add(node); node.position.z -= .01;
      movingPivots.push(pivot);
    }
    components.push({ node, meta, base: node.position.clone() });
  }
  label('omatiller', 'OPEN MARINE HARDWARE / 01', [-106, 0, 40], 108, 27);
  // Rubber power lead, presented separately from the dimensioned component solids.
  const cablePoints = [[-294, 0, -17], [-310, 0, -20], [-326, 2, -45], [-352, 5, -64], [-380, 1, -62]].map(p => new THREE.Vector3(...p.map(v => v / 1000)));
  const cable = new THREE.Mesh(new THREE.TubeGeometry(new THREE.CatmullRomCurve3(cablePoints), 40, .004, 10, false), new THREE.MeshStandardMaterial({ color: '#18232a', roughness: .65 }));
  root.add(cable);
  components.push({ node: cable, base: cable.position.clone(), meta: { group: 'housing', material: 'rubber', explode: [-20, 0, -65] } });

  const raycaster = new THREE.Raycaster();
  let pointerStart = null;
  canvas.addEventListener('pointerdown', e => { pointerStart = [e.clientX, e.clientY]; });
  canvas.addEventListener('pointerup', e => {
    if (!pointerStart || Math.hypot(e.clientX - pointerStart[0], e.clientY - pointerStart[1]) > 5) return;
    const bounds = canvas.getBoundingClientRect();
    raycaster.setFromCamera(new THREE.Vector2((e.clientX - bounds.left) / bounds.width * 2 - 1, -(e.clientY - bounds.top) / bounds.height * 2 + 1), camera);
    const hit = raycaster.intersectObject(root, true).find(h => h.object.userData.part && !(view === 'inside' && ['housing', 'cover'].includes(h.object.userData.part)));
    if (hit) selectPart(hit.object.userData.part, false);
  });
  canvas.addEventListener('keydown', e => {
    if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown', '+', '=', '-', 'Home'].includes(e.key)) return;
    e.preventDefault(); setOrbit(false);
    if (e.key === 'Home') return home();
    if (['+', '=', '-'].includes(e.key)) {
      camera.zoom = THREE.MathUtils.clamp(camera.zoom * (e.key === '-' ? .9 : 1.1), .65, 2.5);
      camera.updateProjectionMatrix();
    } else {
      const spherical = new THREE.Spherical().setFromVector3(camera.position.clone().sub(controls.target));
      spherical.theta += e.key === 'ArrowLeft' ? -.12 : e.key === 'ArrowRight' ? .12 : 0;
      spherical.phi = THREE.MathUtils.clamp(spherical.phi + (e.key === 'ArrowUp' ? -.1 : e.key === 'ArrowDown' ? .1 : 0), .1, Math.PI * .85);
      camera.position.copy(controls.target).add(new THREE.Vector3().setFromSpherical(spherical));
    }
    controls.update(); requestDraw();
  });

  document.querySelectorAll('.controls button, .controls input, .view-tools button').forEach(el => el.disabled = false);
  document.querySelectorAll('button[data-view]').forEach(button => button.addEventListener('click', () => setView(button.dataset.view)));
  travel.addEventListener('input', () => { setRunning(false); setTravel(Number(travel.value)); });
  runButton.addEventListener('click', () => setRunning(!running));
  orbitButton.addEventListener('click', () => setOrbit(!orbiting));
  document.getElementById('reset').addEventListener('click', home);
  reducedMotion.addEventListener('change', () => {
    if (reducedMotion.matches) { setRunning(false); setOrbit(false); explosion = desiredExplosion; requestDraw(); }
  });
  document.addEventListener('visibilitychange', () => {
    if (document.hidden) { setRunning(false); setOrbit(false); }
    else { previous = 0; requestDraw(); }
  });
  new IntersectionObserver(entries => {
    visible = entries[0].isIntersecting;
    if (!visible) { setRunning(false); setOrbit(false); }
    else { previous = 0; requestDraw(); }
  }).observe(stage);
  new ResizeObserver(size).observe(stage);
  stage.classList.add('ready');
  stage.dataset.view = 'assembled';
  status.textContent = 'Interactive assembly ready.';
  home(); size(); setTravel(125);
}

start().catch(error => {
  console.error('Omatiller viewer:', error);
  fail('Static preview · interactive 3D is unavailable in this browser.');
});
