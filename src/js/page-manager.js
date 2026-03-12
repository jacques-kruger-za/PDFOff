// PDFOff — Page Manager Component (Phase 5)
const { invoke } = window.__TAURI__.core;

export class PageManager {
  constructor(app) {
    this.app = app;
  }

  init() {}

  async deletePage() {
    if (!this.app.isDocumentOpen) return;
    if (this.app.metadata.page_count <= 1) {
      this.app.showError('Cannot delete the last page.');
      return;
    }

    const page = this.app.currentPage;
    if (!confirm(`Delete page ${page + 1}?`)) return;

    try {
      await invoke('delete_pdf_page', { pageIndex: page });
      this.app.metadata = await invoke('get_metadata');

      if (this.app.currentPage >= this.app.metadata.page_count) {
        this.app.currentPage = this.app.metadata.page_count - 1;
      }

      await this.app.viewer.renderCurrentPage();
      await this.app.sidebar.refresh();
      this.app.statusbar.update();
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Failed to delete page:', err);
      this.app.showError(`Failed to delete page: ${err}`);
    }
  }

  async insertBlankPage() {
    if (!this.app.isDocumentOpen) return;

    try {
      await invoke('insert_blank_page', {
        afterPage: this.app.currentPage,
        width: 612.0,
        height: 792.0,
      });
      this.app.metadata = await invoke('get_metadata');
      await this.app.viewer.renderCurrentPage();
      await this.app.sidebar.refresh();
      this.app.statusbar.update();
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Failed to insert blank page:', err);
      this.app.showError(`Failed to insert page: ${err}`);
    }
  }

  async rotatePage(degrees) {
    if (!this.app.isDocumentOpen) return;

    try {
      await invoke('rotate_page', {
        pageIndex: this.app.currentPage,
        degrees,
      });
      await this.app.viewer.renderCurrentPage();
      await this.app.sidebar.refresh();
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Failed to rotate page:', err);
      this.app.showError(`Failed to rotate page: ${err}`);
    }
  }

  async mergePdf() {
    if (!this.app.isDocumentOpen) return;

    try {
      const { open } = window.__TAURI__.dialog;
      const selected = await open({
        filters: [{ name: 'PDF Files', extensions: ['pdf'] }],
        multiple: false,
      });
      if (!selected) return;

      const insertAt = this.app.metadata.page_count;
      const count = await invoke('merge_document', {
        sourcePath: selected,
        insertAt,
      });

      this.app.metadata = await invoke('get_metadata');
      await this.app.viewer.renderCurrentPage();
      await this.app.sidebar.refresh();
      this.app.statusbar.update();
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Failed to merge PDF:', err);
      this.app.showError(`Failed to merge PDF: ${err}`);
    }
  }

  async extractPages() {
    if (!this.app.isDocumentOpen) return;

    const rangeStr = prompt('Enter page numbers to extract (e.g., 1-3, 5, 7):');
    if (!rangeStr) return;

    try {
      const pages = await invoke('parse_print_range', {
        rangeStr,
        totalPages: this.app.metadata.page_count,
      });

      const { save } = window.__TAURI__.dialog;
      const outputPath = await save({
        filters: [{ name: 'PDF Files', extensions: ['pdf'] }],
        defaultPath: 'extracted.pdf',
      });
      if (!outputPath) return;

      await invoke('extract_pages', { pages, outputPath });
    } catch (err) {
      console.error('Failed to extract pages:', err);
      this.app.showError(`Failed to extract pages: ${err}`);
    }
  }

  async undo() {
    if (!this.app.isDocumentOpen) return;
    try {
      const result = await invoke('undo_page_edit');
      if (result) {
        this.app.metadata = await invoke('get_metadata');
        await this.app.viewer.renderCurrentPage();
        await this.app.sidebar.refresh();
        this.app.statusbar.update();
      }
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Undo failed:', err);
    }
  }

  async redo() {
    if (!this.app.isDocumentOpen) return;
    try {
      const result = await invoke('redo_page_edit');
      if (result) {
        this.app.metadata = await invoke('get_metadata');
        await this.app.viewer.renderCurrentPage();
        await this.app.sidebar.refresh();
        this.app.statusbar.update();
      }
      this.updateUndoRedoButtons();
    } catch (err) {
      console.error('Redo failed:', err);
    }
  }

  async updateUndoRedoButtons() {
    try {
      const undoBtn = document.getElementById('btn-undo');
      const redoBtn = document.getElementById('btn-redo');
      undoBtn.disabled = !(await invoke('can_undo'));
      redoBtn.disabled = !(await invoke('can_redo'));
    } catch (_) {}
  }
}
