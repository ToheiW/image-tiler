<script lang="ts">
  import type { Tiler } from "$lib/tiler.svelte";

  let { tiler }: { tiler: Tiler } = $props();
</script>

<fieldset>
  <legend>PDF layout</legend>
  <label class="radio">
    <input type="radio" bind:group={tiler.layout} value="one_tile_per_page" />
    <span>One tile per page <span class="hint">print and assemble</span></span>
  </label>
  <label class="radio">
    <input type="radio" bind:group={tiler.layout} value="single_page" />
    <span>Single page <span class="hint">whole image on one page</span></span>
  </label>
</fieldset>
