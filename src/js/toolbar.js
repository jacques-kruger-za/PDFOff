// PDFOff — Toolbar Component
export class Toolbar {
  constructor(app) {
    this.app = app;
  }

  init() {
    document.getElementById('btn-open').addEventListener('click', () => this.app.openFile());
    document.getElementById('btn-save').addEventListener('click', () => this.app.saveFile());
    document.getElementById('btn-print').addEventListener('click', () => this.app.showPrintDialog());

    document.getElementById('btn-prev').addEventListener('click', () => this.app.prevPage());
    document.getElementById('btn-next').addEventListener('click', () => this.app.nextPage());

    document.getElementById('btn-zoom-out').addEventListener('click', () => this.app.zoomOut());
    document.getElementById('btn-zoom-in').addEventListener('click', () => this.app.zoomIn());

    document.getElementById('btn-fit-width').addEventListener('click', () => {
      this.app.setZoom(1.0); // TODO: Calculate fit-width zoom
      const { invoke } = window.__TAURI__.core;
      invoke('set_fit_mode', { mode: 'fit-width' });
    });

    document.getElementById('btn-fit-page').addEventListener('click', () => {
      const { invoke } = window.__TAURI__.core;
      invoke('set_fit_mode', { mode: 'fit-page' });
    });

    // Zoom input — commit on Enter or blur
    const zoomInput = document.getElementById('zoom-input');
    const applyZoomInput = () => {
      const raw = zoomInput.value.replace('%', '').trim();
      const pct = parseFloat(raw);
      if (!isNaN(pct) && pct >= 10 && pct <= 500) {
        this.app.setZoom(pct / 100);
      } else {
        // Revert to current zoom
        zoomInput.value = Math.round(this.app.zoom * 100) + '%';
      }
    };
    zoomInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') { e.preventDefault(); zoomInput.blur(); }
      if (e.key === 'Escape') { zoomInput.value = Math.round(this.app.zoom * 100) + '%'; zoomInput.blur(); }
    });
    zoomInput.addEventListener('blur', applyZoomInput);

    // Page input
    const pageInput = document.getElementById('page-input');
    pageInput.addEventListener('change', () => {
      const page = parseInt(pageInput.value, 10) - 1; // Convert to 0-indexed
      if (!isNaN(page) && page >= 0) {
        this.app.goToPage(page);
      }
    });
    pageInput.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') {
        pageInput.blur();
      }
    });

    // Annotation tool buttons
    document.getElementById('btn-highlight').addEventListener('click', () => {
      this.app.annotationTools.setTool('highlight');
    });
    document.getElementById('btn-sticky-note').addEventListener('click', () => {
      this.app.annotationTools.setTool('sticky-note');
    });
    document.getElementById('btn-freehand').addEventListener('click', () => {
      this.app.annotationTools.setTool('freehand');
    });

    // Undo/Redo
    document.getElementById('btn-undo').addEventListener('click', () => this.app.pageManager.undo());
    document.getElementById('btn-redo').addEventListener('click', () => this.app.pageManager.redo());
  }
}
