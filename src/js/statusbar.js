// PDFOff — Status Bar Component
export class StatusBar {
  constructor(app) {
    this.app = app;
  }

  init() {
    this.update();
  }

  update() {
    const pagEl = document.getElementById('status-page');
    const zoomEl = document.getElementById('status-zoom');
    const fileEl = document.getElementById('status-file');

    if (this.app.isDocumentOpen && this.app.metadata) {
      const page = this.app.currentPage + 1;
      const total = this.app.metadata.page_count;
      pagEl.textContent = `Page ${page} of ${total}`;
      zoomEl.textContent = `${Math.round(this.app.zoom * 100)}%`;

      const sizeKB = Math.round(this.app.metadata.file_size_bytes / 1024);
      const sizeStr = sizeKB > 1024
        ? `${(sizeKB / 1024).toFixed(1)} MB`
        : `${sizeKB} KB`;
      fileEl.textContent = `${this.app.metadata.file_name} (${sizeStr})`;
    } else {
      pagEl.textContent = 'No document';
      zoomEl.textContent = '100%';
      fileEl.textContent = '';
    }

    this.app.updateDirtyState();
  }
}
