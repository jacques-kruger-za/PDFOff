// PDFOff — Main Application Entry Point
import { Toolbar } from './toolbar.js';
import { Viewer } from './viewer.js';
import { Sidebar } from './sidebar.js';
import { StatusBar } from './statusbar.js';
import { FormOverlay } from './form-overlay.js';
import { AnnotationTools } from './annotation-tools.js';
import { PageManager } from './page-manager.js';

const { invoke } = window.__TAURI__.core;

class PDFOffApp {
  constructor() {
    this.isDocumentOpen = false;
    this.metadata = null;
    this.currentPage = 0;
    this.zoom = 1.0;
    this.toolbar = new Toolbar(this);
    this.viewer = new Viewer(this);
    this.sidebar = new Sidebar(this);
    this.statusbar = new StatusBar(this);
    this.formOverlay = new FormOverlay(this);
    this.annotationTools = new AnnotationTools(this);
    this.pageManager = new PageManager(this);
  }

  async init() {
    this.toolbar.init();
    this.viewer.init();
    this.sidebar.init();
    this.statusbar.init();
    this.formOverlay.init();
    this.annotationTools.init();
    this.pageManager.init();
    this.setupMenubar();
    this.setupKeyboardShortcuts();
    this.setupDragAndDrop();
    this.setupScrollZoom();
    this.setupDialogs();
  }

  // ── Document Operations ──

  async openFile(path) {
    try {
      if (path) {
        this.metadata = await invoke('open_document', { path });
      } else {
        // Use Tauri dialog to pick a file
        const { open } = window.__TAURI__.dialog;
        const selected = await open({
          filters: [{ name: 'PDF Files', extensions: ['pdf'] }],
          multiple: false,
        });
        if (!selected) return;
        this.metadata = await invoke('open_document', { path: selected });
      }

      this.isDocumentOpen = true;
      this.currentPage = 0;
      this.zoom = 1.0;

      document.title = `${this.metadata.file_name} — PDFOff`;
      this.enableControls(true);

      await this.viewer.loadDocument();
      await this.sidebar.loadThumbnails();
      await this.formOverlay.detectForms();
      this.statusbar.update();
    } catch (err) {
      console.error('Failed to open document:', err);
      this.showError(`Failed to open PDF: ${err}`);
    }
  }

  async closeFile() {
    if (!this.isDocumentOpen) return;

    const dirty = await invoke('is_dirty');
    if (dirty) {
      if (!confirm('You have unsaved changes. Close anyway?')) return;
    }

    try {
      await invoke('close_document');
      this.isDocumentOpen = false;
      this.metadata = null;
      document.title = 'PDFOff';
      this.enableControls(false);
      this.viewer.clear();
      this.sidebar.clear();
      this.formOverlay.clear();
      this.statusbar.update();
    } catch (err) {
      console.error('Failed to close document:', err);
    }
  }

  async saveFile() {
    if (!this.isDocumentOpen) return;
    try {
      await invoke('save_document');
      this.updateDirtyState();
    } catch (err) {
      console.error('Failed to save:', err);
      this.showError(`Failed to save: ${err}`);
    }
  }

  async saveFileAs() {
    if (!this.isDocumentOpen) return;
    try {
      const { save } = window.__TAURI__.dialog;
      const path = await save({
        filters: [{ name: 'PDF Files', extensions: ['pdf'] }],
      });
      if (!path) return;
      await invoke('save_document_as', { outputPath: path });
      this.metadata = await invoke('get_metadata');
      document.title = `${this.metadata.file_name} — PDFOff`;
      this.updateDirtyState();
    } catch (err) {
      console.error('Failed to save as:', err);
      this.showError(`Failed to save: ${err}`);
    }
  }

  // ── Navigation ──

  async goToPage(page) {
    if (!this.isDocumentOpen) return;
    try {
      this.currentPage = await invoke('go_to_page', { page });
      await this.viewer.renderCurrentPage();
      this.sidebar.highlightPage(this.currentPage);
      this.statusbar.update();
    } catch (err) {
      console.error('Navigation error:', err);
    }
  }

  async nextPage() {
    if (!this.isDocumentOpen) return;
    try {
      this.currentPage = await invoke('next_page');
      await this.viewer.renderCurrentPage();
      this.sidebar.highlightPage(this.currentPage);
      this.statusbar.update();
    } catch (err) {
      console.error('Navigation error:', err);
    }
  }

