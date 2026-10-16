<script lang="ts">
  import { editor } from "./lib/editor.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import Layers from "./components/Layers.svelte";
  import Canvas from "./components/Canvas.svelte";
  import Properties from "./components/Properties.svelte";
  import Bottom from "./components/Bottom.svelte";
  import Preview from "./components/Preview.svelte";

  let previewOpen = $state(false);
  let canvas: ReturnType<typeof Canvas>;

  // ?open=<url of a .vectro.json> loads that project, e.g. from a local file server.
  $effect(() => {
    const url = new URLSearchParams(location.search).get("open");
    if (!url) return;
    fetch(url)
      .then((r) => (r.ok ? r.json() : Promise.reject(new Error(`${r.status} ${r.statusText}`))))
      .then((doc) => {
        if (!doc?.artboard || !Array.isArray(doc.shapes)) throw new Error("not a Vectro project");
        editor.open(doc);
        editor.mode = "animate";
      })
      .catch((e) => (editor.error = `Could not open ${url}: ${e.message ?? e}`));
  });

  // Autosave every change.
  $effect(() => {
    JSON.stringify(editor.doc);
    editor.save();
  });

  function onKey(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement || e.target instanceof HTMLTextAreaElement;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "z") {
      e.preventDefault();
      e.shiftKey ? editor.redo() : editor.undo();
      return;
    }
    if (typing) return;
    const k = e.key.toLowerCase();
    if (mod && k === "g") { e.preventDefault(); e.shiftKey ? editor.ungroup() : editor.group(); return; }
    if (mod && k === "d") { e.preventDefault(); editor.duplicate(); return; }
    if (mod && k === "a") { e.preventDefault(); editor.selectedIds = editor.doc.shapes.filter((s) => !s.parent).map((s) => s.id); return; }
    if (mod && (k === "=" || k === "+")) { e.preventDefault(); editor.zoomAt(1.25, editor.doc.artboard.width / 2, editor.doc.artboard.height / 2); return; }
    if (mod && k === "-") { e.preventDefault(); editor.zoomAt(0.8, editor.doc.artboard.width / 2, editor.doc.artboard.height / 2); return; }
    if (mod && k === "0") { e.preventDefault(); editor.fitView(); return; }
    if (mod) return;
    if (k === "v") editor.tool = "select";
    else if (k === "r") editor.tool = "rectangle";
    else if (k === "e") editor.tool = "ellipse";
    else if (k === "p") editor.tool = "pen";
    else if (k === "n") editor.tool = "node";
    else if (k === " ") {
      e.preventDefault();
      if (editor.tool === "pen") return;
      editor.playing ? editor.pause() : editor.play();
    } else if (k === "enter") {
      if (editor.tool === "pen") canvas.finishPen();
    } else if (k === "backspace" || k === "delete") {
      if (editor.selectedKey) editor.deleteSelectedKey();
      else if (editor.tool === "node" && editor.selectedVertex) canvas.deleteVertex();
      else if (editor.selectedIds.length) editor.deleteSelection();
    } else if (k === "escape") {
      if (editor.tool === "pen") canvas.finishPen();
      else if (editor.tool === "node") editor.tool = "select";
      else {
        editor.selected = null;
        editor.selectedKey = null;
      }
      previewOpen = false;
    } else if (k.startsWith("arrow")) {
      e.preventDefault();
      const step = e.shiftKey ? 10 : 1;
      editor.nudge(k === "arrowleft" ? -step : k === "arrowright" ? step : 0, k === "arrowup" ? -step : k === "arrowdown" ? step : 0);
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  <Toolbar onPreview={() => (previewOpen = true)} />
  <div class="middle">
    <Layers />
    <Canvas bind:this={canvas} />
    <Properties />
  </div>
  <Bottom />
</div>
{#if previewOpen}
  <Preview onClose={() => (previewOpen = false)} />
{/if}

<style>
  .app { display: grid; grid-template-rows: auto 1fr auto; height: 100%; }
  .middle { display: grid; grid-template-columns: 220px 1fr 280px; min-height: 0; }
</style>
