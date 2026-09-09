// Stand-ins for the browser and for three.js, just enough for the prototype's inline script to
// run inside node:vm. Builders that produce data (canvases, geometry attributes, data textures)
// get real recorders; everything else is a chainable stub that accepts any call or assignment.
import vm from 'node:vm';

const STUB = Symbol('stub');

/** A callable object that returns itself when called and grows a child stub for any property read. */
export function stub(label = 'stub', props = {}) {
  const fn = function () { return fn; };
  Object.assign(fn, props);
  fn[STUB] = label;
  return new Proxy(fn, {
    get(target, prop, receiver) {
      if (prop === STUB) return label;
      if (prop === 'then' || prop === Symbol.iterator || prop === Symbol.toStringTag) return undefined;
      if (prop === Symbol.toPrimitive) return () => 0;
      if (prop in target) return Reflect.get(target, prop, receiver);
      const child = stub(`${label}.${String(prop)}`);
      target[prop] = child;
      return child;
    },
  });
}

function vec3(x = 0, y = 0, z = 0) {
  return {
    x, y, z,
    set(a, b, c) { this.x = a; this.y = b; this.z = c; return this; },
    setScalar(s) { this.x = this.y = this.z = s; return this; },
    clone() { return vec3(this.x, this.y, this.z); },
    copy(v) { this.x = v.x; this.y = v.y; this.z = v.z; return this; },
    normalize() {
      const l = Math.hypot(this.x, this.y, this.z) || 1;
      this.x /= l; this.y /= l; this.z /= l;
      return this;
    },
    setFromMatrixColumn() { return this; },
  };
}

class BufferAttribute {
  constructor(arr, itemSize) {
    this.array = Float32Array.from(arr);
    this.itemSize = itemSize;
    this.count = this.array.length / itemSize;
    this.needsUpdate = false;
  }
  getX(i) { return this.array[i * this.itemSize]; }
  getY(i) { return this.array[i * this.itemSize + 1]; }
  getZ(i) { return this.array[i * this.itemSize + 2]; }
  setXYZ(i, x, y, z) {
    const o = i * this.itemSize;
    this.array[o] = x; this.array[o + 1] = y; this.array[o + 2] = z;
    return this;
  }
}

class BufferGeometry {
  constructor() { this.attributes = {}; this.index = null; this.drawRange = null; }
  setAttribute(name, attr) { this.attributes[name] = attr; return this; }
  setIndex(idx) { this.index = idx; return this; }
  setDrawRange(start, count) { this.drawRange = [start, count]; }
  dispose() {}
}

/** three r128 PlaneGeometry: positions and indices are what the vignette needs. */
class PlaneGeometry extends BufferGeometry {
  constructor(width = 1, height = 1, widthSegments = 1, heightSegments = 1) {
    super();
    const wh = width / 2, hh = height / 2;
    const gx = Math.floor(widthSegments), gy = Math.floor(heightSegments);
    const gx1 = gx + 1, gy1 = gy + 1;
    const sw = width / gx, sh = height / gy;
    const pos = [], uv = [], idx = [];
    for (let iy = 0; iy < gy1; iy++) {
      const y = iy * sh - hh;
      for (let ix = 0; ix < gx1; ix++) {
        const x = ix * sw - wh;
        pos.push(x, -y, 0);
        uv.push(ix / gx, 1 - iy / gy);
      }
    }
    for (let iy = 0; iy < gy; iy++) {
      for (let ix = 0; ix < gx; ix++) {
        const a = ix + gx1 * iy, b = ix + gx1 * (iy + 1);
        const c = ix + 1 + gx1 * (iy + 1), d = ix + 1 + gx1 * iy;
        idx.push(a, b, d, b, c, d);
      }
    }
    this.attributes.position = new BufferAttribute(pos, 3);
    this.attributes.uv = new BufferAttribute(uv, 2);
    this.index = idx;
    this.parameters = { width, height, widthSegments, heightSegments };
  }
}

