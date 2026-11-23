import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { basename, dirname, join } from "@tauri-apps/api/path";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import type { ApiError, PdfLayout, PdfReport, PdfRequest } from "./types";

export type SizeMode = "tile" | "grid" | "a4";
export type A4Spec = "grid" | "count";
export type TileUnit = "mm" | "px";

/**
 * All tiling state and behavior in one place. Components receive the returned
 * object and read reactive getters / call methods.
 */
export function createTiler() {
  let inputPath = $state("");
  let outputPath = $state("");
  let previewSrc = $state("");
  let naturalWidth = $state(0);
  let naturalHeight = $state(0);

  let unit = $state<TileUnit>("mm");
  let tileW = $state(100);
  let tileH = $state(100);
  let dpi = $state(300);
  let layout = $state<PdfLayout>("one_tile_per_page");
  let overlapMm = $state(10);
  let marginMm = $state(10);

  let sizeMode = $state<SizeMode>("tile");
  let gridCols = $state(2);
  let gridRows = $state(2);
  let a4Spec = $state<A4Spec>("grid");
  let a4Cols = $state(2);
  let a4Rows = $state(2);
  let a4Total = $state(4);

  let busy = $state(false);
  let error = $state<string | null>(null);
  let report = $state<PdfReport | null>(null);

  // Used so re-picking the input refreshes an untouched default path
  // without clobbering a path the user edited by hand.
  let lastDerivedOutput = "";

  function toPixels(value: number): number {
    if (!Number.isFinite(value)) return 0;
    if (unit === "mm") {
      return dpi > 0 ? Math.round((value / 25.4) * dpi) : 0;
    }
    return Math.round(value);
  }

  /** Picks the rows x cols whose shape best matches the image. */
  function bestGrid(
    total: number,
    imageWidth: number,
    imageHeight: number,
    page: [number, number],
    margin: number,
    overlap: number,
  ): { cols: number; rows: number } {
    const [pageWidth, pageHeight] = page;
    const coreWidth = pageWidth - 2 * (margin + overlap);
    const coreHeight = pageHeight - 2 * (margin + overlap);

    if (
      !Number.isInteger(total) ||
      total < 1 ||
      imageWidth <= 0 ||
      imageHeight <= 0 ||
      !(coreWidth > 0) ||
      !(coreHeight > 0)
    ) {
      return { cols: 1, rows: 1 };
    }

    const target = imageWidth / imageHeight;
    let best = { cols: 1, rows: total, score: Number.POSITIVE_INFINITY };

    for (let cols = 1; cols <= total; cols++) {
      if (total % cols !== 0) continue;
      const rows = total / cols;
      const gridAspect = (cols * coreWidth) / (rows * coreHeight);
      const score = Math.abs(Math.log(gridAspect / target));
      if (score < best.score) {
        best = { cols, rows, score };
      }
    }

    return { cols: best.cols, rows: best.rows };
  }

  let a4Landscape = $derived(naturalWidth > naturalHeight);
  let a4Page = $derived<[number, number]>(
    a4Landscape ? [297, 210] : [210, 297],
  );

  let a4AutoGrid = $derived(
    bestGrid(a4Total, naturalWidth, naturalHeight, a4Page, marginMm, overlapMm),
  );
  let a4Grid = $derived(
    a4Spec === "grid" ? { cols: a4Cols, rows: a4Rows } : a4AutoGrid,
  );

  let gridSpec = $derived(
    sizeMode === "grid"
      ? { cols: gridCols, rows: gridRows }
      : sizeMode === "a4"
        ? a4Grid
        : null,
  );

  let hasValidGridSpec = $derived(
    gridSpec !== null &&
      Number.isInteger(gridSpec.cols) &&
      gridSpec.cols >= 1 &&
      Number.isInteger(gridSpec.rows) &&
      gridSpec.rows >= 1,
  );

  let tilePxW = $derived(
    gridSpec && hasValidGridSpec && naturalWidth > 0
      ? Math.ceil(naturalWidth / gridSpec.cols)
      : toPixels(tileW),
  );
  let tilePxH = $derived(
    gridSpec && hasValidGridSpec && naturalHeight > 0
      ? Math.ceil(naturalHeight / gridSpec.rows)
      : toPixels(tileH),
  );

  let columns = $derived(
    gridSpec
      ? hasValidGridSpec && naturalWidth > 0
        ? gridSpec.cols
        : 0
      : naturalWidth > 0 && tilePxW > 0
        ? Math.ceil(naturalWidth / tilePxW)
        : 0,
  );
  let rows = $derived(
    gridSpec
      ? hasValidGridSpec && naturalHeight > 0
        ? gridSpec.rows
        : 0
      : naturalHeight > 0 && tilePxH > 0
        ? Math.ceil(naturalHeight / tilePxH)
        : 0,
  );
  let pageCount = $derived(
    layout === "single_page" ? (columns > 0 && rows > 0 ? 1 : 0) : columns * rows,
  );

  let a4CoreValid = $derived(
    a4Page[0] - 2 * (marginMm + overlapMm) > 0 &&
      a4Page[1] - 2 * (marginMm + overlapMm) > 0,
  );

  let a4EffectiveDpi = $derived.by(() => {
    if (sizeMode !== "a4" || naturalWidth <= 0 || naturalHeight <= 0) return 0;
    const coreWidth = a4Page[0] - 2 * (marginMm + overlapMm);
    const coreHeight = a4Page[1] - 2 * (marginMm + overlapMm);
    if (!(coreWidth > 0 && coreHeight > 0)) return 0;

    const scale = Math.min(
      (coreWidth * a4Grid.cols) / naturalWidth,
      (coreHeight * a4Grid.rows) / naturalHeight,
    );
    return scale > 0 ? Math.round(25.4 / scale) : 0;
  });

  let gridValid = $derived(
    sizeMode === "tile"
      ? tilePxW > 0 && tilePxH > 0
      : sizeMode === "grid"
        ? hasValidGridSpec
        : hasValidGridSpec && a4CoreValid,
  );

  let canGenerate = $derived(
    !busy &&
      inputPath !== "" &&
      outputPath.trim() !== "" &&
      naturalWidth > 0 &&
      naturalHeight > 0 &&
      tilePxW > 0 &&
      tilePxH > 0 &&
      dpi > 0 &&
      columns > 0 &&
      rows > 0 &&
      gridValid &&
      (layout === "single_page" ||
        (Number.isFinite(overlapMm) &&
          overlapMm >= 0 &&
          Number.isFinite(marginMm) &&
          marginMm >= 0)),
  );

  function round1(value: number): number {
    return Math.round(value * 10) / 10;
  }

  function switchUnit(next: TileUnit) {
    if (next === unit) return;

    const safeDpi = dpi > 0 ? dpi : 300;
    if (next === "mm") {
      tileW = round1((tileW * 25.4) / safeDpi);
      tileH = round1((tileH * 25.4) / safeDpi);
    } else {
      tileW = Math.max(1, Math.round((tileW / 25.4) * safeDpi));
      tileH = Math.max(1, Math.round((tileH / 25.4) * safeDpi));
    }

    unit = next;
  }

  function stripExtension(name: string): string {
    return name.replace(/\.[^.]+$/, "");
  }

  async function applyInputPath(path: string) {
    inputPath = path;
    previewSrc = convertFileSrc(path);
    naturalWidth = 0;
    naturalHeight = 0;
    report = null;
    error = null;

    const derived = await join(
      await dirname(path),
      `${stripExtension(await basename(path))}-tiles.pdf`,
    );
    if (outputPath === "" || outputPath === lastDerivedOutput) {
      outputPath = derived;
    }
    lastDerivedOutput = derived;
  }

  async function chooseInput() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    if (typeof picked !== "string") return;

    await applyInputPath(picked);
  }

  async function chooseOutput() {
    const picked = await save({
      defaultPath: outputPath || undefined,
      filters: [{ name: "PDF", extensions: ["pdf"] }],
    });
    if (typeof picked === "string") {
      outputPath = picked;
    }
  }

  function handleSize(size: { width: number; height: number }) {
    naturalWidth = size.width;
    naturalHeight = size.height;
  }

  async function generate() {
    if (!canGenerate) return;

    busy = true;
    error = null;
    report = null;

    const request: PdfRequest = {
      input: inputPath,
      output: outputPath,
      tile_width: tilePxW,
      tile_height: tilePxH,
      dpi,
      layout,
      overlap_mm: layout === "one_tile_per_page" ? overlapMm : 0,
      margin_mm: layout === "one_tile_per_page" ? marginMm : 0,
      ...(sizeMode === "a4" ? { page_size_mm: a4Page } : {}),
      overwrite: false,
    };

    try {
      report = await invoke<PdfReport>("tile_to_pdf", { request });
    } catch (raw) {
      await handleFailure(raw, request);
    } finally {
      busy = false;
    }
  }

  async function handleFailure(raw: unknown, request: PdfRequest) {
    const apiError = raw as ApiError | undefined;

    if (apiError?.kind === "output_exists") {
      const confirmed = await ask("A PDF already exists at that path. Overwrite it?", {
        title: "Overwrite PDF",
        kind: "warning",
        okLabel: "Overwrite",
        cancelLabel: "Cancel",
      });
      if (!confirmed) return;

      try {
        report = await invoke<PdfReport>("tile_to_pdf", {
          request: { ...request, overwrite: true },
        });
      } catch (retryError) {
        error = (retryError as ApiError)?.message ?? String(retryError);
      }
      return;
    }

    error = apiError?.message ?? String(raw);
  }

  async function reveal() {
    if (report) {
      await revealItemInDir(report.output);
    }
  }

  async function share() {
    if (!report) return;
    try {
      await invoke("share_pdf", { path: report.output });
    } catch (raw) {
      error = typeof raw === "string" ? raw : String(raw);
    }
  }

  return {
    get inputPath() {
      return inputPath;
    },
    get outputPath() {
      return outputPath;
    },
    set outputPath(value: string) {
      outputPath = value;
    },
    get previewSrc() {
      return previewSrc;
    },
    get naturalWidth() {
      return naturalWidth;
    },
    get naturalHeight() {
      return naturalHeight;
    },

    get unit() {
      return unit;
    },
    get tileW() {
      return tileW;
    },
    set tileW(value: number) {
      tileW = value;
    },
    get tileH() {
      return tileH;
    },
    set tileH(value: number) {
      tileH = value;
    },
    get dpi() {
      return dpi;
    },
    set dpi(value: number) {
      dpi = value;
    },
    get layout() {
      return layout;
    },
    set layout(value: PdfLayout) {
      layout = value;
    },
    get overlapMm() {
      return overlapMm;
    },
    set overlapMm(value: number) {
      overlapMm = value;
    },
    get marginMm() {
      return marginMm;
    },
    set marginMm(value: number) {
      marginMm = value;
    },

    get sizeMode() {
      return sizeMode;
    },
    set sizeMode(value: SizeMode) {
      sizeMode = value;
    },
    get gridCols() {
      return gridCols;
    },
    set gridCols(value: number) {
      gridCols = value;
    },
    get gridRows() {
      return gridRows;
    },
    set gridRows(value: number) {
      gridRows = value;
    },
    get a4Spec() {
      return a4Spec;
    },
    set a4Spec(value: A4Spec) {
      a4Spec = value;
    },
    get a4Cols() {
      return a4Cols;
    },
    set a4Cols(value: number) {
      a4Cols = value;
    },
    get a4Rows() {
      return a4Rows;
    },
    set a4Rows(value: number) {
      a4Rows = value;
    },
    get a4Total() {
      return a4Total;
    },
    set a4Total(value: number) {
      a4Total = value;
    },

    get a4Landscape() {
      return a4Landscape;
    },
    get a4Page() {
      return a4Page;
    },
    get a4Grid() {
      return a4Grid;
    },
    get a4EffectiveDpi() {
      return a4EffectiveDpi;
    },

    get tilePxW() {
      return tilePxW;
    },
    get tilePxH() {
      return tilePxH;
    },
    get columns() {
      return columns;
    },
    get rows() {
      return rows;
    },
    get pageCount() {
      return pageCount;
    },

    get busy() {
      return busy;
    },
    get error() {
      return error;
    },
    get report() {
      return report;
    },
    get canGenerate() {
      return canGenerate;
    },

    switchUnit,
    chooseInput,
    chooseOutput,
    handleSize,
    applyInputPath,
    generate,
    reveal,
    share,
  };
}

export type Tiler = ReturnType<typeof createTiler>;
