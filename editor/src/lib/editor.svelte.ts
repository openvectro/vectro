import * as core from "../wasm/vectro_wasm.js";
import {
  EASES, ancestors, localBounds, readProp, worldMatrix, writeProp,
  type Doc, type Ease, type PaintProp, type Prop as AnyProp, type ShapeProp, type Shape, type ShapeKind, type Track, type Vertex,
} from "./types";

/** What the properties panel can key: the shape's own fields and the keyable paint fields. */
type Prop = ShapeProp | PaintProp;

const STORAGE = "vectro.document";
export type Tool = "select" | "rectangle" | "ellipse" | "pen" | "node" | "pivot";
export type KeyRef = { track: number; key: number };
export type Mode = "design" | "animate";
export type Align = "left" | "hcenter" | "right" | "top" | "vcenter" | "bottom";

function load(): Doc {
  try {
    const saved = localStorage.getItem(STORAGE);
    if (saved) return JSON.parse(saved);
  } catch {}
  return JSON.parse(core.newDocument());
}

const PALETTE = [0xff5c7cfa, 0xffff6b6b, 0xff51cf66, 0xfffcc419, 0xffcc5de8, 0xff22b8cf];

class Editor {
  doc = $state<Doc>(load());
  tool = $state<Tool>("select");
  mode = $state<Mode>("design");
  /** Selected shape ids; the last one is the primary selection. */
  selectedIds = $state<number[]>([]);
  /** A point of the selected path, when the node tool is on. */
  selectedVertex = $state<{ path: number; index: number } | null>(null);
  anim = $state(0);
  frame = $state(0);
  playing = $state(false);
  /** Keys picked on the timeline; the last one is the primary. */
  selectedKeys = $state<KeyRef[]>([]);
  /** Keys copied with ⌘C: track identity plus frames relative to the earliest. */
  keyClipboard: { shapeId: number; prop: AnyProp; point?: Track["point"]; keys: { frame: number; value: number; ease: Ease }[] }[] = [];
  /** Ghosts of nearby frames on the canvas while animating. */
  onion = $state(false);
  onionStep = $state(6);
  error = $state<string | null>(null);
  /** The viewport: zoom factor and the artboard point at the top-left of the padding. */
  view = $state({ zoom: 1, x: 0, y: 0 });
  /** Which bottom panel is open. */
  panel = $state<"timeline" | "machines">("timeline");
  /** State machine editing: the machine and layer in view, and what is picked in the graph. */
  machine = $state(0);
  layer = $state(0);
  smSelection = $state<{ type: "state"; index: number } | { type: "transition"; index: number } | null>(null);

  private past: string[] = [];
  private future: string[] = [];

  /** What the canvas shows: design values, or the animation at the playhead. */
  shown = $derived.by<Shape[]>(() => {
    const json = JSON.stringify(this.doc);
    const anim = this.mode === "animate" ? this.anim : -1;
    try {
      return JSON.parse(core.evaluate(json, anim, this.frame));
    } catch (e) {
      return this.doc.shapes;
    }
  });

  get selectedKey(): KeyRef | null {
    return this.selectedKeys.length ? this.selectedKeys[this.selectedKeys.length - 1] : null;
  }

  set selectedKey(k: KeyRef | null) {
    this.selectedKeys = k ? [k] : [];
  }

  isKeySelected(track: number, key: number) {
    return this.selectedKeys.some((k) => k.track === track && k.key === key);
  }

  get selected(): number | null {
    return this.selectedIds.length ? this.selectedIds[this.selectedIds.length - 1] : null;
  }

  set selected(id: number | null) {
    this.selectedIds = id === null ? [] : [id];
    this.selectedVertex = null;
  }

  /** Add to or remove from the selection (shift-click). */
  toggleSelected(id: number) {
    this.selectedIds = this.selectedIds.includes(id) ? this.selectedIds.filter((x) => x !== id) : [...this.selectedIds, id];
    this.selectedVertex = null;
  }

  get animation() {
    return this.doc.animations[this.anim];
  }

  get selectedShape() {
    return this.doc.shapes.find((s) => s.id === this.selected) ?? null;
  }

  shape(id: number | null) {
    return this.doc.shapes.find((s) => s.id === id) ?? null;
  }

