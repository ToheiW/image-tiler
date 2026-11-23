<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="field">
  <div class="grid-3">
    <label>
      DPI
      <input
        type="number"
        min="1"
        step="1"
        bind:value={tiler.dpi}
        disabled={tiler.sizeMode === "a4"}
      />
    </label>
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
    <label>
      Safe margin (mm)
      <input
        type="number"
        min="0"
        step="1"
        bind:value={tiler.marginMm}
        disabled={tiler.layout === "single_page"}
      />
    </label>
  </div>
  {#if tiler.layout === "single_page"}
    <p class="hint">
      Overlap and safe margin only apply when each tile gets its own page.
    </p>
  {:else}
    <p class="hint">
      Print at 100% (actual size), not "fit to page", then trim along the crop
      marks.
    </p>
  {/if}
</section>
