<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";

  import FileFields from "$lib/components/FileFields.svelte";
  import GeneratePanel from "$lib/components/GeneratePanel.svelte";
  import GridControls from "$lib/components/GridControls.svelte";
  import LayoutControls from "$lib/components/LayoutControls.svelte";
  import PreviewSection from "$lib/components/PreviewSection.svelte";
  import PrintControls from "$lib/components/PrintControls.svelte";
  import { createTiler } from "$lib/tiler.svelte";

  const tiler = createTiler();

  onMount(() => {
    let unlisten: (() => void) | undefined;
    let disposed = false;

    (async () => {
      // Attach first so a file opened while the app runs is never missed.
      try {
        unlisten = await listen<string>("open-image", (event) => {
          if (typeof event.payload === "string" && event.payload !== tiler.inputPath) {
            void tiler.applyInputPath(event.payload);
          }
        });
      } catch {
        // Event listening is best-effort.
      }

      // Covers the command line and a cold start opened via the OS.
      try {
        const path = await invoke<string | null>("startup_input");
        if (!disposed && path && path !== tiler.inputPath) {
          await tiler.applyInputPath(path);
        }
      } catch {
        // The startup hint is optional.
      }

      if (disposed) unlisten?.();
    })();

    return () => {
      disposed = true;
      unlisten?.();
    };
  });
</script>

<svelte:head>
  <title>Image Tiler</title>
</svelte:head>

<main>
  <header>
    <h1>Image Tiler</h1>
    <p class="subtitle">
      Split an image into a grid and export it as a print-ready PDF.
    </p>
  </header>

  <FileFields {tiler} />

  {#if tiler.previewSrc}
    <PreviewSection {tiler} />
  {/if}

  <GridControls {tiler} />
  <PrintControls {tiler} />
  <LayoutControls {tiler} />
  <GeneratePanel {tiler} />
</main>