class Texture {
  constructor() {
    this.magFilter = null; this.minFilter = null; this.wrapS = null; this.wrapT = null;
    this.generateMipmaps = true; this.needsUpdate = false;
    this.repeat = { x: 1, y: 1, set(x, y) { this.x = x; this.y = y; } };
    this.offset = { x: 0, y: 0, set(x, y) { this.x = x; this.y = y; } };
  }
  dispose() {}
}
class CanvasTexture extends Texture {
  constructor(canvas) { super(); this.canvas = canvas; }
  get image() { return this.canvas.__image; }
}
class DataTexture extends Texture {
  constructor(data, width, height, format) {
    super();
    this.data = data; this.width = width; this.height = height; this.format = format;
  }
}
class Material {
  constructor(params = {}) { this.params = params; this.userData = {}; this.onBeforeCompile = null; }
  dispose() {}
}
class Object3D {
  constructor() {
    this.children = []; this.visible = true; this.renderOrder = 0; this.frustumCulled = true;
    this.position = vec3(); this.scale = vec3(1, 1, 1); this.quaternion = stub('quaternion');
    this.userData = {}; this.matrixWorld = stub('matrixWorld'); this.aspect = 1;
  }
  add(c) { this.children.push(c); return this; }
  remove(c) { const i = this.children.indexOf(c); if (i >= 0) this.children.splice(i, 1); return this; }
  lookAt() {}
  updateProjectionMatrix() {}
}
class Mesh extends Object3D {
  constructor(geometry, material) { super(); this.geometry = geometry; this.material = material; }
}
class Light extends Object3D {
  constructor(color, intensity) { super(); this.color = color; this.intensity = intensity; }
}
class Color { constructor(value) { this.value = value; } }
class Raycaster { setFromCamera() {} intersectObjects() { return []; } }

export function makeElement(tag, id = '') {
  const el = stub(`<${tag}#${id}>`, {
    tagName: tag, id, children: [], dataset: {}, style: {}, attrs: {},
    clientWidth: 900, clientHeight: 600,
    classList: { add() {}, remove() {}, toggle() {}, contains() { return false; } },
    getBoundingClientRect() { return { left: 0, top: 0, width: 900, height: 600 }; },
    appendChild(c) { el.children.push(c); return c; },
    addEventListener() {}, removeEventListener() {}, setPointerCapture() {},
    setAttribute(k, v) { el.attrs[k] = v; },
    getAttribute(k) { return el.attrs[k] ?? null; },
    querySelectorAll() { return []; },
  });
  if (tag === 'canvas') {
    el.width = 0; el.height = 0;
    el.getContext = () => ({
      createImageData(w, h) { return { width: w, height: h, data: new Uint8ClampedArray(w * h * 4) }; },
      putImageData(img) { el.__image = img; },
    });
  }
  return el;
}

class WebGLRenderer {
  constructor(params) { this.params = params; this.domElement = makeElement('canvas', 'gl'); this.autoClear = true; }
  setClearColor() {}
  setPixelRatio() {}
  setSize() {}
  clear() {}
  clearDepth() {}
  render() {}
}

const classes = {
  BufferGeometry, Float32BufferAttribute: BufferAttribute, PlaneGeometry, CanvasTexture, DataTexture,
  Mesh, LineLoop: Mesh, Group: Object3D, Scene: Object3D, Color, Raycaster, WebGLRenderer,
  PerspectiveCamera: Object3D, OrthographicCamera: Object3D, AmbientLight: Light, DirectionalLight: Light,
  MeshLambertMaterial: Material, MeshBasicMaterial: Material, LineBasicMaterial: Material,
  Vector3: function () { return vec3(); },
  Vector2: function () { return { x: 0, y: 0 }; },
  Quaternion: function () { return stub('Quaternion'); },
};

/** `THREE.Foo` resolves to a recorder class when one exists, otherwise to the string "Foo" (constants). */
export const THREE = new Proxy({}, {
  get(_, name) {
    if (name in classes) return classes[name];
    return typeof name === 'string' ? name : undefined;
  },
});

export function makeContext() {
  const byId = new Map();
  const document = {
    createElement: (tag) => makeElement(tag),
    getElementById: (id) => {
      if (!byId.has(id)) byId.set(id, makeElement('div', id));
      return byId.get(id);
    },
    querySelectorAll: () => [],
    body: makeElement('body'),
  };
  const window = stub('window', {
    addEventListener() {},
    matchMedia: () => ({ matches: false }),
    requestIdleCallback: undefined,
  });
  const sandbox = {
    window, document, THREE, console, performance,
    requestAnimationFrame() { return 0; },
    setTimeout() { return 0; },
    clearTimeout() {},
  };
  return vm.createContext(sandbox);
}
