<script lang="ts">
  import { editor, type Align, type Tool } from "../lib/editor.svelte";
  import type { Doc } from "../lib/types";

  let { onPreview }: { onPreview: () => void } = $props();
  let fileInput: HTMLInputElement;
  let svgInput: HTMLInputElement;

  const tools: { tool: Tool; label: string; key: string; glyph: string }[] = [
    { tool: "select", label: "Select", key: "V", glyph: "↖" },
    { tool: "rectangle", label: "Rectangle", key: "R", glyph: "▭" },
    { tool: "ellipse", label: "Ellipse", key: "E", glyph: "◯" },
    { tool: "pen", label: "Pen", key: "P", glyph: "✎" },
    { tool: "node", label: "Edit points", key: "N", glyph: "⟟" },
  ];
  const aligns: { how: Align; glyph: string; label: string }[] = [
    { how: "left", glyph: "⫷", label: "Align left" },
    { how: "hcenter", glyph: "⫼", label: "Align centre" },
    { how: "right", glyph: "⫸", label: "Align right" },
    { how: "top", glyph: "⤒", label: "Align top" },
    { how: "vcenter", glyph: "⫵", label: "Align middle" },
    { how: "bottom", glyph: "⤓", label: "Align bottom" },
  ];
  const hasSelection = $derived(editor.selectedIds.length > 0);
  const isGroup = $derived(editor.selectedShape?.kind === "group");

  function download(data: BlobPart, name: string, type: string) {
    const url = URL.createObjectURL(new Blob([data], { type }));
    const a = document.createElement("a");
    a.href = url;
    a.download = name;
    a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }

  const slug = () => editor.doc.artboard.name.trim().replace(/\s+/g, "-").toLowerCase() || "vectro";

  function exportRiv() {
    try {
      download(editor.exportRiv() as BlobPart, `${slug()}.riv`, "application/octet-stream");
    } catch (e) {
      editor.error = String(e);
    }
  }

  async function openFile(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const doc = JSON.parse(await file.text()) as Doc;
      if (!doc.artboard || !Array.isArray(doc.shapes) || !Array.isArray(doc.animations)) throw new Error("not a Vectro project");
      editor.open(doc);
    } catch (err) {
      editor.error = `Could not open ${file.name}: ${err}`;
    }
    fileInput.value = "";
  }

  async function importSvg(e: Event) {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    try {
      const n = editor.importSvg(await file.text());
      if (!n) editor.error = `${file.name} had no filled shapes to import`;
    } catch (err) {
      editor.error = `Could not import ${file.name}: ${err}`;
    }
    svgInput.value = "";
  }
</script>

<header>
  <div class="brand">Vectro</div>
  <div class="group">
    {#each tools as t}
      <button class="tool" class:active={editor.tool === t.tool} onclick={() => (editor.tool = t.tool)} title="{t.label} ({t.key})"><span class="glyph">{t.glyph}</span>{t.label}</button>
    {/each}
  </div>
  <div class="group mode">
    <button class:active={editor.mode === "design"} onclick={() => { editor.pause(); editor.mode = "design"; }}>Design</button>
    <button class:active={editor.mode === "animate"} onclick={() => (editor.mode = "animate")}>Animate</button>
  </div>
  <div class="group">
    <button onclick={() => editor.group()} disabled={!hasSelection} title="Group (⌘G)">Group</button>
    <button onclick={() => editor.ungroup()} disabled={!isGroup} title="Ungroup (⇧⌘G)">Ungroup</button>
    <button onclick={() => editor.duplicate()} disabled={!hasSelection} title="Duplicate (⌘D)">Duplicate</button>
  </div>
  <div class="group align">
    {#each aligns as a}
      <button onclick={() => editor.align(a.how)} disabled={!hasSelection} title={a.label}>{a.glyph}</button>
    {/each}
  </div>
  <div class="group">
    <button onclick={() => editor.undo()} title="Undo (⌘Z)">Undo</button>
    <button onclick={() => editor.redo()} title="Redo (⇧⌘Z)">Redo</button>
  </div>
  <div class="spacer"></div>
  {#if editor.error}
    <button class="error" onclick={() => (editor.error = null)} title="Dismiss">{editor.error}</button>
  {/if}
  <div class="group">
    <button onclick={() => svgInput.click()} title="Add the shapes from an SVG file">Import SVG</button>
    <button onclick={() => fileInput.click()}>Open</button>
    <button onclick={() => download(JSON.stringify(editor.doc, null, 1), `${slug()}.vectro.json`, "application/json")}>Save</button>
    <button onclick={onPreview}>Preview</button>
    <button class="active" onclick={exportRiv}>Export .riv</button>
  </div>
  <input type="file" accept=".json,application/json" bind:this={fileInput} onchange={openFile} hidden />
  <input type="file" accept=".svg,image/svg+xml" bind:this={svgInput} onchange={importSvg} hidden />
</header>

<style>
  header { display: flex; align-items: center; gap: 12px; padding: 8px 12px; background: var(--panel); border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  .brand { font-weight: 700; letter-spacing: 0.02em; font-size: 15px; }
  .group { display: flex; gap: 4px; }
  .tool { display: flex; gap: 6px; align-items: center; }
  .glyph { font-size: 14px; line-height: 1; opacity: 0.85; }
  .align button { padding: 4px 7px; font-size: 14px; line-height: 1; }
  .mode button.active { background: var(--key); border-color: var(--key); color: #000; }
  .mode button:first-child.active { background: var(--accent); border-color: var(--accent); color: #fff; }
  .spacer { flex: 1; }
  .error { border-color: var(--danger); color: var(--danger); max-width: 360px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
</style>
