<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { argbToCss, localBounds, pathData, type Shape } from "../lib/types";

  let svg: SVGSVGElement;
  const pad = 60;
  const ab = $derived(editor.doc.artboard);

  // Pointer position in artboard coordinates.
  function point(e: PointerEvent) {
    const p = svg.createSVGPoint();
    p.x = e.clientX;
    p.y = e.clientY;
    const m = svg.getScreenCTM()!.inverse();
    const r = p.matrixTransform(m);
    return { x: r.x, y: r.y };
  }

  type Drag =
    | { kind: "create"; x0: number; y0: number; id: number | null }
    | { kind: "move"; id: number; dx: number; dy: number; moved: boolean }
    | { kind: "resize"; id: number; cx: number; cy: number; rot: number; sx: number; sy: number; corner: { x: number; y: number } | null; moved: boolean };
  let drag: Drag | null = null;

  function down(e: PointerEvent) {
    if (editor.playing) editor.pause();
    const p = point(e);
    svg.setPointerCapture(e.pointerId);
    if (editor.tool !== "select") {
      if (editor.mode === "animate") editor.mode = "design";
      drag = { kind: "create", x0: p.x, y0: p.y, id: null };
      return;
    }
    const target = (e.target as Element).closest("[data-id]");
    const handle = (e.target as Element).closest("[data-handle]");
    if (handle && editor.selected !== null) {
      const s = editor.shown.find((s) => s.id === editor.selected)!;
      editor.checkpoint();
      const b = localBounds(s);
      drag = { kind: "resize", id: s.id, cx: s.x, cy: s.y, rot: (s.rotation * Math.PI) / 180, sx: s.scaleX, sy: s.scaleY,
        corner: s.kind === "path" ? { x: b.x + b.w, y: b.y + b.h } : null, moved: false };
      return;
    }
    if (target) {
      const id = Number(target.getAttribute("data-id"));
      editor.selected = id;
      const s = editor.shown.find((s) => s.id === id)!;
      editor.checkpoint();
      drag = { kind: "move", id, dx: p.x - s.x, dy: p.y - s.y, moved: false };
    } else {
      editor.selected = null;
    }
  }

  function move(e: PointerEvent) {
    if (!drag) return;
    const p = point(e);
    if (drag.kind === "create") {
      const w = Math.abs(p.x - drag.x0), h = Math.abs(p.y - drag.y0);
      if (drag.id === null) {
        if (w < 3 && h < 3) return;
        drag.id = editor.addShape(editor.tool === "ellipse" ? "ellipse" : "rectangle", 0, 0, 1, 1);
      }
      const s = editor.doc.shapes.find((s) => s.id === drag!.id)!;
      s.x = Math.round((p.x + drag.x0) / 2);
      s.y = Math.round((p.y + drag.y0) / 2);
      s.width = Math.max(1, Math.round(w));
      s.height = Math.max(1, Math.round(h));
    } else if (drag.kind === "move") {
      drag.moved = true;
      editor.setProp(drag.id, "x", Math.round(p.x - drag.dx), false);
      editor.setProp(drag.id, "y", Math.round(p.y - drag.dy), false);
    } else {
      // Resize from the bottom-right handle: measure in the shape's own rotated frame.
      drag.moved = true;
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
    }
  }

  function up() {
    if (drag?.kind === "create") editor.tool = "select";
    drag = null;
  }

  const transform = (s: Shape) => `translate(${s.x} ${s.y}) rotate(${s.rotation}) scale(${s.scaleX} ${s.scaleY})`;
</script>

<div class="stage" class:creating={editor.tool !== "select"}>
  <svg
    bind:this={svg}
    viewBox="{-pad} {-pad} {ab.width + pad * 2} {ab.height + pad * 2}"
    onpointerdown={down}
    onpointermove={move}
    onpointerup={up}
    role="application"
    aria-label="Artboard"
  >
    <defs>
      <clipPath id="artboard-clip"><rect width={ab.width} height={ab.height} /></clipPath>
    </defs>
    <text x="0" y="-10" class="ab-name">{ab.name} · {ab.width} × {ab.height}</text>
    <rect width={ab.width} height={ab.height} fill={argbToCss(ab.background)} class="artboard" />
    <g clip-path="url(#artboard-clip)">
      {#each editor.shown as s (s.id)}
        <g transform={transform(s)} opacity={s.opacity} data-id={s.id} class="shape">
          {#if s.kind === "rectangle"}
            <rect x={-s.width / 2} y={-s.height / 2} width={s.width} height={s.height}
              rx={Math.min(s.cornerRadius, s.width / 2, s.height / 2)} fill={argbToCss(s.fill)} />
          {:else if s.kind === "ellipse"}
            <ellipse rx={s.width / 2} ry={s.height / 2} fill={argbToCss(s.fill)} />
          {:else}
            <path d={pathData(s)} fill={argbToCss(s.fill)} fill-rule={s.fillRule === "evenOdd" ? "evenodd" : "nonzero"} />
          {/if}
        </g>
      {/each}
    </g>
    {#each editor.shown.filter((s) => s.id === editor.selected) as s (s.id)}
      {@const b = localBounds(s)}
      <g transform={transform(s)} class="selection">
        <rect x={b.x} y={b.y} width={b.w} height={b.h} vector-effect="non-scaling-stroke" />
        <circle cx={b.x + b.w} cy={b.y + b.h} r={6 / Math.max(0.05, Math.abs(s.scaleX))} data-handle="resize" vector-effect="non-scaling-stroke" />
      </g>
    {/each}
  </svg>
  {#if editor.mode === "animate"}
    <div class="badge">Animate · frame {Math.round(editor.frame)}</div>
  {/if}
</div>

<style>
  .stage { position: relative; background: var(--bg); min-width: 0; min-height: 0; }
  .stage.creating svg { cursor: crosshair; }
  svg { width: 100%; height: 100%; display: block; user-select: none; touch-action: none; }
  .artboard { stroke: var(--line); }
  .ab-name { fill: var(--muted); font-size: 12px; }
  .shape { cursor: move; }
  .selection rect { fill: none; stroke: var(--accent); stroke-width: 1.5; pointer-events: none; }
  .selection circle { fill: #fff; stroke: var(--accent); stroke-width: 1.5; cursor: nwse-resize; }
  .badge { position: absolute; top: 10px; left: 50%; transform: translateX(-50%); background: var(--key); color: #000;
    padding: 2px 10px; border-radius: 99px; font-weight: 600; font-size: 12px; }
</style>
