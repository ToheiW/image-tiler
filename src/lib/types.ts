export type PdfLayout = "one_tile_per_page" | "single_page";
export type SizeMode = "pages" | "tile";
export type PageSize = "a4" | "a3";
export type PageOrientation = "portrait" | "landscape";

export interface MarginsMm {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

/** Exactly mirrors the Rust `LayoutSpec`; both the preview and the PDF use it. */
export interface LayoutSpec {
  mode: SizeMode;
  layout: PdfLayout;
  page_size: PageSize;
  orientation: PageOrientation;
  columns: number;
  rows: number;
  dpi: number;
  /** Tile size in source pixels (converted from mm in the UI when needed). */
  tile_width: number;
  tile_height: number;
  margins_mm: MarginsMm;
  overlap_mm: number;
}

export interface PlanRequest {
  input: string;
  spec: LayoutSpec;
}

export interface RectMm {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface RectPx {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface PagePlan {
  row: number;
  col: number;
  page_width_mm: number;
  page_height_mm: number;
  core: RectMm;
  image: RectMm;
  source: RectPx;
  dest: RectMm;
}

/** The single layout calculation the canvas preview and the PDF writer share. */
export interface LayoutPlan {
  image_width: number;
  image_height: number;
  page_width_mm: number;
  page_height_mm: number;
  columns: number;
  rows: number;
  scale_mm_per_px: number;
  effective_dpi: number;
  safe_margin_mm: number;
  margins: MarginsMm;
  overlap_mm: number;
  pages: PagePlan[];
}

export interface PdfRequest {
  input: string;
  output: string;
  spec: LayoutSpec;
  overwrite: boolean;
}

export interface PdfReport {
  columns: number;
  rows: number;
  pages: number;
  output: string;
  effective_dpi: number;
  page_width_mm: number;
  page_height_mm: number;
}

export type ApiErrorKind =
  | "output_exists"
  | "invalid_tile_size"
  | "invalid_dpi"
  | "invalid_overlap"
  | "invalid_margin"
  | "invalid_grid"
  | "page_too_small"
  | "page_too_large"
  | "empty_image"
  | "image_too_large"
  | "too_many_pages"
  | "output_path_not_absolute"
  | "io"
  | "image"
  | "pdf"
  | "task_failed";

export interface ApiError {
  kind: ApiErrorKind;
  message: string;
}
