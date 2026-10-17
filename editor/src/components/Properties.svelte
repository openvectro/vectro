<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { PAINT_PROPS, PROPS, argbToHex, hexToArgb, readProp, type ShapeProp as Prop, type TrimMode } from "../lib/types";

  const TRIM_PROPS = PAINT_PROPS.filter((p) => p.prop.startsWith("trim"));

  const shape = $derived(editor.shown.find((s) => s.id === editor.selected) ?? null);
  const real = $derived(editor.selectedShape);
  const animating = $derived(editor.mode === "animate");
  const ab = $derived(editor.doc.artboard);
  const many = $derived(editor.selectedIds.length > 1);

  const visible = $derived(
    PROPS.filter((p) => {
      if (p.prop === "fill") return false;
      if (shape?.kind === "group") return !["width", "height", "cornerRadius"].includes(p.prop);
      if (p.prop === "cornerRadius") return shape?.kind === "rectangle";
      if (p.prop === "width" || p.prop === "height") return shape?.kind !== "path";
      return true;
    }),
  );
  const display = (prop: Prop, v: number) => (prop === "opacity" || prop.startsWith("scale") ? +v.toFixed(2) : +v.toFixed(1));
  const alpha = (c: number) => Math.round(((c >>> 24) / 255) * 100);
  const withAlpha = (c: number, pct: number) => ((Math.round((pct / 100) * 255) << 24) | (c & 0xffffff)) >>> 0;

  function setArtboard(field: "name" | "width" | "height", value: string) {
    editor.checkpoint();
    if (field === "name") ab.name = value;
    else {
      const n = Number(value);
      if (Number.isFinite(n) && n > 0) ab[field] = n;
    }
  }

  /** Edit a paint field on the real shape (paints are not keyable yet, except the fill colour). */
  function edit(fn: () => void) {
    if (!real) return;
    editor.checkpoint();
    fn();
  }

  function num(e: Event) {
    return Number((e.currentTarget as HTMLInputElement).value);
  }
</script>

