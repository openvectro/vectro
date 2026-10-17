<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import {
    ancestors, argbToCss, featherStyle, isRoot, localBounds, paintOf, pathData, strokeAttrs, transformOf, worldMatrix,
    type Shape, type Vertex,
  } from "../lib/types";
  import * as core from "../wasm/vectro_wasm.js";

  let svg: SVGSVGElement;
  const pad = 60;
  const ab = $derived(editor.doc.artboard);
  const view = $derived(editor.view);
  const viewBox = $derived(`${-pad + view.x} ${-pad + view.y} ${(ab.width + pad * 2) / view.zoom} ${(ab.height + pad * 2) / view.zoom}`);
  /** Screen pixels to artboard units at the current zoom, for handle sizes. */
  const px = $derived.by(() => {
    const w = svg?.clientWidth || 800;
    return ((ab.width + pad * 2) / view.zoom) / w;
  });

  // Pointer position in artboard coordinates.
  function point(e: PointerEvent | MouseEvent | WheelEvent) {
    const p = svg.createSVGPoint();
    p.x = e.clientX;
    p.y = e.clientY;
    return p.matrixTransform(svg.getScreenCTM()!.inverse());
  }

  /** A point in artboard space, expressed in the space a shape's x/y live in (its group's). */
  function toParent(s: Shape, p: { x: number; y: number }) {
    return worldMatrix(editor.shown, s, false).inverse().transformPoint(new DOMPoint(p.x, p.y));
  }

  /** A point in artboard space, in a shape's own (path point) space. */
  function toLocal(s: Shape, p: { x: number; y: number }) {
    return worldMatrix(editor.shown, s).inverse().transformPoint(new DOMPoint(p.x, p.y));
  }

  type Drag =
    | { kind: "create"; x0: number; y0: number; id: number | null }
    | { kind: "move"; ids: number[]; start: Map<number, { x: number; y: number }>; from: { x: number; y: number }; moved: boolean }
    | { kind: "resize"; id: number; cx: number; cy: number; rot: number; sx: number; sy: number; corner: { x: number; y: number } | null }
    | { kind: "box"; x0: number; y0: number; x1: number; y1: number; add: boolean }
    | { kind: "pan"; x0: number; y0: number; vx: number; vy: number }
    | { kind: "pen"; id: number; index: number }
    | { kind: "vertex"; id: number; path: number; index: number; what: "point" | "in" | "out"; alt: boolean }
    | { kind: "pivot"; id: number };
  let drag = $state<Drag | null>(null);
  let cursor = $state<{ x: number; y: number } | null>(null);
  let spaceDown = $state(false);

  const pathOf = (s: Shape) => s.paths?.[editor.selectedVertex?.path ?? 0];
  const nodeShape = $derived.by(() => {
    const s = editor.shown.find((s) => s.id === editor.selected);
    return editor.tool === "node" && s?.kind === "path" ? s : null;
  });
  const penShape = $derived.by(() => {
    const s = editor.shown.find((s) => s.id === editor.selected);
    return editor.tool === "pen" && s?.kind === "path" && !(s.paths?.[0]?.closed) && penOpen ? s : null;
  });
  let penOpen = $state(false);

  function down(e: PointerEvent) {
    if (editor.playing) editor.pause();
    const p = point(e);
    svg.setPointerCapture(e.pointerId);
    if (e.button === 1 || spaceDown) {
      drag = { kind: "pan", x0: e.clientX, y0: e.clientY, vx: view.x, vy: view.y };
      return;
    }
    const target = (e.target as Element).closest("[data-id]");
    const handle = (e.target as Element).closest("[data-handle]");
    const vertexEl = (e.target as Element).closest("[data-vertex]");

    if (editor.tool === "pen") {
      if (editor.mode === "animate") editor.mode = "design";
      const s = penShape;
      const v = (x: number, y: number): Vertex => ({ x, y, inX: x, inY: y, outX: x, outY: y });
      if (s && s.paths) {
        const first = s.paths[0].points[0];
        if (s.paths[0].points.length > 2 && Math.hypot(first.x - p.x, first.y - p.y) < 8 * px) {
          editor.checkpoint();
          editor.shape(s.id)!.paths![0].closed = true;
          finishPen();
          return;
        }
        editor.checkpoint();
        const real = editor.shape(s.id)!;
        real.paths![0].points.push(v(p.x, p.y));
        drag = { kind: "pen", id: s.id, index: real.paths![0].points.length - 1 };
      } else {
        const id = editor.addPath(v(p.x, p.y));
        penOpen = true;
        drag = { kind: "pen", id, index: 0 };
      }
      return;
    }

    if (editor.tool === "node" && nodeShape) {
      if (vertexEl) {
        const index = Number(vertexEl.getAttribute("data-vertex"));
        let what = (vertexEl.getAttribute("data-what") as "point" | "in" | "out") ?? "point";
        editor.selectedVertex = { path: editor.selectedVertex?.path ?? 0, index };
        editor.checkpoint();
        // Alt-drag on a point pulls a mirrored pair of handles out of it (a corner becomes a curve).
        const pull = what === "point" && e.altKey;
        if (pull) what = "out";
        drag = { kind: "vertex", id: nodeShape.id, path: editor.selectedVertex.path, index, what, alt: e.altKey && !pull };
        return;
      }
      if (target && Number(target.getAttribute("data-id")) === nodeShape.id) return;
      // Clicking elsewhere leaves node editing.
      editor.tool = "select";
    }

    if (editor.tool === "pivot" && editor.selectedShape) {
      if (editor.mode === "animate") editor.mode = "design";
      drag = { kind: "pivot", id: editor.selectedShape.id };
      return;
    }

    if (editor.tool === "rectangle" || editor.tool === "ellipse") {
      if (editor.mode === "animate") editor.mode = "design";
      drag = { kind: "create", x0: p.x, y0: p.y, id: null };
      return;
    }

    if (handle && editor.selected !== null) {
      const s = editor.shown.find((s) => s.id === editor.selected)!;
      editor.checkpoint();
      const b = localBounds(s);
      const m = worldMatrix(editor.shown, s);
      const c = m.transformPoint(new DOMPoint(0, 0));
      drag = { kind: "resize", id: s.id, cx: c.x, cy: c.y, rot: Math.atan2(m.b, m.a), sx: s.scaleX, sy: s.scaleY,
        corner: s.kind === "path" ? { x: b.x + b.w, y: b.y + b.h } : null };
      return;
    }
    if (target) {
      // A click picks the outermost group unless a piece of it is already selected; see dblclick() for the rest.
      let s = editor.shown.find((s) => s.id === Number(target.getAttribute("data-id")))!;
      const outer = ancestors(editor.shown, s)[0];
      if (outer && !editor.selectedIds.includes(s.id)) s = outer;
      if (e.shiftKey) editor.toggleSelected(s.id);
      else if (!editor.selectedIds.includes(s.id)) editor.selected = s.id;
      const ids = editor.selectedIds.filter((id) => editor.shown.some((x) => x.id === id));
      editor.checkpoint();
      const start = new Map(ids.map((id) => { const x = editor.shown.find((x) => x.id === id)!; return [id, { x: x.x, y: x.y }]; }));
      drag = { kind: "move", ids, start, from: p, moved: false };
    } else {
      if (!e.shiftKey) editor.selected = null;
      drag = { kind: "box", x0: p.x, y0: p.y, x1: p.x, y1: p.y, add: e.shiftKey };
    }
  }

  function move(e: PointerEvent) {
    const p = point(e);
    cursor = { x: p.x, y: p.y };
    if (!drag) return;
    if (drag.kind === "pan") {
      const k = ((ab.width + pad * 2) / view.zoom) / svg.clientWidth;
      editor.view = { zoom: view.zoom, x: drag.vx - (e.clientX - drag.x0) * k, y: drag.vy - (e.clientY - drag.y0) * k };
    } else if (drag.kind === "create") {
      const w = Math.abs(p.x - drag.x0), h = Math.abs(p.y - drag.y0);
      if (drag.id === null) {
        if (w < 3 * px && h < 3 * px) return;
        drag.id = editor.addShape(editor.tool === "ellipse" ? "ellipse" : "rectangle", 0, 0, 1, 1);
      }
      const s = editor.shape(drag.id)!;
      const square = e.shiftKey;
      const size = Math.max(w, h);
      s.x = Math.round((p.x + drag.x0) / 2);
      s.y = Math.round((p.y + drag.y0) / 2);
      s.width = Math.max(1, Math.round(square ? size : w));
      s.height = Math.max(1, Math.round(square ? size : h));
    } else if (drag.kind === "move") {
      drag.moved = true;
      for (const id of drag.ids) {
        const s = editor.shown.find((s) => s.id === id)!;
        const a = toParent(s, drag.from), b = toParent(s, p);
        const st = drag.start.get(id)!;
        editor.setProp(id, "x", Math.round(st.x + b.x - a.x), false);
        editor.setProp(id, "y", Math.round(st.y + b.y - a.y), false);
      }
    } else if (drag.kind === "resize") {
      const dx = p.x - drag.cx, dy = p.y - drag.cy;
      const lx = dx * Math.cos(-drag.rot) - dy * Math.sin(-drag.rot);
      const ly = dx * Math.sin(-drag.rot) + dy * Math.cos(-drag.rot);
      if (drag.corner) {
        // Paths have no width or height to change: resizing scales them uniformly.
        const k = Math.max(Math.abs(lx / drag.corner.x || 0), Math.abs(ly / drag.corner.y || 0), 0.01);
        editor.setProp(drag.id, "scaleX", +k.toFixed(3), false);
        editor.setProp(drag.id, "scaleY", +k.toFixed(3), false);
      } else {
        editor.setProp(drag.id, "width", Math.max(1, Math.round((2 * Math.abs(lx)) / Math.abs(drag.sx || 1))), false);
        editor.setProp(drag.id, "height", Math.max(1, Math.round((2 * Math.abs(ly)) / Math.abs(drag.sy || 1))), false);
      }
    } else if (drag.kind === "box") {
      drag.x1 = p.x;
      drag.y1 = p.y;
    } else if (drag.kind === "pen") {
      // Dragging out of a new point pulls its handles out, mirrored.
      const s = editor.shape(drag.id)!;
      const v = s.paths![0].points[drag.index];
      v.outX = p.x; v.outY = p.y;
      v.inX = 2 * v.x - p.x; v.inY = 2 * v.y - p.y;
    } else if (drag.kind === "pivot") {
      // The pivot follows the pointer; the shape stays put on screen.
      const id = drag.id;
      const s = editor.shown.find((x) => x.id === id)!;
      const lp = toLocal(s, p);
      editor.setPivot(id, Math.round(lp.x), Math.round(lp.y));
      editor.undoMerge();
    } else if (drag.kind === "vertex") {
      const id = drag.id;
      const s = editor.shape(id)!;
      const v = s.paths![drag.path].points[drag.index];
      const lp = toLocal(editor.shown.find((x) => x.id === id)!, p);
      if (drag.what === "point") {
        const dx = lp.x - v.x, dy = lp.y - v.y;
        v.x += dx; v.y += dy; v.inX += dx; v.inY += dy; v.outX += dx; v.outY += dy;
      } else {
        const mirrored = !drag.alt && (isMirrored(v) || drag.what === "out" && v.inX === v.x && v.inY === v.y);
        if (drag.what === "in") { v.inX = lp.x; v.inY = lp.y; if (mirrored) { v.outX = 2 * v.x - lp.x; v.outY = 2 * v.y - lp.y; } }
        else { v.outX = lp.x; v.outY = lp.y; if (mirrored) { v.inX = 2 * v.x - lp.x; v.inY = 2 * v.y - lp.y; } }
      }
    }
  }

  function up(e: PointerEvent) {
    if (!drag) return;
    if (drag.kind === "create") editor.tool = "select";
    if (drag.kind === "box") {
      const l = Math.min(drag.x0, drag.x1), r = Math.max(drag.x0, drag.x1), t = Math.min(drag.y0, drag.y1), b = Math.max(drag.y0, drag.y1);
      if (r - l > 2 * px || b - t > 2 * px) {
        const hit = editor.shown.filter((s) => isRoot(editor.shown, s) && s.kind !== "group" || (s.kind === "group" && isRoot(editor.shown, s)))
          .filter((s) => { const wb = editor.worldBounds(s); return wb.l < r && wb.r > l && wb.t < b && wb.b > t; })
          .map((s) => s.id);
        editor.selectedIds = drag.add ? [...new Set([...editor.selectedIds, ...hit])] : hit;
      }
    }
    drag = null;
  }

  /** Double click: into a group, into a path's points, onto an outline to add a point, or finish the pen. */
  function dblclick(e: MouseEvent) {
    e.preventDefault();
    if (editor.tool === "pen") {
      finishPen();
      return;
    }
    // Pointer capture from the first press retargets this event to the svg, so hit-test the position.
    const target = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-id]");
    if (!target) return;
    const s = editor.shown.find((s) => s.id === Number(target.getAttribute("data-id")))!;
    const p = point(e);
    if (editor.tool === "node" && nodeShape && s.id === nodeShape.id) {
      insertVertexNear(nodeShape, p);
      return;
    }
    editor.selected = s.id;
    if (s.kind === "path") {
      editor.tool = "node";
      if (editor.mode === "animate") editor.mode = "design";
    }
  }

  const isMirrored = (v: Vertex) => Math.abs(v.inX + v.outX - 2 * v.x) < 0.01 && Math.abs(v.inY + v.outY - 2 * v.y) < 0.01;

  /** End the path being drawn: centre its origin and hand over to point editing. */
  export function finishPen() {
    const s = editor.selectedShape;
    if (!s || s.kind !== "path") return;
    if ((s.paths?.[0]?.points.length ?? 0) < 2) editor.deleteShape(s.id);
    else editor.recentre(s.id);
    penOpen = false;
    editor.tool = "select";
    drag = null;
  }

  /** Split the segment nearest to p (artboard space) by de Casteljau, adding a point there. */
  function insertVertexNear(s: Shape, p: { x: number; y: number }) {
    const lp = toLocal(s, p);
    const path = pathOf(s)!;
    const pts = path.points;
    const n = path.closed ? pts.length : pts.length - 1;
    let best = { d: Infinity, seg: 0, t: 0 };
    const at = (a: Vertex, b: Vertex, t: number) => {
      const u = 1 - t;
      return {
        x: u * u * u * a.x + 3 * u * u * t * a.outX + 3 * u * t * t * b.inX + t * t * t * b.x,
        y: u * u * u * a.y + 3 * u * u * t * a.outY + 3 * u * t * t * b.inY + t * t * t * b.y,
      };
    };
    for (let i = 0; i < n; i++) {
      const a = pts[i], b = pts[(i + 1) % pts.length];
      for (let k = 1; k < 40; k++) {
        const q = at(a, b, k / 40);
        const d = Math.hypot(q.x - lp.x, q.y - lp.y);
        if (d < best.d) best = { d, seg: i, t: k / 40 };
      }
    }
    if (best.d > 12 * px) return;
    editor.checkpoint();
    const real = editor.shape(s.id)!.paths![editor.selectedVertex?.path ?? 0];
    const a = real.points[best.seg], b = real.points[(best.seg + 1) % real.points.length];
    const t = best.t, u = 1 - t;
    const lerp = (p0: number, p1: number) => u * p0 + t * p1;
    const ax = lerp(a.x, a.outX), ay = lerp(a.y, a.outY);
    const mx = lerp(a.outX, b.inX), my = lerp(a.outY, b.inY);
    const bx = lerp(b.inX, b.x), by = lerp(b.inY, b.y);
    const inX = lerp(ax, mx), inY = lerp(ay, my), outX = lerp(mx, bx), outY = lerp(my, by);
    const x = lerp(inX, outX), y = lerp(inY, outY);
    a.outX = ax; a.outY = ay; b.inX = bx; b.inY = by;
    real.points.splice(best.seg + 1, 0, { x, y, inX, inY, outX, outY });
    editor.selectedVertex = { path: editor.selectedVertex?.path ?? 0, index: best.seg + 1 };
  }

  export function deleteVertex() {
    const s = editor.selectedShape, sv = editor.selectedVertex;
    if (!s || !sv || !s.paths) return;
    editor.checkpoint();
    s.paths[sv.path].points.splice(sv.index, 1);
    if (s.paths[sv.path].points.length < 2) editor.deleteShape(s.id);
    editor.selectedVertex = null;
  }

  function wheel(e: WheelEvent) {
    e.preventDefault();
    const p = point(e);
    if (e.ctrlKey || e.metaKey) editor.zoomAt(Math.exp(-e.deltaY * 0.0025), p.x, p.y);
    else {
      const k = ((ab.width + pad * 2) / view.zoom) / svg.clientWidth;
      editor.view = { zoom: view.zoom, x: view.x + e.deltaX * k, y: view.y + e.deltaY * k };
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === " " && !(e.target instanceof HTMLInputElement)) spaceDown = e.type === "keydown";
  }

  const transform = transformOf;
  const ox = (s: Shape) => -s.width * (s.originX ?? 0.5);
  const oy = (s: Shape) => -s.height * (s.originY ?? 0.5);
  const roots = $derived(editor.shown.filter((s) => isRoot(editor.shown, s)));

  /** Ghosts of the frames either side of the playhead, when onion skin is on. */
  const ghosts = $derived.by<{ shapes: Shape[]; colour: string }[]>(() => {
    if (!editor.onion || editor.mode !== "animate" || !editor.animation) return [];
    const json = JSON.stringify(editor.doc);
    const out: { shapes: Shape[]; colour: string }[] = [];
    for (const [d, colour] of [[-editor.onionStep, "#ff6b6b"], [editor.onionStep, "#51cf66"]] as const) {
      const f = editor.frame + d;
      if (f < 0 || f > editor.animation.duration) continue;
      try { out.push({ shapes: JSON.parse(core.evaluate(json, editor.anim, f)), colour }); } catch {}
    }
    return out;
  });
  const ghostChain = (shapes: Shape[], s: Shape) => [...ancestors(shapes, s), s].map(transformOf).join(" ");
  const childrenOf = (id: number) => editor.shown.filter((c) => c.parent === id);
  const chain = (s: Shape) => [...ancestors(editor.shown, s), s].map(transformOf).join(" ");
  const selectedShapes = $derived(editor.shown.filter((s) => editor.selectedIds.includes(s.id)));