  /** Record the current document so the next change can be undone. */
  checkpoint() {
    this.past.push(JSON.stringify(this.doc));
    if (this.past.length > 200) this.past.shift();
    this.future = [];
  }

  /** Fold the last checkpoint into the one before it (a drag that calls setPivot per move). */
  undoMerge() {
    if (this.past.length >= 2) this.past.splice(this.past.length - 1, 1);
  }

  undo() {
    const prev = this.past.pop();
    if (!prev) return;
    this.future.push(JSON.stringify(this.doc));
    this.doc = JSON.parse(prev);
    this.afterRestore();
  }

  redo() {
    const next = this.future.pop();
    if (!next) return;
    this.past.push(JSON.stringify(this.doc));
    this.doc = JSON.parse(next);
    this.afterRestore();
  }

  private afterRestore() {
    this.selectedIds = this.selectedIds.filter((id) => this.doc.shapes.some((s) => s.id === id));
    this.selectedVertex = null;
    this.anim = Math.min(this.anim, this.doc.animations.length - 1);
    this.selectedKeys = [];
    this.machine = Math.max(0, Math.min(this.machine, (this.doc.stateMachines?.length ?? 1) - 1));
    this.layer = Math.max(0, Math.min(this.layer, (this.doc.stateMachines?.[this.machine]?.layers.length ?? 1) - 1));
    this.smSelection = null;
  }

  get stateMachine() {
    return this.doc.stateMachines?.[this.machine] ?? null;
  }

  get smLayer() {
    return this.stateMachine?.layers[this.layer] ?? null;
  }

  save() {
    try {
      localStorage.setItem(STORAGE, JSON.stringify(this.doc));
    } catch {}
  }

  private nextId() {
    return Math.max(0, ...this.doc.shapes.map((s) => s.id)) + 1;
  }

  private blank(id: number, name: string, kind: ShapeKind): Shape {
    return {
      id, name, kind,
      x: 0, y: 0, width: 1, height: 1,
      rotation: 0, scaleX: 1, scaleY: 1, opacity: 1,
      cornerRadius: 0,
      fill: PALETTE[(id - 1) % PALETTE.length],
    };
  }

  addShape(kind: ShapeKind, x: number, y: number, width: number, height: number) {
    this.checkpoint();
    const id = this.nextId();
    const count = this.doc.shapes.filter((s) => s.kind === kind).length + 1;
    const s = this.blank(id, `${kind === "rectangle" ? "Rectangle" : "Ellipse"} ${count}`, kind);
    Object.assign(s, { x, y, width, height, cornerRadius: kind === "rectangle" ? 12 : 0 });
    this.doc.shapes.push(s);
    this.selected = id;
    return id;
  }

  /** Start a path with its first point (artboard coordinates). */
  addPath(first: Vertex) {
    this.checkpoint();
    const id = this.nextId();
    const count = this.doc.shapes.filter((s) => s.kind === "path").length + 1;
    const s = this.blank(id, `Path ${count}`, "path");
    s.paths = [{ closed: false, points: [first] }];
    s.fill = 0xff5c7cfa;
    this.doc.shapes.push(s);
    this.selected = id;
    return id;
  }

  /** Move a path's origin to the centre of its points without moving it on screen. */
  recentre(id: number) {
    const s = this.shape(id);
    if (!s || s.kind !== "path" || !s.paths?.length) return;
    const b = localBounds(s);
    const cx = b.x + b.w / 2, cy = b.y + b.h / 2;
    for (const v of s.paths.flatMap((p) => p.points)) {
      v.x -= cx; v.inX -= cx; v.outX -= cx;
      v.y -= cy; v.inY -= cy; v.outY -= cy;
    }
    s.x += cx * s.scaleX;
    s.y += cy * s.scaleY;
    s.width = b.w;
    s.height = b.h;
  }

  /** Every shape inside a group, at any depth. */
  descendants(id: number): number[] {
    const out = new Set<number>();
    for (let grew = true; grew; ) {
      grew = false;
      for (const s of this.doc.shapes) if (s.parent !== undefined && (s.parent === id || out.has(s.parent)) && !out.has(s.id)) (out.add(s.id), (grew = true));
    }
    return [...out];
  }

