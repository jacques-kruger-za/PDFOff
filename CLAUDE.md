# PDFOff — Claude Code Instructions

Tauri v2 PDF reader for Windows. Rust backend + vanilla HTML/CSS/JS frontend. Lightweight is the core design goal — keep all changes minimal and purposeful.

---

## Commands

```bash
# Dev server
cargo tauri dev

# Production build
cargo tauri build

# Tests (Rust only — run before every commit)
cargo test --manifest-path src-tauri/Cargo.toml
```

All 29 tests must pass before committing.

---

## Architecture

**Frontend → Backend** via Tauri `invoke()` commands. All PDF logic lives in Rust; the frontend is a thin display layer.

**Rendering flow:** MuPDF (Rust) renders pages to PNG bitmaps → base64 → displayed as `<img>` tags in the viewer DOM.

**Coordinate system:** PDF origin is bottom-left. Canvas/DOM origin is top-left. Always apply Y-flip when converting canvas coordinates to PDF coordinates:
```js
const pdfY = (pageHeightPx - canvasY - height) / dpiScale;
```

---

## Key Files

| File | Purpose |
|------|---------|
| `src-tauri/src/lib.rs` | All Tauri command handlers — the IPC surface |
| `src-tauri/src/renderer.rs` | MuPDF page rendering, zoom, print render |
| `src-tauri/src/printer.rs` | Windows GDI print pipeline |
| `src-tauri/src/annotations.rs` | Annotation create/modify (MuPDF) |
| `src-tauri/src/navigator.rs` | Zoom state, page navigation |
| `src/js/app.js` | Main app controller, state, dialog logic |
| `src/js/viewer.js` | Page rendering loop, IntersectionObserver |
| `src/js/toolbar.js` | Toolbar event wiring, zoom input |
| `src/js/annotation-tools.js` | Annotation drag/draw UI, coordinate conversion |

---

## Rust Notes

**MuPDF annotation color** — import with alias to avoid clash with local `AnnotationColor` struct:
```rust
use mupdf::color::AnnotationColor as MupdfAnnotationColor;
```

**Windows GDI print functions** (`StartDocW`, `StartPage`, `EndPage`, `EndDoc`, `DOCINFOW`) are not exposed in `windows` crate 0.58's module structure. They are declared directly via `extern "system"` in `printer.rs`. Do not try to import them from `windows::Win32::Graphics::Gdi` or `windows::Win32::Graphics::Printing` — they won't resolve.

**`GetDefaultPrinterW`** returns `BOOL`, not `Result`. Use `.as_bool()`, not `.is_ok()`.

**`EndDoc(hdc)`** ends a GDI print job. `EndDocPrinter` is for spooler handles — wrong function for DC-based printing.

**Cargo.toml** — windows crate is under `[target.'cfg(windows)'.dependencies]`. All `#[cfg(windows)]` guards in printer.rs must match.

---

## Frontend Notes

**All pages are rendered upfront** (not just ±1). IntersectionObserver tracks which page is visible for the page counter. This avoids the scroll-stops-at-page-N bug.

**Zoom re-render** — save `scrollTargetPage` before clearing the DOM, then `scrollIntoView` after re-render to preserve position.

**Zoom input** — `#zoom-input` is a text `<input>` showing "105%" etc. Not a `<select>`. Updated by both toolbar buttons and Ctrl+wheel handler.

**Ctrl+wheel zoom** — 5% increments, 120ms debounce before invoking Rust and re-rendering.

---

## Known Quirks

- Highlight annotations may not render in external readers — `mupdf-rs` 0.6 has no `set_quad_points()`. Workaround: FreeText/Square annotation as fallback (not yet implemented).
- Sticky note text stored via `set_author()` — no `set_contents()` in mupdf 0.6.
- Print DPI is passed from the dialog; default 300 DPI. High DPI (600) is slow on large documents.

---

## Design Principles

- **Lightweight first** — no frameworks, no bloat. Vanilla JS. Rust where it matters.
- **Windows native** — use Windows APIs (GDI, NSIS) rather than cross-platform abstractions.
- **No over-engineering** — don't add error handling for impossible cases, don't abstract for one-off operations.
- **Commit per logical unit** — each bug fix or feature gets its own commit with a clear message.

---

## Roadmap Reference

See [PDFOff-Roadmap.md](PDFOff-Roadmap.md) for the full design doc, feature phases, known issues table, and architecture diagrams.