</script>

<svelte:window onkeydown={key} onkeyup={key} />

<div class="stage" class:creating={editor.tool === "rectangle" || editor.tool === "ellipse"} class:pen={editor.tool === "pen"} class:pivoting={editor.tool === "pivot"} class:panning={spaceDown}>
  <svg
    bind:this={svg}
    {viewBox}
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    onwheel={wheel}
    ondblclick={dblclick}
    role="application"
    aria-label="Artboard"
  >
    <defs>
      <clipPath id="artboard-clip"><rect width={ab.width} height={ab.height} /></clipPath>
    </defs>
    <text x="0" y={-10 * px} class="ab-name" font-size={12 * px}>{ab.name} · {ab.width} × {ab.height}</text>
    <rect width={ab.width} height={ab.height} fill={argbToCss(ab.background)} class="artboard" vector-effect="non-scaling-stroke" />
    <g clip-path="url(#artboard-clip)">
      {#snippet node(s: Shape)}
        <g transform={transform(s)} opacity={s.opacity} data-id={s.kind === "group" ? undefined : s.id} class:shape={s.kind !== "group"} style={featherStyle(s)}>
          {#if s.gradient}
            <defs>
              {#if s.gradient.kind === "linear"}
                <linearGradient id="grad-{s.id}" gradientUnits="userSpaceOnUse" x1={s.gradient.start[0]} y1={s.gradient.start[1]} x2={s.gradient.end[0]} y2={s.gradient.end[1]}>
                  {#each s.gradient.stops as st}<stop offset={st.position} stop-color={argbToCss(st.color)} />{/each}
                </linearGradient>
              {:else}
                <radialGradient id="grad-{s.id}" gradientUnits="userSpaceOnUse" cx={s.gradient.start[0]} cy={s.gradient.start[1]} fx={s.gradient.start[0]} fy={s.gradient.start[1]}
                  r={Math.hypot(s.gradient.end[0] - s.gradient.start[0], s.gradient.end[1] - s.gradient.start[1])}>
                  {#each s.gradient.stops as st}<stop offset={st.position} stop-color={argbToCss(st.color)} />{/each}
                </radialGradient>
              {/if}
            </defs>
          {/if}
          {#if s.kind === "group"}
            {#each childrenOf(s.id) as c (c.id)}{@render node(c)}{/each}
          {:else if s.kind === "rectangle"}
            <rect x={ox(s)} y={oy(s)} width={s.width} height={s.height}
              rx={Math.min(s.cornerRadius, s.width / 2, s.height / 2)} fill={paintOf(s)} {...strokeAttrs(s)} />
          {:else if s.kind === "ellipse"}
            <ellipse cx={ox(s) + s.width / 2} cy={oy(s) + s.height / 2} rx={s.width / 2} ry={s.height / 2} fill={paintOf(s)} {...strokeAttrs(s)} />
          {:else}
            <path d={pathData(s)} fill={paintOf(s)} fill-rule={s.fillRule === "evenOdd" ? "evenodd" : "nonzero"} {...strokeAttrs(s)}
              stroke={s.stroke ? argbToCss(s.stroke.color) : s.paths?.[0]?.closed === false && !s.stroke ? "#888" : undefined}
              stroke-width={s.stroke ? s.stroke.thickness : s.paths?.[0]?.closed === false ? 1.5 * px : undefined} />
          {/if}
        </g>
      {/snippet}
      {#each roots as s (s.id)}{@render node(s)}{/each}
    </g>

    <!-- Onion skin: outlines of the frames either side of the playhead -->
    {#each ghosts as g}
      <g class="ghost" style="color: {g.colour}">
        {#each g.shapes.filter((s) => s.kind !== "group") as s (s.id)}
          <g transform={ghostChain(g.shapes, s)} opacity={0.6 * s.opacity}>
            {#if s.kind === "rectangle"}
              <rect x={ox(s)} y={oy(s)} width={s.width} height={s.height} rx={Math.min(s.cornerRadius, s.width / 2, s.height / 2)} vector-effect="non-scaling-stroke" />
            {:else if s.kind === "ellipse"}
              <ellipse cx={ox(s) + s.width / 2} cy={oy(s) + s.height / 2} rx={s.width / 2} ry={s.height / 2} vector-effect="non-scaling-stroke" />
            {:else}
              <path d={pathData(s)} vector-effect="non-scaling-stroke" />
            {/if}
          </g>
        {/each}
      </g>
    {/each}

    <!-- Selection outlines -->
    {#if editor.tool !== "node" && !penShape}
      {#each selectedShapes as s (s.id)}
        {@const b = localBounds(s)}
        <g transform={chain(s)} class="selection">
          {#if s.kind === "group"}
            <path d="M{-10 * px} 0H{10 * px}M0 {-10 * px}V{10 * px}" vector-effect="non-scaling-stroke" class="origin" />
          {:else}
            <rect x={b.x} y={b.y} width={b.w} height={b.h} vector-effect="non-scaling-stroke" />
            {#if s.id === editor.selected}
              <circle cx={b.x + b.w} cy={b.y + b.h} r={6 * px / Math.max(0.05, Math.abs(s.scaleX))} data-handle="resize" vector-effect="non-scaling-stroke" />
            {/if}
          {/if}
        </g>
      {/each}
    {/if}

    <!-- The pivot, while the pivot tool is on -->
    {#if editor.tool === "pivot" && editor.selectedShape}
      {@const s = editor.shown.find((x) => x.id === editor.selected)}
      {#if s}
        <g transform={chain(s)} class="pivot">
          <circle r={9 * px} vector-effect="non-scaling-stroke" />
          <path d="M{-14 * px} 0H{14 * px}M0 {-14 * px}V{14 * px}" vector-effect="non-scaling-stroke" />
        </g>
      {/if}
    {/if}

    <!-- Point editing -->
    {#if nodeShape || penShape}
      {@const s = (nodeShape ?? penShape)!}
      {@const path = s.paths?.[editor.selectedVertex?.path ?? 0]}
      {#if path}
        <g transform={chain(s)} class="nodes">
          <path d={pathData(s)} vector-effect="non-scaling-stroke" class="outline" />
          {#if penShape && cursor && path.points.length}
            {@const last = path.points[path.points.length - 1]}
            {@const lc = toLocal(s, cursor)}
            <path d="M{last.outX} {last.outY}L{lc.x} {lc.y}" vector-effect="non-scaling-stroke" class="rubber" />
          {/if}
          {#each path.points as v, i}
            {@const sel = editor.selectedVertex?.index === i}
            {#if nodeShape && (sel || editor.selectedVertex === null)}
              <path d="M{v.inX} {v.inY}L{v.x} {v.y}L{v.outX} {v.outY}" vector-effect="non-scaling-stroke" class="handle-line" />
              <circle cx={v.inX} cy={v.inY} r={4 * px} data-vertex={i} data-what="in" class="handle" vector-effect="non-scaling-stroke" />
              <circle cx={v.outX} cy={v.outY} r={4 * px} data-vertex={i} data-what="out" class="handle" vector-effect="non-scaling-stroke" />
            {/if}
            <rect x={v.x - 4 * px} y={v.y - 4 * px} width={8 * px} height={8 * px} data-vertex={i} data-what="point"
              class="vertex" class:sel class:first={penShape && i === 0} vector-effect="non-scaling-stroke" />
          {/each}
        </g>
      {/if}
    {/if}

    {#if drag?.kind === "box"}
      <rect x={Math.min(drag.x0, drag.x1)} y={Math.min(drag.y0, drag.y1)} width={Math.abs(drag.x1 - drag.x0)} height={Math.abs(drag.y1 - drag.y0)} class="box" vector-effect="non-scaling-stroke" />
    {/if}
  </svg>
  <div class="hud">
    {#if editor.mode === "animate"}<span class="badge">Animate · frame {Math.round(editor.frame)}</span>{/if}
    {#if editor.tool === "pen"}<span class="tip">Click to add points, drag for curves, click the first point to close, Enter to finish</span>{/if}
    {#if editor.tool === "node"}<span class="tip">Drag points and handles · Alt breaks a handle pair · double-click the outline to add a point · ⌫ removes one</span>{/if}
    {#if editor.tool === "pivot"}<span class="tip">Drag anywhere to put the pivot there; the shape stays where it is. Rotation and scale turn about the pivot.</span>{/if}
    <span class="zoom">{Math.round(view.zoom * 100)}%</span>
  </div>
</div>

<style>
  .stage { position: relative; background: var(--bg); min-width: 0; min-height: 0; overflow: hidden; }
  .stage.creating svg, .stage.pen svg { cursor: crosshair; }
  .stage.panning svg { cursor: grab; }
  svg { width: 100%; height: 100%; display: block; user-select: none; touch-action: none; }
  .artboard { stroke: var(--line); }
  .ab-name { fill: var(--muted); }
  .shape { cursor: move; }
  .selection rect { fill: none; stroke: var(--accent); stroke-width: 1.5; pointer-events: none; }
  .selection .origin { stroke: var(--accent); stroke-width: 2; }
  .selection circle { fill: #fff; stroke: var(--accent); stroke-width: 1.5; cursor: nwse-resize; }
  .nodes .outline { fill: none; stroke: var(--accent); stroke-width: 1; pointer-events: none; }
  .nodes .rubber { fill: none; stroke: var(--accent); stroke-width: 1; stroke-dasharray: 4 3; pointer-events: none; }
  .nodes .handle-line { fill: none; stroke: #8b8b96; stroke-width: 1; pointer-events: none; }
  .nodes .handle { fill: var(--panel); stroke: #8b8b96; stroke-width: 1.5; cursor: grab; }
  .nodes .vertex { fill: #fff; stroke: var(--accent); stroke-width: 1.5; cursor: grab; }
  .nodes .vertex.sel { fill: var(--accent); }
  .nodes .vertex.first { stroke: var(--key); }
  .box { fill: var(--accent-soft); stroke: var(--accent); stroke-width: 1; pointer-events: none; }
  .ghost * { fill: none; stroke: currentColor; stroke-width: 1.2; stroke-dasharray: 4 3; pointer-events: none; }
  .pivot circle { fill: var(--key); stroke: #000; stroke-width: 1; pointer-events: none; }
  .pivot path { stroke: var(--key); stroke-width: 1.5; pointer-events: none; }
  .stage.pivoting svg { cursor: crosshair; }
  .hud { position: absolute; top: 10px; left: 12px; right: 12px; display: flex; gap: 8px; align-items: center; pointer-events: none; }
  .badge { background: var(--key); color: #000; padding: 2px 10px; border-radius: 99px; font-weight: 600; font-size: 12px; }
  .tip { color: var(--muted); font-size: 12px; background: var(--panel); padding: 2px 10px; border-radius: 99px; border: 1px solid var(--line); }
  .zoom { margin-left: auto; color: var(--muted); font-size: 12px; font-variant-numeric: tabular-nums; }
</style>
