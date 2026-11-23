# Adversarial code review — image_tiler

**Date:** 2026-11-23
**Scope:** Rust/Tauri backend + Svelte 5 frontend in this repository. Generated output (`node_modules`, `build`, `.svelte-kit`, `src-tauri/target`, `src-tauri/gen`) was out of scope.
**Basis:** `cargo test` (24 passed) and `npm run check` (0 errors, 0 warnings).

> This document describes the code **as it stood before the fixes were applied**. Fixes will move line numbers; the references below are a historical record of the reviewed snapshot, not a map of the current tree.

**Out of scope by decision:** multi-image selection / the banner of selected images that switches the main display is not part of this product iteration and is not reviewed below.

---

## What is genuinely right

The tiling/PDF core is stronger than the product surface. These parts are sound and were verified by reading the maths and the vendored `printpdf` transform code:

- **A4 "contain" geometry is correct.** `render_fixed_pages` (`src-tauri/src/tiling/pdf.rs`, ~L263–L363) computes `scale = min(total_w/img_w, total_h/img_h)`, centres with `offset_*`, and maps each page window back to source pixels correctly. Re-deriving the mm ↔ px ↔ pt chain against `printpdf`'s `get_ctms` (row-vector matrices, translation in page points, natural size `px*72/dpi`) confirms the image lands at `margin + x0*scale + offset - window_x`, and the trim marks at `insets` coincide with the core-cell edges. There is no off-by-margin shift and no crop.
- **Page order is right** (row-major, top-left first) in all three layouts, including the single-page and tile-per-page paths.
- **Seams work.** `crop_for_tile` clamps overlap without underflow. With `overlap == 0` the shared boundary is rounded from the identical continuous coordinate on both pages; with overlap the duplicate strip absorbs the ±0.5 px rounding. No lost pixels.
- **Units are consistent:** mm for margins/overlap/page size, px for tile size, DPI only for conversion. No mm/px/pt confusion was found.
- **EXIF mapping is correct for all eight orientation values** (`orientation.rs`), and orientation is applied *before* planning — the important part.
- **Startup/open-with handling is race-safe:** the frontend attaches its listener before `startup_input`, pending files are buffered, and duplicate application is guarded (`+page.svelte`, `lib.rs`).
- **Overwrite is refused by default** and surfaced as a typed error the UI turns into a confirmation.
- The two existing suites pass, and `svelte-check` is clean.

---

## HIGH

### 1. Preview cannot match the PDF in A4 mode (explicit requirement)

`src/lib/TileGridPreview.svelte`, L21–L68; `src/lib/components/PreviewSection.svelte`, L8–L20.

The preview draws red lines on the raw image at `tileWidth`/`tileHeight` in image pixels. It never shows page rectangles, the printer margin, the overlap band, the A4 page aspect, or the letterboxing the export performs (`pdf.rs` L284–L341). In A4 mode a 2×2 "preview" shows 2×2 image cells, while the PDF is 2×2 A4 sheets with 10 mm margins, 10 mm overlaps, and possibly blank bands. In tile mode the split lines match, but margins and overlap are invisible.

**Fix:** extract the layout maths into one calculation used by both sides, and render page frames + margin/overlap/core boxes (including the contain box) instead of bare lines. This is the single biggest gap versus the stated design.

### 2. Export does not ask where to save

`src/lib/tiler.svelte.ts`, L275–L289 and L232–L239; `src/lib/components/FileFields.svelte`, L20–L29.

`generate()` posts whatever `outputPath` holds; by default that is an auto-derived sibling of the source. There is a separate "Save as…" button, but the export button itself never prompts. Consequences: silent writes next to the source (which may be read-only, e.g. a mounted volume), or a relative placeholder written against the app's cwd.

**Fix:** on Generate, if the user has not chosen a path via the dialogue, call `save()` first; reject non-absolute paths.

### 3. Defaults and page options don't match the spec

