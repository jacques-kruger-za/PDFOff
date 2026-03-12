// PDFOff — Sidebar Component (Thumbnails)
const { invoke } = window.__TAURI__.core;

const THUMBNAIL_WIDTH = 150;

export class Sidebar {
  constructor(app) {
    this.app = app;
    this.container = null;
    this.sidebarEl = null;
    this.isVisible = true;
  }

  init() {
    this.container = document.getElementById('thumbnail-container');
    this.sidebarEl = document.getElementById('sidebar');

    document.getElementById('btn-toggle-sidebar').addEventListener('click', () => {
      this.toggle();
    });
  }

  toggle() {
    this.isVisible = !this.isVisible;
    this.sidebarEl.classList.toggle('hidden', !this.isVisible);
  }

  async loadThumbnails() {
    if (!this.app.metadata) return;
    this.container.innerHTML = '';

    const pageCount = this.app.metadata.page_count;

    for (let i = 0; i < pageCount; i++) {
      const wrapper = document.createElement('div');
      wrapper.className = 'thumbnail-wrapper';
      wrapper.dataset.page = i;

      // Placeholder
      wrapper.innerHTML = `<div class="loading"><div class="spinner"></div></div>
        <div class="thumbnail-label">${i + 1}</div>`;

      wrapper.addEventListener('click', () => {
        this.app.goToPage(i);
      });

      this.container.appendChild(wrapper);

      // Load thumbnail asynchronously
      this.loadThumbnail(i, wrapper);
    }

    this.highlightPage(this.app.currentPage);
  }

  async loadThumbnail(pageIndex, wrapper) {
    try {
      const thumb = await invoke('render_thumbnail', {
        pageIndex,
        maxWidth: THUMBNAIL_WIDTH,
      });

      const img = document.createElement('img');
      img.src = `data:image/png;base64,${thumb.image_data}`;
      img.alt = `Page ${pageIndex + 1}`;
      img.draggable = false;

      const label = wrapper.querySelector('.thumbnail-label');
      wrapper.innerHTML = '';
      wrapper.appendChild(img);
      wrapper.appendChild(label || (() => {
        const l = document.createElement('div');
        l.className = 'thumbnail-label';
        l.textContent = `${pageIndex + 1}`;
        return l;
      })());
    } catch (err) {
      console.error(`Failed to load thumbnail ${pageIndex}:`, err);
    }
  }

  highlightPage(pageIndex) {
    this.container.querySelectorAll('.thumbnail-wrapper').forEach(w => {
      w.classList.toggle('active', parseInt(w.dataset.page) === pageIndex);
    });

    // Scroll thumbnail into view
    const active = this.container.querySelector('.thumbnail-wrapper.active');
    if (active) {
      active.scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    }
  }

  clear() {
    if (this.container) {
      this.container.innerHTML = '';
    }
  }

  async refresh() {
    await this.loadThumbnails();
  }
}