  async prevPage() {
    if (!this.isDocumentOpen) return;
    try {
      this.currentPage = await invoke('prev_page');
      await this.viewer.renderCurrentPage();
      this.sidebar.highlightPage(this.currentPage);
      this.statusbar.update();
    } catch (err) {
      console.error('Navigation error:', err);
    }
  }

  async firstPage() {
    if (!this.isDocumentOpen) return;
    try {
      this.currentPage = await invoke('first_page');
      await this.viewer.renderCurrentPage();
      this.sidebar.highlightPage(this.currentPage);
      this.statusbar.update();
    } catch (err) {
      console.error('Navigation error:', err);
    }
  }

  async lastPage() {
    if (!this.isDocumentOpen) return;
    try {
      this.currentPage = await invoke('last_page');
      await this.viewer.renderCurrentPage();
      this.sidebar.highlightPage(this.currentPage);
      this.statusbar.update();
    } catch (err) {
      console.error('Navigation error:', err);
    }
  }

  // ── Zoom ──

  async zoomIn() {
    this.zoom = await invoke('zoom_in');
    this.onZoomChanged();
  }

  async zoomOut() {
    this.zoom = await invoke('zoom_out');
    this.onZoomChanged();
  }

  async setZoom(level) {
    this.zoom = await invoke('set_zoom', { level });
    this.onZoomChanged();
  }

  async onZoomChanged() {
    document.getElementById('zoom-input').value = Math.round(this.zoom * 100) + '%';
    await this.viewer.renderCurrentPage();
    this.statusbar.update();
  }

  // ── UI State ──

  enableControls(enabled) {
    const controls = [
      'btn-save', 'btn-print', 'btn-prev', 'btn-next',
      'page-input', 'btn-undo', 'btn-redo',
      'btn-highlight', 'btn-sticky-note', 'btn-freehand',
    ];
    controls.forEach(id => {
      const el = document.getElementById(id);
      if (el) el.disabled = !enabled;
    });

    // Enable menu items
    document.querySelectorAll('.menu-dropdown button[data-action]').forEach(btn => {
      const action = btn.dataset.action;
      const alwaysEnabled = ['open', 'about', 'shortcuts', 'zoom-in', 'zoom-out',
        'actual-size', 'fit-width', 'fit-page', 'toggle-sidebar'];
      if (!alwaysEnabled.includes(action)) {
        btn.disabled = !enabled;
      }
    });
  }

  async updateDirtyState() {
    if (!this.isDocumentOpen) return;
    try {
      const dirty = await invoke('is_dirty');
      const el = document.getElementById('status-dirty');
      el.style.display = dirty ? 'inline' : 'none';
      const titlePrefix = dirty ? '* ' : '';
      document.title = `${titlePrefix}${this.metadata.file_name} — PDFOff`;
    } catch (_) {}
  }

  showError(message) {
    // Simple alert for now; could be replaced with a toast
    alert(message);
  }

  // ── Menu Bar ──

  setupMenubar() {
    const menuItems = document.querySelectorAll('.menu-item');
    let activeMenu = null;

    menuItems.forEach(item => {
      const label = item.querySelector('.menu-label');
      label.addEventListener('click', (e) => {
        e.stopPropagation();
        if (activeMenu === item) {
          item.classList.remove('active');
          activeMenu = null;
        } else {
          if (activeMenu) activeMenu.classList.remove('active');
          item.classList.add('active');
          activeMenu = item;
        }
      });

      label.addEventListener('mouseenter', () => {
        if (activeMenu && activeMenu !== item) {
          activeMenu.classList.remove('active');
          item.classList.add('active');
          activeMenu = item;
        }
      });
    });

    document.addEventListener('click', () => {
      if (activeMenu) {
        activeMenu.classList.remove('active');
        activeMenu = null;
      }
    });

    // Menu actions
    document.querySelectorAll('.menu-dropdown button').forEach(btn => {
      btn.addEventListener('click', (e) => {
        e.stopPropagation();
        if (activeMenu) {
          activeMenu.classList.remove('active');
          activeMenu = null;
        }
        this.handleMenuAction(btn.dataset.action);
      });
    });
  }

