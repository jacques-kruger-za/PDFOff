# PDFOff — Project Roadmap & Design Document

> A lightweight, fast, open-source PDF reader for Windows. Because bloatware can F off.

---

## 1. Project Overview

### Motivation
Commercial PDF readers (Adobe Acrobat, Foxit, etc.) are bloated, slow, and increasingly paywalled. PDFOff is a focused alternative: fast to open, small to install, and capable of the features that actually matter.

### Name
**PDFOff** — a playful nod to telling bloatware where to go.

### License
**AGPL v3** — required by the MuPDF rendering engine. The entire application is open-source.

### Platform
**Windows only** (Windows 10/11). No cross-platform requirement.

---

## 2. Technology Stack

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **App shell** | [Tauri v2](https://v2.tauri.app/) | Window management, system integration, installer, file associations |
| **Backend** | Rust | PDF rendering pipeline, file I/O, PDF manipulation |
| **PDF engine** | [MuPDF](https://mupdf.com/) via `mupdf-rs` crate | Rendering, form handling, annotations, page manipulation |
| **Frontend** | HTML / CSS / vanilla JS | UI: toolbar, viewer, sidebar, dialogs |
| **Installer** | NSIS (via Tauri bundler) | `.exe` installer with file association, Start Menu, Add/Remove Programs |

### Why These Choices

- **Tauri over Electron**: ~10-20MB installed vs ~80-150MB. Native Windows integration. Rust backend aligns with our language choice.
- **MuPDF over PDF.js/PDFium**: Fastest rendering engine. Native support for forms, annotations, and page manipulation — covers all 5 roadmap features. AGPL license is acceptable.
- **Vanilla JS over React/Vue**: The frontend is a thin UI layer over Rust commands. No need for a framework. Keeps the build simple and the bundle small.
- **Rust**: Learning opportunity + perfect pairing with Tauri. Handles all performance-critical work.

---

## 3. Architecture

### High-Level Structure

```
┌─────────────────────────────────────────┐
│           Frontend (Webview)            │
│                                         │
│  ┌─────────┐ ┌──────────┐ ┌─────────┐  │
│  │ Toolbar  │ │  Viewer  │ │ Sidebar │  │
│  │ & Menus  │ │ (canvas) │ │ (thumb) │  │
│  └─────────┘ └──────────┘ └─────────┘  │
│                                         │
│         Tauri invoke() commands         │
├─────────────────────────────────────────┤
│           Backend (Rust)                │
│                                         │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│  │ MuPDF    │ │ Document │ │ Print   │ │
│  │ Renderer │ │ Manager  │ │ Handler │ │
│  └──────────┘ └──────────┘ └─────────┘ │
│                                         │
│  ┌──────────┐ ┌──────────┐ ┌─────────┐ │
│  │ Form     │ │Annotation│ │ Page    │ │
│  │ Handler  │ │ Handler  │ │ Editor  │ │
│  └──────────┘ └──────────┘ └─────────┘ │
├─────────────────────────────────────────┤
│           Tauri Shell                   │
│  Window management, file dialogs,       │
│  file associations, system tray         │
└─────────────────────────────────────────┘
```

### Core Data Flow — Viewing a PDF

1. User opens a PDF (file dialog, double-click `.pdf`, or drag-and-drop)
2. Tauri passes the file path to the Rust backend
3. Rust opens the file via MuPDF, extracts metadata (page count, title, bookmarks)
4. Frontend requests page N at zoom level Z
5. Rust renders the page to a PNG bitmap via MuPDF
6. Image data is sent to the frontend (base64 or binary via Tauri IPC)
7. Frontend displays the image in a scrollable container
8. Pages are rendered on-demand with a small lookahead cache (current ± 2 pages)

### Data Flow — Manipulation (Phases 3-5)

1. User performs an action in the frontend (fill form field, add highlight, delete page)
2. Frontend sends a structured command via `invoke()` to Rust backend
3. Rust modifies the in-memory PDF document via MuPDF API
4. Changes are tracked as "dirty" state (unsaved indicator in title bar)
5. On save/save-as, Rust writes the modified PDF to disk

### Rust Backend Modules

| Module | Responsibility | Phase |
|--------|---------------|-------|
| `document` | Open/close PDF, metadata, state management | 1 |
| `renderer` | Page-to-image rendering, zoom, caching | 1 |
| `navigator` | Page navigation, scroll position tracking | 1 |
| `printer` | Windows print pipeline, page range, scaling | 2 |
| `forms` | Read/write AcroForm fields | 3 |
| `annotations` | Create/edit/delete annotations | 4 |
| `page_editor` | Insert, delete, reorder, merge, extract pages | 5 |

### Frontend Components

| Component | Responsibility | Phase |
|-----------|---------------|-------|
| `toolbar` | Buttons: open, zoom, print, navigation | 1 |
| `menubar` | File, View, Tools, Help menus | 1 |
| `viewer` | Main content area, page display, scroll | 1 |
| `sidebar` | Page thumbnails, bookmarks (future) | 1 |
| `statusbar` | Page number, zoom %, file info | 1 |
| `form-overlay` | Interactive form field widgets | 3 |
| `annotation-tools` | Annotation toolbar and canvas overlay | 4 |
| `page-manager` | Drag-to-reorder, merge dialog | 5 |

---

## 4. Feature Roadmap

### Phase 1 — View & Read (v0.1) ★ MVP

**Goal:** Open and read any PDF comfortably. This alone replaces Adobe for basic use.

#### Features
- **Open PDF**: File > Open dialog, double-click `.pdf` (file association), drag-and-drop onto window
- **Page rendering**: MuPDF renders pages to bitmaps, displayed in scrollable viewer
- **Zoom**: Zoom in/out (Ctrl+=/Ctrl+-), fit-to-width, fit-to-page, actual size (100%)
- **Navigation**:
  - Scroll through pages continuously
  - Previous/next page buttons
  - Jump to page (Ctrl+G)
  - Page thumbnails in collapsible sidebar
- **Keyboard shortcuts**:
  - Arrow keys, PgUp/PgDn for scrolling
  - Home/End for first/last page
  - Ctrl+O to open, Ctrl+W to close
- **UI layout**:
  - Menu bar: File, View, Help
  - Toolbar: open, zoom controls, page navigation, fit modes
  - Status bar: current page / total pages, zoom percentage
  - Classic Windows/Office aesthetic: light theme, familiar iconography, clean lines
- **Performance**:
  - Render on demand (current page + 2 pages lookahead)
  - Lazy thumbnail generation
- **Remember position**: Reopen at last-viewed page per file
- **Installer**: NSIS `.exe` installer with:
  - `.pdf` file association (register as "Open with" option, optionally set as default)
  - Start Menu shortcut
  - Desktop shortcut (optional)
  - Add/Remove Programs entry
  - Uninstaller

#### Acceptance Criteria
- Can open any standard PDF and render it correctly
- Zoom and navigation feel responsive
- Double-clicking a `.pdf` file opens it in PDFOff
- Installed size under 25MB

---

### Phase 2 — Print (v0.2)

**Goal:** Print PDFs with the standard Windows print experience.

#### Features
- **Print dialog**: Trigger via Ctrl+P or File > Print
- **Windows native print dialog**: Printer selection, copies, collation
- **Page range**: All pages, current page, custom range (e.g., "1-3, 7, 12-15")
- **Scaling options**: Fit to page, actual size, custom percentage
- **Orientation**: Auto-detect from PDF page, with manual override

#### Technical Approach
- MuPDF renders pages at print DPI (300-600)
- Pass rendered bitmaps to Windows print API via Rust
- Use `windows-rs` crate for native print dialog integration

#### Acceptance Criteria
- Prints correctly to any installed printer
- Page range and scaling work as expected
- Print quality matches or exceeds Adobe Reader

---

### Phase 3 — Fill Forms (v0.3)

**Goal:** Fill in PDF forms (tax forms, applications, etc.) and save them.

#### Features
- **Detect form fields**: Auto-detect AcroForm fields when opening a PDF
- **Supported field types**: Text input, checkbox, radio button, dropdown/combobox, date fields
- **Interactive filling**: Click on a field to activate it, type to fill, Tab to next field
- **Visual indicators**: Highlight fillable fields (subtle blue overlay, toggleable)
- **Save**: Save filled form (Ctrl+S), Save As (Ctrl+Shift+S)
- **Clear form**: Reset all fields to empty
- **Form validation**: Respect field-level validation rules embedded in the PDF (max length, format)

#### Technical Approach
- MuPDF reads AcroForm field definitions (position, type, constraints)
- Frontend renders interactive widgets overlaid on the page at correct positions
- On save, frontend sends field values to Rust backend
- Rust writes values into the PDF via MuPDF's form API

#### Acceptance Criteria
- Can fill and save standard government/tax forms
- Field positions align correctly at all zoom levels
- Saved forms open correctly in other PDF readers

---

### Phase 4 — Annotate (v0.4)

**Goal:** Mark up PDFs with highlights, notes, and drawings.

#### Features
- **Highlight text**: Select text, apply highlight in chosen color (yellow, green, blue, pink)
- **Underline / strikethrough**: Text markup annotations
- **Sticky notes**: Click to place a note icon, click to open/edit text
- **Freehand ink**: Draw with mouse/pen, configurable color and thickness
- **Text box**: Place a text annotation anywhere on the page
- **Annotation list**: Sidebar or panel listing all annotations with jump-to navigation
- **Edit/delete**: Click any annotation to modify or remove it
- **Save**: Annotations are saved into the PDF standard annotation layer

#### Technical Approach
- Frontend handles annotation creation UX (selection, drawing, placement)
- Annotation data (type, position, content, style) sent to Rust backend
- MuPDF creates/modifies standard PDF annotation objects
- Annotations are interoperable with other PDF readers

#### Acceptance Criteria
- Annotations render correctly in PDFOff and in Adobe Reader
- Freehand drawing feels responsive (< 16ms latency)
- Can annotate, save, close, reopen, and see annotations preserved

---

### Phase 5 — Page-Level Editing (v0.5)

**Goal:** Reorganize PDFs — add, remove, reorder pages and combine documents.

#### Features
- **Thumbnail sidebar (enhanced)**: Full drag-and-drop reordering of pages
- **Delete pages**: Select one or more thumbnails, delete (with undo)
- **Insert blank page**: Add a blank page before/after any page
- **Rotate pages**: 90° clockwise/counterclockwise per page
- **Merge documents**:
  - File > Merge or drag-and-drop a second PDF onto the sidebar
  - Choose insertion point
  - Interleave pages from multiple sources
- **Extract pages**: Select pages, save as new PDF
- **Undo/redo**: For all page-level operations in the current session

#### Technical Approach
- MuPDF's document manipulation API handles all page operations natively
- Thumbnail sidebar becomes the primary interaction surface
- Operations modify the in-memory document; saved on explicit save
- Merge loads a second document and copies pages into the working document

#### Acceptance Criteria
- Can reorder, delete, and insert pages via drag-and-drop
- Merged documents maintain formatting, forms, and annotations
- Extract produces a valid, independent PDF
- Undo works for all page operations

---

## 5. Future Ideas (Unprioritized)

These are potential additions beyond the core 5 phases:

| Feature | Description | Complexity |
|---------|-------------|------------|
| **Dark mode** | Dark theme toggle, respects Windows system setting | Low |
| **Tabs** | Open multiple PDFs in tabs within one window | Medium |
| **Search (Ctrl+F)** | Full-text search within the document with highlighting | Medium |
| **Bookmarks/TOC** | Navigate via document's table of contents | Low |
| **Recent files** | File > Recent menu with last 10 opened files | Low |
| **Digital signatures** | View and verify digital signatures | Medium |
| **Text selection & copy** | Select text, copy to clipboard | Medium |
| **Redaction** | Permanently remove content from PDF | Medium |
| **Command-line interface** | `pdfoff file.pdf` opens from terminal | Low |
| **Auto-update** | Check for and install updates | Medium |
| **Portable mode** | Run from USB without installation | Low |

---

## 6. Technical Reference

### MuPDF Capabilities Used

| Capability | MuPDF API | Used In |
|-----------|-----------|---------|
| Open/close document | `fz_open_document` | Phase 1 |
| Render page to pixmap | `fz_new_pixmap_from_page` | Phase 1 |
| Get page count | `fz_count_pages` | Phase 1 |
| Get page dimensions | `fz_bound_page` | Phase 1 |
| Print rendering | Render at high DPI | Phase 2 |
| Read form fields | `pdf_widget` API | Phase 3 |
| Write form values | `pdf_set_widget_value` | Phase 3 |
| Create annotations | `pdf_create_annot` | Phase 4 |
| Modify annotations | `pdf_set_annot_*` | Phase 4 |
| Delete/insert pages | `pdf_delete_page`, `pdf_graft_page` | Phase 5 |
| Save document | `pdf_save_document` | Phases 3-5 |

### Rust Crates (Expected)

| Crate | Purpose |
|-------|---------|
| `tauri` | App framework, IPC, window management |
| `mupdf` (mupdf-rs) | MuPDF Rust bindings |
| `serde` / `serde_json` | Serialization for IPC |
| `windows-rs` | Windows API (print dialog, file associations) |
| `dirs` | Platform directories for config/cache |
| `image` | Image format conversion if needed |

### Tauri Installer Configuration (NSIS)

```
- File association: .pdf → PDFOff
- Registry: HKCU\Software\Classes\.pdf
- Start Menu: PDFOff shortcut
- Add/Remove Programs: PDFOff entry with uninstaller
- Install size: target < 25MB
```

---

## 7. Development Setup

### Prerequisites
- **Rust** (latest stable) — https://rustup.rs
- **Node.js** (LTS) — for Tauri frontend build tooling
- **Tauri CLI** — `cargo install tauri-cli`
- **Visual Studio Build Tools** (Windows) — C++ build tools for MuPDF compilation
- **Git** — version control

### Project Structure (Expected)

```
PDFOff/
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── main.rs         # Tauri entry point
│   │   ├── document.rs     # PDF document management
│   │   ├── renderer.rs     # MuPDF rendering pipeline
│   │   ├── navigator.rs    # Page navigation state
│   │   ├── printer.rs      # Print pipeline (Phase 2)
│   │   ├── forms.rs        # Form handling (Phase 3)
│   │   ├── annotations.rs  # Annotation handling (Phase 4)
│   │   └── page_editor.rs  # Page manipulation (Phase 5)
│   ├── Cargo.toml
│   └── tauri.conf.json     # Tauri config (window, bundler, file associations)
├── src/                    # Frontend (HTML/CSS/JS)
│   ├── index.html
│   ├── styles/
│   │   └── main.css
│   └── js/
│       ├── app.js          # Main app logic
│       ├── toolbar.js      # Toolbar component
│       ├── viewer.js       # Page viewer component
│       ├── sidebar.js      # Thumbnail sidebar
│       └── statusbar.js    # Status bar component
├── icons/                  # App icons (various sizes)
├── PDFOff-Roadmap.md       # This document
├── LICENSE                 # AGPL v3
├── .gitignore
└── README.md               # Quick start guide (created later)
```

### Build & Run Commands

```bash
# Development (hot reload)
cargo tauri dev

# Production build (creates installer)
cargo tauri build

# Run tests
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 8. Design Decisions Log

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Platform | Windows only | User requirement; simplifies development |
| App framework | Tauri v2 | Light (~10-20MB), Rust backend, native installer |
| PDF engine | MuPDF | Fastest rendering, full feature coverage for all 5 phases |
| License | AGPL v3 | Required by MuPDF; user accepts open-source |
| Frontend tech | Vanilla HTML/CSS/JS | Simple UI layer; no framework overhead needed |
| UI style | Clean + classic Windows | Content-first, familiar toolbar/menu/statusbar layout |
| Rendering approach | Backend (Rust) → bitmap → frontend | MuPDF runs in Rust; images sent to webview for display |
| Installer | NSIS via Tauri | .exe installer with file associations, Start Menu, uninstall |

---

*Document created: 2026-03-12*
*Status: Design complete — ready for Phase 1 implementation*
