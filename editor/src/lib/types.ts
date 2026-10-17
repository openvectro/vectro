// Mirrors crates/vectro-doc. The document travels to Rust as JSON.

export type ShapeKind = "rectangle" | "ellipse" | "path" | "group";
export interface Vertex { x: number; y: number; inX: number; inY: number; outX: number; outY: number }
export interface SubPath { closed: boolean; points: Vertex[] }
export type ShapeProp =
  | "x" | "y" | "width" | "height" | "rotation"
  | "scaleX" | "scaleY" | "opacity" | "cornerRadius" | "fill";
export type PointProp = "pointX" | "pointY" | "inAngle" | "inLength" | "outAngle" | "outLength";
/** The last stop of a gradient fill ("fill" is the first stop or the flat colour), and the stroke's trim. */
export type PaintProp = "fillEnd" | "trimStart" | "trimEnd" | "trimOffset";
export type TrimMode = "sequential" | "synchronized";
export interface Trim { start: number; end: number; offset: number; mode: TrimMode }
/** How far along its motion path a following shape sits, 0..1 (`Shape.follow.distance`). */
export type FollowProp = "followDistance";
/** What the Properties panel can key: a shape's own values, its paints, and its motion path distance. */
export type KeyProp = ShapeProp | PaintProp | FollowProp;
export type Prop = ShapeProp | PointProp | PaintProp | FollowProp;
export type Ease =
  | { type: "hold" }
  | { type: "linear" }
  | { type: "cubic"; x1: number; y1: number; x2: number; y2: number };
export type LoopMode = "oneShot" | "loop" | "pingPong";

export interface Shape {
  id: number;
  name: string;
  kind: ShapeKind;
  x: number;
  y: number;
  width: number;
  height: number;
  rotation: number;
  scaleX: number;
  scaleY: number;
  opacity: number;
  cornerRadius: number;
  fill: number; // ARGB
  paths?: SubPath[];
  fillRule?: "nonZero" | "evenOdd";
  parent?: number;
  /** Where the origin sits in a rectangle's or ellipse's box, 0..1 (0.5 is the centre). */
  originX?: number;
  originY?: number;
  gradient?: { kind: "linear" | "radial"; start: [number, number]; end: [number, number]; stops: { position: number; color: number }[] };
  stroke?: { color: number; thickness: number; cap?: number; join?: number; trim?: Trim };
  feather?: { strength: number; offset?: [number, number]; inner?: boolean };
  /** A motion path: the shape sits `distance` (0..1) along path shape `path`; its own x/y count only when `offset` is on. */
  follow?: { path: number; distance: number; orient?: boolean; offset?: boolean };
}

/** The value of a keyable property as a shape shows it. */

/** The label a property gets on the timeline and in the panel. */

/** SVG paint for a shape: a gradient reference or the flat colour. */
export const paintOf = (s: Shape) => (s.gradient ? `url(#grad-${s.id})` : argbToCss(s.fill));
/** A soft blur that stands in for Rive's feather (drawn properly by the runtime). */
export const featherStyle = (s: Shape) => (s.feather && !s.feather.inner ? `filter: blur(${(s.feather.strength / 2).toFixed(1)}px)` : "");
const CAPS = ["butt", "round", "square"] as const, JOINS = ["miter", "round", "bevel"] as const;
export const strokeAttrs = (s: Shape) =>
  s.stroke
    ? { stroke: argbToCss(s.stroke.color), "stroke-width": s.stroke.thickness, "stroke-linecap": CAPS[s.stroke.cap ?? 0], "stroke-linejoin": JOINS[s.stroke.join ?? 0], ...trimAttrs(s) }
    : {};

/** Length of a cubic segment by sampling (plenty for a preview). */
function cubicLength(a: Vertex, b: Vertex, steps = 24) {
  let len = 0, px = a.x, py = a.y;
  for (let i = 1; i <= steps; i++) {
    const t = i / steps, u = 1 - t;
    const x = u * u * u * a.x + 3 * u * u * t * a.outX + 3 * u * t * t * b.inX + t * t * t * b.x;
    const y = u * u * u * a.y + 3 * u * u * t * a.outY + 3 * u * t * t * b.inY + t * t * t * b.y;
    len += Math.hypot(x - px, y - py);
    px = x; py = y;
  }
  return len;
}

/** Length of what a shape outlines, in its own space: all outlines of a path, or the rectangle's or ellipse's rim. */
export function outlineLength(s: Shape): number {
  if (s.kind === "path") {
    let len = 0;
    for (const sub of s.paths ?? []) {
      const pts = sub.points, n = sub.closed ? pts.length : pts.length - 1;
      for (let i = 0; i < n; i++) len += cubicLength(pts[i], pts[(i + 1) % pts.length]);
    }
    return len;
  }
  if (s.kind === "ellipse") {
    const a = s.width / 2, b = s.height / 2, h = ((a - b) / (a + b)) ** 2;
    return Math.PI * (a + b) * (1 + (3 * h) / (10 + Math.sqrt(4 - 3 * h)));
  }
  const r = Math.min(s.cornerRadius, s.width / 2, s.height / 2);
  return 2 * (s.width + s.height) - 8 * r + 2 * Math.PI * r;
}

