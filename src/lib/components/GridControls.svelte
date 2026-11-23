<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="field">
  <div class="field-header">
    <span class="label">Grid</span>
    <div class="segmented" role="group" aria-label="Grid mode">
      <button
        type="button"
        class:active={tiler.sizeMode === "tile"}
        onclick={() => (tiler.sizeMode = "tile")}>Tile size</button
      >
      <button
        type="button"
        class:active={tiler.sizeMode === "grid"}
        onclick={() => (tiler.sizeMode = "grid")}>Rows × cols</button
      >
      <button
        type="button"
        class:active={tiler.sizeMode === "a4"}
        onclick={() => (tiler.sizeMode = "a4")}>A4 pages</button
      >
    </div>
  </div>

  {#if tiler.sizeMode === "tile"}
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
      <span class="hint">
        = {tiler.tilePxW} × {tiler.tilePxH} px at {tiler.dpi} DPI
      </span>
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
  {:else if tiler.sizeMode === "grid"}
    <div class="grid-2">
      <label>
        Columns
        <input type="number" min="1" step="1" bind:value={tiler.gridCols} />
      </label>
      <label>
        Rows
        <input type="number" min="1" step="1" bind:value={tiler.gridRows} />
      </label>
    </div>
    <p class="hint">
      Tile = {tiler.tilePxW} × {tiler.tilePxH} px at {tiler.dpi} DPI ·
      {tiler.columns} × {tiler.rows} = {tiler.columns * tiler.rows} page{tiler.columns *
        tiler.rows ===
      1
        ? ""
        : "s"}
    </p>
  {:else}
    <div class="field-header">
      <span class="hint">
        A4 {tiler.a4Landscape ? "landscape" : "portrait"} ({tiler.a4Page[0]} ×
        {tiler.a4Page[1]} mm)
      </span>
      <div class="segmented" role="group" aria-label="A4 grid mode">
        <button
          type="button"
          class:active={tiler.a4Spec === "grid"}
          onclick={() => (tiler.a4Spec = "grid")}>Rows × cols</button
        >
        <button
          type="button"
          class:active={tiler.a4Spec === "count"}
          onclick={() => (tiler.a4Spec = "count")}>Page count</button
        >
      </div>
    </div>

    {#if tiler.a4Spec === "grid"}
      <div class="grid-2">
        <label>
          Columns
          <input type="number" min="1" step="1" bind:value={tiler.a4Cols} />
        </label>
        <label>
          Rows
          <input type="number" min="1" step="1" bind:value={tiler.a4Rows} />
        </label>
      </div>
    {:else}
      <label>
        Total A4 pages
        <input type="number" min="1" step="1" bind:value={tiler.a4Total} />
      </label>
    {/if}

    <p class="hint">
      {tiler.a4Grid.cols} × {tiler.a4Grid.rows} A4 pages · image scaled to fit ≈
      {tiler.a4EffectiveDpi} DPI
    </p>
  {/if}
</section>
