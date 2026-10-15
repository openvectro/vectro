<script lang="ts">
  import { editor } from "./lib/editor.svelte";
  import Toolbar from "./components/Toolbar.svelte";
  import Layers from "./components/Layers.svelte";
  import Canvas from "./components/Canvas.svelte";
  import Properties from "./components/Properties.svelte";
  import Timeline from "./components/Timeline.svelte";
  import Preview from "./components/Preview.svelte";

  let previewOpen = $state(false);

  // Autosave every change.
  $effect(() => {
    JSON.stringify(editor.doc);
    editor.save();
  });

  function onKey(e: KeyboardEvent) {
    const typing = e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement;
    const mod = e.metaKey || e.ctrlKey;
    if (mod && e.key.toLowerCase() === "z") {
      e.preventDefault();
      e.shiftKey ? editor.redo() : editor.undo();
      return;
    }
    if (typing) return;
    if (e.key === "v") editor.tool = "select";
    else if (e.key === "r") editor.tool = "rectangle";
    else if (e.key === "e") editor.tool = "ellipse";
    else if (e.key === " ") {
      e.preventDefault();
      editor.playing ? editor.pause() : editor.play();
    } else if (e.key === "Backspace" || e.key === "Delete") {
      if (editor.selectedKey) editor.deleteSelectedKey();
      else if (editor.selected !== null) editor.deleteShape(editor.selected);
    } else if (e.key === "Escape") {
      editor.selected = null;
      editor.selectedKey = null;
      previewOpen = false;
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="app">
  <Toolbar onPreview={() => (previewOpen = true)} />
  <div class="middle">
    <Layers />
    <Canvas />
    <Properties />
  </div>
  <Timeline />
</div>
{#if previewOpen}
  <Preview onClose={() => (previewOpen = false)} />
{/if}

<style>
  .app { display: grid; grid-template-rows: auto 1fr auto; height: 100%; }
  .middle { display: grid; grid-template-columns: 220px 1fr 260px; min-height: 0; }
</style>
