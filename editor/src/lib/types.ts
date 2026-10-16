// Mirrors crates/vectro-doc. The document travels to Rust as JSON.

export type ShapeKind = "rectangle" | "ellipse" | "path" | "group";
export interface Vertex { x: number; y: number; inX: number; inY: number; outX: number; outY: number }
export interface SubPath { closed: boolean; points: Vertex[] }
export type ShapeProp =
  | "x" | "y" | "width" | "height" | "rotation"
  | "scaleX" | "scaleY" | "opacity" | "cornerRadius" | "fill";
export type PointProp = "pointX" | "pointY" | "inAngle" | "inLength" | "outAngle" | "outLength";
/** The last stop of a gradient fill; "fill" is the first stop or the flat colour. */
export type PaintProp = "fillEnd";
export type Prop = ShapeProp | PointProp | PaintProp;
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
  gradient?: { kind: "linear" | "radial"; start: [number, number]; end: [number, number]; stops: { position: number; color: number }[] };
  stroke?: { color: number; thickness: number; cap?: number; join?: number };
  feather?: { strength: number; offset?: [number, number]; inner?: boolean };
}

/** SVG paint for a shape: a gradient reference or the flat colour. */
export const paintOf = (s: Shape) => (s.gradient ? `url(#grad-${s.id})` : argbToCss(s.fill));
/** A soft blur that stands in for Rive's feather (drawn properly by the runtime). */
export const featherStyle = (s: Shape) => (s.feather && !s.feather.inner ? `filter: blur(${(s.feather.strength / 2).toFixed(1)}px)` : "");
const CAPS = ["butt", "round", "square"] as const, JOINS = ["miter", "round", "bevel"] as const;
export const strokeAttrs = (s: Shape) =>
  s.stroke ? { stroke: argbToCss(s.stroke.color), "stroke-width": s.stroke.thickness, "stroke-linecap": CAPS[s.stroke.cap ?? 0], "stroke-linejoin": JOINS[s.stroke.join ?? 0] } : {};

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
  if (s.kind !== "path" || !s.paths?.length) return { x: -s.width / 2, y: -s.height / 2, w: s.width, h: s.height };
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
export interface Doc { version: number; artboard: Artboard; shapes: Shape[]; animations: Animation[] }

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
