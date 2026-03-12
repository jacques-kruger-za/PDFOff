# PDFOff

> A lightweight, fast, open-source PDF reader for Windows. Because bloatware can F off.

PDFOff is a no-nonsense PDF reader built with [Tauri v2](https://v2.tauri.app/) and [MuPDF](https://mupdf.com/). Fast to open, small to install, and focused on the features that actually matter.

---

## Why PDFOff?

Commercial PDF readers are bloated, slow, and increasingly paywalled. PDFOff aims to be the opposite: under 25MB installed, instant to open, and free forever under AGPL v3.

---

## Features (current)

- Open and render any standard PDF
- Smooth scrolling with continuous page layout
- Zoom: Ctrl+wheel, +/- buttons, or type any % directly in the toolbar
- Fit-to-width and fit-to-page modes
- Page navigation: buttons, keyboard (arrows, PgUp/PgDn, Home/End), jump-to-page
- Native Windows printing — real printer selection via Windows GDI
- Annotations: highlight, freehand ink, sticky notes (Phase 4, in progress)
- Form filling (Phase 3, in progress)

---

## Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Open file | Ctrl+O |
| Close file | Ctrl+W |
| Print | Ctrl+P |
| Zoom in | Ctrl+= |
| Zoom out | Ctrl+- |
| Fit to width | Ctrl+Shift+W |
| Fit to page | Ctrl+Shift+H |
| Next page | → / PgDn |
| Previous page | ← / PgUp |
| First page | Home |
| Last page | End |
| Jump to page | Ctrl+G |

---

## Tech Stack

| Layer | Technology |
|-------|-----------|
| App shell | Tauri v2 |
| Backend | Rust |
| PDF engine | MuPDF via `mupdf-rs` 0.6 |
| Frontend | Vanilla HTML / CSS / JS |
| Installer | NSIS (via Tauri bundler) |
| Print pipeline | Windows GDI (`windows` crate 0.58) |

---

## Development Setup

**Prerequisites**

- [Rust](https://rustup.rs) (latest stable)
- [Node.js](https://nodejs.org) LTS
- Tauri CLI: `cargo install tauri-cli`
- Visual Studio Build Tools (Windows) — C++ tools for MuPDF compilation

**Commands**

```bash
# Clone
git clone https://github.com/jacques-kruger-za/PDFOff.git
cd PDFOff

# Dev server (hot reload)
cargo tauri dev

# Production build + installer
cargo tauri build

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## Project Structure

```
PDFOff/
├── src-tauri/              # Rust backend
│   └── src/
│       ├── lib.rs          # Tauri commands (IPC surface)
│       ├── document.rs     # PDF open/close, metadata
│       ├── renderer.rs     # MuPDF rendering pipeline
│       ├── navigator.rs    # Zoom, page navigation state
│       ├── printer.rs      # Windows GDI print pipeline
│       ├── annotations.rs  # Annotation create/edit
│       ├── forms.rs        # AcroForm field handling
│       └── page_editor.rs  # Page insert/delete/rotate
├── src/                    # Frontend
│   ├── index.html
│   ├── styles/main.css
│   └── js/
│       ├── app.js          # App controller
│       ├── toolbar.js      # Toolbar + zoom input
│       ├── viewer.js       # Page rendering + scroll
│       ├── sidebar.js      # Thumbnail sidebar
│       ├── statusbar.js    # Page counter + zoom %
│       ├── annotation-tools.js
│       ├── form-overlay.js
│       └── page-manager.js
└── PDFOff-Roadmap.md       # Full design doc + known issues
```

---

## Roadmap

| Phase | Feature | Status |
|-------|---------|--------|
| v0.1 | View & Read | ✅ Complete |
| v0.2 | Print | ✅ Complete |
| v0.3 | Fill Forms | 🔧 In progress |
| v0.4 | Annotate | 🔧 In progress |
| v0.5 | Page Editing | 📋 Planned |

See [PDFOff-Roadmap.md](PDFOff-Roadmap.md) for the full design document, architecture, and known issues.

---

## License

[AGPL v3](LICENSE) — required by the MuPDF rendering engine. The entire application is open-source.
