// PDFOff — Annotation Tools Component (Phase 4)
const { invoke } = window.__TAURI__.core;

export class AnnotationTools {
  constructor(app) {
    this.app = app;
    this.activeTool = null;
    this.isDrawing = false;
    this.currentStrokes = [];
    this.inkColor = { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
    this.inkThickness = 2;
    this.highlightColor = { r: 1.0, g: 1.0, b: 0.0, a: 0.5 };
  }

  init() {}

  setTool(tool) {
    if (this.activeTool === tool) {
      this.deactivate();
      return;
    }

    this.activeTool = tool;
    this.updateToolButtons();

    // Enable annotation canvases
    document.querySelectorAll('.annotation-canvas').forEach(canvas => {
      canvas.classList.toggle('active', true);
      this.setupCanvasEvents(canvas);
    });
  }

  deactivate() {
    this.activeTool = null;
    this.updateToolButtons();
    document.querySelectorAll('.annotation-canvas').forEach(canvas => {
      canvas.classList.remove('active');
    });
  }

  updateToolButtons() {
    const toolMap = {
      'highlight': 'btn-highlight',
      'sticky-note': 'btn-sticky-note',
      'freehand': 'btn-freehand',
    };

    Object.entries(toolMap).forEach(([tool, btnId]) => {
      const btn = document.getElementById(btnId);
      if (btn) btn.classList.toggle('active', this.activeTool === tool);
    });
  }

  setupCanvasEvents(canvas) {
    // Remove old listeners by cloning
    const newCanvas = canvas.cloneNode(true);
    canvas.parentNode.replaceChild(newCanvas, canvas);

    if (!this.activeTool) return;

    const ctx = newCanvas.getContext('2d');
    const pageIndex = parseInt(newCanvas.dataset.page);
    let points = [];

    newCanvas.addEventListener('mousedown', (e) => {
      if (!this.activeTool) return;

      const rect = newCanvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (this.activeTool === 'freehand') {
        this.isDrawing = true;
        points = [{ x, y }];
        ctx.beginPath();
        ctx.moveTo(x, y);
        ctx.strokeStyle = `rgba(${this.inkColor.r * 255}, ${this.inkColor.g * 255}, ${this.inkColor.b * 255}, ${this.inkColor.a})`;
        ctx.lineWidth = this.inkThickness;
        ctx.lineCap = 'round';
        ctx.lineJoin = 'round';
      } else if (this.activeTool === 'sticky-note') {
        this.createStickyNote(pageIndex, x, y, newCanvas);
      } else if (this.activeTool === 'highlight') {
        this.isDrawing = true;
        points = [{ x, y }];
      }
    });

    newCanvas.addEventListener('mousemove', (e) => {
      if (!this.isDrawing) return;

      const rect = newCanvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (this.activeTool === 'freehand') {
        points.push({ x, y });
        ctx.lineTo(x, y);
        ctx.stroke();
      }
    });

    newCanvas.addEventListener('mouseup', async (e) => {
      if (!this.isDrawing) return;
      this.isDrawing = false;

      const rect = newCanvas.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (this.activeTool === 'freehand' && points.length > 1) {
        await this.saveInkAnnotation(pageIndex, points, newCanvas);
      } else if (this.activeTool === 'highlight' && points.length > 0) {
        const startX = Math.min(points[0].x, x);
        const startY = Math.min(points[0].y, y);
        const width = Math.abs(x - points[0].x);
        const height = Math.abs(y - points[0].y);

        if (width > 5 && height > 5) {
          await this.createHighlight(pageIndex, startX, startY, width, height, newCanvas);
        }
      }

      points = [];
    });
  }

  async createHighlight(pageIndex, x, y, width, height, canvas) {
    const zoom = this.app.zoom;
    const dpiScale = zoom * 144 / 72;

    try {
      await invoke('create_annotation', {
        request: {
          page_index: pageIndex,
          annotation_type: 'Highlight',
          rect: {
            x: x / dpiScale,
            y: y / dpiScale,
            width: width / dpiScale,
            height: height / dpiScale,
          },
          content: '',
          color: this.highlightColor,
          ink_strokes: null,
        },
      });
      this.app.updateDirtyState();
    } catch (err) {
      console.error('Failed to create highlight:', err);
    }
  }

  async createStickyNote(pageIndex, x, y, canvas) {
    const note = prompt('Enter note text:');
    if (!note) return;

    const zoom = this.app.zoom;
    const dpiScale = zoom * 144 / 72;

    try {
      await invoke('create_annotation', {
        request: {
          page_index: pageIndex,
          annotation_type: 'StickyNote',
          rect: {
            x: x / dpiScale,
            y: y / dpiScale,
            width: 24,
            height: 24,
          },
          content: note,
          color: this.highlightColor,
          ink_strokes: null,
        },
      });
      this.app.updateDirtyState();
    } catch (err) {
      console.error('Failed to create sticky note:', err);
    }
  }

  async saveInkAnnotation(pageIndex, points, canvas) {
    const zoom = this.app.zoom;
    const dpiScale = zoom * 144 / 72;

    // Calculate bounding rect
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const p of points) {
      minX = Math.min(minX, p.x);
      minY = Math.min(minY, p.y);
      maxX = Math.max(maxX, p.x);
      maxY = Math.max(maxY, p.y);
    }

    const scaledPoints = points.map(p => [p.x / dpiScale, p.y / dpiScale]);

    try {
      await invoke('create_annotation', {
        request: {
          page_index: pageIndex,
          annotation_type: 'FreehandInk',
          rect: {
            x: minX / dpiScale,
            y: minY / dpiScale,
            width: (maxX - minX) / dpiScale,
            height: (maxY - minY) / dpiScale,
          },
          content: '',
          color: this.inkColor,
          ink_strokes: [{
            points: scaledPoints,
            color: this.inkColor,
            thickness: this.inkThickness,
          }],
        },
      });
      this.app.updateDirtyState();
    } catch (err) {
      console.error('Failed to save ink annotation:', err);
    }
  }
}
