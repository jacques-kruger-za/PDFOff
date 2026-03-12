// PDFOff — Viewer Component (Page Display)
const { invoke } = window.__TAURI__.core;

export class Viewer {
  constructor(app) {
    this.app = app;
    this.pageContainer = null;
    this.welcomeScreen = null;
    this.renderedPages = new Map();
  }

  init() {
    this.pageContainer = document.getElementById('page-container');
    this.welcomeScreen = document.getElementById('welcome-screen');
  }

  async loadDocument() {
    this.clear();
    this.welcomeScreen.style.display = 'none';
    this.pageContainer.style.display = 'flex';

    await this.renderCurrentPage();
  }

  async renderCurrentPage() {
    if (!this.app.isDocumentOpen) return;

    const page = this.app.currentPage;
    const zoom = this.app.zoom;

    // Render current page and adjacent pages (lookahead cache)
    const pagesToRender = [page];
    if (page > 0) pagesToRender.unshift(page - 1);
    if (this.app.metadata && page < this.app.metadata.page_count - 1) {
      pagesToRender.push(page + 1);
    }

    this.pageContainer.innerHTML = '';

    for (const pageIdx of pagesToRender) {
      try {
        const rendered = await invoke('render_page', {
          pageIndex: pageIdx,
          zoom,
        });

        const wrapper = document.createElement('div');
        wrapper.className = 'page-wrapper';
        wrapper.dataset.page = pageIdx;

        const img = document.createElement('img');
        img.src = `data:image/png;base64,${rendered.image_data}`;
        img.alt = `Page ${pageIdx + 1}`;
        img.draggable = false;

        wrapper.appendChild(img);

        // Add form overlay container
        const formOverlay = document.createElement('div');
        formOverlay.className = 'form-overlay';
        formOverlay.dataset.page = pageIdx;
        wrapper.appendChild(formOverlay);

        // Add annotation canvas
        const annotCanvas = document.createElement('canvas');
        annotCanvas.className = 'annotation-canvas';
        annotCanvas.dataset.page = pageIdx;
        annotCanvas.width = rendered.width;
        annotCanvas.height = rendered.height;
        wrapper.appendChild(annotCanvas);

        this.pageContainer.appendChild(wrapper);
        this.renderedPages.set(pageIdx, rendered);
      } catch (err) {
        console.error(`Failed to render page ${pageIdx}:`, err);
        const wrapper = document.createElement('div');
        wrapper.className = 'page-wrapper';
        wrapper.innerHTML = `<div class="loading">Failed to render page ${pageIdx + 1}</div>`;
        this.pageContainer.appendChild(wrapper);
      }
    }

    // Scroll to current page
    const currentEl = this.pageContainer.querySelector(`[data-page="${page}"]`);
    if (currentEl) {
      currentEl.scrollIntoView({ behavior: 'instant', block: 'start' });
    }

    // Update page input
    const pageInput = document.getElementById('page-input');
    pageInput.value = page + 1;
    document.getElementById('page-total').textContent = `/ ${this.app.metadata.page_count}`;
    pageInput.max = this.app.metadata.page_count;

    // Load form fields for visible pages
    await this.app.formOverlay.renderFields(pagesToRender);
  }

  clear() {
    if (this.pageContainer) {
      this.pageContainer.innerHTML = '';
      this.pageContainer.style.display = 'none';
    }
    if (this.welcomeScreen) {
      this.welcomeScreen.style.display = 'flex';
    }
    this.renderedPages.clear();
    document.getElementById('page-input').value = 1;
    document.getElementById('page-total').textContent = '/ 0';
  }

  getPageWrapper(pageIndex) {
    return this.pageContainer?.querySelector(`.page-wrapper[data-page="${pageIndex}"]`);
  }

  getPageDimensions(pageIndex) {
    const rendered = this.renderedPages.get(pageIndex);
    if (rendered) {
      return { width: rendered.width, height: rendered.height };
    }
    return null;
  }
}