  deleteShape(id: number) {
    this.checkpoint();
    const gone = new Set([id, ...this.descendants(id)]);
    this.doc.shapes = this.doc.shapes.filter((s) => !gone.has(s.id));
    for (const a of this.doc.animations) a.tracks = a.tracks.filter((t) => !gone.has(t.shapeId));
    this.selectedIds = this.selectedIds.filter((x) => !gone.has(x));
    this.selectedVertex = null;
    this.selectedKey = null;
  }

  deleteSelection() {
    for (const id of [...this.selectedIds]) if (this.shape(id)) this.deleteShape(id);
  }

  /** Copies of the selected shapes (and what they contain), offset a little, with their keys. */
  duplicate() {
    const ids = this.selectedIds.filter((id) => this.shape(id));
    if (!ids.length) return;
    this.checkpoint();
    const map = new Map<number, number>();
    let next = this.nextId();
    const all = ids.flatMap((id) => [id, ...this.descendants(id)]);
    for (const id of all) map.set(id, next++);
    const copies: Shape[] = [];
    for (const s of this.doc.shapes) {
      if (!map.has(s.id)) continue;
      const c: Shape = JSON.parse(JSON.stringify(s));
      c.id = map.get(s.id)!;
      if (s.parent !== undefined && map.has(s.parent)) c.parent = map.get(s.parent);
      if (ids.includes(s.id)) {
        c.x += 16;
        c.y += 16;
        c.name = `${s.name} copy`;
      }
      copies.push(c);
    }
    this.doc.shapes.push(...copies);
    for (const a of this.doc.animations) {
      const extra = a.tracks.filter((t) => map.has(t.shapeId)).map((t) => ({ ...JSON.parse(JSON.stringify(t)), shapeId: map.get(t.shapeId)! }));
      a.tracks.push(...extra);
    }
    this.selectedIds = ids.map((id) => map.get(id)!);
  }

  /** Put the selected shapes (same parent) into a new group at the centre of their bounds. */
  group() {
    const members = this.selectedIds.map((id) => this.shape(id)).filter((s): s is Shape => !!s);
    if (!members.length || new Set(members.map((s) => s.parent)).size > 1) return;
    this.checkpoint();
    const parent = members[0].parent;
    // Bounds in the parent's space (members may be rotated; use their world corners).
    const pm = worldMatrix(this.doc.shapes, members[0], false).inverse();
    let l = Infinity, t = Infinity, r = -Infinity, b = -Infinity;
    for (const s of members) {
      const lb = localBounds(s);
      const m = worldMatrix(this.doc.shapes, s);
      for (const [x, y] of [[lb.x, lb.y], [lb.x + lb.w, lb.y], [lb.x, lb.y + lb.h], [lb.x + lb.w, lb.y + lb.h]]) {
        const p = pm.transformPoint(m.transformPoint(new DOMPoint(x, y)));
        l = Math.min(l, p.x); t = Math.min(t, p.y); r = Math.max(r, p.x); b = Math.max(b, p.y);
      }
    }
    const id = this.nextId();
    const g = this.blank(id, `Group ${this.doc.shapes.filter((s) => s.kind === "group").length + 1}`, "group");
    g.x = Math.round((l + r) / 2);
    g.y = Math.round((t + b) / 2);
    if (parent !== undefined) g.parent = parent;
    // Insert the group where the front-most member was, so stacking stays the same.
    const at = Math.max(...members.map((s) => this.doc.shapes.indexOf(s)));
    this.doc.shapes.splice(at + 1, 0, g);
    for (const s of members) {
      s.parent = id;
      s.x -= g.x;
      s.y -= g.y;
    }
    this.selected = id;
  }

  /** Dissolve the selected group, keeping its pieces where they are on screen. */
  ungroup() {
    const g = this.selectedShape;
    if (!g || g.kind !== "group") return;
    this.checkpoint();
    const children = this.doc.shapes.filter((s) => s.parent === g.id);
    const rot = (g.rotation * Math.PI) / 180;
    for (const s of children) {
      const x = s.x * g.scaleX, y = s.y * g.scaleY;
      s.x = g.x + x * Math.cos(rot) - y * Math.sin(rot);
      s.y = g.y + x * Math.sin(rot) + y * Math.cos(rot);
      s.rotation += g.rotation;
      s.scaleX *= g.scaleX;
      s.scaleY *= g.scaleY;
      s.opacity *= g.opacity;
      if (g.parent !== undefined) s.parent = g.parent;
      else delete s.parent;
    }
    this.doc.shapes = this.doc.shapes.filter((s) => s.id !== g.id);
    for (const a of this.doc.animations) a.tracks = a.tracks.filter((t) => t.shapeId !== g.id);
    this.selectedIds = children.map((s) => s.id);
  }

