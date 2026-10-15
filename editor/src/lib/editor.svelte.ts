import * as core from "../wasm/vectro_wasm.js";
import { EASES, type Doc, type Ease, type Prop, type Shape, type ShapeKind, type Track } from "./types";

const STORAGE = "vectro.document";
export type Tool = "select" | "rectangle" | "ellipse";
export type Mode = "design" | "animate";

function load(): Doc {
  try {
    const saved = localStorage.getItem(STORAGE);
    if (saved) return JSON.parse(saved);
  } catch {}
  return JSON.parse(core.newDocument());
}

class Editor {
  doc = $state<Doc>(load());
  tool = $state<Tool>("select");
  mode = $state<Mode>("design");
  selected = $state<number | null>(null);
  anim = $state(0);
  frame = $state(0);
  playing = $state(false);
  selectedKey = $state<{ track: number; key: number } | null>(null);
  error = $state<string | null>(null);

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

  get animation() {
    return this.doc.animations[this.anim];
  }

  get selectedShape() {
    return this.doc.shapes.find((s) => s.id === this.selected) ?? null;
  }

  /** Record the current document so the next change can be undone. */
  checkpoint() {
    this.past.push(JSON.stringify(this.doc));
    if (this.past.length > 200) this.past.shift();
    this.future = [];
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
    if (!this.doc.shapes.some((s) => s.id === this.selected)) this.selected = null;
    this.anim = Math.min(this.anim, this.doc.animations.length - 1);
    this.selectedKey = null;
  }

  save() {
    try {
      localStorage.setItem(STORAGE, JSON.stringify(this.doc));
    } catch {}
  }

  addShape(kind: ShapeKind, x: number, y: number, width: number, height: number) {
    this.checkpoint();
    const id = Math.max(0, ...this.doc.shapes.map((s) => s.id)) + 1;
    const count = this.doc.shapes.filter((s) => s.kind === kind).length + 1;
    const palette = [0xff5c7cfa, 0xffff6b6b, 0xff51cf66, 0xfffcc419, 0xffcc5de8, 0xff22b8cf];
    this.doc.shapes.push({
      id,
      name: `${kind === "rectangle" ? "Rectangle" : "Ellipse"} ${count}`,
      kind,
      x, y, width, height,
      rotation: 0, scaleX: 1, scaleY: 1, opacity: 1,
      cornerRadius: kind === "rectangle" ? 12 : 0,
      fill: palette[(id - 1) % palette.length],
    });
    this.selected = id;
    return id;
  }

  deleteShape(id: number) {
    this.checkpoint();
    this.doc.shapes = this.doc.shapes.filter((s) => s.id !== id);
    for (const a of this.doc.animations) a.tracks = a.tracks.filter((t) => t.shapeId !== id);
    if (this.selected === id) this.selected = null;
    this.selectedKey = null;
  }

  /** Move a shape up (towards the front) or down the stack. */
  reorder(id: number, delta: number) {
    const i = this.doc.shapes.findIndex((s) => s.id === id);
    const j = i + delta;
    if (i < 0 || j < 0 || j >= this.doc.shapes.length) return;
    this.checkpoint();
    const [s] = this.doc.shapes.splice(i, 1);
    this.doc.shapes.splice(j, 0, s);
  }

  track(shapeId: number, prop: Prop): Track | undefined {
    return this.animation?.tracks.find((t) => t.shapeId === shapeId && t.prop === prop);
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
      const s = this.doc.shapes.find((s) => s.id === shapeId);
      if (s) s[prop] = value;
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
      if (shown) this.setProp(shapeId, prop, shown[prop]);
    }
  }

  deleteSelectedKey() {
    const sk = this.selectedKey;
    if (!sk) return;
    this.checkpoint();
    const t = this.animation.tracks[sk.track];
    t.keys.splice(sk.key, 1);
    if (!t.keys.length) this.animation.tracks.splice(sk.track, 1);
    this.selectedKey = null;
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
    this.selected = added.length ? added[added.length - 1].id : this.selected;
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
