<script lang="ts">
  import TileGridPreview from "$lib/TileGridPreview.svelte";
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="preview-section">
  <TileGridPreview
    src={tiler.previewSrc}
    plan={tiler.plan}
    error={tiler.planError}
  />
  {#if tiler.plan}
    <p class="hint">
      {tiler.plan.image_width} × {tiler.plan.image_height} px · {tiler.plan.page_width_mm}
      × {tiler.plan.page_height_mm} mm · grid {tiler.plan.columns} ×
      {tiler.plan.rows} · {tiler.pageCount} page{tiler.pageCount === 1 ? "" : "s"} · ≈
      {tiler.effectiveDpi} DPI
    </p>
  {:else if tiler.planLoading}
    <p class="hint">Calculating layout…</p>
  {/if}
</section>
