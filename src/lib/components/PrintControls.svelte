<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="field">
  <div class="field-header">
    <span class="label">Margins (mm)</span>
    <span class="hint">Plus a fixed 5 mm printer-safe edge</span>
  </div>

  <div class="grid-2">
    <label>
      Top
      <input type="number" min="0" step="1" bind:value={tiler.margins.top} />
    </label>
    <label>
      Right
      <input type="number" min="0" step="1" bind:value={tiler.margins.right} />
    </label>
    <label>
      Bottom
      <input type="number" min="0" step="1" bind:value={tiler.margins.bottom} />
    </label>
    <label>
      Left
      <input type="number" min="0" step="1" bind:value={tiler.margins.left} />
    </label>
  </div>

  <label>
    Overlap (mm)
    <input
      type="number"
      min="0"
      step="1"
      bind:value={tiler.overlapMm}
      disabled={tiler.layout === "single_page"}
    />
  </label>

  {#if tiler.layout === "single_page"}
    <p class="hint">Overlap only applies when each tile gets its own page.</p>
  {:else}
    <p class="hint">
      Overlap is limited to the margin on each side. Print at 100% (actual size),
      not "fit to page", then trim along the crop marks.
    </p>
  {/if}
</section>