  /** World-space bounds of a shape. */
  worldBounds(s: Shape) {
    const lb = localBounds(s);
    const m = worldMatrix(this.doc.shapes, s);
    let l = Infinity, t = Infinity, r = -Infinity, b = -Infinity;
    for (const [x, y] of [[lb.x, lb.y], [lb.x + lb.w, lb.y], [lb.x, lb.y + lb.h], [lb.x + lb.w, lb.y + lb.h]]) {
      const p = m.transformPoint(new DOMPoint(x, y));
      l = Math.min(l, p.x); t = Math.min(t, p.y); r = Math.max(r, p.x); b = Math.max(b, p.y);
    }
    return { l, t, r, b };
  }

  /** Align the selection to the artboard (one shape) or to its own bounds (several). */
  align(how: Align) {
    const members = this.selectedIds.map((id) => this.shape(id)).filter((s): s is Shape => !!s);
    if (!members.length) return;
    this.checkpoint();
    const ab = this.doc.artboard;
    let ref = { l: 0, t: 0, r: ab.width, b: ab.height };
    if (members.length > 1) {
      const bs = members.map((s) => this.worldBounds(s));
      ref = { l: Math.min(...bs.map((b) => b.l)), t: Math.min(...bs.map((b) => b.t)), r: Math.max(...bs.map((b) => b.r)), b: Math.max(...bs.map((b) => b.b)) };
    }
    for (const s of members) {
      const wb = this.worldBounds(s);
      let dx = 0, dy = 0;
      if (how === "left") dx = ref.l - wb.l;
      if (how === "hcenter") dx = (ref.l + ref.r) / 2 - (wb.l + wb.r) / 2;
      if (how === "right") dx = ref.r - wb.r;
      if (how === "top") dy = ref.t - wb.t;
      if (how === "vcenter") dy = (ref.t + ref.b) / 2 - (wb.t + wb.b) / 2;
      if (how === "bottom") dy = ref.b - wb.b;
      this.moveBy(s, dx, dy);
    }
  }

  /** Move a shape by a world-space delta, whatever group it sits in. */
  moveBy(s: Shape, dx: number, dy: number) {
    const m = worldMatrix(this.doc.shapes, s, false).inverse();
    const o = m.transformPoint(new DOMPoint(0, 0));
    const d = m.transformPoint(new DOMPoint(dx, dy));
    s.x = Math.round((s.x + d.x - o.x) * 100) / 100;
    s.y = Math.round((s.y + d.y - o.y) * 100) / 100;
  }

  nudge(dx: number, dy: number) {
    const members = this.selectedIds.map((id) => this.shape(id)).filter((s): s is Shape => !!s);
    if (!members.length) return;
    if (this.mode === "animate") {
      for (const s of members) {
        const shown = this.shown.find((x) => x.id === s.id)!;
        this.setProp(s.id, "x", shown.x + dx);
        this.setProp(s.id, "y", shown.y + dy, false);
      }
      return;
    }
    this.checkpoint();
    for (const s of members) this.moveBy(s, dx, dy);
  }

  /** Swap a shape with its next sibling towards the front (delta 1) or back (-1). */
  reorder(id: number, delta: number) {
    const shapes = this.doc.shapes;
    const i = shapes.findIndex((s) => s.id === id);
    if (i < 0) return;
    let j = i + delta;
    while (j >= 0 && j < shapes.length && shapes[j].parent !== shapes[i].parent) j += delta;
    if (j < 0 || j >= shapes.length) return;
    this.checkpoint();
    [shapes[i], shapes[j]] = [shapes[j], shapes[i]];
  }

  // Paints

  setGradient(id: number, on: boolean) {
    const s = this.shape(id);
    if (!s) return;
    this.checkpoint();
    if (!on) {
      if (s.gradient) s.fill = s.gradient.stops[0]?.color ?? s.fill;
      delete s.gradient;
      return;
    }
    const b = localBounds(s);
    const light = (c: number) => {
      const ch = (sh: number) => Math.min(255, ((c >>> sh) & 255) + 70);
      return ((c >>> 24) << 24 | (ch(16) << 16) | (ch(8) << 8) | ch(0)) >>> 0;
    };
    s.gradient = { kind: "linear", start: [b.x, 0], end: [b.x + b.w, 0], stops: [{ position: 0, color: s.fill }, { position: 1, color: light(s.fill) }] };
  }

