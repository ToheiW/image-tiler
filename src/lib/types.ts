export type PdfLayout = "one_tile_per_page" | "single_page";

export interface PdfRequest {
  input: string;
  output: string;
  /** Tile width in pixels (converted from mm in the UI when needed). */
  tile_width: number;
  tile_height: number;
  dpi: number;
  layout: PdfLayout;
  overlap_mm: number;
  margin_mm: number;
  /** Fixed page size in millimetres (A4 mode); omitted for exact-size pages. */
  page_size_mm?: [number, number];
  overwrite: boolean;
}

export interface PdfReport {
  layout: PdfLayout;
  columns: number;
  rows: number;
  pages: number;
  output: string;
}

export type ApiErrorKind =
  | "output_exists"
  | "invalid_tile_size"
  | "invalid_dpi"
  | "invalid_overlap"
  | "invalid_margin"
  | "invalid_page_size"
  | "page_too_small"
  | "invalid_input_name"
  | "io"
  | "image"
  | "pdf"
  | "task_failed";

export interface ApiError {
  kind: ApiErrorKind;
  message: string;
}
