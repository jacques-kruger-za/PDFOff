// PDFOff — Form Overlay Component (Phase 3)
const { invoke } = window.__TAURI__.core;

export class FormOverlay {
  constructor(app) {
    this.app = app;
    this.fields = [];
    this.highlightEnabled = false;
    this.hasForms = false;
  }

  init() {}

  async detectForms() {
    if (!this.app.isDocumentOpen) return;
    try {
      this.hasForms = await invoke('has_forms');
      if (this.hasForms) {
        this.fields = await invoke('get_form_fields');
      } else {
        this.fields = [];
      }
    } catch (err) {
      console.error('Form detection failed:', err);
      this.fields = [];
      this.hasForms = false;
    }
  }

  async renderFields(visiblePages) {
    if (!this.hasForms || this.fields.length === 0) return;

    for (const pageIdx of visiblePages) {
      const overlay = document.querySelector(`.form-overlay[data-page="${pageIdx}"]`);
      if (!overlay) continue;

      overlay.innerHTML = '';
      const pageFields = this.fields.filter(f => f.page_index === pageIdx);

      const dims = this.app.viewer.getPageDimensions(pageIdx);
      if (!dims) continue;

      for (const field of pageFields) {
        if (field.is_read_only) continue;

        const widget = document.createElement('div');
        widget.className = `form-field-widget ${this.highlightEnabled ? 'highlight' : ''}`;

        // Scale field rect to rendered size
        // The field rect is in PDF coordinates; we need to scale to pixel coords
        const pageInfo = this.app.viewer.renderedPages.get(pageIdx);
        if (!pageInfo) continue;

        const scaleX = dims.width / (field.rect.width > 0 ? dims.width / this.app.zoom / 2 : 1);
        const scaleY = dims.height / (field.rect.height > 0 ? dims.height / this.app.zoom / 2 : 1);

        // Position using percentage-based layout
        const zoom = this.app.zoom;
        const dpiScale = zoom * 144 / 72;
        widget.style.left = `${field.rect.x * dpiScale}px`;
        widget.style.top = `${field.rect.y * dpiScale}px`;
        widget.style.width = `${field.rect.width * dpiScale}px`;
        widget.style.height = `${field.rect.height * dpiScale}px`;

        let input;
        switch (field.field_type) {
          case 'Checkbox':
            input = document.createElement('input');
            input.type = 'checkbox';
            input.checked = field.value === 'Yes' || field.value === 'true';
            input.addEventListener('change', () => {
              this.updateField(field.page_index, field.name, input.checked ? 'Yes' : 'Off');
            });
            break;

          case 'Dropdown':
          case 'ComboBox':
            input = document.createElement('select');
            field.options.forEach(opt => {
              const option = document.createElement('option');
              option.value = opt;
              option.textContent = opt;
              if (opt === field.value) option.selected = true;
              input.appendChild(option);
            });
            input.addEventListener('change', () => {
              this.updateField(field.page_index, field.name, input.value);
            });
            break;

          default: // Text
            input = document.createElement('input');
            input.type = 'text';
            input.value = field.value;
            if (field.max_length) input.maxLength = field.max_length;
            input.placeholder = field.name;
            input.addEventListener('change', () => {
              this.updateField(field.page_index, field.name, input.value);
            });
            break;
        }

        widget.appendChild(input);
        overlay.appendChild(widget);
      }
    }
  }

  async updateField(pageIndex, fieldName, value) {
    try {
      await invoke('set_form_field', {
        update: {
          page_index: pageIndex,
          field_name: fieldName,
          value,
        },
      });
      this.app.updateDirtyState();
    } catch (err) {
      console.error('Failed to update form field:', err);
      this.app.showError(`Failed to update field: ${err}`);
    }
  }

  toggleHighlight() {
    this.highlightEnabled = !this.highlightEnabled;
    document.querySelectorAll('.form-field-widget').forEach(w => {
      w.classList.toggle('highlight', this.highlightEnabled);
    });
  }

  async clearForm() {
    if (!this.hasForms) return;
    try {
      await invoke('clear_form_fields');
      this.fields = await invoke('get_form_fields');
      await this.app.viewer.renderCurrentPage();
      this.app.updateDirtyState();
    } catch (err) {
      console.error('Failed to clear form:', err);
    }
  }

  clear() {
    this.fields = [];
    this.hasForms = false;
    document.querySelectorAll('.form-overlay').forEach(o => { o.innerHTML = ''; });
  }
}