  setStroke(id: number, on: boolean) {
    const s = this.shape(id);
    if (!s) return;
    this.checkpoint();
    if (on) s.stroke = { color: 0xffffffff, thickness: 2, cap: 1, join: 1 };
    else delete s.stroke;
  }

  /** Draw-on: trim the stroke to a run of its outline (needs a stroke; adds one if missing). */
  setTrim(id: number, on: boolean) {
    const s = this.shape(id);
    if (!s) return;
    this.checkpoint();
    if (!on) {
      if (s.stroke) delete s.stroke.trim;
      return;
    }
    s.stroke ??= { color: 0xffffffff, thickness: 2, cap: 1, join: 1 };
    s.stroke.trim = { start: 0, end: 1, offset: 0, mode: "sequential" };
  }

  setFeather(id: number, on: boolean) {
    const s = this.shape(id);
    if (!s) return;
    this.checkpoint();
    if (on) s.feather = { strength: 12, offset: [0, 4], inner: false };
    else delete s.feather;
  }

  // Animation

  track(shapeId: number, prop: Prop): Track | undefined {
    return this.animation?.tracks.find((t) => t.shapeId === shapeId && t.prop === prop && !t.point);
  }

  keyAtPlayhead(shapeId: number, prop: Prop) {
    return this.track(shapeId, prop)?.keys.find((k) => k.frame === Math.round(this.frame));
  }

  /**
   * Change a property. In design mode this edits the shape; in animate mode it sets a
   * keyframe at the playhead. Pass checkpoint=false while dragging (one undo per drag).
   */
  setProp(shapeId: number, prop: Prop, value: number, checkpoint = true) {
    if (!Number.isFinite(value)) return;
    if (checkpoint) this.checkpoint();
    if (this.mode === "design" || !this.animation) {
      const s = this.shape(shapeId);
      if (s) writeProp(s, prop, value);
      return;
    }
    const frame = Math.round(this.frame);
    let t = this.track(shapeId, prop);
    if (!t) {
      t = { shapeId, prop, keys: [] };
      this.animation.tracks.push(t);
      t = this.animation.tracks[this.animation.tracks.length - 1];
    }
    const k = t.keys.find((k) => k.frame === frame);
    if (k) k.value = value;
    else {
      t.keys.push({ frame, value, ease: { ...EASES[0].ease } as Ease });
      t.keys.sort((a, b) => a.frame - b.frame);
    }
  }

  /** Add a key holding the current value, or remove the key under the playhead. */
  toggleKey(shapeId: number, prop: Prop) {
    if (this.keyAtPlayhead(shapeId, prop)) {
      this.checkpoint();
      const t = this.track(shapeId, prop)!;
      t.keys = t.keys.filter((k) => k.frame !== Math.round(this.frame));
      if (!t.keys.length) this.animation.tracks = this.animation.tracks.filter((x) => x !== t);
      this.selectedKey = null;
    } else {
      const shown = this.shown.find((s) => s.id === shapeId);
      if (shown) this.setProp(shapeId, prop, readProp(shown, prop));
    }
  }

  deleteSelectedKey() {
    this.deleteSelectedKeys();
  }

  deleteSelectedKeys() {
    if (!this.selectedKeys.length || !this.animation) return;
    this.checkpoint();
    const a = this.animation;
    // Highest key index first within each track, so earlier indices stay valid.
    const byTrack = new Map<number, number[]>();
    for (const k of this.selectedKeys) byTrack.set(k.track, [...(byTrack.get(k.track) ?? []), k.key]);
    for (const [ti, keys] of byTrack) for (const ki of [...new Set(keys)].sort((x, y) => y - x)) a.tracks[ti]?.keys.splice(ki, 1);
    a.tracks = a.tracks.filter((t) => t.keys.length);
    this.selectedKeys = [];
  }