  handleMenuAction(action) {
    switch (action) {
      case 'open': this.openFile(); break;
      case 'save': this.saveFile(); break;
      case 'save-as': this.saveFileAs(); break;
      case 'print': this.showPrintDialog(); break;
      case 'close': this.closeFile(); break;
      case 'zoom-in': this.zoomIn(); break;
      case 'zoom-out': this.zoomOut(); break;
      case 'actual-size': this.setZoom(1.0); break;
      case 'fit-width': invoke('set_fit_mode', { mode: 'fit-width' }); break;
      case 'fit-page': invoke('set_fit_mode', { mode: 'fit-page' }); break;
      case 'toggle-sidebar': this.sidebar.toggle(); break;
      case 'highlight-fields': this.formOverlay.toggleHighlight(); break;
      case 'clear-form': this.formOverlay.clearForm(); break;
      case 'tool-highlight': this.annotationTools.setTool('highlight'); break;
      case 'tool-underline': this.annotationTools.setTool('underline'); break;
      case 'tool-strikethrough': this.annotationTools.setTool('strikethrough'); break;
      case 'tool-sticky-note': this.annotationTools.setTool('sticky-note'); break;
      case 'tool-freehand': this.annotationTools.setTool('freehand'); break;
      case 'tool-textbox': this.annotationTools.setTool('textbox'); break;
      case 'delete-page': this.pageManager.deletePage(); break;
      case 'insert-blank': this.pageManager.insertBlankPage(); break;
      case 'rotate-cw': this.pageManager.rotatePage(90); break;
      case 'rotate-ccw': this.pageManager.rotatePage(-90); break;
      case 'merge-pdf': this.pageManager.mergePdf(); break;
      case 'extract-pages': this.pageManager.extractPages(); break;
      case 'about': this.showAboutDialog(); break;
      case 'shortcuts': this.showShortcutsDialog(); break;
    }
  }

  // ── Keyboard Shortcuts ──

