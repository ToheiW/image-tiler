# Image Tiler

Image Tiler prints one large image across several sheets of paper. Open an image, choose how the pages are sized, and export a PDF you can print and assemble. The preview is rendered from the same layout calculation as the PDF, so page sizes, image placement, margins, and crop marks match the print.

It supports PNG, JPEG, and WebP, and applies EXIF orientation before tiling.

## Install on Windows

Windows 10/11 x64. The app requires the Microsoft Edge WebView2 runtime. It is present on most Windows 10/11 systems, and the installers can add it if it is missing.

Download from [Releases](https://github.com/ToheiW/image-tiler/releases/latest):

| File | Use |
| --- | --- |
| `image_tiler_0.1.0_x64-setup.exe` | NSIS installer |
| `image_tiler_0.1.0_x64_en-US.msi` | MSI installer |
| `image_tiler.exe` | Standalone executable: no installation, just run it |

These builds are not code-signed, so Windows SmartScreen may warn on first launch. Choose "More info" and then "Run anyway".

## Build and run on macOS

Prerequisites:

- Node.js 20 or later
- Rust stable (the project uses edition 2024, so Rust 1.85 or later)
- Xcode Command Line Tools

Install dependencies and start the app in development:

```sh
npm install
npm run tauri dev
```

Build a macOS app bundle:

```sh
npm run tauri build
```

The app is written to `src-tauri/target/release/bundle/macos/image_tiler.app`, with a disk image under `src-tauri/target/release/bundle/dmg/`. No macOS binary is published in Releases yet; build it from source.

Checks:

```sh
npm run check
cd src-tauri && cargo test
```

## Opening an image

Use the Choose button in the app, or launch it with an image path. The app reads the first command-line argument that is an existing file:

```sh
/Applications/image_tiler.app/Contents/MacOS/image_tiler /path/to/photo.jpg
```

From a local build:

```sh
./src-tauri/target/release/image_tiler /path/to/photo.jpg
```

On macOS, double-click and "Open With" for PNG, JPEG, and WebP also work through the app's file associations. One image is edited at a time; if several paths are passed, only the first existing file is opened.

### Finder Quick Action

1. Open Automator and create a new Quick Action.
2. Set "Workflow receives current" to `files or folders` in `Finder`.
3. Add a "Run Shell Script" action, set "Pass input" to `as arguments`, and use:

```sh
"/Applications/image_tiler.app/Contents/MacOS/image_tiler" "$1"
```

4. Save the action (for example, "Tile Image"). It appears in Finder's Quick Actions menu and in the context menu under Quick Actions.

## Tiling options

### Sizing mode

- **Paper pages** - choose a real sheet size (A4 or A3) and a grid of columns and rows. The whole image is scaled to fit the combined grid and centred. It is never cropped or stretched; if the image's aspect ratio differs from the grid's, blank bands appear at the edges.
- **Exact tile size** - choose the printed size of each tile, in millimetres or in pixels at a chosen DPI. The app derives the number of columns and rows from the image size, and each page is that tile plus margins and the printer-safe band. Use this when a printed piece must be an exact size.

### Paper and orientation

A4 is 210 x 297 mm and A3 is 297 x 420 mm. Portrait and landscape are independent of the paper choice; landscape swaps width and height.

### Layout

- **One tile per page** - standard tiled output: one PDF page per grid cell.
- **Single page** - the whole image on one sheet. In paper-page mode it is fitted to the chosen sheet; in exact-tile mode the single page is sized to the whole image at the chosen DPI.

### Margins, safe band, and overlap

- Each side has its own **user margin** (0-100 mm, default 10 mm).
- A fixed **5 mm printer-safe band** is reserved outside the user margins on every page, so content and marks stay clear of the printer's non-printable edge.
- **Overlap** (0-100 mm, default 10 mm) duplicates a strip of the image along each shared page edge, so neighbouring pages can be aligned and taped before trimming. It is clamped to that side's user margin and never enters the safe band. Overlap applies only when each tile gets its own page.
- For multi-page output, short **crop marks** are printed in the margins at the tile corners.

Print the PDF at 100% ("actual size"), not "fit to page", so the tiles reassemble at the intended scale.

## Saving

Generate PDF asks where to save on every export and suggests `<image name>-tiles.pdf` next to the source image. If the chosen file already exists, the app asks before overwriting.

## Repository layout

- `src/` - Svelte 5 frontend and canvas preview
- `src-tauri/src/commands.rs` - the IPC surface
- `src-tauri/src/tiling/layout.rs` - the shared layout plan used by both the preview and the PDF
- `src-tauri/src/tiling/pdf.rs` - PDF rendering and output writing
- `.github/workflows/windows-build.yml` - Windows CI that uploads the installers and the standalone executable
