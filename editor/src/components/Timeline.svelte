<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { EASES, propLabel as label, type Ease, type LoopMode } from "../lib/types";
  import * as core from "../wasm/vectro_wasm.js";

  const anim = $derived(editor.animation);
  const pct = (frame: number) => `${(frame / Math.max(1, anim.duration)) * 100}%`;
  const ticks = $derived(Array.from({ length: Math.floor(anim.duration / anim.fps) + 1 }, (_, i) => i));

  /** Dope sheet rows: one header per shape that has keys, its tracks underneath unless folded. */
  let folded = $state<Set<number>>(new Set());
  type Row = { kind: "shape"; shapeId: number; name: string; frames: number[] } | { kind: "track"; shapeId: number; track: number; name: string };
  const rows = $derived.by<Row[]>(() => {
    const out: Row[] = [];
    const seen = new Set<number>();
    // Shapes in layer order (front first), then any track whose shape is gone.
    const order = [...editor.doc.shapes].reverse().map((s) => s.id);
    for (const id of order) {
      const tracks = anim.tracks.map((t, i) => [t, i] as const).filter(([t]) => t.shapeId === id);
      if (!tracks.length) continue;
      seen.add(id);
      out.push({ kind: "shape", shapeId: id, name: editor.shape(id)?.name ?? `shape ${id}`, frames: editor.keyFramesOf(id) });
      if (folded.has(id)) continue;
      for (const [t, i] of tracks) out.push({ kind: "track", shapeId: id, track: i, name: t.point ? `point ${t.point.index} ${t.prop.replace("point", "").toLowerCase()}` : label(t.prop) });
    }
    anim.tracks.forEach((t, i) => { if (!seen.has(t.shapeId)) out.push({ kind: "track", shapeId: t.shapeId, track: i, name: `${t.prop} (shape ${t.shapeId} is gone)` }); });
    return out;
  });

  function toggleFold(id: number) {
    const next = new Set(folded);
    next.has(id) ? next.delete(id) : next.add(id);
    folded = next;
  }

  let lane: HTMLDivElement;
  let body: HTMLDivElement;
  let scrubbing = false;
  let dragKeys: { startFrame: number; lastDelta: number; moved: boolean } | null = null;
  let box = $state<{ x0: number; y0: number; x1: number; y1: number; add: boolean } | null>(null);

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

  /** Press on a key: select it (shift adds), or start moving every selected key. */
  function keyDown(e: PointerEvent, track: number, key: number) {
    e.stopPropagation();
    editor.pause();
    editor.mode = "animate";
    if (e.shiftKey) {
      editor.selectedKeys = editor.isKeySelected(track, key) ? editor.selectedKeys.filter((k) => !(k.track === track && k.key === key)) : [...editor.selectedKeys, { track, key }];
    } else if (!editor.isKeySelected(track, key)) {
      editor.selectedKey = { track, key };
    }
    const k = anim.tracks[track].keys[key];
    editor.frame = k.frame;
    editor.selected = anim.tracks[track].shapeId;
    dragKeys = { startFrame: frameFrom(e), lastDelta: 0, moved: false };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  /** Press on a shape's summary diamond: every key of that shape at that frame. */
  function summaryDown(e: PointerEvent, shapeId: number, frame: number) {
    e.stopPropagation();
    editor.pause();
    editor.mode = "animate";
    editor.selectKeysAt(shapeId, frame, e.shiftKey);
    editor.frame = frame;
    editor.selected = shapeId;
    dragKeys = { startFrame: frameFrom(e), lastDelta: 0, moved: false };
    (e.currentTarget as Element).setPointerCapture(e.pointerId);
  }

  /** Press on empty lane space: scrub, or with shift start a box selection over keys. */
  function laneDown(e: PointerEvent) {
    if (e.shiftKey || e.altKey) {
      const r = body.getBoundingClientRect();
      box = { x0: e.clientX - r.left + body.scrollLeft, y0: e.clientY - r.top + body.scrollTop, x1: e.clientX - r.left + body.scrollLeft, y1: e.clientY - r.top + body.scrollTop, add: e.shiftKey && editor.selectedKeys.length > 0 };
      (e.currentTarget as Element).setPointerCapture(e.pointerId);
      return;
    }
    scrubStart(e);
  }

  function pointerMove(e: PointerEvent) {
    if (scrubbing) editor.frame = frameFrom(e);
    else if (dragKeys) {
      const delta = frameFrom(e) - dragKeys.startFrame;
      const step = delta - dragKeys.lastDelta;
      if (!step) return;
      if (!dragKeys.moved) {
        editor.checkpoint();
        dragKeys.moved = true;
      }
      if (editor.moveSelectedKeys(step)) {
        dragKeys.lastDelta = delta;
        editor.frame = Math.min(anim.duration, Math.max(0, editor.frame + step));
      }
    } else if (box) {
      const r = body.getBoundingClientRect();
      box.x1 = e.clientX - r.left + body.scrollLeft;
      box.y1 = e.clientY - r.top + body.scrollTop;
    }
  }

  function pointerUp() {
    if (box) {
      const l = Math.min(box.x0, box.x1), r = Math.max(box.x0, box.x1), t = Math.min(box.y0, box.y1), b = Math.max(box.y0, box.y1);
      const br = body.getBoundingClientRect();
      const hits: { track: number; key: number }[] = [];
      for (const el of body.querySelectorAll<HTMLElement>("[data-track]")) {
        const k = el.getBoundingClientRect();
        const cx = k.left + k.width / 2 - br.left + body.scrollLeft, cy = k.top + k.height / 2 - br.top + body.scrollTop;
        if (cx >= l && cx <= r && cy >= t && cy <= b) hits.push({ track: Number(el.dataset.track), key: Number(el.dataset.key) });
      }
      editor.selectedKeys = box.add ? [...editor.selectedKeys, ...hits.filter((h) => !editor.isKeySelected(h.track, h.key))] : hits;
      box = null;
    }
    scrubbing = false;
    dragKeys = null;
  }

  const selKey = $derived(editor.selectedKey ? anim.tracks[editor.selectedKey.track]?.keys[editor.selectedKey.key] : undefined);
  const sameEase = (a: Ease, b: Ease) => JSON.stringify(a) === JSON.stringify(b);

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

  const valueText = (prop: string, v: number) => (prop === "fill" || prop === "fillEnd" ? "#" + (v >>> 0).toString(16).padStart(8, "0").slice(2) : String(+v.toFixed(2)));
</script>

<section>
  <div class="head">
    <select value={editor.anim} onchange={(e) => { editor.pause(); editor.anim = Number(e.currentTarget.value); editor.frame = 0; editor.selectedKeys = []; }}>
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
    <span class="sep"></span>
    <button class:active={editor.onion} onclick={() => (editor.onion = !editor.onion)} title="Onion skin: ghosts of nearby frames on the canvas">Onion</button>
    {#if editor.onion}<input type="number" min="1" max="60" value={editor.onionStep} title="Frames either side" onchange={(e) => (editor.onionStep = Math.max(1, Number(e.currentTarget.value)))} />{/if}
    {#if selKey}
      <span class="sep"></span>
      <span class="count">{editor.selectedKeys.length} key{editor.selectedKeys.length === 1 ? "" : "s"}</span>
      <span class="eases">
        {#each EASES as e}
          <button class:active={sameEase(selKey.ease, e.ease)} onclick={() => editor.setSelectedEase(e.ease)} title={e.name}>
            <svg width="40" height="32" viewBox="-2 0 44 34"><polyline points={curve(e.ease)} /></svg>
          </button>
        {/each}
      </span>
      <button onclick={() => editor.copyKeys()} title="Copy keys (⌘C)">Copy</button>
      <button onclick={() => editor.pasteKeys()} disabled={!editor.keyClipboard.length} title="Paste at the playhead (⌘V)">Paste</button>
      <button class="danger" onclick={() => editor.deleteSelectedKeys()} title="Delete keys (⌫)">Delete</button>
    {:else if editor.keyClipboard.length}
      <span class="sep"></span>
      <button onclick={() => editor.pasteKeys()} title="Paste at the playhead (⌘V)">Paste {editor.keyClipboard.reduce((n, c) => n + c.keys.length, 0)} keys</button>
    {/if}
  </div>

  <div class="body" bind:this={body} onpointermove={pointerMove} onpointerup={pointerUp} role="presentation">
    <div class="labels">
      <div class="ruler-label">{#if rows.length}<span class="hint">shift-drag selects keys</span>{/if}</div>
      {#each rows as r (r.kind === "shape" ? "s" + r.shapeId : "t" + r.track)}
        {#if r.kind === "shape"}
          <button class="row-label shape" class:sel={editor.selectedIds.includes(r.shapeId)} onclick={() => (editor.selected = r.shapeId)}>
            <span class="fold" role="button" tabindex="-1" onpointerdown={(e) => { e.stopPropagation(); toggleFold(r.shapeId); }}>{folded.has(r.shapeId) ? "▸" : "▾"}</span>{r.name}
          </button>
        {:else}
          <button class="row-label track" class:sel={editor.selectedIds.includes(r.shapeId)} onclick={() => (editor.selected = r.shapeId)}>{r.name}</button>
        {/if}
      {/each}
      {#if !rows.length}<div class="empty">Select a shape, switch to Animate, move the playhead and change a property.</div>{/if}
    </div>
    <div class="lanes" bind:this={lane}>
      <div class="ruler" onpointerdown={scrubStart} role="slider" aria-label="Playhead" aria-valuenow={Math.round(editor.frame)} tabindex="0">
        {#each ticks as s}<span class="tick" style:left={pct(s * anim.fps)}>{s}s</span>{/each}
      </div>
      {#each rows as r (r.kind === "shape" ? "s" + r.shapeId : "t" + r.track)}
        {#if r.kind === "shape"}
          <div class="lane shape" onpointerdown={laneDown} role="presentation">
            {#each r.frames as f (f)}
              <span class="summary" style:left={pct(f)} onpointerdown={(e) => summaryDown(e, r.shapeId, f)} role="button" tabindex="-1" title="All keys of {r.name} at frame {f}"></span>
            {/each}
          </div>
        {:else}
          {@const t = anim.tracks[r.track]}
          <div class="lane" onpointerdown={laneDown} role="presentation">
            {#each t.keys as k, ki (k.frame)}
              <span class="diamond" class:sel={editor.isKeySelected(r.track, ki)} data-track={r.track} data-key={ki}
                style:left={pct(k.frame)} onpointerdown={(e) => keyDown(e, r.track, ki)} role="button" tabindex="-1"
                title="{r.name} = {valueText(t.prop, k.value)} at frame {k.frame}"></span>
            {/each}
          </div>
        {/if}
      {/each}
      <div class="playhead" style:left={pct(editor.frame)}></div>
    </div>
    {#if box}
      <div class="box" style:left="{Math.min(box.x0, box.x1)}px" style:top="{Math.min(box.y0, box.y1)}px" style:width="{Math.abs(box.x1 - box.x0)}px" style:height="{Math.abs(box.y1 - box.y0)}px"></div>
    {/if}
  </div>
</section>

<style>
  section { background: var(--panel); border-top: 1px solid var(--line); height: 260px; display: grid; grid-template-rows: auto 1fr; }
  .head { display: flex; align-items: center; gap: 6px; padding: 6px 10px; border-bottom: 1px solid var(--line); flex-wrap: wrap; }
  .head input[type="number"] { width: 56px; }
  .head label { color: var(--muted); display: flex; gap: 4px; align-items: center; }
  .name { width: 130px; }
  .sep { width: 1px; height: 20px; background: var(--line); margin: 0 4px; }
  .time { font-variant-numeric: tabular-nums; color: var(--muted); min-width: 110px; }
  .count { color: var(--muted); font-size: 12px; }
  .play { width: 36px; }
  .eases { display: flex; gap: 2px; }
  .eases button { padding: 0 2px; }
  .eases polyline { fill: none; stroke: currentColor; stroke-width: 2; }
  .danger { color: var(--danger); }
  .body { display: grid; grid-template-columns: 200px 1fr; overflow: auto; min-height: 0; position: relative; }
  .labels { border-right: 1px solid var(--line); }
  .ruler-label, .ruler { height: 24px; border-bottom: 1px solid var(--line); }
  .ruler-label .hint { color: var(--muted); font-size: 11px; padding: 5px 10px; display: block; }
  .row-label { display: block; width: 100%; height: 26px; text-align: left; background: none; border: 0; border-radius: 0;
    padding: 0 10px; color: var(--muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .row-label.shape { color: var(--text); font-weight: 600; background: #1b1b21; }
  .row-label.track { padding-left: 26px; }
  .row-label.sel { color: var(--text); }
  .fold { display: inline-block; width: 14px; color: var(--muted); cursor: pointer; }
  .empty { color: var(--muted); padding: 8px 10px; font-size: 12px; }
  .lanes { position: relative; margin: 0 14px; }
  .ruler { position: relative; cursor: ew-resize; }
  .tick { position: absolute; top: 4px; transform: translateX(-50%); font-size: 11px; color: var(--muted); }
  .lane { position: relative; height: 26px; border-bottom: 1px solid #2a2a31; cursor: ew-resize; }
  .lane.shape { background: #1b1b21; }
  .diamond, .summary { position: absolute; top: 7px; width: 11px; height: 11px; margin-left: -5.5px; transform: rotate(45deg); cursor: grab; }
  .diamond { background: var(--key); border: 1px solid #0006; }
  .summary { background: #6e6e7a; border: 1px solid #0006; }
  .diamond.sel { background: #fff; box-shadow: 0 0 0 2px var(--accent); }
  .playhead { position: absolute; top: 0; bottom: 0; width: 2px; margin-left: -1px; background: var(--danger); pointer-events: none; }
  .box { position: absolute; background: var(--accent-soft); border: 1px solid var(--accent); pointer-events: none; }
</style>
