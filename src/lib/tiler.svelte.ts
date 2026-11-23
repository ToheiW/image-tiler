import { convertFileSrc, invoke } from "@tauri-apps/api/core";
import { basename, dirname, join } from "@tauri-apps/api/path";
import { ask, open, save } from "@tauri-apps/plugin-dialog";
import { revealItemInDir } from "@tauri-apps/plugin-opener";

import type {
  ApiError,
  LayoutPlan,
  LayoutSpec,
  MarginsMm,
  PageOrientation,
  PageSize,
  PdfLayout,
  PdfReport,
  PdfRequest,
  PlanRequest,
  SizeMode,
} from "./types";

export type TileUnit = "mm" | "px";

/**
 * All tiling state and behavior in one place. Components receive the returned
 * object and read reactive getters / call methods.
 */
export function createTiler() {
  let inputPath = $state("");
  let outputPath = $state("");
  let previewSrc = $state("");

  let unit = $state<TileUnit>("mm");
  let tileW = $state(100);
  let tileH = $state(100);
  let dpi = $state(300);
  let layout = $state<PdfLayout>("one_tile_per_page");
  let mode = $state<SizeMode>("pages");
  let pageSize = $state<PageSize>("a4");
  let orientation = $state<PageOrientation>("portrait");
  let columns = $state(2);
  let rows = $state(2);
  let overlapMm = $state(10);
  let margins = $state<MarginsMm>({ top: 10, right: 10, bottom: 10, left: 10 });

  // The backend owns the geometry; this plan is exactly what the PDF uses.
  let plan = $state<LayoutPlan | null>(null);
  let planError = $state<string | null>(null);
  let planLoading = $state(false);

  let busy = $state(false);
  let error = $state<string | null>(null);
  let report = $state<PdfReport | null>(null);
  let shareSupported = $state(false);

  // Used so re-picking the input refreshes an untouched default path
  // without clobbering a path the user edited by hand.
  let lastDerivedOutput = "";
  let inputSeq = 0;
  let planSeq = 0;

  function toPixels(value: number): number {
    if (!Number.isFinite(value) || value <= 0) return 0;
    return unit === "mm" ? Math.round((value / 25.4) * Math.max(1, dpi)) : Math.round(value);
  }

  let tilePxW = $derived(toPixels(tileW));
  let tilePxH = $derived(toPixels(tileH));

  let spec = $derived<LayoutSpec>({
    mode,
    layout,
    page_size: pageSize,
    orientation,
    columns,
    rows,
    dpi,
    tile_width: tilePxW,
    tile_height: tilePxH,
    margins_mm: { ...margins },
    overlap_mm: overlapMm,
  });

  let planReady = $derived(plan !== null && planError === null && !planLoading);
  let pageCount = $derived(plan?.pages.length ?? 0);
  let effectiveDpi = $derived(plan ? Math.round(plan.effective_dpi) : 0);

  let canGenerate = $derived(
    !busy && inputPath !== "" && planReady && pageCount > 0,
  );

  // Ask the backend for the shared plan whenever the image or settings change.
  $effect(() => {
    const path = inputPath;
    const nextSpec = spec;
    const seq = ++planSeq;

    if (!path) {
      plan = null;
      planError = null;
      planLoading = false;
      return;
    }

    planLoading = true;
    const timer = setTimeout(async () => {
      const request: PlanRequest = { input: path, spec: nextSpec };
      try {
        const next = await invoke<LayoutPlan>("plan_layout", { request });
        if (seq !== planSeq) return;
        plan = next;
        planError = null;
      } catch (raw) {
        if (seq !== planSeq) return;
        plan = null;
        planError = (raw as ApiError)?.message ?? String(raw);
      } finally {
        if (seq === planSeq) planLoading = false;
      }
    }, 120);

    return () => clearTimeout(timer);
  });

  // A finished report describes the old settings; drop it when they change.
  $effect(() => {
    inputPath;
    spec;
    report = null;
    error = null;
  });

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
    const seq = ++inputSeq;
    inputPath = path;
    previewSrc = "";
    report = null;
    error = null;

    // The asset protocol only serves paths it has been told about.
    try {
      const allowed = await invoke<string>("allow_preview", { path });
      if (seq === inputSeq) {
        previewSrc = convertFileSrc(allowed);
      }
    } catch (raw) {
      if (seq === inputSeq) {
        error = (raw as ApiError)?.message ?? String(raw);
      }
    }

    try {
      const derived = await join(
        await dirname(path),
        `${stripExtension(await basename(path))}-tiles.pdf`,
      );
      if (seq !== inputSeq) return;
      if (outputPath === "" || outputPath === lastDerivedOutput) {
        outputPath = derived;
      }
      lastDerivedOutput = derived;
    } catch {
      // Path derivation is optional; the save dialog still supplies a path.
    }
  }

  async function chooseInput() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "Images", extensions: ["png", "jpg", "jpeg", "webp"] }],
      });
      if (typeof picked === "string") {
        await applyInputPath(picked);
      }
    } catch (raw) {
      error = (raw as ApiError)?.message ?? String(raw);
    }
  }

  async function generate() {
    if (!canGenerate) return;

    // Always ask where to save: the previous path is only a suggestion.
    let picked: string | null;
    try {
      picked = await save({
        defaultPath: outputPath || undefined,
        filters: [{ name: "PDF", extensions: ["pdf"] }],
      });
    } catch (raw) {
      error = (raw as ApiError)?.message ?? String(raw);
      return;
    }
    if (typeof picked !== "string") return;
    outputPath = picked;

    busy = true;
    error = null;
    report = null;

    const request: PdfRequest = {
      input: inputPath,
      output: picked,
      spec,
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
    if (!report) return;
    try {
      await revealItemInDir(report.output);
    } catch (raw) {
      error = (raw as ApiError)?.message ?? String(raw);
    }
  }

  async function share() {
    if (!report || !shareSupported) return;
    try {
      await invoke("share_pdf", { path: report.output });
    } catch (raw) {
      error = typeof raw === "string" ? raw : String(raw);
    }
  }

  async function loadShareSupport() {
    try {
      shareSupported = await invoke<boolean>("share_supported");
    } catch {
      shareSupported = false;
    }
  }

  return {
    get inputPath() {
      return inputPath;
    },
    get outputPath() {
      return outputPath;
    },
    get previewSrc() {
      return previewSrc;
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
    get margins() {
      return margins;
    },
    set margins(value: MarginsMm) {
      margins = value;
    },

    get mode() {
      return mode;
    },
    set mode(value: SizeMode) {
      mode = value;
    },
    get pageSize() {
      return pageSize;
    },
    set pageSize(value: PageSize) {
      pageSize = value;
    },
    get orientation() {
      return orientation;
    },
    set orientation(value: PageOrientation) {
      orientation = value;
    },
    get columns() {
      return columns;
    },
    set columns(value: number) {
      columns = value;
    },
    get rows() {
      return rows;
    },
    set rows(value: number) {
      rows = value;
    },

    get tilePxW() {
      return tilePxW;
    },
    get tilePxH() {
      return tilePxH;
    },

    get plan() {
      return plan;
    },
    get planError() {
      return planError;
    },
    get planLoading() {
      return planLoading;
    },
    get pageCount() {
      return pageCount;
    },
    get effectiveDpi() {
      return effectiveDpi;
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
    get shareSupported() {
      return shareSupported;
    },

    switchUnit,
    chooseInput,
    applyInputPath,
    generate,
    reveal,
    share,
    loadShareSupport,
  };
}

export type Tiler = ReturnType<typeof createTiler>;
