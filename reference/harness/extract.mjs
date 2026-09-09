#!/usr/bin/env node
// Golden-fixture extractor. Runs the frozen prototype (reference/hex-planet.html) in node:vm and
// writes the values the Rust crates must reproduce into fixtures/.
//   node reference/harness/extract.mjs          # writes fixtures/
//   node reference/harness/extract.mjs --full   # also dumps complete arrays into reference/harness/out/
// The reference is frozen: this script changes only to extract more, never to change what the
// prototype computes.
import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { createHash } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { makeContext } from './shims.mjs';
import { encodePng } from './png.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const repo = path.resolve(here, '..', '..');
const argv = process.argv.slice(2);
const FULL = argv.includes('--full');
const outIdx = argv.indexOf('--out');
const OUT = outIdx >= 0 ? path.resolve(argv[outIdx + 1]) : path.join(repo, 'fixtures');
const FULL_OUT = path.join(here, 'out');

const html = fs.readFileSync(path.join(here, '..', 'hex-planet.html'), 'utf8');
const match = html.match(/<script>([\s\S]*?)<\/script>\s*<\/body>/);
if (!match) throw new Error('inline script not found in hex-planet.html');
const source = match[1];
const ctx = makeContext();
const t0 = performance.now();
vm.runInContext(source, ctx, { filename: 'hex-planet.inline.js' });
console.log(`prototype loaded in ${(performance.now() - t0).toFixed(0)} ms`);
const G = (expr) => vm.runInContext(expr, ctx);

// ---------------------------------------------------------------- helpers
const failures = [];
function step(name, fn) {
  const t = performance.now();
  try {
    fn();
    console.log(`ok   ${name} (${(performance.now() - t).toFixed(0)} ms)`);
  } catch (e) {
    failures.push(name);
    console.error(`FAIL ${name}: ${e.stack || e}`);
  }
}
function write(rel, buf) {
  const p = path.join(OUT, rel);
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, buf);
}
const replacer = (_, v) => (typeof v === 'number' && !Number.isFinite(v) ? String(v) : v);
function writeJson(rel, obj) { write(rel, JSON.stringify(obj, replacer) + '\n'); }
function writeFull(rel, obj) {
  if (!FULL) return;
  const p = path.join(FULL_OUT, rel);
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, JSON.stringify(obj, replacer));
}
function writePng(rel, img) {
  const bytes = Buffer.from(img.data.buffer, img.data.byteOffset, img.data.byteLength);
  write(rel, encodePng(img.width, img.height, bytes));
}
const sha256 = (buf) => createHash('sha256').update(buf).digest('hex');
const jsRound = (x) => Math.floor(x + 0.5);
/** Hash of the little-endian f32 bytes: exact. */
const hashF32 = (a) => sha256(Buffer.from(a.buffer, a.byteOffset, a.byteLength));
/** Hash of the values rounded to 1e-6 (as integers): tolerant to last-bit float noise. */
const hashQ6 = (a) => {
  let s = '';
  for (let i = 0; i < a.length; i++) s += jsRound(a[i] * 1e6) + ',';
  return sha256(s);
};
function summarize(attr) {
  const a = attr.array;
  const sample = [];
  for (let i = 0; i < a.length; i += 97) sample.push(a[i]);
  return {
    itemSize: attr.itemSize, count: attr.count, len: a.length,
    sha256_f32: hashF32(a), sha256_q6: hashQ6(a),
    head: Array.from(a.subarray(0, 64)), sample,
  };
}
function geometry(g, fullRel) {
  const out = {};
  for (const [name, attr] of Object.entries(g.attributes)) out[name] = summarize(attr);
  if (fullRel) {
    const fullObj = {};
    for (const [name, attr] of Object.entries(g.attributes)) fullObj[name] = Array.from(attr.array);
    writeFull(fullRel, fullObj);
  }
  return out;
}
function textureMeta(t) {
  return {
    wrapS: t.wrapS, wrapT: t.wrapT, magFilter: t.magFilter, minFilter: t.minFilter,
    generateMipmaps: t.generateMipmaps,
    repeat: { x: t.repeat.x, y: t.repeat.y }, offset: { x: t.offset.x, y: t.offset.y },
  };
}
function materialMeta(m) {
  const p = { ...m.params };
  if (p.map) p.map = '<texture>';
  if (p.color && typeof p.color === 'object') p.color = p.color.value;
  return p;
}
/** Deterministic input generator, independent of the prototype. */
function lcg(seed) {
  let s = seed >>> 0;
  return () => { s = (Math.imul(s, 1664525) + 1013904223) >>> 0; return s / 4294967296; };
}

