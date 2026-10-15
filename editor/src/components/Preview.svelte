<script lang="ts">
  import { onMount } from "svelte";
  import { Rive, Layout, Fit, Alignment } from "@rive-app/canvas";
  import { editor } from "../lib/editor.svelte";

  let { onClose }: { onClose: () => void } = $props();
  let canvas: HTMLCanvasElement;
  let status = $state("Loading Rive runtime…");
  let size = $state(0);

  onMount(() => {
    let rive: Rive | null = null;
    try {
      const bytes = editor.exportRiv();
      size = bytes.length;
      rive = new Rive({
        buffer: bytes.slice().buffer as ArrayBuffer,
        canvas,
        animations: editor.animation.name,
        autoplay: true,
        layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
        onLoad: () => {
          rive?.resizeDrawingSurfaceToCanvas();
          status = `Playing "${editor.animation.name}" with Rive's own runtime`;
        },
        onLoadError: () => (status = "The runtime could not load this file."),
      });
    } catch (e) {
      status = String(e);
    }
    return () => rive?.cleanup();
  });
</script>

<div class="backdrop" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Preview" tabindex="-1">
    <div class="top">
      <strong>Preview</strong>
      <span class="muted">{status} · {size} bytes</span>
      <button onclick={onClose}>Close</button>
    </div>
    <canvas bind:this={canvas}></canvas>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: #000a; display: grid; place-items: center; z-index: 10; }
  .modal { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; width: min(720px, 92vw); padding: 12px; }
  .top { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
  .top button { margin-left: auto; }
  .muted { color: var(--muted); font-size: 12px; }
  canvas { width: 100%; aspect-ratio: 4 / 3; display: block; background: #0d0d10; border-radius: 6px; }
</style>
