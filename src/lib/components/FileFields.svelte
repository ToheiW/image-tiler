<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<section class="field">
  <label for="input-path">Source image</label>
  <div class="row">
    <input
      id="input-path"
      readonly
      value={tiler.inputPath}
      placeholder="No image selected"
    />
    <button type="button" onclick={tiler.chooseInput}>Choose…</button>
  </div>
</section>

<section class="field">
  <label for="output-path">Output PDF</label>
  <div class="row">
    <input
      id="output-path"
      bind:value={tiler.outputPath}
      placeholder="tiles.pdf"
    />
    <button type="button" onclick={tiler.chooseOutput}>Save as…</button>
  </div>
</section>