// ---------------------------------------------------------------- 0. constants
step('constants', () => {
  // Every UPPER_CASE name declared by a top-level `const` statement, including multi-name
  // statements like `const CLIFF_W = 24, CLIFF_H = 4;` and multi-line array literals.
  const names = [];
  const re = /^const\s/gm;
  let m;
  while ((m = re.exec(source))) {
    let depth = 0, i = m.index + m[0].length, start = i;
    for (; i < source.length; i++) {
      const ch = source[i];
      if (ch === '[' || ch === '{' || ch === '(') depth++;
      else if (ch === ']' || ch === '}' || ch === ')') depth--;
      else if (ch === ';' && depth === 0) break;
    }
    const stmt = source.slice(start, i);
    let d = 0;
    for (let k = 0; k < stmt.length; k++) {
      const ch = stmt[k];
      if (ch === '[' || ch === '{' || ch === '(') d++;
      else if (ch === ']' || ch === '}' || ch === ')') d--;
      else if (d === 0) {
        const id = /^([A-Z][A-Z0-9_]*)\s*=/.exec(stmt.slice(k));
        if (id && (k === 0 || /[\s,]/.test(stmt[k - 1]))) { names.push(id[1]); k += id[1].length - 1; }
      }
    }
  }
  const out = {};
  for (const n of names) { const v = G(n); if (typeof v !== 'function') out[n] = v; }
  writeJson('constants.json', out);
});

// ---------------------------------------------------------------- 1. noise & JS semantics
step('noise', () => {
  const hash3i = G('hash3i'), hash2 = G('hash2'), vnoise3 = G('vnoise3'), fbm3 = G('fbm3');
  const mulberry32 = G('mulberry32');
  const r = lcg(0xC1A1);
  const i32 = () => (r() * 4294967296 | 0);
  const small = () => Math.floor(r() * 2000) - 1000;
  const h3 = [];
  for (let i = 0; i < 400; i++) {
    const x = i % 2 ? i32() : small(), y = i % 3 ? small() : i32(), z = i % 5 ? small() : i32();
    h3.push({ in: [x, y, z], out: hash3i(x, y, z) });
  }
  const edge3 = [[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], [-1, -1, -1], [2147483647, -2147483648, 12345],
    [4294967297, -4294967297, 2147483648], [1e12, -1e12, 3.0], [8, 17, 320], [1000, 7, 2]];
  for (const [x, y, z] of edge3) h3.push({ in: [x, y, z], out: hash3i(x, y, z) });
  const h2 = [];
  for (let i = 0; i < 300; i++) {
    const x = (r() * 200 - 100) * 1.7, y = (r() * 50) * 3.1;
    h2.push({ in: [x, y], out: hash2(x, y) });
  }
  for (let x = 0; x < 24; x++) for (const y of [0, 1, 2, 3]) h2.push({ in: [x * 1.7, y * 3.1], out: hash2(x * 1.7, y * 3.1) });
  for (let x = 0; x < 24; x++) h2.push({ in: [x * 1.3, 7.7], out: hash2(x * 1.3, 7.7) });
  const vn = [];
  for (let i = 0; i < 300; i++) {
    const p = [r() * 40 - 20, r() * 40 - 20, r() * 40 - 20];
    vn.push({ in: p, out: vnoise3(p[0], p[1], p[2]) });
  }
  const dirs = [];
  const rd = lcg(0xD1F5);
  for (let i = 0; i < 80; i++) {
    const v = [rd() * 2 - 1, rd() * 2 - 1, rd() * 2 - 1];
    const l = Math.hypot(v[0], v[1], v[2]) || 1;
    dirs.push([v[0] / l, v[1] / l, v[2] / l]);
  }
  const fb = [];
  const variants = [[31676, 4, 5.5], [63352 + 8681, 3, 30], [1234, 2, 3.0], [(31676 | 0) ^ 0x5bf, 2, 11.0],
    [3521, 3, 3.4], [63352, undefined, undefined]];
  for (const [seed, oct, f0] of variants) {
    for (const p of dirs) fb.push({ p, seed, oct: oct ?? null, f0: f0 ?? null, out: fbm3(p, seed, oct, f0) });
  }
  const mb = [];
  const seeds = [0, 1, 31676, 63352, 1234, -5, 2147483647, (31676 ^ 0x5bf03635) | 0, 4294967295, 0x7fffffff + 2];
  for (const seed of seeds) {
    const g = mulberry32(seed);
    const out = [];
    for (let i = 0; i < 32; i++) out.push(g());
    mb.push({ seed, out });
  }
  writeJson('noise/hash3i.json', h3);
  writeJson('noise/hash2.json', h2);
  writeJson('noise/vnoise3.json', vn);
  writeJson('noise/fbm3.json', fb);
  writeJson('noise/mulberry32.json', mb);
});