/**
 * The stroke's trim as an SVG dash: the visible run from start to end (offset slides it,
 * wrapping) over the outline's length. This stands in for Rive's TrimPath, which the runtime
 * draws exactly. Limits: SVG runs one dash pattern through every outline of a path end to end,
 * which is Rive's sequential mode; a synchronized trim on a shape with several outlines (each
 * trimmed by the same fractions) shows here as sequential. Dashes on a rectangle or ellipse
 * start where SVG starts the rim, which may not be where Rive does.
 */
export function trimAttrs(s: Shape): Record<string, string | number> {
  const t = s.stroke?.trim;
  if (!t) return {};
  const L = outlineLength(s);
  if (!(L > 0)) return {};
  let a = t.start + t.offset, b = t.end + t.offset;
  if (b < a) [a, b] = [b, a];
  const vis = Math.min(1, Math.max(0, b - a)) * L;
  if (vis <= 0) return { "stroke-dasharray": `0 ${L + 1}` };
  if (vis >= L) return {};
  const from = (((a % 1) + 1) % 1) * L;
  return { "stroke-dasharray": `${vis} ${L - vis}`, "stroke-dashoffset": -from };
}

export const transformOf = (s: Shape) => `translate(${s.x} ${s.y}) rotate(${s.rotation}) scale(${s.scaleX} ${s.scaleY})`;

/** Ancestors of a shape, outermost first. */
export function ancestors(shapes: Shape[], s: Shape): Shape[] {
  const out: Shape[] = [];
  let p = shapes.find((x) => x.id === s.parent && x.kind === "group");
  while (p && out.length < 64) {
    out.unshift(p);
    p = shapes.find((x) => x.id === p!.parent && x.kind === "group");
  }
  return out;
}

/** Matrix from a shape's own space to artboard space. */
export function worldMatrix(shapes: Shape[], s: Shape, includeSelf = true): DOMMatrix {
  let m = new DOMMatrix();
  for (const a of [...ancestors(shapes, s), ...(includeSelf ? [s] : [])]) {
    m = m.translate(a.x, a.y).rotate(a.rotation).scale(a.scaleX, a.scaleY);
  }
  return m;
}

export const isRoot = (shapes: Shape[], s: Shape) => !shapes.some((x) => x.id === s.parent && x.kind === "group");

/** SVG path data for a path shape, relative to its origin. */
export function pathData(s: Shape): string {
  let d = "";
  for (const sub of s.paths ?? []) {
    const pts = sub.points;
    if (!pts.length) continue;
    d += `M${pts[0].x} ${pts[0].y}`;
    const n = sub.closed ? pts.length : pts.length - 1;
    for (let i = 0; i < n; i++) {
      const a = pts[i], b = pts[(i + 1) % pts.length];
      d += `C${a.outX} ${a.outY} ${b.inX} ${b.inY} ${b.x} ${b.y}`;
    }
    if (sub.closed) d += "Z";
  }
  return d;
}

/** Bounds of what a shape draws, relative to its origin (before rotation and scale). */
export function localBounds(s: Shape): { x: number; y: number; w: number; h: number } {
  if (s.kind !== "path" || !s.paths?.length) return { x: -s.width * (s.originX ?? 0.5), y: -s.height * (s.originY ?? 0.5), w: s.width, h: s.height };
  let l = Infinity, t = Infinity, r = -Infinity, b = -Infinity;
  for (const v of s.paths.flatMap((p) => p.points)) {
    for (const [x, y] of [[v.x, v.y], [v.inX, v.inY], [v.outX, v.outY]]) {
      l = Math.min(l, x); t = Math.min(t, y); r = Math.max(r, x); b = Math.max(b, y);
    }
  }
  return { x: l, y: t, w: r - l, h: b - t };
}

export interface Key { frame: number; value: number; ease: Ease }
export interface Track { shapeId: number; prop: Prop; point?: { path: number; index: number }; keys: Key[] }
export interface Animation { name: string; fps: number; duration: number; loop: LoopMode; tracks: Track[] }
export interface Artboard { name: string; width: number; height: number; background: number }
// State machines, mirroring crates/vectro-doc.
export type InputKind = "bool" | "number" | "trigger";
export interface Input { name: string; kind: InputKind; value: number }
export type StateKind =
  | { type: "entry" } | { type: "any" } | { type: "exit" }
  | { type: "animation"; animation: number; speed?: number }
  | { type: "blend1D"; input: number; children: [number, number][] };
