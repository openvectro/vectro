// State machine edits that touch indices, kept together so nothing dangles.
import type { Action, Doc, Input, InputKind, Layer, State, StateMachine, Transition } from "./types";

export const OPS: { op: import("./types").Op; label: string }[] = [
  { op: "equal", label: "=" }, { op: "notEqual", label: "≠" }, { op: "less", label: "<" },
  { op: "lessOrEqual", label: "≤" }, { op: "greater", label: ">" }, { op: "greaterOrEqual", label: "≥" },
];
export const POINTERS: { kind: import("./types").PointerKind; label: string }[] = [
  { kind: "enter", label: "Pointer enters" }, { kind: "exit", label: "Pointer leaves" }, { kind: "down", label: "Press" },
  { kind: "up", label: "Release" }, { kind: "click", label: "Click" }, { kind: "move", label: "Pointer moves" },
];

export function newLayer(name: string): Layer {
  return {
    name,
    states: [
      { name: "Entry", kind: { type: "entry" }, ui: [40, 40] },
      { name: "Any", kind: { type: "any" }, ui: [40, 130] },
      { name: "Exit", kind: { type: "exit" }, ui: [40, 220] },
    ],
    transitions: [],
  };
}

export function newMachine(name: string): StateMachine {
  return { name, inputs: [], layers: [newLayer("Layer 1")], listeners: [] };
}

export function ensureMachines(doc: Doc) {
  if (!doc.stateMachines) doc.stateMachines = [];
  if (!doc.events) doc.events = [];
}

export function addState(layer: Layer, state: State) {
  // Place new states in a loose grid to the right of the fixed three.
  const n = layer.states.length - 3;
  state.ui ??= [220 + (n % 3) * 190, 40 + Math.floor(n / 3) * 110];
  layer.states.push(state);
  return layer.states.length - 1;
}

export function deleteState(layer: Layer, index: number) {
  if (index < 3) return; // Entry, Any and Exit stay
  layer.states.splice(index, 1);
  layer.transitions = layer.transitions
    .filter((t) => t.from !== index && t.to !== index)
    .map((t) => ({ ...t, from: t.from > index ? t.from - 1 : t.from, to: t.to > index ? t.to - 1 : t.to }));
}

export function addTransition(layer: Layer, from: number, to: number): Transition {
  const t: Transition = { from, to, conditions: [] };
  layer.transitions.push(t);
  return t;
}

export function addInput(sm: StateMachine, kind: InputKind): Input {
  const base = kind === "bool" ? "flag" : kind === "number" ? "amount" : "go";
  let n = 1;
  while (sm.inputs.some((i) => i.name === `${base}${n > 1 ? n : ""}`)) n++;
  const input: Input = { name: `${base}${n > 1 ? n : ""}`, kind, value: 0 };
  sm.inputs.push(input);
  return input;
}

/** Remove an input and everything that pointed at it. */
export function deleteInput(sm: StateMachine, index: number) {
  sm.inputs.splice(index, 1);
  const fix = (i: number) => (i > index ? i - 1 : i);
  for (const layer of sm.layers) {
    for (const t of layer.transitions) t.conditions = t.conditions.filter((c) => c.input !== index).map((c) => ({ ...c, input: fix(c.input) }));
    layer.states = layer.states.filter((s) => !(s.kind.type === "blend1D" && s.kind.input === index));
    for (const s of layer.states) if (s.kind.type === "blend1D") s.kind.input = fix(s.kind.input);
  }
  for (const l of sm.listeners) {
    l.actions = l.actions.filter((a) => a.type === "fireEvent" || a.input !== index).map((a) => (a.type === "fireEvent" ? a : { ...a, input: fix(a.input) }));
  }
}

/** Remove an event and the actions that fired it. */
export function deleteEvent(doc: Doc, index: number) {
  doc.events!.splice(index, 1);
  for (const sm of doc.stateMachines ?? []) {
    for (const l of sm.listeners) {
      l.actions = l.actions.filter((a) => !(a.type === "fireEvent" && a.event === index)).map((a) => (a.type === "fireEvent" && a.event > index ? { ...a, event: a.event - 1 } : a));
    }
  }
}

/** Keep states pointing at the right animations when one is removed. */
export function animationRemoved(doc: Doc, index: number) {
  for (const sm of doc.stateMachines ?? []) {
    for (const layer of sm.layers) {
      const gone = new Set<number>();
      layer.states.forEach((s, i) => {
        if (s.kind.type === "animation") {
          if (s.kind.animation === index) gone.add(i);
          else if (s.kind.animation > index) s.kind.animation--;
        } else if (s.kind.type === "blend1D") {
          s.kind.children = s.kind.children.filter(([a]) => a !== index).map(([a, v]) => [a > index ? a - 1 : a, v]);
        }
      });
      for (const i of [...gone].sort((a, b) => b - a)) deleteState(layer, i);
    }
  }
}

export function defaultAction(sm: StateMachine, doc: Doc): Action | null {
  const bool = sm.inputs.findIndex((i) => i.kind === "bool");
  if (bool >= 0) return { type: "setBool", input: bool, value: true };
  const trig = sm.inputs.findIndex((i) => i.kind === "trigger");
  if (trig >= 0) return { type: "fireTrigger", input: trig };
  const num = sm.inputs.findIndex((i) => i.kind === "number");
  if (num >= 0) return { type: "setNumber", input: num, value: 1 };
  if (doc.events?.length) return { type: "fireEvent", event: 0 };
  return null;
}

/** One line describing a transition, for the arrow label. */
export function describe(t: Transition, sm: StateMachine) {
  const parts: string[] = [];
  for (const c of t.conditions) {
    const i = sm.inputs[c.input];
    if (!i) continue;
    if (i.kind === "bool") parts.push(`${i.name} ${c.value ? "on" : "off"}`);
    else if (i.kind === "trigger") parts.push(`⚡ ${i.name}`);
    else parts.push(`${i.name} ${OPS.find((o) => o.op === c.op)?.label ?? "="} ${c.value}`);
  }
  if (t.exitAfterMs !== undefined) parts.push(`after ${t.exitAfterMs} ms`);
  return parts.join(" · ") || "always";
}
