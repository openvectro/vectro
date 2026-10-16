<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { OPS, POINTERS, addInput, addState, addTransition, defaultAction, deleteEvent, deleteInput, deleteState, describe, ensureMachines, newLayer, newMachine } from "../lib/sm";
  import type { Action, Condition, InputKind, State, Transition } from "../lib/types";

  const doc = $derived(editor.doc);
  const machines = $derived(doc.stateMachines ?? []);
  const sm = $derived(editor.stateMachine);
  const layer = $derived(editor.smLayer);
  const sel = $derived(editor.smSelection);
  const selState = $derived(sel?.type === "state" && layer ? layer.states[sel.index] : null);
  const selTransition = $derived(sel?.type === "transition" && layer ? layer.transitions[sel.index] : null);

  const W = 150, H = 44;
  const pos = (s: State, i: number): [number, number] => s.ui ?? [220 + ((i - 3) % 3) * 190, 40 + Math.floor((i - 3) / 3) * 110];

  function edit(fn: () => void) {
    editor.checkpoint();
    ensureMachines(editor.doc);
    fn();
  }

  // Graph interaction: drag a state card, or drag from a port onto another state to connect.
  let svg: SVGSVGElement;
  let drag = $state<{ kind: "state"; index: number; dx: number; dy: number } | { kind: "wire"; from: number; x: number; y: number } | null>(null);

  function local(e: PointerEvent) {
    const r = svg.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }

  function down(e: PointerEvent) {
    if (!layer) return;
    const el = e.target as Element;
    const port = el.closest("[data-port]");
    const card = el.closest("[data-state]");
    const wire = el.closest("[data-transition]");
    svg.setPointerCapture(e.pointerId);
    const p = local(e);
    if (port) {
      drag = { kind: "wire", from: Number(port.getAttribute("data-port")), x: p.x, y: p.y };
    } else if (card) {
      const index = Number(card.getAttribute("data-state"));
      editor.smSelection = { type: "state", index };
      const [x, y] = pos(layer.states[index], index);
      editor.checkpoint();
      drag = { kind: "state", index, dx: p.x - x, dy: p.y - y };
    } else if (wire) {
      editor.smSelection = { type: "transition", index: Number(wire.getAttribute("data-transition")) };
    } else {
      editor.smSelection = null;
    }
  }

  function move(e: PointerEvent) {
    if (!drag || !layer) return;
    const p = local(e);
    if (drag.kind === "state") layer.states[drag.index].ui = [Math.max(0, Math.round(p.x - drag.dx)), Math.max(0, Math.round(p.y - drag.dy))];
    else { drag.x = p.x; drag.y = p.y; }
  }

  function up(e: PointerEvent) {
    if (drag?.kind === "wire" && layer) {
      const target = document.elementFromPoint(e.clientX, e.clientY)?.closest("[data-state]");
      if (target) {
        const to = Number(target.getAttribute("data-state"));
        const from = drag.from;
        if (to !== from && layer.states[to].kind.type !== "entry" && layer.states[to].kind.type !== "any") {
          edit(() => addTransition(layer, from, to));
          editor.smSelection = { type: "transition", index: layer.transitions.length - 1 };
        }
      }
    }
    drag = null;
  }

  /** Arrow geometry between two cards; parallel arrows between the same pair fan out. */
  function arrow(t: Transition, i: number) {
    if (!layer) return "";
    const a = pos(layer.states[t.from], t.from), b = pos(layer.states[t.to], t.to);
    const x1 = a[0] + W, y1 = a[1] + H / 2, x2 = b[0], y2 = b[1] + H / 2;
    const siblings = layer.transitions.filter((o) => (o.from === t.from && o.to === t.to) || (o.from === t.to && o.to === t.from));
    const k = siblings.indexOf(t) - (siblings.length - 1) / 2;
    const bend = 40 + k * 26;
    const back = x2 < x1 + 20;
    const c1 = back ? [x1 + bend + 40, y1 - bend] : [x1 + bend, y1 + k * 10];
    const c2 = back ? [x2 - bend - 40, y2 - bend] : [x2 - bend, y2 + k * 10];
    return `M${x1} ${y1}C${c1[0]} ${c1[1]} ${c2[0]} ${c2[1]} ${x2} ${y2}`;
  }

  function labelAt(t: Transition) {
    if (!layer) return [0, 0];
    const a = pos(layer.states[t.from], t.from), b = pos(layer.states[t.to], t.to);
    const siblings = layer.transitions.filter((o) => (o.from === t.from && o.to === t.to) || (o.from === t.to && o.to === t.from));
    const k = siblings.indexOf(t) - (siblings.length - 1) / 2;
    const back = b[0] < a[0] + W + 20;
    return [(a[0] + W + b[0]) / 2, (a[1] + b[1]) / 2 + H / 2 + k * 18 - (back ? 60 : 12)];
  }

  const animName = (i: number) => doc.animations[i]?.name ?? `animation ${i}`;
  const shapeName = (id: number) => doc.shapes.find((s) => s.id === id)?.name ?? `shape ${id}`;

  function setConditionInput(c: Condition, input: number) {
    c.input = input;
    const kind = sm!.inputs[input]?.kind;
    if (kind === "bool") { c.op = "equal"; c.value = 1; }
    if (kind === "trigger") { c.op = "equal"; c.value = 0; }
  }

  function setActionType(l: { actions: Action[] }, i: number, type: Action["type"]) {
    const firstOf = (k: InputKind) => Math.max(0, sm!.inputs.findIndex((x) => x.kind === k));
    l.actions[i] =
      type === "setBool" ? { type, input: firstOf("bool"), value: true }
      : type === "toggleBool" ? { type, input: firstOf("bool") }
      : type === "setNumber" ? { type, input: firstOf("number"), value: 0 }
      : type === "fireTrigger" ? { type, input: firstOf("trigger") }
      : { type: "fireEvent", event: 0 };
  }