  /** Move every selected key by `delta` frames, keeping order and refusing collisions. */
  moveSelectedKeys(delta: number) {
    if (!delta || !this.animation) return false;
    const a = this.animation;
    const groups = new Map<number, Set<number>>();
    for (const k of this.selectedKeys) groups.set(k.track, (groups.get(k.track) ?? new Set()).add(k.key));
    for (const [ti, kis] of groups) {
      const t = a.tracks[ti];
      if (!t) continue;
      for (const ki of kis) {
        const f = t.keys[ki].frame + delta;
        if (f < 0 || f > a.duration) return false;
        if (t.keys.some((o, i) => !kis.has(i) && o.frame === f)) return false;
      }
    }
    const moved: KeyRef[] = [];
    for (const [ti, kis] of groups) {
      const t = a.tracks[ti];
      if (!t) continue;
      const chosen = [...kis].map((ki) => t.keys[ki]);
      for (const k of chosen) k.frame += delta;
      t.keys.sort((x, y) => x.frame - y.frame);
      for (const k of chosen) moved.push({ track: ti, key: t.keys.indexOf(k) });
    }
    this.selectedKeys = moved;
    return true;
  }

  /** Apply an easing to every selected key. */
  setSelectedEase(ease: Ease) {
    if (!this.selectedKeys.length || !this.animation) return;
    this.checkpoint();
    for (const k of this.selectedKeys) {
      const key = this.animation.tracks[k.track]?.keys[k.key];
      if (key) key.ease = { ...ease };
    }
  }

  copyKeys() {
    if (!this.selectedKeys.length || !this.animation) return 0;
    const a = this.animation;
    const first = Math.min(...this.selectedKeys.map((k) => a.tracks[k.track]?.keys[k.key]?.frame ?? Infinity));
    const byTrack = new Map<number, number[]>();
    for (const k of this.selectedKeys) byTrack.set(k.track, [...(byTrack.get(k.track) ?? []), k.key]);
    this.keyClipboard = [...byTrack].map(([ti, kis]) => {
      const t = a.tracks[ti];
      return { shapeId: t.shapeId, prop: t.prop, point: t.point, keys: [...new Set(kis)].map((ki) => ({ ...t.keys[ki], ease: { ...t.keys[ki].ease } as Ease, frame: t.keys[ki].frame - first })) };
    });
    return this.selectedKeys.length;
  }

  /** Paste copied keys so the earliest lands on the playhead; same shapes and properties. */
  pasteKeys() {
    if (!this.keyClipboard.length || !this.animation) return 0;
    this.checkpoint();
    const a = this.animation;
    const at = Math.round(this.frame);
    const pasted: KeyRef[] = [];
    for (const c of this.keyClipboard) {
      let t = a.tracks.find((t) => t.shapeId === c.shapeId && t.prop === c.prop && JSON.stringify(t.point) === JSON.stringify(c.point));
      if (!t) {
        a.tracks.push({ shapeId: c.shapeId, prop: c.prop, ...(c.point ? { point: c.point } : {}), keys: [] });
        t = a.tracks[a.tracks.length - 1];
      }
      for (const k of c.keys) {
        const frame = Math.min(a.duration, at + k.frame);
        const existing = t.keys.find((o) => o.frame === frame);
        if (existing) Object.assign(existing, { value: k.value, ease: { ...k.ease } });
        else t.keys.push({ frame, value: k.value, ease: { ...k.ease } as Ease });
      }
      t.keys.sort((x, y) => x.frame - y.frame);
      const ti = a.tracks.indexOf(t);
      for (const k of c.keys) pasted.push({ track: ti, key: t.keys.findIndex((o) => o.frame === Math.min(a.duration, at + k.frame)) });
    }
    this.selectedKeys = pasted;
    return pasted.length;
  }

  /** The frames that carry a key on any track of a shape (for the dope sheet's summary row). */
  keyFramesOf(shapeId: number): number[] {
    const set = new Set<number>();
    for (const t of this.animation?.tracks ?? []) if (t.shapeId === shapeId) for (const k of t.keys) set.add(k.frame);
    return [...set].sort((x, y) => x - y);
  }

  /** Select every key of a shape at one frame. */
  selectKeysAt(shapeId: number, frame: number, add = false) {
    const a = this.animation;
    if (!a) return;
    const hits: KeyRef[] = [];
    a.tracks.forEach((t, ti) => { if (t.shapeId === shapeId) t.keys.forEach((k, ki) => { if (k.frame === frame) hits.push({ track: ti, key: ki }); }); });
    this.selectedKeys = add ? [...this.selectedKeys, ...hits] : hits;
  }