`src/lib/tiler.svelte.ts`, L23–L37 and L95–L98; `src/lib/components/GridControls.svelte`, L76–L117; `src/lib/components/PrintControls.svelte`, L29–L38.

Default mode is 100 mm tiles, not A4. A4 orientation is *derived* from the image aspect rather than selectable. There is no A3 option. There is one uniform margin, not a printer-safe margin plus a user margin per side. The backend already accepts an arbitrary `page_size_mm`, so A3 and independent orientation are mostly frontend work; the margin model needs real geometry changes.

---

## MEDIUM

### 4. EXIF may desync preview and export (verify on WKWebView)

`src/lib/TileGridPreview.svelte`, L24–L25 and L85–L89; `src/lib/tiler.svelte.ts`, L123–L151; `pdf.rs`, L100–L109.

The backend plans on rotated dimensions; the frontend takes `naturalWidth/Height` from `<img>` and derives tile pixels from it. If WebKit reports unrotated dimensions (browsers have differed here), a 4000×3000 JPEG with orientation 6 gives the frontend 2×2 but the backend `plan_grid(3000,4000,2000,1500)` → 2×3, i.e. six pages, and the A4 orientation flips.

**Fix:** stop treating the `<img>` element as the source of truth. Add a Rust command returning post-orientation dimensions and use those for layout.

### 5. A4 + "Single page" silently ignores the chosen page size

`src/lib/tiler.svelte.ts`, L284; `pdf.rs`, L112–L136 and L227–L259.

The request includes `page_size_mm` whenever `sizeMode === "a4"`, but `PdfLayout::SinglePage` never reads it; the page becomes image-size-at-DPI. The UI offers a combination that quietly does something else.

**Fix:** implement "whole image contained on one A4/A3 page with margins", or reject the combination in the UI.

### 6. Preview fails outside the asset scope, and the scope is far too broad

`src-tauri/tauri.conf.json`, L20–L32; `src-tauri/capabilities/default.json`, L6–L10.

`assetProtocol.scope` lists `$HOME/**` plus a few directories. So (a) a legitimate file on `/Volumes/...`, `/tmp`, etc. exports fine but shows "Preview unavailable", and (b) any webview script can read the entire home directory, with `"csp": null` adding no defence.

**Fix:** serve previews through a command or custom protocol restricted to user-picked files (or extend the runtime scope per chosen path), and set a minimal CSP.

### 7. Preview error state is sticky

`src/lib/TileGridPreview.svelte`, L16, L85–L107.

`failed` is only cleared inside `handleLoad`, but when `failed` is true the `<img>` isn't rendered, so it can never load. One bad image (out-of-scope path, corrupt file) bricks the preview for every subsequent selection.

**Fix:** reset `failed = false` when `src` changes.

### 8. A4 mode ignores DPI but still requires it to be valid

`src/lib/tiler.svelte.ts`, L190; `src/lib/components/PrintControls.svelte`, L9–L18; `pdf.rs`, L73.

The DPI field is disabled in A4 mode, yet `canGenerate` demands `dpi > 0` unconditionally, and the backend rejects `dpi == 0` before layout even though fixed-page rendering never uses it. If DPI is ever 0 or blank, A4 export is permanently blocked with no editable control.

**Fix:** skip DPI validation for fixed-page layout.

### 9. A4 mode builds an unused tile plan; grid dimensions and page count are unbounded

`pdf.rs`, L109; `src-tauri/src/tiling/grid.rs`, L35; `pdf.rs`, L295–L363.

`tile_to_pdf` always calls `plan_grid`, and `plan_grid` allocates `columns*rows` tile records. In A4 mode those tiles are discarded, but with a large column count the tile size collapses to 1 px and the allocation becomes the whole image pixel count. The fixed-page renderer then emits `columns*rows` pages with no cap. A typo like 100000 columns hangs or OOMs the app.

