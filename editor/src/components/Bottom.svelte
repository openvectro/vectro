<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import Timeline from "./Timeline.svelte";
  import StateMachine from "./StateMachine.svelte";
</script>

<section>
  <nav>
    <button class:active={editor.panel === "timeline"} onclick={() => (editor.panel = "timeline")}>Timeline</button>
    <button class:active={editor.panel === "machines"} onclick={() => (editor.panel = "machines")}>State machines</button>
  </nav>
  {#if editor.panel === "timeline"}
    <Timeline />
  {:else}
    <StateMachine />
  {/if}
</section>

<style>
  section { display: grid; grid-template-rows: auto 1fr; }
  nav { display: flex; gap: 2px; background: var(--bg); border-top: 1px solid var(--line); padding: 4px 8px 0; }
  nav button { border-radius: 6px 6px 0 0; border-bottom: 0; background: var(--panel); color: var(--muted); }
  nav button.active { color: var(--text); background: var(--panel-2); }
</style>
