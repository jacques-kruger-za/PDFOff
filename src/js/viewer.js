// PDFOff — Viewer Component (Page Display)
const { invoke } = window.__TAURI__.core;

export class Viewer {
  constructor(app) {
    this.app = app;
    this.pageContainer = null;
    this.welcomeScreen = null;
    this.renderedPages = new Map();
    this.viewerEl = null;
    this._pageObserver = null;
  }

  init() {
    this.pageContainer = document.getElementById('page-container');
    this.welcomeScreen = document.getElementById('welcome-screen');
    this.viewerEl = document.getElementById('viewer');
    this._setupScrollObserver();
  }

  _setupScrollObserver() {
    this._pageObserver = new IntersectionObserver((entries) => {
      let best = null;
      let bestRatio = 0;
      for (const entry of entries) {
        if (entry.intersectionRatio > bestRatio) {
          bestRatio = entry.intersectionRatio;
          best = entry.target;
        }
      }
      if (best) {
        const pageIdx = parseInt(best.dataset.page);
        if (pageIdx !== this.app.currentPage) {
          this.app.currentPage = pageIdx;
          document.getElementById('page-input').value = pageIdx + 1;
          this.app.sidebar.highlightPage(pageIdx);
          this.app.statusbar.update();
        }
      }
    }, {
      root: this.viewerEl,
      threshold: [0, 0.25, 0.5, 0.75, 1.0],
    });
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
    const totalPages = this.app.metadata.page_count;

    // Render all pages so the full document is scrollable
    const pagesToRender = Array.from({ length: totalPages }, (_, i) => i);

    // Disconnect observer before rebuilding DOM
    if (this._pageObserver) {
      this._pageObserver.disconnect();
    }

    // Remember which page was most visible so we can restore position after re-render
    const scrollTargetPage = this.app.currentPage;

    this.pageContainer.innerHTML = '';
    this.renderedPages.clear();

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

        // Observe each page for scroll-based page tracking
        if (this._pageObserver) {
          this._pageObserver.observe(wrapper);
        }
      } catch (err) {
        console.error(`Failed to render page ${pageIdx}:`, err);
        const wrapper = document.createElement('div');
        wrapper.className = 'page-wrapper';
        wrapper.innerHTML = `<div class="loading">Failed to render page ${pageIdx + 1}</div>`;
        this.pageContainer.appendChild(wrapper);
      }
    }

    // Scroll back to the page that was in view before re-render
    const currentEl = this.pageContainer.querySelector(`[data-page="${scrollTargetPage}"]`);
    if (currentEl) {
      currentEl.scrollIntoView({ behavior: 'instant', block: 'start' });
    }

    // Update page input
    const pageInput = document.getElementById('page-input');
    pageInput.value = page + 1;
    document.getElementById('page-total').textContent = `/ ${totalPages}`;
    pageInput.max = totalPages;

    // Load form fields for visible pages
    await this.app.formOverlay.renderFields(pagesToRender);
  }

  clear() {
    if (this._pageObserver) {
      this._pageObserver.disconnect();
    }
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
