<script lang="ts">
  import { editor } from "../lib/editor.svelte";
  import { argbToCss } from "../lib/types";

  import { isRoot, type Shape } from "../lib/types";

  // Front-most first, the way design tools list layers; a group's pieces sit indented under it.
  const layers = $derived.by(() => {
    const out: { s: Shape; depth: number }[] = [];
    const walk = (list: Shape[], depth: number) => {
      for (const s of [...list].reverse()) {
        out.push({ s, depth });
        if (s.kind === "group" && depth < 32) walk(editor.doc.shapes.filter((c) => c.parent === s.id), depth + 1);
      }
    };
    walk(editor.doc.shapes.filter((s) => isRoot(editor.doc.shapes, s)), 0);
    return out;
  });
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
    {#each layers as { s, depth } (s.id)}
      <li class:selected={editor.selectedIds.includes(s.id)} style:padding-left="{depth * 14}px">
        <button class="row" onclick={(e) => (e.shiftKey ? editor.toggleSelected(s.id) : (editor.selected = s.id))} ondblclick={() => (renaming = s.id)}>
          {#if s.kind === "group"}<span class="group-icon">▾</span>{:else}<span class="swatch" class:round={s.kind === "ellipse"} class:path={s.kind === "path"} style:background={argbToCss(s.fill)}></span>{/if}
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
  .group-icon { width: 14px; text-align: center; color: var(--muted); flex: none; }
  .swatch.path { border-radius: 2px 7px 2px 7px; }
  .order { display: none; gap: 2px; padding-right: 4px; }
  li:hover .order { display: flex; }
  .order button { padding: 0 5px; font-size: 11px; }
  .empty { color: var(--muted); padding: 0 12px; }
</style>
