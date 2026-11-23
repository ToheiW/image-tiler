<script lang="ts">
  import type { LayoutPlan } from "$lib/types";

  let {
    src,
    plan,
    error = null,
  }: {
    src: string;
    plan: LayoutPlan | null;
    error?: string | null;
  } = $props();

  let canvasEl = $state<HTMLCanvasElement>();
  let imgEl = $state<HTMLImageElement>();
  let loadedSrc = $state("");
  let failedSrc = $state<string | null>(null);

  const PREVIEW_WIDTH = 760;
  const PREVIEW_HEIGHT = 460;

  /** Draws the pages exactly as the PDF lays them out: one sheet per plan page. */
  function draw() {
    const canvas = canvasEl;
    const image = imgEl;
    if (!canvas || !image || !plan || loadedSrc !== src) return;
    if (!image.naturalWidth || !image.naturalHeight) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const cols = plan.columns;
    const rows = plan.rows;
    const pageW = plan.page_width_mm;
    const pageH = plan.page_height_mm;
    if (!(pageW > 0) || !(pageH > 0) || cols < 1 || rows < 1) return;

    // The pages form a grid; laid out side by side they reconstruct the poster.
    const sheetW = pageW * cols;
    const sheetH = pageH * rows;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    const pxPerMm = Math.min(PREVIEW_WIDTH / sheetW, PREVIEW_HEIGHT / sheetH);
    if (!(pxPerMm > 0)) return;

    const cssW = Math.max(1, Math.round(sheetW * pxPerMm));
    const cssH = Math.max(1, Math.round(sheetH * pxPerMm));
    canvas.width = Math.max(1, Math.round(cssW * dpr));
    canvas.height = Math.max(1, Math.round(cssH * dpr));
    canvas.style.width = `${cssW}px`;
    canvas.style.height = `${cssH}px`;

    const k = pxPerMm * dpr;
    ctx.setTransform(k, 0, 0, k, 0, 0);
    ctx.clearRect(0, 0, sheetW, sheetH);
    ctx.imageSmoothingQuality = "high";

    for (const page of plan.pages) {
      const ox = page.col * pageW;
      const oy = page.row * pageH;

      ctx.save();
      ctx.beginPath();
      ctx.rect(ox, oy, pageW, pageH);
      ctx.clip();

      ctx.fillStyle = "#ffffff";
      ctx.fillRect(ox, oy, pageW, pageH);

      // The whole image at the plan's rect; the page clip hides the rest.
      ctx.drawImage(
        image,
        0,
        0,
        image.naturalWidth,
        image.naturalHeight,
        ox + page.image.x,
        oy + page.image.y,
        page.image.width,
        page.image.height,
      );
      ctx.restore();
    }

    // Sheet edges are UI chrome, not printed marks.
    ctx.strokeStyle = "rgba(0, 0, 0, 0.18)";
    ctx.lineWidth = 1 / k;
    for (const page of plan.pages) {
      ctx.strokeRect(
        page.col * pageW,
        page.row * pageH,
        pageW,
        pageH,
      );
    }

    drawCropMarks(ctx, plan);
  }

  /** Same marks as the PDF writer: from the safe boundary to each core edge. */
  function drawCropMarks(ctx: CanvasRenderingContext2D, layout: LayoutPlan) {
    if (layout.pages.length < 2) return;

    const safe = layout.safe_margin_mm;
    const pageW = layout.page_width_mm;
    const pageH = layout.page_height_mm;
    const segments: Array<[number, number, number, number]> = [];

    for (const page of layout.pages) {
      const ox = page.col * pageW;
      const oy = page.row * pageH;
      const core = page.core;

      for (const x of [core.x, core.x + core.width]) {
        if (core.y > safe) {
          segments.push([ox + x, oy + safe, ox + x, oy + core.y]);
        }
        const start = core.y + core.height;
        const end = pageH - safe;
        if (end > start) {
          segments.push([ox + x, oy + start, ox + x, oy + end]);
        }
      }

      for (const y of [core.y, core.y + core.height]) {
        if (core.x > safe) {
          segments.push([ox + safe, oy + y, ox + core.x, oy + y]);
        }
        const start = core.x + core.width;
        const end = pageW - safe;
        if (end > start) {
          segments.push([ox + start, oy + y, ox + end, oy + y]);
        }
      }
    }

    if (segments.length === 0) return;

    ctx.strokeStyle = "#111111";
    ctx.lineWidth = 0.25;
    ctx.beginPath();
    for (const [x1, y1, x2, y2] of segments) {
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
    }
    ctx.stroke();
  }

  $effect(() => {
    // Redraw when the source finishes loading, or the plan changes.
    src;
    plan;
    loadedSrc;
    draw();
  });

  $effect(() => {
    const canvas = canvasEl;
    if (!canvas) return;

    const observer = new ResizeObserver(() => draw());
    observer.observe(canvas);
    return () => observer.disconnect();
  });

  function handleLoad() {
    loadedSrc = src;
    draw();
  }
</script>

<div class="preview">
  <img
    bind:this={imgEl}
    class="source"
    {src}
    alt=""
    onload={handleLoad}
    onerror={() => (failedSrc = src)}
  />

  {#if error}
    <p class="preview-error">{error}</p>
  {:else if failedSrc === src}
    <p class="preview-error">
      Preview unavailable for this file. It can still be tiled and exported.
    </p>
  {:else if plan}
    <canvas bind:this={canvasEl}></canvas>
  {:else}
    <p class="preview-pending">Calculating layout…</p>
  {/if}
</div>

<style>
  .preview {
    display: flex;
    justify-content: center;
  }

  .source {
    position: absolute;
    width: 1px;
    height: 1px;
    opacity: 0;
    pointer-events: none;
  }

  canvas {
    display: block;
    max-width: 100%;
    height: auto;
    border-radius: 4px;
    box-shadow: 0 1px 6px rgba(0, 0, 0, 0.18);
  }

  .preview-error,
  .preview-pending {
    margin: 0;
    padding: 12px;
    line-height: 1.4;
    font-size: 13px;
    border-radius: 6px;
  }

  .preview-error {
    color: #a33;
    background: rgba(255, 0, 0, 0.06);
  }

  .preview-pending {
    color: #666;
    background: rgba(0, 0, 0, 0.04);
  }
</style>
