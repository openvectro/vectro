<script lang="ts">
  import { onMount } from "svelte";
  import { Rive, Layout, Fit, Alignment, EventType, StateMachineInputType, type StateMachineInput } from "@rive-app/canvas";
  import { editor } from "../lib/editor.svelte";

  let { onClose }: { onClose: () => void } = $props();
  let canvas: HTMLCanvasElement;
  let status = $state("Loading Rive runtime…");
  let size = $state(0);
  let inputs = $state<StateMachineInput[]>([]);
  let events = $state<string[]>([]);
  let states = $state<string[]>([]);
  const machines = $derived(editor.doc.stateMachines ?? []);
  // What to play: the machine in view when the machines panel is open, else the current animation.
  let choice = $state<string>(editor.panel === "machines" && (editor.doc.stateMachines?.length ?? 0) ? `sm:${editor.machine}` : `anim:${editor.anim}`);
  let rive: Rive | null = null;

  function start() {
    rive?.cleanup();
    rive = null;
    inputs = [];
    events = [];
    states = [];
    try {
      const bytes = editor.exportRiv();
      size = bytes.length;
      const [kind, idx] = choice.split(":");
      const smName = kind === "sm" ? machines[Number(idx)]?.name : undefined;
      const animName = kind === "anim" ? editor.doc.animations[Number(idx)]?.name : undefined;
      Rive.suppressDeprecationWarnings = ["state-change-events", "state-machines-param", "rive-events", "state-machine-inputs"];
      const r = new Rive({
        buffer: bytes.slice().buffer as ArrayBuffer,
        canvas,
        stateMachines: smName,
        animations: animName,
        autoplay: true,
        layout: new Layout({ fit: Fit.Contain, alignment: Alignment.Center }),
        onLoad: () => {
          r.resizeDrawingSurfaceToCanvas();
          status = `Rive's runtime · ${smName ? `state machine "${smName}"` : `animation "${animName}"`}`;
          if (smName) inputs = r.stateMachineInputs(smName) ?? [];
        },
        onLoadError: () => (status = "The runtime could not load this file."),
        onStateChange: (e) => { states = [...states, ...((e.data as string[]) ?? [])].slice(-12); },
      });
      r.on(EventType.RiveEvent, (e: any) => { events = [e.data?.name, ...events].slice(0, 12); });
      rive = r;
    } catch (e) {
      status = String(e);
    }
  }

  onMount(() => {
    start();
    return () => rive?.cleanup();
  });

  const Bool = StateMachineInputType.Boolean, Num = StateMachineInputType.Number;
</script>

<div class="backdrop" onclick={onClose} role="presentation">
  <div class="modal" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Preview" tabindex="-1">
    <div class="top">
      <strong>Preview</strong>
      <select bind:value={choice} onchange={start}>
        {#each editor.doc.animations as a, i}<option value="anim:{i}">Animation · {a.name}</option>{/each}
        {#each machines as m, i}<option value="sm:{i}">State machine · {m.name}</option>{/each}
      </select>
      <span class="muted">{status} · {size} bytes</span>
      <button onclick={onClose}>Close</button>
    </div>
    <div class="body" class:with-side={choice.startsWith("sm")}>
      <canvas bind:this={canvas}></canvas>
      {#if choice.startsWith("sm")}
        <div class="side">
          <div class="panel-title">Inputs</div>
          {#each inputs as input}
            <label class="row">
              <span>{input.name}</span>
              {#if input.type === Bool}
                <input type="checkbox" checked={!!input.value} onchange={(e) => (input.value = e.currentTarget.checked)} />
              {:else if input.type === Num}
                <input type="range" min="0" max="100" step="1" value={Number(input.value) || 0} oninput={(e) => (input.value = Number(e.currentTarget.value))} />
              {:else}
                <button class="fire" onclick={() => input.fire()}>Fire</button>
              {/if}
            </label>
          {:else}
            <p class="muted pad">No inputs on this machine.</p>
          {/each}
          <div class="panel-title">Events</div>
          <div class="pills">{#each events as ev}<span class="pill">{ev}</span>{:else}<span class="muted">none yet</span>{/each}</div>
          <div class="panel-title">States</div>
          <div class="pills">{#each states as st}<span class="pill">{st}</span>{/each}</div>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .backdrop { position: fixed; inset: 0; background: #000a; display: grid; place-items: center; z-index: 10; }
  .modal { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; width: min(960px, 94vw); padding: 12px; }
  .top { display: flex; align-items: center; gap: 10px; margin-bottom: 10px; }
  .top button { margin-left: auto; }
  .muted { color: var(--muted); font-size: 12px; }
  .pad { padding: 0 12px; }
  .body { display: grid; gap: 12px; }
  .body.with-side { grid-template-columns: 1fr 240px; }
  canvas { width: 100%; aspect-ratio: 4 / 3; display: block; background: #0d0d10; border-radius: 6px; }
  .side { overflow: auto; max-height: 70vh; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 8px; padding: 4px 12px; }
  .row input[type="range"] { width: 120px; accent-color: var(--accent); }
  .row input[type="checkbox"] { accent-color: var(--accent); width: 16px; height: 16px; }
  .fire { background: var(--accent); border-color: var(--accent); color: #1a1000; font-weight: 600; }
  .pills { display: flex; flex-wrap: wrap; gap: 4px; padding: 0 12px; }
  .pill { background: var(--panel-2); border: 1px solid var(--line); border-radius: 99px; padding: 1px 8px; font-size: 12px; }
</style>