export interface State { name: string; kind: StateKind; random?: boolean; ui?: [number, number] }
export type Op = "equal" | "notEqual" | "lessOrEqual" | "greaterOrEqual" | "less" | "greater";
export interface Condition { input: number; op: Op; value: number }
export interface Transition { from: number; to: number; durationMs?: number; exitAfterMs?: number; conditions: Condition[]; randomWeight?: number }
export interface Layer { name: string; states: State[]; transitions: Transition[] }
export type PointerKind = "enter" | "exit" | "down" | "up" | "move" | "click";
export type Action =
  | { type: "setBool"; input: number; value: boolean }
  | { type: "toggleBool"; input: number }
  | { type: "setNumber"; input: number; value: number }
  | { type: "fireTrigger"; input: number }
  | { type: "fireEvent"; event: number };
export interface Listener { target: number; event: PointerKind; actions: Action[] }
export interface StateMachine { name: string; inputs: Input[]; layers: Layer[]; listeners: Listener[] }

export interface Doc { version: number; artboard: Artboard; shapes: Shape[]; animations: Animation[]; events?: string[]; stateMachines?: StateMachine[] }

export const PROPS: { prop: ShapeProp; label: string; step: number; min?: number; max?: number }[] = [
  { prop: "x", label: "X", step: 1 },
  { prop: "y", label: "Y", step: 1 },
  { prop: "width", label: "Width", step: 1, min: 0 },
  { prop: "height", label: "Height", step: 1, min: 0 },
  { prop: "rotation", label: "Rotation", step: 1 },
  { prop: "scaleX", label: "Scale X", step: 0.05 },
  { prop: "scaleY", label: "Scale Y", step: 0.05 },
  { prop: "opacity", label: "Opacity", step: 0.05, min: 0, max: 1 },
  { prop: "cornerRadius", label: "Corner", step: 1, min: 0 },
  { prop: "fill", label: "Fill", step: 1 },
];

/** Keyable paint fields that live beside the shape's own properties. */
export const PAINT_PROPS: { prop: PaintProp; label: string; step: number; min?: number; max?: number }[] = [
  { prop: "fillEnd", label: "Fill end", step: 1 },
  { prop: "trimStart", label: "Trim start", step: 0.05, min: 0, max: 1 },
  { prop: "trimEnd", label: "Trim end", step: 0.05, min: 0, max: 1 },
  { prop: "trimOffset", label: "Trim offset", step: 0.05 },
];

/** The label a track shows on the timeline. */
export const propLabel = (prop: string) => prop === "followDistance" ? "Path distance" : PROPS.find((p) => p.prop === prop)?.label ?? PAINT_PROPS.find((p) => p.prop === prop)?.label ?? prop;

/** Read a keyable property off a shape (design or evaluated values). */
export function readProp(s: Shape, prop: KeyProp): number {
  switch (prop) {
    case "followDistance": return s.follow?.distance ?? 0;
    case "fillEnd": return s.gradient?.stops[s.gradient.stops.length - 1]?.color ?? s.fill;
    case "trimStart": return s.stroke?.trim?.start ?? 0;
    case "trimEnd": return s.stroke?.trim?.end ?? 1;
    case "trimOffset": return s.stroke?.trim?.offset ?? 0;
    default: return s[prop];
  }
}

/** Write a keyable property to a shape's design values. A trim key needs "draw on" to be on. */
export function writeProp(s: Shape, prop: KeyProp, value: number) {
  switch (prop) {
    case "followDistance": if (s.follow) s.follow.distance = value; return;
    case "fillEnd": {
      const last = s.gradient?.stops[s.gradient.stops.length - 1];
      if (last) last.color = value; else s.fill = value;
      return;
    }
    case "trimStart": if (s.stroke?.trim) s.stroke.trim.start = value; return;
    case "trimEnd": if (s.stroke?.trim) s.stroke.trim.end = value; return;
    case "trimOffset": if (s.stroke?.trim) s.stroke.trim.offset = value; return;
    default: s[prop] = value;
  }
}

export const EASES: { name: string; ease: Ease }[] = [
  { name: "Smooth", ease: { type: "cubic", x1: 0.42, y1: 0, x2: 0.58, y2: 1 } },
  { name: "Ease in", ease: { type: "cubic", x1: 0.42, y1: 0, x2: 1, y2: 1 } },
  { name: "Ease out", ease: { type: "cubic", x1: 0, y1: 0, x2: 0.58, y2: 1 } },
  { name: "Bounce out", ease: { type: "cubic", x1: 0.34, y1: 1.56, x2: 0.64, y2: 1 } },
  { name: "Linear", ease: { type: "linear" } },
  { name: "Hold", ease: { type: "hold" } },
];

export const argbToCss = (c: number) => {
  const a = ((c >>> 24) & 255) / 255;
  return `rgba(${(c >>> 16) & 255}, ${(c >>> 8) & 255}, ${c & 255}, ${a})`;
};
export const argbToHex = (c: number) => "#" + (c & 0xffffff).toString(16).padStart(6, "0");
export const hexToArgb = (hex: string, alpha = 255) => ((alpha << 24) | parseInt(hex.slice(1), 16)) >>> 0;
