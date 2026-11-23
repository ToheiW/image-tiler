<script lang="ts">
  import TileGridPreview from "$lib/TileGridPreview.svelte";
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="preview-section">
  <TileGridPreview
    src={tiler.previewSrc}
    tileWidth={tiler.tilePxW}
    tileHeight={tiler.tilePxH}
    onsize={tiler.handleSize}
  />
  {#if tiler.columns > 0 && tiler.rows > 0}
    <p class="hint">
      {tiler.naturalWidth} × {tiler.naturalHeight} px · grid {tiler.columns} ×
      {tiler.rows} · {tiler.pageCount} page{tiler.pageCount === 1 ? "" : "s"}
    </p>
  {/if}
</section>
