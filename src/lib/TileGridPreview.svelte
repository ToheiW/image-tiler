<script lang="ts">
  let {
    src,
    tileWidth,
    tileHeight,
    onsize,
  }: {
    src: string;
    tileWidth: number;
    tileHeight: number;
    onsize?: (size: { width: number; height: number }) => void;
  } = $props();

  let imgEl = $state<HTMLImageElement>();
  let canvasEl = $state<HTMLCanvasElement>();
  let failed = $state(false);

  // Keep the overlay usable for very fine grids: draw at most this many lines.
  const MAX_LINES = 400;

  function draw() {
    if (!imgEl || !canvasEl) return;

    const width = imgEl.naturalWidth;
    const height = imgEl.naturalHeight;
    if (!width || !height) return;

    const rect = imgEl.getBoundingClientRect();
    if (rect.width === 0 || rect.height === 0) return;

    const dpr = window.devicePixelRatio || 1;
    canvasEl.width = Math.max(1, Math.round(rect.width * dpr));
    canvasEl.height = Math.max(1, Math.round(rect.height * dpr));
    canvasEl.style.width = `${rect.width}px`;
    canvasEl.style.height = `${rect.height}px`;

    const ctx = canvasEl.getContext("2d");
    if (!ctx) return;

    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, rect.width, rect.height);

    if (tileWidth <= 0 || tileHeight <= 0) return;

    const scaleX = rect.width / width;
    const scaleY = rect.height / height;
    const columns = Math.ceil(width / tileWidth);
    const rows = Math.ceil(height / tileHeight);
    const stepX = Math.max(1, Math.ceil(columns / MAX_LINES)) * tileWidth;
    const stepY = Math.max(1, Math.ceil(rows / MAX_LINES)) * tileHeight;

    ctx.strokeStyle = "rgba(255, 68, 68, 0.9)";
    ctx.lineWidth = 1;
    ctx.beginPath();

    for (let x = stepX; x < width; x += stepX) {
      const px = Math.round(x * scaleX) + 0.5;
      ctx.moveTo(px, 0);
      ctx.lineTo(px, rect.height);
    }
    for (let y = stepY; y < height; y += stepY) {
      const py = Math.round(y * scaleY) + 0.5;
      ctx.moveTo(0, py);
      ctx.lineTo(rect.width, py);
    }

    ctx.stroke();
  }

  $effect(() => {
    src;
    tileWidth;
    tileHeight;
    draw();
  });

  $effect(() => {
    if (!imgEl) return;

    const observer = new ResizeObserver(() => draw());
    observer.observe(imgEl);
    return () => observer.disconnect();
  });

  function handleLoad() {
    failed = false;
    if (imgEl) {
      onsize?.({ width: imgEl.naturalWidth, height: imgEl.naturalHeight });
    }
    draw();
  }
</script>

<div class="preview">
  {#if !failed}
    <img
      bind:this={imgEl}
      {src}
      alt="Tile grid overlay"
      onload={handleLoad}
      onerror={() => (failed = true)}
    />
    <canvas bind:this={canvasEl}></canvas>
  {:else}
    <p class="preview-error">
      Preview unavailable for this file. It can still be tiled and exported.
    </p>
  {/if}
</div>

<style>
  .preview {
    position: relative;
    display: inline-block;
    max-width: 100%;
    line-height: 0;
  }

  img {
    display: block;
    max-width: 100%;
    max-height: 42vh;
    border-radius: 6px;
  }

  canvas {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }

  .preview-error {
    margin: 0;
    padding: 12px;
    line-height: 1.4;
    font-size: 13px;
    color: #a33;
    background: rgba(255, 0, 0, 0.06);
    border-radius: 6px;
  }
</style>
