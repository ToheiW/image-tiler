<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<button
  type="button"
  class="primary"
  onclick={tiler.generate}
  disabled={!tiler.canGenerate}
>
  {tiler.busy ? "Generating…" : "Generate PDF"}
</button>

<div class="feedback" aria-live="polite">
  {#if tiler.error}
    <p class="error">{tiler.error}</p>
  {/if}

  {#if tiler.report}
    {@const report = tiler.report}
    <section class="result">
      <h2>PDF created</h2>
      <p>
        {report.pages} page{report.pages === 1 ? "" : "s"} · {report.columns} ×
        {report.rows} tiles · {report.page_width_mm} × {report.page_height_mm} mm ·
        ≈ {Math.round(report.effective_dpi)} DPI
      </p>
      <p class="path">{report.output}</p>
      <div class="actions">
        <button type="button" onclick={tiler.reveal}>Reveal in Finder</button>
        {#if tiler.shareSupported}
          <button type="button" onclick={tiler.share}>Share…</button>
        {/if}
      </div>
    </section>
  {/if}
</div>