step('js-semantics', () => {
  const r = lcg(0x5EED);
  const toInt32 = [3.7, -3.7, 0.5, -0.5, 4294967296, 4294967297, -4294967297, 2147483648, 2147483647.9,
    -2147483649, 1e12, -1e12, 1e21, NaN, Infinity, -Infinity, -0, 0.9999999, 4294967295.5, 1e-9]
    .map((x) => ({ in: x, out: x | 0 }));
  const round = [0.5, 1.5, 2.5, -0.5, -1.5, -2.5, 0.49999999999999994, -0.49999999999999994,
    2.4999999999999996, 7.5000000000000001, -7.5, 1e15 + 0.5, 3.25, -3.25]
    .map((x) => ({ in: x, out: Math.round(x) }));
  const toFixed6 = [0.1234565, 0.1234575, 1e-7, -1e-7, -1e-10, 0.5, 123456.123456789, -0.0000004, 5e-7,
    1.0000005, 0.9999995, 0.000001, -0, 0.000000499999, 0.5257311121191336, -0.8506508083520399,
    0.30901699437494745, 2.5e-7, 1.5e-6, -2.5e-7].map((x) => ({ in: x, out: x.toFixed(6) }));
  const hypot3 = [];
  for (let i = 0; i < 200; i++) {
    const v = [r() * 2 - 1, r() * 2 - 1, r() * 2 - 1];
    hypot3.push({ in: v, out: Math.hypot(v[0], v[1], v[2]) });
  }
  const edgeH = [[0, 0, 0], [3, 4, 0], [1e-200, 1e-200, 1e-200], [1e200, 1e200, 0], [1, 1, 1], [0.1, 0.2, 0.3],
    [-0.5, 0.25, 0.125], [Infinity, 1, NaN], [NaN, 1, 2]];
  for (const v of edgeH) hypot3.push({ in: v, out: Math.hypot(v[0], v[1], v[2]) });
  const hypot2 = [];
  for (let i = 0; i < 100; i++) {
    const v = [r() * 20 - 10, r() * 20 - 10];
    hypot2.push({ in: v, out: Math.hypot(v[0], v[1]) });
  }
  const imul = [];
  for (let i = 0; i < 100; i++) {
    const a = r() * 4294967296 | 0, b = r() * 4294967296 | 0;
    imul.push({ in: [a, b], out: Math.imul(a, b) });
  }
  const ushr = [-1, -2, 2147483647, -2147483648, 12345, -12345, 4294967295, 1e10]
    .flatMap((x) => [7, 13, 14, 15, 16].map((s) => ({ in: [x, s], out: x >>> s })));
  const trig = [];
  const addTrig = (x, y, u) => trig.push({
    x, y, u, sin: Math.sin(x), cos: Math.cos(x), atan2: Math.atan2(y, x), acos: Math.acos(y),
    pow17: Math.pow(u, 1.7), pow22: Math.pow(u, 2.2), pow16: Math.pow(u, 1.6), sqrt: Math.sqrt(u),
  });
  for (let i = 0; i < 200; i++) addTrig(r() * 40 - 20, r() * 2 - 1, r());
  for (const x of [0, Math.PI, -Math.PI, Math.PI / 2, 1e-9, 100, 1000000, 0.1, 43758.5453]) addTrig(x, 0.5, 0.5);
  // Past 2^19*(pi/2) the reduction switches from the 3-step pi/2 subtraction to the 2/pi table,
  // so these rows pin __kernel_rem_pio2: the boundary and its neighbours, exact powers of two, the
  // worst case for the reduction, and the largest finite double. The last two are the other end,
  // where the kernels return the argument untouched: the smallest subnormal and the smallest normal.
  const big = 524288 * Math.PI / 2;
  for (const x of [big, big + 1, big * 1.0000001, 1e6, 1e7, 12345678.9, 1e13, 1e15, 1e17, 1e22,
    2 ** 30, 2 ** 52, 2 ** 60, 2 ** 120, 2 ** 500, 6381956970095103 * 2 ** 797, 1.7976931348623157e308,
    5e-324, 2.2250738585072014e-308, 1e100 * Math.PI / 2, 123456789 * Math.PI / 2]) {
    addTrig(x, 0.5, 0.5);
    addTrig(-x, -0.5, 0.25);
  }
  const sinHash = [];
  for (let i = 0; i < 100; i++) {
    const x = r() * 100000;
    const s = Math.sin(x) * 43758.5453;
    sinHash.push({ x, sin: Math.sin(x), fract: s - Math.floor(s) });
  }
  writeJson('noise/js-semantics.json', { toInt32, round, toFixed6, hypot3, hypot2, imul, ushr, trig, sinHash });
});

