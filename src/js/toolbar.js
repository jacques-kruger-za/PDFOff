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

    // Zoom select
    document.getElementById('zoom-select').addEventListener('change', (e) => {
      this.app.setZoom(parseFloat(e.target.value));
    });

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
