<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { argbToCss } from "../lib/types";

  // Front-most shape first, the way design tools list layers.
  const layers = $derived([...editor.doc.shapes].reverse());
  let renaming = $state<number | null>(null);

  function rename(id: number, name: string) {
    const s = editor.doc.shapes.find((s) => s.id === id);
    if (s && name.trim() && name !== s.name) {
      editor.checkpoint();
      s.name = name.trim();
    }
    renaming = null;
  }
</script>

<aside>
  <div class="panel-title">Layers</div>
  {#if !layers.length}
    <p class="empty">Pick Rectangle (R) or Ellipse (E) and drag on the artboard.</p>
  {/if}
  <ul>
    {#each layers as s (s.id)}
      <li class:selected={editor.selected === s.id}>
        <button class="row" onclick={() => (editor.selected = s.id)} ondblclick={() => (renaming = s.id)}>
          <span class="swatch" class:round={s.kind === "ellipse"} style:background={argbToCss(s.fill)}></span>
          {#if renaming === s.id}
            <!-- svelte-ignore a11y_autofocus -->
            <input value={s.name} autofocus onblur={(e) => rename(s.id, e.currentTarget.value)}
              onkeydown={(e) => e.key === "Enter" && e.currentTarget.blur()} />
          {:else}
            <span class="name">{s.name}</span>
          {/if}
        </button>
        <span class="order">
          <button onclick={() => editor.reorder(s.id, 1)} title="Bring forward">↑</button>
          <button onclick={() => editor.reorder(s.id, -1)} title="Send backward">↓</button>
        </span>
      </li>
    {/each}
  </ul>
</aside>

<style>
  aside { background: var(--panel); border-right: 1px solid var(--line); overflow: auto; }
  ul { list-style: none; margin: 0; padding: 0 6px; }
  li { display: flex; align-items: center; border-radius: 6px; }
  li.selected { background: var(--accent-soft); }
  .row { flex: 1; display: flex; align-items: center; gap: 8px; background: none; border: 0; text-align: left; padding: 6px; min-width: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .swatch { width: 14px; height: 14px; border-radius: 3px; flex: none; }
  .swatch.round { border-radius: 50%; }
  .order { display: none; gap: 2px; padding-right: 4px; }
  li:hover .order { display: flex; }
  .order button { padding: 0 5px; font-size: 11px; }
  .empty { color: var(--muted); padding: 0 12px; }
</style>