  /**
   * Move a shape's pivot to a new point in its own space without moving what it draws.
   * Rectangles and ellipses keep an origin fraction; paths shift their points; groups shift their children.
   */
  setPivot(id: number, lx: number, ly: number) {
    const s = this.shape(id);
    if (!s) return;
    this.checkpoint();
    // Keep the shape where it is: the old origin moves by (lx, ly) in local space, so the
    // shape's own position moves by that offset after the local rotation and scale.
    const rot = (s.rotation * Math.PI) / 180;
    const dx = lx * s.scaleX, dy = ly * s.scaleY;
    s.x = Math.round((s.x + dx * Math.cos(rot) - dy * Math.sin(rot)) * 100) / 100;
    s.y = Math.round((s.y + dx * Math.sin(rot) + dy * Math.cos(rot)) * 100) / 100;
    if (s.kind === "rectangle" || s.kind === "ellipse") {
      s.originX = Math.round(((s.originX ?? 0.5) + lx / s.width) * 1000) / 1000;
      s.originY = Math.round(((s.originY ?? 0.5) + ly / s.height) * 1000) / 1000;
    } else if (s.kind === "path") {
      for (const v of s.paths?.flatMap((p) => p.points) ?? []) { v.x -= lx; v.inX -= lx; v.outX -= lx; v.y -= ly; v.inY -= ly; v.outY -= ly; }
    } else {
      for (const c of this.doc.shapes) if (c.parent === id) { c.x -= lx; c.y -= ly; }
    }
  }

  addAnimation() {
    this.checkpoint();
    const n = this.doc.animations.length + 1;
    this.doc.animations.push({ name: `Animation ${n}`, fps: 60, duration: 60, loop: "loop", tracks: [] });
    this.anim = this.doc.animations.length - 1;
    this.frame = 0;
    this.selectedKey = null;
  }

  deleteAnimation() {
    if (this.doc.animations.length <= 1) return;
    this.checkpoint();
    this.doc.animations.splice(this.anim, 1);
    this.anim = Math.max(0, this.anim - 1);
    this.frame = 0;
    this.selectedKey = null;
  }

  importSvg(svg: string) {
    const next = JSON.parse(core.importSvg(JSON.stringify(this.doc), svg)) as Doc;
    this.checkpoint();
    const before = new Set(this.doc.shapes.map((s) => s.id));
    this.doc = next;
    this.mode = "design";
    const added = next.shapes.filter((s) => !before.has(s.id));
    this.selectedIds = added.map((s) => s.id);
    return added.length;
  }

  exportRiv(): Uint8Array {
    return core.exportRiv(JSON.stringify(this.doc));
  }

  open(doc: Doc) {
    this.checkpoint();
    this.doc = doc;
    this.selected = null;
    this.anim = 0;
    this.frame = 0;
    this.selectedKey = null;
    this.fitView();
  }

  // Viewport

  fitView() {
    this.view = { zoom: 1, x: 0, y: 0 };
  }

  /** Zoom by a factor about an artboard point, which stays put on screen. */
  zoomAt(factor: number, px: number, py: number) {
    const z = Math.min(32, Math.max(0.1, this.view.zoom * factor));
    const k = this.view.zoom / z;
    this.view = { zoom: z, x: px - (px - this.view.x) * k, y: py - (py - this.view.y) * k };
  }

  // Playback
  private startedAt = 0;
  private raf = 0;

  play() {
    if (this.playing || !this.animation) return;
    this.mode = "animate";
    this.playing = true;
    const a = this.animation;
    this.startedAt = performance.now() - (this.frame / a.fps) * 1000;
    const tick = (now: number) => {
      if (!this.playing) return;
      const seconds = (now - this.startedAt) / 1000;
      this.frame = core.frameAt(JSON.stringify(this.doc), this.anim, seconds);
      if (this.animation.loop === "oneShot" && this.frame >= this.animation.duration) {
        this.playing = false;
        return;
      }
      this.raf = requestAnimationFrame(tick);
    };
    this.raf = requestAnimationFrame(tick);
  }

  pause() {
    this.playing = false;
    cancelAnimationFrame(this.raf);
    this.frame = Math.round(this.frame);
  }
}

export const editor = new Editor();
export { ancestors };
