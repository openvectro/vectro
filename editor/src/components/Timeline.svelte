<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { EASES, PROPS, type Ease, type LoopMode } from "../lib/types";
  import * as core from "../wasm/vectro_wasm.js";

  const anim = $derived(editor.animation);
  const label = (prop: string) => PROPS.find((p) => p.prop === prop)?.label ?? prop;
  const shapeName = (id: number) => editor.doc.shapes.find((s) => s.id === id)?.name ?? "?";
  const pct = (frame: number) => `${(frame / Math.max(1, anim.duration)) * 100}%`;
  const ticks = $derived(Array.from({ length: Math.floor(anim.duration / anim.fps) + 1 }, (_, i) => i));

  let lane: HTMLDivElement;
  let scrubbing = false;
  let dragKey: { track: number; key: number; moved: boolean } | null = null;

  const frameFrom = (e: PointerEvent) => {
    const r = lane.getBoundingClientRect();
    return Math.round(Math.min(1, Math.max(0, (e.clientX - r.left) / r.width)) * anim.duration);
  };

  function scrubStart(e: PointerEvent) {
    editor.pause();
    editor.mode = "animate";
    scrubbing = true;
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
    editor.frame = frameFrom(e);
  }

  function keyDown(e: PointerEvent, track: number, key: number) {
    e.stopPropagation();
    editor.pause();
    editor.mode = "animate";
    editor.selectedKey = { track, key };
    editor.frame = anim.tracks[track].keys[key].frame;
    editor.selected = anim.tracks[track].shapeId;
    dragKey = { track, key, moved: false };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  function pointerMove(e: PointerEvent) {
    if (scrubbing) editor.frame = frameFrom(e);
    else if (dragKey) {
      const t = anim.tracks[dragKey.track];
      const f = frameFrom(e);
      const k = t.keys[dragKey.key];
      if (f === k.frame || t.keys.some((o) => o !== k && o.frame === f)) return;
      if (!dragKey.moved) {
        editor.checkpoint();
        dragKey.moved = true;
      }
      const moving = t.keys[dragKey.key];
      moving.frame = f;
      t.keys.sort((a, b) => a.frame - b.frame);
      dragKey.key = t.keys.indexOf(moving);
      editor.selectedKey = { track: dragKey.track, key: dragKey.key };
      editor.frame = f;
    }
  }

  function pointerUp() {
    scrubbing = false;
    dragKey = null;
  }

  const selKey = $derived(editor.selectedKey ? anim.tracks[editor.selectedKey.track]?.keys[editor.selectedKey.key] : undefined);
  const sameEase = (a: Ease, b: Ease) => JSON.stringify(a) === JSON.stringify(b);

  function setEase(e: Ease) {
    if (!selKey) return;
    editor.checkpoint();
    selKey.ease = { ...e };
  }

  function curve(e: Ease) {
    const pts: string[] = [];
    for (let i = 0; i <= 24; i++) {
      const p = i / 24;
      const y = e.type === "hold" ? 0 : e.type === "linear" ? p : core.easeAt(e.x1, e.y1, e.x2, e.y2, p);
      pts.push(`${(p * 40).toFixed(1)},${(30 - y * 24).toFixed(1)}`);
    }
    return pts.join(" ");
  }

  function setAnim(field: "name" | "fps" | "seconds" | "loop", value: string) {
    editor.checkpoint();
    if (field === "name") anim.name = value.trim() || anim.name;
    else if (field === "loop") anim.loop = value as LoopMode;
    else if (field === "fps") {
      const fps = Math.round(Number(value));
      if (fps > 0 && fps <= 240) {
        const seconds = anim.duration / anim.fps;
        anim.fps = fps;
        anim.duration = Math.max(1, Math.round(seconds * fps));
      }
    } else {
      const s = Number(value);
      if (s > 0) anim.duration = Math.max(1, Math.round(s * anim.fps));
    }
    editor.frame = Math.min(editor.frame, anim.duration);
  }
</script>

<section>
  <div class="head">
    <select value={editor.anim} onchange={(e) => { editor.pause(); editor.anim = Number(e.currentTarget.value); editor.frame = 0; editor.selectedKey = null; }}>
      {#each editor.doc.animations as a, i}<option value={i}>{a.name}</option>{/each}
    </select>
    <input class="name" value={anim.name} onchange={(e) => setAnim("name", e.currentTarget.value)} aria-label="Animation name" />
    <button onclick={() => editor.addAnimation()} title="New animation">+</button>
    <button onclick={() => editor.deleteAnimation()} disabled={editor.doc.animations.length <= 1} title="Delete animation">−</button>
    <span class="sep"></span>
    <button onclick={() => { editor.pause(); editor.frame = 0; }} title="To start">⏮</button>
    <button class="play" onclick={() => (editor.playing ? editor.pause() : editor.play())} title="Play (Space)">{editor.playing ? "❚❚" : "▶"}</button>
    <span class="time">{(editor.frame / anim.fps).toFixed(2)}s · {Math.round(editor.frame)}/{anim.duration}</span>
    <span class="sep"></span>
    <label>Length <input type="number" min="0.1" step="0.1" value={+(anim.duration / anim.fps).toFixed(2)} onchange={(e) => setAnim("seconds", e.currentTarget.value)} />s</label>
    <label>FPS <input type="number" min="1" max="240" value={anim.fps} onchange={(e) => setAnim("fps", e.currentTarget.value)} /></label>
    <select value={anim.loop} onchange={(e) => setAnim("loop", e.currentTarget.value)} aria-label="Loop">
      <option value="oneShot">Play once</option>
      <option value="loop">Loop</option>
      <option value="pingPong">Ping-pong</option>
    </select>
    {#if selKey}
      <span class="sep"></span>
      <span class="eases">
        {#each EASES as e}
          <button class:active={sameEase(selKey.ease, e.ease)} onclick={() => setEase(e.ease)} title={e.name}>
            <svg width="40" height="32" viewBox="-2 0 44 34"><polyline points={curve(e.ease)} /></svg>
          </button>
        {/each}
      </span>
      <button class="danger" onclick={() => editor.deleteSelectedKey()} title="Delete key (⌫)">Delete key</button>
    {/if}
  </div>

  <div class="body" onpointermove={pointerMove} onpointerup={pointerUp} role="presentation">
    <div class="labels">
      <div class="ruler-label"></div>
      {#each anim.tracks as t, ti}
        <button class="track-label" class:sel={editor.selected === t.shapeId} onclick={() => (editor.selected = t.shapeId)}>
          {shapeName(t.shapeId)} · {label(t.prop)}
        </button>
      {/each}
      {#if !anim.tracks.length}<div class="empty">Select a shape, switch to Animate, move the playhead and change a property.</div>{/if}
    </div>
    <div class="lanes" bind:this={lane}>
      <div class="ruler" onpointerdown={scrubStart} role="slider" aria-label="Playhead" aria-valuenow={Math.round(editor.frame)} tabindex="0">
        {#each ticks as s}<span class="tick" style:left={pct(s * anim.fps)}>{s}s</span>{/each}
      </div>
      {#each anim.tracks as t, ti}
        <div class="lane" onpointerdown={scrubStart} role="presentation">
          {#each t.keys as k, ki}
            <span class="diamond" class:sel={editor.selectedKey?.track === ti && editor.selectedKey?.key === ki}
              style:left={pct(k.frame)} onpointerdown={(e) => keyDown(e, ti, ki)} role="button" tabindex="-1"
              title="{label(t.prop)} = {t.prop === 'fill' ? '#' + (k.value >>> 0).toString(16).slice(2) : +k.value.toFixed(2)} at frame {k.frame}"></span>
          {/each}
        </div>
      {/each}
      <div class="playhead" style:left={pct(editor.frame)}></div>
    </div>
  </div>
</section>

<style>
  section { background: var(--panel); border-top: 1px solid var(--line); height: 230px; display: grid; grid-template-rows: auto 1fr; }
  .head { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  .head input[type="number"] { width: 56px; }
  .head label { color: var(--muted); display: flex; gap: 4px; align-items: center; }
  .name { width: 130px; }
  .sep { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  .time { font-variant-numeric: tabular-nums; color: var(--muted); min-width: 110px; }
  .play { width: 36px; }
  .eases { display: flex; gap: 2px; }
  .eases button { padding: 0 2px; }
  .eases polyline { fill: none; stroke: currentColor; stroke-width: 2; }
  .danger { color: var(--danger); }
  .body { display: grid; grid-template-columns: 180px 1fr; overflow: auto; min-height: 0; }
  .labels { border-right: 1px solid var(--line); }
  .ruler-label, .ruler { height: 24px; border-bottom: 1px solid var(--line); }
  .track-label { display: block; width: 100%; height: 26px; text-align: left; background: none; border: 0; border-radius: 0;
    padding: 0 10px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .track-label.sel { color: var(--text); }
  .empty { color: var(--muted); padding: 8px 10px; font-size: 12px; }
  .lanes { position: relative; margin: 0 14px; }
  .ruler { position: relative; cursor: ew-resize; }
  .tick { position: absolute; top: 4px; transform: translateX(-50%); font-size: 11px; color: var(--muted); }
  .lane { position: relative; height: 26px; border-bottom: 1px solid #2a2a31; cursor: ew-resize; }
  .diamond { position: absolute; top: 7px; width: 11px; height: 11px; margin-left: -5.5px; background: var(--key);
    transform: rotate(45deg); cursor: grab; border: 1px solid #0006; }
  .diamond.sel { background: #fff; box-shadow: 0 0 0 2px var(--accent); }
  .playhead { position: absolute; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: var(--danger); pointer-events: none; }
</style>