</script>

<div class="sm">
  <!-- Machines, inputs, events, listeners -->
  <div class="side">
    <div class="head">
      <select value={editor.machine} onchange={(e) => { editor.machine = Number(e.currentTarget.value); editor.layer = 0; editor.smSelection = null; }} disabled={!machines.length}>
        {#each machines as m, i}<option value={i}>{m.name}</option>{/each}
      </select>
      <button onclick={() => edit(() => { editor.doc.stateMachines!.push(newMachine(`Machine ${machines.length + 1}`)); editor.machine = editor.doc.stateMachines!.length - 1; editor.layer = 0; })} title="New state machine">+</button>
      <button disabled={!sm} onclick={() => edit(() => { editor.doc.stateMachines!.splice(editor.machine, 1); editor.machine = 0; editor.layer = 0; editor.smSelection = null; })} title="Delete this machine">−</button>
    </div>
    {#if sm}
      <input class="name" value={sm.name} onchange={(e) => edit(() => (sm.name = e.currentTarget.value))} aria-label="Machine name" />
      <div class="panel-title">Inputs</div>
      {#each sm.inputs as input, i}
        <div class="row">
          <span class="kind">{input.kind === "bool" ? "on/off" : input.kind === "number" ? "number" : "trigger"}</span>
          <input value={input.name} onchange={(e) => edit(() => (input.name = e.currentTarget.value))} aria-label="Input name" />
          {#if input.kind === "bool"}
            <input type="checkbox" checked={!!input.value} title="Starts on" onchange={(e) => edit(() => (input.value = e.currentTarget.checked ? 1 : 0))} />
          {:else if input.kind === "number"}
            <input type="number" class="num" value={input.value} title="Starting value" onchange={(e) => edit(() => (input.value = Number(e.currentTarget.value)))} />
          {:else}<span></span>{/if}
          <button class="x" onclick={() => edit(() => deleteInput(sm, i))} title="Remove input">×</button>
        </div>
      {/each}
      <div class="adders">
        <button onclick={() => edit(() => addInput(sm, "bool"))}>+ on/off</button>
        <button onclick={() => edit(() => addInput(sm, "number"))}>+ number</button>
        <button onclick={() => edit(() => addInput(sm, "trigger"))}>+ trigger</button>
      </div>

      <div class="panel-title">Events <small>the app hears these</small></div>
      {#each doc.events ?? [] as ev, i}
        <div class="row">
          <span class="kind">event</span>
          <input value={ev} onchange={(e) => edit(() => (editor.doc.events![i] = e.currentTarget.value))} aria-label="Event name" />
          <span></span>
          <button class="x" onclick={() => edit(() => deleteEvent(editor.doc, i))} title="Remove event">×</button>
        </div>
      {/each}
      <div class="adders"><button onclick={() => edit(() => editor.doc.events!.push(`event ${(doc.events?.length ?? 0) + 1}`))}>+ event</button></div>

      <div class="panel-title">Listeners <small>pointer on a shape</small></div>
      {#each sm.listeners as l, li}
        <div class="listener">
          <div class="row">
            <select value={l.target} onchange={(e) => edit(() => (l.target = Number(e.currentTarget.value)))} aria-label="Target shape">
              {#each doc.shapes as s}<option value={s.id}>{s.name}</option>{/each}
            </select>
            <select value={l.event} onchange={(e) => edit(() => (l.event = e.currentTarget.value as typeof l.event))} aria-label="Pointer event">
              {#each POINTERS as p}<option value={p.kind}>{p.label}</option>{/each}
            </select>
            <button class="x" onclick={() => edit(() => sm.listeners.splice(li, 1))} title="Remove listener">×</button>
          </div>
          {#each l.actions as a, ai}
            <div class="row action">
              <select value={a.type} onchange={(e) => edit(() => setActionType(l, ai, e.currentTarget.value as Action["type"]))} aria-label="Action">
                <option value="setBool">set on/off</option><option value="toggleBool">toggle</option><option value="setNumber">set number</option>
                <option value="fireTrigger">fire trigger</option><option value="fireEvent">send event</option>
              </select>
              {#if a.type === "fireEvent"}
                <select value={a.event} onchange={(e) => edit(() => (a.event = Number(e.currentTarget.value)))} aria-label="Event">
                  {#each doc.events ?? [] as ev, i}<option value={i}>{ev}</option>{/each}
                </select>
              {:else}
                <select value={a.input} onchange={(e) => edit(() => (a.input = Number(e.currentTarget.value)))} aria-label="Input">
                  {#each sm.inputs as inp, i}
                    {#if (a.type === "setNumber" && inp.kind === "number") || (a.type === "fireTrigger" && inp.kind === "trigger") || ((a.type === "setBool" || a.type === "toggleBool") && inp.kind === "bool")}
                      <option value={i}>{inp.name}</option>
                    {/if}
                  {/each}
                </select>
              {/if}
              {#if a.type === "setBool"}
                <select value={a.value ? 1 : 0} onchange={(e) => edit(() => (a.value = e.currentTarget.value === "1"))} aria-label="Value"><option value={1}>on</option><option value={0}>off</option></select>
              {:else if a.type === "setNumber"}
                <input type="number" class="num" value={a.value} onchange={(e) => edit(() => (a.value = Number(e.currentTarget.value)))} aria-label="Value" />
              {:else}<span></span>{/if}
              <button class="x" onclick={() => edit(() => l.actions.splice(ai, 1))} title="Remove action">×</button>
            </div>
          {/each}
          <div class="adders"><button onclick={() => edit(() => { const a = defaultAction(sm, editor.doc); if (a) l.actions.push(a); else editor.error = "Add an input or an event first"; })}>+ action</button></div>
        </div>
      {/each}
      <div class="adders">
        <button disabled={!doc.shapes.length} onclick={() => edit(() => sm.listeners.push({ target: editor.selected ?? doc.shapes[0].id, event: "click", actions: [] }))}>+ listener{editor.selectedShape ? ` on ${editor.selectedShape.name}` : ""}</button>
      </div>
    {:else}
      <p class="hint">No state machine yet. Add one to make the animation interactive: inputs the app sets, states that play animations, and transitions between them.</p>
    {/if}
  </div>

  <!-- The graph -->
  <div class="graph">
    {#if sm && layer}
      <div class="tabs">
        {#each sm.layers as l, i}
          <button class:active={editor.layer === i} onclick={() => { editor.layer = i; editor.smSelection = null; }} ondblclick={() => { const n = prompt("Layer name", l.name); if (n) edit(() => (l.name = n)); }}>{l.name}</button>
        {/each}
        <button onclick={() => edit(() => { sm.layers.push(newLayer(`Layer ${sm.layers.length + 1}`)); editor.layer = sm.layers.length - 1; })} title="New layer (layers run at the same time)">+ layer</button>
        {#if sm.layers.length > 1}<button onclick={() => edit(() => { sm.layers.splice(editor.layer, 1); editor.layer = 0; editor.smSelection = null; })} title="Delete this layer">− layer</button>{/if}
        <span class="spacer"></span>
        <button onclick={() => edit(() => { const i = addState(layer, { name: animName(editor.anim), kind: { type: "animation", animation: editor.anim, speed: 1 } }); editor.smSelection = { type: "state", index: i }; })}>+ state</button>
        <button disabled={!sm.inputs.some((i) => i.kind === "number")} title="Mixes animations by a number input"
          onclick={() => edit(() => { const input = sm.inputs.findIndex((i) => i.kind === "number"); const i = addState(layer, { name: "Blend", kind: { type: "blend1D", input, children: [[0, 0], [Math.min(1, doc.animations.length - 1), 100]] } }); editor.smSelection = { type: "state", index: i }; })}>+ blend</button>
      </div>
      <svg bind:this={svg} onpointerdown={down} onpointermove={move} onpointerup={up} role="application" aria-label="State graph">
        <defs>
          <marker id="arrow" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" /></marker>
          <marker id="arrow-sel" viewBox="0 0 10 10" refX="9" refY="5" markerWidth="8" markerHeight="8" orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" /></marker>
        </defs>
        {#each layer.transitions as t, i}
          {@const sel = selTransition === t}
          <path d={arrow(t, i)} class="wire" class:sel data-transition={i} marker-end={sel ? "url(#arrow-sel)" : "url(#arrow)"} />
          <path d={arrow(t, i)} class="wire-hit" data-transition={i} />
          {@const [lx, ly] = labelAt(t)}
          <text x={lx} y={ly} class="wire-label" class:sel data-transition={i} text-anchor="middle">{describe(t, sm)}</text>
        {/each}
        {#if drag?.kind === "wire"}
          {@const a = pos(layer.states[drag.from], drag.from)}
          <path d="M{a[0] + W} {a[1] + H / 2}L{drag.x} {drag.y}" class="wire ghost" />
        {/if}
        {#each layer.states as s, i}
          {@const [x, y] = pos(s, i)}
          {@const fixed = s.kind.type === "entry" || s.kind.type === "any" || s.kind.type === "exit"}
          <g transform="translate({x} {y})" data-state={i} class="card" class:fixed class:sel={selState === s} class:random={s.random}>
            <rect width={W} height={H} rx="8" />
            <text x="12" y="19" class="title">{s.name}</text>
            <text x="12" y="34" class="sub">
              {#if s.kind.type === "animation"}{animName(s.kind.animation)}{s.kind.speed && s.kind.speed !== 1 ? ` × ${s.kind.speed}` : ""}
              {:else if s.kind.type === "blend1D"}blend by {sm.inputs[s.kind.input]?.name ?? "?"}
              {:else if s.kind.type === "entry"}starts here
              {:else if s.kind.type === "any"}from anywhere
              {:else}ends the layer{/if}
            </text>
            {#if s.kind.type !== "exit"}
              <circle cx={W} cy={H / 2} r="7" class="port" data-port={i} />
            {/if}
          </g>
        {/each}
      </svg>
    {:else}
      <div class="empty">
        <p>A state machine decides which animation plays, from inputs your app or the pointer sets.</p>
        <button onclick={() => edit(() => { editor.doc.stateMachines!.push(newMachine("State Machine 1")); editor.machine = editor.doc.stateMachines!.length - 1; editor.layer = 0; })}>Create a state machine</button>
      </div>
    {/if}
  </div>

  <!-- Inspector -->
  <div class="inspector">
    {#if selState && sm && layer}
      <div class="panel-title">State</div>
      <div class="grid">
        <label>Name</label>
        <input value={selState.name} onchange={(e) => edit(() => (selState.name = e.currentTarget.value))} />
        {#if selState.kind.type === "animation"}
          <label>Plays</label>
          <select value={selState.kind.animation} onchange={(e) => edit(() => ((selState.kind as any).animation = Number(e.currentTarget.value)))}>
            {#each doc.animations as a, i}<option value={i}>{a.name}</option>{/each}
          </select>
          <label>Speed</label>
          <input type="number" step="0.1" value={selState.kind.speed ?? 1} onchange={(e) => edit(() => ((selState.kind as any).speed = Number(e.currentTarget.value)))} />
        {:else if selState.kind.type === "blend1D"}
          <label>Blend by</label>
          <select value={selState.kind.input} onchange={(e) => edit(() => ((selState.kind as any).input = Number(e.currentTarget.value)))}>
            {#each sm.inputs as inp, i}{#if inp.kind === "number"}<option value={i}>{inp.name}</option>{/if}{/each}
          </select>
          {#each selState.kind.children as child, ci}
            <label>At {child[1]}</label>
            <div class="pair">
              <select value={child[0]} onchange={(e) => edit(() => (child[0] = Number(e.currentTarget.value)))} aria-label="Animation">
                {#each doc.animations as a, i}<option value={i}>{a.name}</option>{/each}
              </select>
              <input type="number" value={child[1]} onchange={(e) => edit(() => (child[1] = Number(e.currentTarget.value)))} aria-label="Input value" />
              <button class="x" onclick={() => edit(() => (selState.kind as any).children.splice(ci, 1))} disabled={selState.kind.children.length <= 2}>×</button>
            </div>
          {/each}
          <span></span><button class="small" onclick={() => edit(() => (selState.kind as any).children.push([0, 50]))}>+ pose</button>
        {/if}
        {#if selState.kind.type !== "entry" && selState.kind.type !== "any" && selState.kind.type !== "exit"}
          <label for="st-random">Random</label>
          <input id="st-random" type="checkbox" checked={!!selState.random} title="Pick among this state's transitions at random, by weight" onchange={(e) => edit(() => (selState.random = e.currentTarget.checked))} />
          <span></span><button class="small danger" onclick={() => edit(() => { deleteState(layer, sel!.index); editor.smSelection = null; })}>Delete state</button>
        {/if}
      </div>
      <p class="hint">Drag the port on the right of a card onto another card to add a transition.</p>
    {:else if selTransition && sm && layer}
      <div class="panel-title">Transition <small>{layer.states[selTransition.from]?.name} → {layer.states[selTransition.to]?.name}</small></div>
      <div class="grid">
        <label>Blend</label>
        <div class="pair"><input type="number" min="0" value={selTransition.durationMs ?? 0} onchange={(e) => edit(() => (selTransition.durationMs = Math.max(0, Number(e.currentTarget.value))))} /><span class="unit">ms</span></div>
        <label for="tr-exit">After</label>
        <div class="pair">
          <input id="tr-exit" type="checkbox" checked={selTransition.exitAfterMs !== undefined} onchange={(e) => edit(() => { if (e.currentTarget.checked) selTransition.exitAfterMs = 1000; else delete selTransition.exitAfterMs; })} />
          {#if selTransition.exitAfterMs !== undefined}
            <input type="number" min="0" value={selTransition.exitAfterMs} onchange={(e) => edit(() => (selTransition.exitAfterMs = Math.max(0, Number(e.currentTarget.value))))} /><span class="unit">ms</span>
          {:else}<span class="unit">of the source playing</span>{/if}
        </div>
        <label>Weight</label>
        <input type="number" min="1" value={selTransition.randomWeight ?? 1} title="Only matters when the source state is random" onchange={(e) => edit(() => (selTransition.randomWeight = Math.max(1, Number(e.currentTarget.value))))} />
      </div>
      <div class="panel-title">Conditions <small>all must hold</small></div>
      {#each selTransition.conditions as c, ci}
        {@const kind = sm.inputs[c.input]?.kind}
        <div class="row cond">
          <select value={c.input} onchange={(e) => edit(() => setConditionInput(c, Number(e.currentTarget.value)))} aria-label="Input">
            {#each sm.inputs as inp, i}<option value={i}>{inp.name}</option>{/each}
          </select>
          {#if kind === "bool"}
            <select value={c.value ? 1 : 0} onchange={(e) => edit(() => (c.value = Number(e.currentTarget.value)))} aria-label="Is"><option value={1}>is on</option><option value={0}>is off</option></select>
          {:else if kind === "number"}
            <select value={c.op} onchange={(e) => edit(() => (c.op = e.currentTarget.value as typeof c.op))} aria-label="Compare">{#each OPS as o}<option value={o.op}>{o.label}</option>{/each}</select>
            <input type="number" class="num" value={c.value} onchange={(e) => edit(() => (c.value = Number(e.currentTarget.value)))} aria-label="Value" />
          {:else}<span class="unit">fires</span>{/if}
          <button class="x" onclick={() => edit(() => selTransition.conditions.splice(ci, 1))}>×</button>
        </div>
      {/each}
      <div class="adders">
        <button disabled={!sm.inputs.length} onclick={() => edit(() => { const c: Condition = { input: 0, op: "equal", value: sm.inputs[0].kind === "bool" ? 1 : 0 }; selTransition.conditions.push(c); })}>+ condition</button>
        <button class="danger" onclick={() => edit(() => { layer.transitions.splice(sel!.index, 1); editor.smSelection = null; })}>Delete transition</button>
      </div>
    {:else}
      <p class="hint">Select a state or a transition to edit it. Use <b>Preview</b> to play the machine with Rive's runtime and try the inputs.</p>
    {/if}
  </div>
</div>

<style>
  .sm { display: grid; grid-template-columns: 300px 1fr 280px; height: 320px; background: var(--panel); min-height: 0; }
  .side, .inspector { overflow: auto; padding-bottom: 12px; }
  .side { border-right: 1px solid var(--line); }
  .inspector { border-left: 1px solid var(--line); }
  .head { display: flex; gap: 4px; padding: 8px 10px 0; }
  .head select { flex: 1; }
  .name { margin: 6px 10px 0; width: calc(100% - 20px); }
  .panel-title small { font-weight: 400; text-transform: none; letter-spacing: 0; margin-left: 6px; }
  .row { display: grid; grid-template-columns: 58px 1fr auto 22px; gap: 4px; align-items: center; padding: 2px 10px; }
  .row input, .row select { min-width: 0; }
  .row.action { grid-template-columns: 1fr 1fr auto 22px; padding-left: 18px; }
  .row.cond { grid-template-columns: 1fr auto auto 22px; }
  .kind { color: var(--muted); font-size: 11px; }
  .num { width: 56px; }
  .x { padding: 0; width: 20px; height: 20px; color: var(--muted); background: none; border-color: transparent; }
  .adders { display: flex; gap: 4px; padding: 4px 10px; flex-wrap: wrap; }
  .adders button, .small { padding: 2px 8px; font-size: 12px; }
  .listener { border-top: 1px solid var(--line); padding: 4px 0; }
  .danger { color: var(--danger); }
  .hint { color: var(--muted); padding: 6px 12px; font-size: 12px; }
  .graph { display: grid; grid-template-rows: auto 1fr; min-width: 0; min-height: 0; }
  .tabs { display: flex; gap: 4px; padding: 6px 8px; border-bottom: 1px solid var(--line); align-items: center; }
  .tabs .spacer { flex: 1; }
  .tabs button.active { background: var(--accent); border-color: var(--accent); color: #fff; }
  svg { width: 100%; height: 100%; display: block; touch-action: none; user-select: none;
    background: radial-gradient(circle, #2a2a33 1px, transparent 1px) 0 0 / 20px 20px, var(--bg); }
  .card rect { fill: var(--panel-2); stroke: var(--line); stroke-width: 1.5; cursor: grab; }
  .card.fixed rect { fill: #1a1a20; stroke-dasharray: 4 3; }
  .card.sel rect { stroke: var(--accent); }
  .card.random rect { stroke: var(--key); }
  .card .title { fill: var(--text); font-size: 13px; font-weight: 600; pointer-events: none; }
  .card .sub { fill: var(--muted); font-size: 11px; pointer-events: none; }
  .port { fill: var(--accent); stroke: var(--panel); stroke-width: 2; cursor: crosshair; }
  .wire { fill: none; stroke: #6b6b78; stroke-width: 1.5; pointer-events: none; }
  .wire.sel { stroke: var(--accent); stroke-width: 2; }
  .wire.ghost { stroke: var(--accent); stroke-dasharray: 4 3; }
  .wire-hit { fill: none; stroke: transparent; stroke-width: 12; cursor: pointer; }
  .wire-label { fill: var(--muted); font-size: 11px; cursor: pointer; paint-order: stroke; stroke: var(--bg); stroke-width: 4px; }
  .wire-label.sel { fill: var(--text); }
  marker path { fill: #6b6b78; }
  #arrow-sel path { fill: var(--accent); }
  .grid { display: grid; grid-template-columns: 60px 1fr; gap: 6px 8px; align-items: center; padding: 0 12px; }
  .grid label { color: var(--muted); }
  .pair { display: flex; gap: 4px; align-items: center; }
  .pair input[type="number"] { width: 70px; }
  .unit { color: var(--muted); font-size: 12px; }
  .empty { display: grid; place-items: center; align-content: center; gap: 10px; color: var(--muted); text-align: center; padding: 20px; }
</style>
