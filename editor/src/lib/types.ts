// Mirrors crates/vectro-doc. The document travels to Rust as JSON.

export type ShapeKind = "rectangle" | "ellipse";
export type Prop =
  | "x" | "y" | "width" | "height" | "rotation"
  | "scaleX" | "scaleY" | "opacity" | "cornerRadius" | "fill";
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
}

export interface Key { frame: number; value: number; ease: Ease }
export interface Track { shapeId: number; prop: Prop; keys: Key[] }
export interface Animation { name: string; fps: number; duration: number; loop: LoopMode; tracks: Track[] }
export interface Artboard { name: string; width: number; height: number; background: number }
export interface Doc { version: number; artboard: Artboard; shapes: Shape[]; animations: Animation[] }

export const PROPS: { prop: Prop; label: string; step: number; min?: number; max?: number }[] = [
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