// ---------------------------------------------------------------- 2. hex sphere
step('hexsphere', () => {
  const buildHexSphere = G('buildHexSphere'), tilesAround = G('tilesAround');
  const geodesic = G('geodesic'), icosahedron = G('icosahedron');
  writeJson('hexsphere/icosahedron.json', icosahedron());
  for (let n = 2; n <= 12; n++) {
    const tiles = buildHexSphere(n);
    const geo = geodesic(n);
    const pent = tiles.filter((t) => t.sides === 5).map((t) => t.id);
    const floats = new Float64Array(tiles.length * 9);
    tiles.forEach((t, i) => { floats.set(t.center, i * 9); floats.set(t.e1, i * 9 + 3); floats.set(t.e2, i * 9 + 6); });
    const corners = [];
    for (const t of tiles) for (const c of t.corners) corners.push(...c);
    const out = {
      n, count: tiles.length, expected: 10 * n * n + 2, pentagons: pent, around: tilesAround(tiles),
      geodesic: { verts: geo.verts.length, tris: geo.tris.length },
      sha256_q6_centers_e1_e2: hashQ6(floats), sha256_q6_corners: hashQ6(Float64Array.from(corners)),
      sides: tiles.map((t) => t.sides),
      neighbors: tiles.map((t) => t.neighbors),
      edgeNeighbors: tiles.map((t) => t.edgeNeighbors),
      cornerTiles: tiles.map((t) => t.cornerTiles),
    };
    const tileData = tiles.map((t) => ({ id: t.id, center: t.center, corners: t.corners, e1: t.e1, e2: t.e2 }));
    if (n <= 6) {
      out.tiles = tileData;
      out.geodesicVerts = geo.verts;
      out.geodesicTris = geo.tris;
    }
    writeJson(`hexsphere/n${n}.json`, out);
    writeFull(`hexsphere/n${n}.json`, { tiles: tileData, geodesic: geo });
  }
});

