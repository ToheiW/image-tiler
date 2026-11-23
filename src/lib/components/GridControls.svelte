<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="field">
  <div class="field-header">
    <span class="label">Grid</span>
    <div class="segmented" role="group" aria-label="Sizing mode">
      <button
        type="button"
        class:active={tiler.mode === "pages"}
        onclick={() => (tiler.mode = "pages")}>Paper pages</button
      >
      <button
        type="button"
        class:active={tiler.mode === "tile"}
        onclick={() => (tiler.mode = "tile")}>Exact tile size</button
      >
    </div>
  </div>

  {#if tiler.mode === "pages"}
    <div class="field-header">
      <span class="label">Paper</span>
      <div class="segmented" role="group" aria-label="Paper size">
        <button
          type="button"
          class:active={tiler.pageSize === "a4"}
          onclick={() => (tiler.pageSize = "a4")}>A4</button
        >
        <button
          type="button"
          class:active={tiler.pageSize === "a3"}
          onclick={() => (tiler.pageSize = "a3")}>A3</button
        >
      </div>
    </div>

    <div class="field-header">
      <span class="label">Orientation</span>
      <div class="segmented" role="group" aria-label="Page orientation">
        <button
          type="button"
          class:active={tiler.orientation === "portrait"}
          onclick={() => (tiler.orientation = "portrait")}>Portrait</button
        >
        <button
          type="button"
          class:active={tiler.orientation === "landscape"}
          onclick={() => (tiler.orientation = "landscape")}>Landscape</button
        >
      </div>
    </div>

    {#if tiler.layout === "one_tile_per_page"}
      <div class="grid-2">
        <label>
          Columns
          <input type="number" min="1" step="1" bind:value={tiler.columns} />
        </label>
        <label>
          Rows
          <input type="number" min="1" step="1" bind:value={tiler.rows} />
        </label>
      </div>
    {:else}
      <p class="hint">Single page: the whole image is fitted to one sheet.</p>
    {/if}

    {#if tiler.plan}
      <p class="hint">
        {tiler.plan.page_width_mm} × {tiler.plan.page_height_mm} mm · grid
        {tiler.plan.columns} × {tiler.plan.rows} · {tiler.pageCount} page{tiler.pageCount ===
        1
          ? ""
          : "s"} · ≈ {tiler.effectiveDpi} DPI
      </p>
    {/if}
  {:else}
    <div class="grid-2">
      <label>
        Width ({tiler.unit})
        <input type="number" min="1" step="1" bind:value={tiler.tileW} />
      </label>
      <label>
        Height ({tiler.unit})
        <input type="number" min="1" step="1" bind:value={tiler.tileH} />
      </label>
    </div>
    <div class="field-header">
      <span class="hint">= {tiler.tilePxW} × {tiler.tilePxH} px</span>
      <div class="segmented" role="group" aria-label="Tile size unit">
        <button
          type="button"
          class:active={tiler.unit === "mm"}
          onclick={() => tiler.switchUnit("mm")}>mm</button
        >
        <button
          type="button"
          class:active={tiler.unit === "px"}
          onclick={() => tiler.switchUnit("px")}>px</button
        >
      </div>
    </div>
    <label>
      DPI
      <input type="number" min="1" step="1" bind:value={tiler.dpi} />
    </label>
    {#if tiler.plan}
      <p class="hint">
        Pages are {tiler.plan.page_width_mm} × {tiler.plan.page_height_mm} mm · grid
        {tiler.plan.columns} × {tiler.plan.rows} · {tiler.pageCount} page{tiler.pageCount ===
        1
          ? ""
          : "s"}
      </p>
    {/if}
  {/if}
</section>
