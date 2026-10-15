<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { PROPS, argbToHex, hexToArgb, type Prop } from "../lib/types";

  const shape = $derived(editor.shown.find((s) => s.id === editor.selected) ?? null);
  const animating = $derived(editor.mode === "animate");
  const ab = $derived(editor.doc.artboard);

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

  function setArtboard(field: "name" | "width" | "height", value: string) {
    editor.checkpoint();
    if (field === "name") ab.name = value;
    else {
      const n = Number(value);
      if (Number.isFinite(n) && n > 0) ab[field] = n;
    }
  }
</script>

<aside>
  {#if shape}
    <div class="panel-title">{shape.name}</div>
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
      {#if shape.kind !== "group"}
      <label for="p-fill">Fill</label>
      <input id="p-fill" type="color" value={argbToHex(shape.fill)}
        onchange={(e) => editor.setProp(shape.id, "fill", hexToArgb(e.currentTarget.value, shape.fill >>> 24))} />
      {#if animating}
        <button class="key" class:on={!!editor.keyAtPlayhead(shape.id, "fill")} class:has={!!editor.track(shape.id, "fill")}
          onclick={() => editor.toggleKey(shape.id, "fill")} title="Key at playhead">◆</button>
      {:else}<span></span>{/if}
      {/if}
    </div>
    <p class="hint">
      {animating ? "Changes here set a key at the playhead." : "Switch to Animate to key these over time."}
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
  aside { background: var(--panel); border-left: 1px solid var(--line); overflow: auto; }
  .grid { display: grid; grid-template-columns: 70px 1fr 26px; gap: 6px 8px; align-items: center; padding: 0 12px; }
  label { color: var(--muted); }
  input[type="color"] { padding: 0 2px; height: 26px; width: 100%; }
  .key { padding: 0; width: 24px; height: 24px; color: var(--muted); background: none; border-color: transparent; font-size: 12px; }
  .key.has { color: #b38b12; }
  .key.on { color: var(--key); }
  .hint { color: var(--muted); padding: 4px 12px; font-size: 12px; }
</style>