// ---------------------------------------------------------------- 3. worlds
const WORLDS = [[4, 31676], [4, 1234], [8, 63352]];
step('worlds', () => {
  const makeWorld = G('makeWorld'), buildAtmosphere = G('buildAtmosphere');
  const buildCloudShell = G('buildCloudShell'), facetPlane = G('facetPlane');
  const texelDir = G('texelDir'), coverLift = G('coverLift');
  for (const [n, seed] of WORLDS) {
    const tag = `n${n}-s${seed}`;
    const w = makeWorld(n, seed);
    const tiles = w.tiles;
    const isSmall = n <= 4;
    writeJson(`worldgen/${tag}.json`, {
      n, seed, level: tiles.map((t) => t.level), cover: tiles.map((t) => t.cover),
      land: tiles.filter((t) => t.level === 0).length,
      counts: {
        fields: tiles.filter((t) => t.cover === 'fields').length,
        forest: tiles.filter((t) => t.cover === 'forest').length,
        houses: tiles.filter((t) => t.cover === 'houses').length,
      },
    });
    const frames = tiles.map((t) => ({
      radius: t.radius, mid: t.mid, normal: t.normal, apothem: t.apothem, texel: t.texel,
      unitCircum: t.unitCircum, plane: facetPlane(t),
    }));
    const tex0 = [];
    for (const t of tiles.slice(0, 3)) {
      const pl = facetPlane(t);
      for (let px = 0; px < 24; px++) tex0.push(texelDir(t, px, 0, pl));
    }
    const frameFloats = Float64Array.from(frames.flatMap((f) =>
      [f.radius, ...f.mid, ...f.normal, f.apothem, f.texel, f.unitCircum, f.plane.d, f.plane.dc]));
    writeJson(`scenery/${tag}-frames.json`, {
      n, seed, px: w.px, pent: w.pent, around: w.around,
      frames: isSmall ? frames : undefined, framesHash: hashQ6(frameFloats),
      texelDirTiles0to2Row0: tex0,
    });
    const img = w.ground.tex.canvas.__image;
    writePng(`pixelart/atlas-${tag}.png`, img);
    writeJson(`pixelart/atlas-${tag}.json`, {
      n, seed, cols: w.ground.cols, rows: w.ground.rows, size: w.ground.size,
      cell: tiles.map((t) => [t.cellCol, t.cellRow]),
      prox: tiles.map((t) => t.prox), shallow: tiles.map((t) => t.shallow),
      cornerProx: tiles.map((t) => t.cornerProx), cornerShallow: tiles.map((t) => t.cornerShallow),
      built: tiles.map((t) => t.built), cornerBuilt: tiles.map((t) => t.cornerBuilt),
      texture: textureMeta(w.ground.tex),
    });
    const fullTag = isSmall ? `scenery/${tag}` : null;
    const F = (name) => (fullTag ? `${fullTag}-${name}.json` : null);
    writeJson(`scenery/${tag}-terrain.json`, {
      n, seed, px: w.px,
      mesh: geometry(w.mesh.geometry, F('mesh')), meshMaterial: materialMeta(w.mesh.material),
      walls: geometry(w.walls.geometry, F('walls')), wallsMaterial: materialMeta(w.walls.material),
      foam: geometry(w.foam.geometry, F('foam')), foamMaterial: materialMeta(w.foam.material),
      foamRenderOrder: w.foam.renderOrder,
      edges: geometry(w.edges.geometry, F('edges')), edgesMaterial: materialMeta(w.edges.material),
      faceTile: isSmall ? w.faceTile : undefined, faceTileHash: sha256(w.faceTile.join(',')), faceTileLen: w.faceTile.length,
      wallTile: isSmall ? w.wallTile : undefined, wallTileHash: sha256(w.wallTile.join(',')), wallTileLen: w.wallTile.length,
      vertexStart: tiles.map((t) => t.vertexStart), vertexCount: tiles.map((t) => t.vertexCount),
    });
    const f = w.fields, fo = w.forest, h = w.houses;
    writeJson(`scenery/${tag}-cover.json`, {
      n, seed,
      fields: f ? {
        zones: f.zones, parcels: f.parcels, fences: f.fences, tiles: f.tiles,
        surface: geometry(f.surface.geometry, F('fields')), surfaceMaterial: materialMeta(f.surface.material),
        posts: f.posts ? geometry(f.posts.geometry, F('posts')) : null,
      } : null,
      forest: fo ? { zones: fo.zones, tiles: fo.tiles, crowns: fo.crowns, surface: geometry(fo.surface.geometry, F('forest')) } : null,
      houses: h ? { zones: h.zones, tiles: h.tiles, houses: h.houses, wings: h.wings, surface: geometry(h.surface.geometry, F('houses')) } : null,
      coverLift: tiles.map((t) => coverLift(t)),
    });
    const air = buildAtmosphere(tiles, w.px);
    const shell = buildCloudShell(tiles, w.px);
    writeJson(`scenery/${tag}-sky.json`, {
      n, seed, px: w.px,
      atmosphere: { radius: air.radius, geometry: geometry(air.geometry, F('atmosphere')), material: materialMeta(air.material) },
      cloudShell: { radius: shell.__radius, geometry: geometry(shell, F('cloudshell')) },
    });
  }
});

// ---------------------------------------------------------------- 4. strips, sky, glow
step('pixelart', () => {
  const cliff = G('cliffTex'), foam = G('foamTex'), field = G('fieldTex');
  writePng('pixelart/cliff.png', cliff.image); writeJson('pixelart/cliff.json', textureMeta(cliff));
  writePng('pixelart/foam.png', foam.image); writeJson('pixelart/foam.json', textureMeta(foam));
  writePng('pixelart/field.png', field.image); writeJson('pixelart/field.json', textureMeta(field));
  const glow = G('makeGlow')();
  writePng('pixelart/glow.png', glow.material.params.map.image);
  writeJson('pixelart/glow.json', {
    texture: textureMeta(glow.material.params.map), material: materialMeta(glow.material), renderOrder: glow.renderOrder,
  });
  const seed = (63352 % 9973) + 7;
  const sky = G('makeCloudSky')(seed);
  sky.maps.forEach((m, k) => writePng(`pixelart/sky-s${seed}-deck${k}.png`, { width: m.width, height: m.height, data: m.data }));
  writeJson(`pixelart/sky-s${seed}.json`, { seed, W: sky.W, H: sky.H, decks: sky.maps.map(textureMeta) });
});