<aside>
  {#if shape && real}
    <div class="panel-title">{many ? `${editor.selectedIds.length} selected` : shape.name}</div>
    <div class="grid">
      {#each visible as p}
        <label for="p-{p.prop}">{p.label}</label>
        <input id="p-{p.prop}" type="number" step={p.step} min={p.min} max={p.max} value={display(p.prop, shape[p.prop])}
          onchange={(e) => editor.setProp(shape.id, p.prop, Number(e.currentTarget.value))} />
        {#if animating}
          <button class="key" class:on={!!editor.keyAtPlayhead(shape.id, p.prop)} class:has={!!editor.track(shape.id, p.prop)}
            onclick={() => editor.toggleKey(shape.id, p.prop)} title="Key at playhead">◆</button>
        {:else}<span></span>{/if}
      {/each}
    </div>

    {#if shape.kind !== "group"}
      <div class="panel-title">Fill</div>
      <div class="grid">
        {#if !real.gradient}
          <label for="p-fill">Colour</label>
          <div class="colour">
            <input id="p-fill" type="color" value={argbToHex(shape.fill)}
              onchange={(e) => editor.setProp(shape.id, "fill", hexToArgb(e.currentTarget.value, shape.fill >>> 24))} />
            <input type="number" min="0" max="100" value={alpha(shape.fill)} title="Opacity %"
              onchange={(e) => editor.setProp(shape.id, "fill", withAlpha(shape.fill, num(e)))} />
          </div>
          {#if animating}
            <button class="key" class:on={!!editor.keyAtPlayhead(shape.id, "fill")} class:has={!!editor.track(shape.id, "fill")}
              onclick={() => editor.toggleKey(shape.id, "fill")} title="Key at playhead">◆</button>
          {:else}<span></span>{/if}
        {/if}
        <label for="p-grad">Gradient</label>
        <input id="p-grad" type="checkbox" checked={!!real.gradient} onchange={(e) => editor.setGradient(real.id, e.currentTarget.checked)} /><span></span>
        {#if real.gradient}
          {@const g = real.gradient}
          <label for="p-gkind">Type</label>
          <select id="p-gkind" value={g.kind} onchange={(e) => edit(() => (g.kind = e.currentTarget.value as "linear" | "radial"))}>
            <option value="linear">Linear</option><option value="radial">Radial</option>
          </select><span></span>
          <label>From</label>
          <div class="pair">
            <input type="number" value={g.start[0]} onchange={(e) => edit(() => (g.start[0] = num(e)))} />
            <input type="number" value={g.start[1]} onchange={(e) => edit(() => (g.start[1] = num(e)))} />
          </div><span></span>
          <label>{g.kind === "radial" ? "Rim" : "To"}</label>
          <div class="pair">
            <input type="number" value={g.end[0]} onchange={(e) => edit(() => (g.end[0] = num(e)))} />
            <input type="number" value={g.end[1]} onchange={(e) => edit(() => (g.end[1] = num(e)))} />
          </div><span></span>
          {#each g.stops as st, i}
            <label>Stop {i + 1}</label>
            <div class="colour">
              <input type="color" value={argbToHex(st.color)} onchange={(e) => edit(() => (st.color = hexToArgb(e.currentTarget.value, st.color >>> 24)))} />
              <input type="number" min="0" max="1" step="0.05" value={st.position} title="Position 0 to 1" onchange={(e) => edit(() => (st.position = num(e)))} />
              <input type="number" min="0" max="100" value={alpha(st.color)} title="Opacity %" onchange={(e) => edit(() => (st.color = withAlpha(st.color, num(e))))} />
            </div>
            <button class="key" disabled={g.stops.length <= 2} title="Remove stop" onclick={() => edit(() => g.stops.splice(i, 1))}>×</button>
          {/each}
          <span></span>
          <button class="small" onclick={() => edit(() => g.stops.push({ position: 0.5, color: g.stops[g.stops.length - 1].color }))}>Add stop</button><span></span>
        {/if}
      </div>

      <div class="panel-title">Stroke</div>
      <div class="grid">
        <label for="p-stroke">Stroke</label>
        <input id="p-stroke" type="checkbox" checked={!!real.stroke} onchange={(e) => editor.setStroke(real.id, e.currentTarget.checked)} /><span></span>
        {#if real.stroke}
          {@const st = real.stroke}
          <label>Colour</label>
          <div class="colour">
            <input type="color" value={argbToHex(st.color)} onchange={(e) => edit(() => (st.color = hexToArgb(e.currentTarget.value, st.color >>> 24)))} />
            <input type="number" min="0" max="100" value={alpha(st.color)} title="Opacity %" onchange={(e) => edit(() => (st.color = withAlpha(st.color, num(e))))} />
          </div><span></span>
          <label>Width</label>
          <input type="number" min="0" step="0.5" value={st.thickness} onchange={(e) => edit(() => (st.thickness = Math.max(0, num(e))))} /><span></span>
          <label>Ends</label>
          <select value={st.cap ?? 0} onchange={(e) => edit(() => (st.cap = Number(e.currentTarget.value)))}>
            <option value={0}>Flat</option><option value={1}>Round</option><option value={2}>Square</option>
          </select><span></span>
          <label>Corners</label>
          <select value={st.join ?? 0} onchange={(e) => edit(() => (st.join = Number(e.currentTarget.value)))}>
            <option value={0}>Sharp</option><option value={1}>Round</option><option value={2}>Bevel</option>
          </select><span></span>
          <label for="p-trim">Draw on</label>
          <input id="p-trim" type="checkbox" checked={!!st.trim} onchange={(e) => editor.setTrim(real.id, e.currentTarget.checked)} /><span></span>
          {#if st.trim}
            {#each TRIM_PROPS as p}
              <label for="p-{p.prop}">{p.label.replace("Trim ", "").replace(/^./, (c) => c.toUpperCase())}</label>
              <input id="p-{p.prop}" type="number" step={p.step} min={p.min} max={p.max} value={+readProp(shape, p.prop).toFixed(3)}
                onchange={(e) => editor.setProp(shape.id, p.prop, Number(e.currentTarget.value))} />
              {#if animating}
                <button class="key" class:on={!!editor.keyAtPlayhead(shape.id, p.prop)} class:has={!!editor.track(shape.id, p.prop)}
                  onclick={() => editor.toggleKey(shape.id, p.prop)} title="Key at playhead">◆</button>
              {:else}<span></span>{/if}
            {/each}
            <label for="p-trim-mode">Mode</label>
            <select id="p-trim-mode" value={st.trim.mode} onchange={(e) => edit(() => (st.trim!.mode = e.currentTarget.value as TrimMode))}>
              <option value="sequential">Sequential</option><option value="synchronized">Synchronized</option>
            </select><span></span>
          {/if}
        {/if}
      </div>
      {#if real.stroke?.trim}
        <p class="hint">Start and end are fractions of the outline; key End from 0 to 1 to draw the line on. The canvas approximates it with a dash; the runtime trims exactly.</p>
      {/if}

      <div class="panel-title">Shadow / glow</div>
      <div class="grid">
        <label for="p-feather">Feather</label>
        <input id="p-feather" type="checkbox" checked={!!real.feather} onchange={(e) => editor.setFeather(real.id, e.currentTarget.checked)} /><span></span>
        {#if real.feather}
          {@const f = real.feather}
          <label>Softness</label>
          <input type="number" min="0" step="1" value={f.strength} onchange={(e) => edit(() => (f.strength = Math.max(0, num(e))))} /><span></span>
          <label>Offset</label>
          <div class="pair">
            <input type="number" value={f.offset?.[0] ?? 0} onchange={(e) => edit(() => (f.offset = [num(e), f.offset?.[1] ?? 0]))} />
            <input type="number" value={f.offset?.[1] ?? 0} onchange={(e) => edit(() => (f.offset = [f.offset?.[0] ?? 0, num(e)]))} />
          </div><span></span>
          <label for="p-inner">Inset</label>
          <input id="p-inner" type="checkbox" checked={!!f.inner} onchange={(e) => edit(() => (f.inner = e.currentTarget.checked))} /><span></span>
        {/if}
      </div>
      <p class="hint">Feather is a soft blur the Rive runtime draws; a feathered copy behind a shape makes a glow or a shadow.</p>
      {#if shape.kind === "path" && real.paths?.length}
        <div class="panel-title">Path</div>
        <div class="grid">
          <label for="p-closed">Closed</label>
          <input id="p-closed" type="checkbox" checked={real.paths[0].closed} onchange={(e) => edit(() => (real.paths![0].closed = e.currentTarget.checked))} /><span></span>
          <label for="p-rule">Holes</label>
          <select id="p-rule" value={real.fillRule ?? "nonZero"} onchange={(e) => edit(() => (real.fillRule = e.currentTarget.value as "nonZero" | "evenOdd"))}>
            <option value="nonZero">Filled</option><option value="evenOdd">Cut out</option>
          </select><span></span>
        </div>
      {/if}
    {/if}
    {#if shape.kind !== "path"}
      {@const paths = editor.doc.shapes.filter((x) => x.kind === "path" && x.id !== real.id)}
      <div class="panel-title">Motion path</div>
      <div class="grid">
        <label for="p-follow">Path</label>
        <select id="p-follow" value={real.follow?.path ?? ""}
          onchange={(e) => editor.setFollow(real.id, e.currentTarget.value === "" ? null : Number(e.currentTarget.value))}>
          <option value="">None</option>
          {#each paths as p (p.id)}<option value={p.id}>{p.name}</option>{/each}
        </select><span></span>
        {#if real.follow}
          <label for="p-fdist">Distance</label>
          <input id="p-fdist" type="number" step="0.01" min="0" max="1" value={+(shape.follow?.distance ?? 0).toFixed(3)}
            onchange={(e) => editor.setProp(shape.id, "followDistance", num(e))} />
          {#if animating}
            <button class="key" class:on={!!editor.keyAtPlayhead(shape.id, "followDistance")} class:has={!!editor.track(shape.id, "followDistance")}
              onclick={() => editor.toggleKey(shape.id, "followDistance")} title="Key at playhead">◆</button>
          {:else}<span></span>{/if}
          <label for="p-orient">Orient</label>
          <input id="p-orient" type="checkbox" checked={!!real.follow.orient} onchange={(e) => editor.setFollowFlag(real.id, "orient", e.currentTarget.checked)} /><span></span>
          <label for="p-offset">Offset</label>
          <input id="p-offset" type="checkbox" checked={!!real.follow.offset} onchange={(e) => editor.setFollowFlag(real.id, "offset", e.currentTarget.checked)} /><span></span>
        {/if}
      </div>
      {#if real.follow}<p class="hint">The path sets the position{real.follow.offset ? "; X and Y shift it" : "; X and Y are ignored unless Offset is on"}. Orient turns the shape along the path.</p>{/if}
    {/if}
    <div class="panel-title">Pivot</div>
    <div class="grid">
      {#if shape.kind === "rectangle" || shape.kind === "ellipse"}
        <label>Origin</label>
        <div class="pair">
          <input type="number" step="0.05" value={real.originX ?? 0.5} title="0 left, 0.5 centre, 1 right" onchange={(e) => edit(() => (real.originX = num(e)))} />
          <input type="number" step="0.05" value={real.originY ?? 0.5} title="0 top, 0.5 centre, 1 bottom" onchange={(e) => edit(() => (real.originY = num(e)))} />
        </div><span></span>
      {/if}
      <span></span>
      <button class="small" class:active={editor.tool === "pivot"} onclick={() => (editor.tool = editor.tool === "pivot" ? "select" : "pivot")} title="Drag the pivot on the canvas (O)">{editor.tool === "pivot" ? "Done" : "Move pivot"}</button><span></span>
    </div>
    <p class="hint">
      {animating ? "Changes above set a key at the playhead." : "Switch to Animate to key these over time."}
    </p>
  {:else}
    <div class="panel-title">Artboard</div>
    <div class="grid">
      <label for="ab-name">Name</label>
      <input id="ab-name" value={ab.name} onchange={(e) => setArtboard("name", e.currentTarget.value)} /><span></span>
      <label for="ab-w">Width</label>
      <input id="ab-w" type="number" min="1" value={ab.width} onchange={(e) => setArtboard("width", e.currentTarget.value)} /><span></span>
      <label for="ab-h">Height</label>
      <input id="ab-h" type="number" min="1" value={ab.height} onchange={(e) => setArtboard("height", e.currentTarget.value)} /><span></span>
      <label for="ab-bg">Background</label>
      <input id="ab-bg" type="color" value={argbToHex(ab.background)}
        onchange={(e) => { editor.checkpoint(); ab.background = hexToArgb(e.currentTarget.value); }} /><span></span>
    </div>
  {/if}
</aside>

<style>
  aside { background: var(--panel); border-left: 1px solid var(--line); overflow: auto; padding-bottom: 20px; }
  .grid { display: grid; grid-template-columns: 64px 1fr 26px; gap: 6px 8px; align-items: center; padding: 0 12px; }
  label { color: var(--muted); }
  input[type="color"] { padding: 0 2px; height: 26px; width: 100%; min-width: 36px; }
  input[type="checkbox"] { justify-self: start; width: 16px; height: 16px; accent-color: var(--accent); }
  .colour { display: grid; grid-template-columns: 1fr 52px auto; gap: 4px; }
  .colour input[type="number"] { width: 52px; }
  .pair { display: grid; grid-template-columns: 1fr 1fr; gap: 4px; }
  .key { padding: 0; width: 24px; height: 24px; color: var(--muted); background: none; border-color: transparent; font-size: 12px; }
  .key.has { color: #b38b12; }
  .key.on { color: var(--key); }
  .small { padding: 3px 8px; font-size: 12px; justify-self: start; }
  .hint { color: var(--muted); padding: 4px 12px; font-size: 12px; }
</style>