**Fix:** derive `columns/rows` directly for fixed pages, skip `plan_grid`, and clamp total pages with a typed error.

### 10. Large-image memory pressure and decompression bombs

`pdf.rs`, L96–L98 and L428–L432.

The image is fully decoded, then each tile is copied by `crop_imm` and forced to RGBA, with every page's raw bitmap retained until `save`. Peak usage is roughly 2–3× the decoded image. A small compressed 30k×30k PNG reliably OOM-aborts.

**Fix:** use `image`'s `Limits` or a header dimension check with a pixel cap and a clear error; avoid `into_rgba8` when the source is already 8-bit RGB/gray and has no alpha.

### 11. Output-path failures are opaque, and writes are non-atomic

`pdf.rs`, L91–L93 and L147; `src-tauri/src/tiling/error.rs`, L7–L8.

A relative `outputPath` is resolved against the app's cwd, and the displayed reason is just `"I/O failure"` because the error display drops the underlying error and path. `std::fs::write` also leaves a truncated PDF on disk-full, and `exists()` → create is a TOCTOU window.

**Fix:** include the IO error and path in the message, write to a sibling temp file then rename, and use `create_new` when `!overwrite`.

### 12. Extreme inputs cause an arithmetic-overflow abort, not an error

`pdf.rs`, L177–L178 and L295; `src-tauri/Cargo.toml`, L44.

`tile_width + 2*overlap_px + 2*margin_px` and `columns * rows` can overflow `u32`. With `panic = "abort"` in release that kills the app instead of returning `TaskFailed`. Reachable via e.g. a 1 mm tile, DPI 4e9, and 10 mm margins.

**Fix:** bound DPI/margins/page size and use `checked_*`/`saturating_*`.

### 13. Computed page size is never checked against the PDF limit

`pdf.rs`, L253–L257.

Single-page and large-tile pages are sized as `px*25.4/dpi` mm; a large panorama at 300 DPI can exceed PDF's 14 400 pt (5080 mm) maximum and produce a file viewers reject.

**Fix:** validate page dimensions and return a typed error (or force A4 tiling).

## LOW

- **`startup_input` only reads `argv[1]`** and silently drops non-UTF-8 filenames (`commands.rs`, L88–L94). A Quick Action that passes several files uses only the first. Iterate arguments, skip flags, and report a clear error for unusable names.
- **`report` is not cleared when settings/output change** (`tiler.svelte.ts`, L39–L45 and L268–L295), so "PDF created"/Share can point at a PDF made with different settings. Clear it on any settings change.
- **Unhandled promise rejections:** `chooseInput`, `chooseOutput`, `reveal` have no `try/catch` (`tiler.svelte.ts`, L242–L261 and L322–L326); `applyInputPath`'s two `await`s can race on rapid re-picks. Low impact, but noisy.
- **`greet` is leftover scaffold** (`lib.rs`, L27–L30). Delete it.
- **Share button is shown on non-macOS** but always errors (`GeneratePanel.svelte`, L32; `commands.rs`, L122–L126). Gate it by platform.
- **`InvalidInputName` is triggered by a merely non-UTF-8 filename**, although the stem only feeds the PDF title (`pdf.rs`, L103–L107). Fall back to a constant title.
- **`switchUnit` mutates hidden values in grid/A4 modes** (`tiler.svelte.ts`, L205–L218); harmless but confusing when the user later returns to tile mode.
- **Tests never assert geometry.** They check page counts and unit conversion (`pdf/tests.rs`), but not XObject placement, MediaBox sizes, or crop-mark coordinates — exactly where seam/DPI bugs hide.

---

## Bottom line

The tiling/PDF core is in better shape than the product surface. The real risks are (a) preview-versus-export divergence, which breaks a stated invariant, (b) the missing save prompt, (c) EXIF being sourced from the wrong place, and (d) unvalidated extremes that can freeze or abort the app. Fixing 1, 2, 4, 8–11 resolves most practical exposure.