// ---------------------------------------------------------------- 5. clouds group + injected shader
step('clouds', () => {
  const w = G('worlds').get(8);
  const seed = (w.seed % 9973) + 7;
  const group = G('buildClouds')(w.tiles, w.px, seed);
  const decks = group.__decks.map((m) => {
    const fake = {
      uniforms: {},
      vertexShader: 'void main(){\n#include <begin_vertex>\n}',
      fragmentShader: 'void main(){\n#include <alphatest_fragment>\n}',
    };
    if (m.material.onBeforeCompile) m.material.onBeforeCompile(fake);
    const uniforms = {};
    for (const [k, u] of Object.entries(fake.uniforms)) {
      uniforms[k] = typeof u.value === 'object' ? { x: u.value.x, y: u.value.y, z: u.value.z } : u.value;
    }
    return {
      scale: m.scale.x, renderOrder: m.renderOrder, material: materialMeta(m.material), uniforms,
      vertexShader: fake.vertexShader, fragmentShader: fake.fragmentShader,
    };
  });
  writeJson('scenery/clouds-n8.json', { seed, tileCount: group.tileCount, atlas: group.atlas, radius: group.__geo.__radius, decks });
});

// ---------------------------------------------------------------- 6. territory border ribbons (n=4 default world)
step('border', () => {
  G('rebuild')(4);
  const tiles = G('tiles');
  const owned = tiles.filter((t) => t.level >= 0 && t.id < 60).map((t) => t.id);
  for (const t of tiles) t.owner = owned.includes(t.id) ? 0 : null;
  G('rebuildBorder')();
  const g = G('borderMesh').geometry;
  writeJson('scenery/n4-s31676-border.json', {
    owned, position: summarize(g.attributes.position), material: materialMeta(G('borderMat')),
    coverLift: tiles.map((t) => G('coverLift')(t)),
  });
  writeFull('scenery/n4-s31676-border.json', { position: Array.from(g.attributes.position.array) });
  const setRing = G('setRing'), ring = G('hoverRing');
  const rings = {};
  for (const t of [tiles[0], tiles.find((x) => x.cover === 'forest') || tiles[1]]) {
    setRing(ring, t);
    rings[t.id] = { drawRange: ring.geometry.drawRange, position: Array.from(ring.geometry.attributes.position.array) };
  }
  writeJson('scenery/n4-s31676-ring.json', rings);
  for (const t of tiles) t.owner = null;
});

// ---------------------------------------------------------------- 7. space pass (vignette + stars)
step('space', () => {
  const buildSpace = G('buildSpace'), stepStars = G('stepStars');
  for (const [W, H] of [[320, 180], [300, 200]]) {
    buildSpace(W, H);
    const [grad, stars] = G('bgScene').children;
    const starList = G('starList');
    const colAt0 = Array.from(G('starCol').array);
    stepStars(1234);
    const colAt1234 = Array.from(G('starCol').array);
    stepStars(5678);
    const colAt5678 = Array.from(G('starCol').array);
    writeJson(`scenery/space-${W}x${H}.json`, {
      W, H,
      vignette: {
        position: Array.from(grad.geometry.attributes.position.array), index: grad.geometry.index,
        color: Array.from(grad.geometry.attributes.color.array), material: materialMeta(grad.material),
      },
      stars: {
        position: Array.from(stars.geometry.attributes.position.array), color: colAt0,
        colorAt1234: colAt1234, colorAt5678: colAt5678, material: materialMeta(stars.material), list: starList,
      },
    });
  }
});

// ---------------------------------------------------------------- 8. index
step('index', () => {
  const sections = [...source.matchAll(/^\s*([0-9]+[a-z]?\.|[A-Z]{3,}\.)\s+([A-Z][A-Z0-9 ]+)/gm)]
    .map((m) => `${m[1]} ${m[2].trim()}`);
  writeJson('index.json', {
    generatedBy: 'reference/harness/extract.mjs', node: process.version, reference: 'reference/hex-planet.html',
    referenceSha256: sha256(html), worlds: WORLDS, sections,
  });
});

if (failures.length) {
  console.error(`\n${failures.length} fixture step(s) failed: ${failures.join(', ')}`);
  process.exit(1);
}
console.log(`\nfixtures written to ${OUT}${FULL ? ` (full dumps in ${FULL_OUT})` : ''}`);