  setupKeyboardShortcuts() {
    document.addEventListener('keydown', (e) => {
      // Don't intercept when typing in an input
      if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT' || e.target.tagName === 'TEXTAREA') {
        if (e.key === 'Escape') e.target.blur();
        return;
      }

      if (e.ctrlKey) {
        switch (e.key) {
          case 'o': case 'O': e.preventDefault(); this.openFile(); break;
          case 's':
            e.preventDefault();
            if (e.shiftKey) this.saveFileAs();
            else this.saveFile();
            break;
          case 'p': case 'P': e.preventDefault(); this.showPrintDialog(); break;
          case 'w': case 'W': e.preventDefault(); this.closeFile(); break;
          case 'g': case 'G':
            e.preventDefault();
            const input = document.getElementById('page-input');
            input.focus();
            input.select();
            break;
          case '=': case '+': e.preventDefault(); this.zoomIn(); break;
          case '-': e.preventDefault(); this.zoomOut(); break;
          case '0': e.preventDefault(); this.setZoom(1.0); break;
          case 'z': case 'Z': e.preventDefault(); this.pageManager.undo(); break;
          case 'y': case 'Y': e.preventDefault(); this.pageManager.redo(); break;
        }
      } else {
        switch (e.key) {
          case 'PageDown': this.nextPage(); break;
          case 'PageUp': this.prevPage(); break;
          case 'Home': this.firstPage(); break;
          case 'End': this.lastPage(); break;
          case 'ArrowRight': this.nextPage(); break;
          case 'ArrowLeft': this.prevPage(); break;
          case 'Escape': this.annotationTools.deactivate(); break;
        }
      }
    });
  }

  // ── Ctrl + Wheel Zoom ──

  setupScrollZoom() {
    const viewer = document.getElementById('viewer');
    let _zoomTimer = null;

    viewer.addEventListener('wheel', (e) => {
      if (!e.ctrlKey || !this.isDocumentOpen) return;
      e.preventDefault();

      const delta = e.deltaY < 0 ? 0.05 : -0.05;
      this.zoom = Math.min(5.0, Math.max(0.1, this.zoom + delta));

      // Update zoom display immediately
      document.getElementById('zoom-input').value = Math.round(this.zoom * 100) + '%';

      // Debounce the backend call + re-render so rapid scrolling stays smooth
      clearTimeout(_zoomTimer);
      _zoomTimer = setTimeout(async () => {
        this.zoom = await invoke('set_zoom', { level: this.zoom });
        await this.viewer.renderCurrentPage();
        this.statusbar.update();
      }, 120);
    }, { passive: false });
  }

  // ── Drag and Drop ──

  setupDragAndDrop() {
    const viewer = document.getElementById('viewer');

    viewer.addEventListener('dragover', (e) => {
      e.preventDefault();
      viewer.classList.add('drag-over');
    });

    viewer.addEventListener('dragleave', () => {
      viewer.classList.remove('drag-over');
    });

    viewer.addEventListener('drop', async (e) => {
      e.preventDefault();
      viewer.classList.remove('drag-over');

      // In Tauri v2, file drop events are handled via the window event listener
      // For now, we handle the basic case
      if (e.dataTransfer.files.length > 0) {
        const file = e.dataTransfer.files[0];
        if (file.name.toLowerCase().endsWith('.pdf')) {
          await this.openFile(file.path || file.name);
        }
      }
    });

    // Tauri file drop event
    if (window.__TAURI__?.event) {
      window.__TAURI__.event.listen('tauri://drag-drop', async (event) => {
        const paths = event.payload.paths;
        if (paths && paths.length > 0 && paths[0].toLowerCase().endsWith('.pdf')) {
          await this.openFile(paths[0]);
        }
      });
    }
  }

  // ── Dialogs ──

  setupDialogs() {
    // Print dialog
    document.getElementById('print-cancel')?.addEventListener('click', () => {
      document.getElementById('print-dialog').style.display = 'none';
    });
    document.getElementById('print-confirm')?.addEventListener('click', () => {
      this.executePrint();
    });
    document.getElementById('print-range-type')?.addEventListener('change', (e) => {
      const custom = document.getElementById('print-range-custom');
      custom.style.display = e.target.value === 'custom' ? 'block' : 'none';
    });

    // About dialog
    document.getElementById('about-close')?.addEventListener('click', () => {
      document.getElementById('about-dialog').style.display = 'none';
    });

    // Shortcuts dialog
    document.getElementById('shortcuts-close')?.addEventListener('click', () => {
      document.getElementById('shortcuts-dialog').style.display = 'none';
    });

    // Close modals on backdrop click
    document.querySelectorAll('.modal').forEach(modal => {
      modal.addEventListener('click', (e) => {
        if (e.target === modal) modal.style.display = 'none';
      });
    });
  }

  showPrintDialog() {
    if (!this.isDocumentOpen) return;
    invoke('get_printers').then(([printers, defaultPrinter]) => {
      const sel = document.getElementById('print-printer');
      if (sel) {
        sel.innerHTML = printers.length
          ? printers.map(p =>
              `<option value="${p}"${p === defaultPrinter ? ' selected' : ''}>${p}</option>`
            ).join('')
          : '<option value="">No printers found</option>';
      }
    }).catch(() => {});
    document.getElementById('print-dialog').style.display = 'flex';
  }

  showAboutDialog() {
    document.getElementById('about-dialog').style.display = 'flex';
  }

  showShortcutsDialog() {
    document.getElementById('shortcuts-dialog').style.display = 'flex';
  }

  async executePrint() {
    const printerName = document.getElementById('print-printer')?.value;
    if (!printerName) {
      this.showError('No printer selected.');
      return;
    }

    const rangeType = document.getElementById('print-range-type').value;
    const dpi = parseFloat(document.getElementById('print-dpi').value) || 300;
    const copies = parseInt(document.getElementById('print-copies').value, 10) || 1;

    let pages;
    try {
      if (rangeType === 'all') {
        pages = Array.from({ length: this.metadata.page_count }, (_, i) => i);
      } else if (rangeType === 'current') {
        pages = [this.currentPage];
      } else {
        const rangeStr = document.getElementById('print-range-custom').value.trim();
        pages = await invoke('parse_print_range', {
          rangeStr,
          totalPages: this.metadata.page_count,
        });
      }

      await invoke('execute_print', { printerName, pages, copies, dpi });
      document.getElementById('print-dialog').style.display = 'none';
    } catch (err) {
      console.error('Print failed:', err);
      this.showError(`Print failed: ${err}`);
    }
  }
}

// ── Initialize ──
const app = new PDFOffApp();
document.addEventListener('DOMContentLoaded', () => app.init());
