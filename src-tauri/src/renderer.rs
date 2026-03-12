use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use base64::Engine;
use mupdf::{Colorspace, Matrix};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

const DEFAULT_DPI: f32 = 144.0;
const PDF_DPI: f32 = 72.0;
const MAX_CACHE_ENTRIES: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct RenderedPage {
    pub page_index: u32,
    pub image_data: String, // base64 PNG
    pub width: u32,
    pub height: u32,
    pub zoom: f32,
}

#[derive(Debug, Hash, Eq, PartialEq, Clone)]
struct CacheKey {
    page_index: u32,
    zoom_percent: u32,
}

pub struct Renderer {
    cache: Mutex<HashMap<CacheKey, RenderedPage>>,
}

impl Renderer {
    pub fn new() -> Self {
        Self {
            cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn render_page(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        zoom: f32,
    ) -> Result<RenderedPage> {
        let zoom_percent = (zoom * 100.0) as u32;
        let cache_key = CacheKey {
            page_index,
            zoom_percent,
        };

        // Check cache
        {
            let cache = self.cache.lock();
            if let Some(cached) = cache.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        let rendered = doc_manager.with_document(|doc| {
            if page_index >= doc.metadata.page_count {
                return Err(PdfOffError::InvalidPage(
                    page_index,
                    doc.metadata.page_count,
                ));
            }

            let page = doc
                .document
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let scale = zoom * DEFAULT_DPI / PDF_DPI;
            let matrix = Matrix::new_scale(scale, scale);

            let pixmap = page
                .to_pixmap(&matrix, &Colorspace::device_rgb(), 1.0, true)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let png_data = pixmap
                .to_png()
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_data);

            Ok(RenderedPage {
                page_index,
                image_data: base64_data,
                width: pixmap.width() as u32,
                height: pixmap.height() as u32,
                zoom,
            })
        })?;

        // Store in cache
        {
            let mut cache = self.cache.lock();
            if cache.len() >= MAX_CACHE_ENTRIES {
                // Simple eviction: remove entries not near current page
                let keys_to_remove: Vec<CacheKey> = cache
                    .keys()
                    .filter(|k| {
                        (k.page_index as i32 - page_index as i32).unsigned_abs() > 3
                    })
                    .cloned()
                    .collect();
                for key in keys_to_remove {
                    cache.remove(&key);
                }
                // If still too full, clear everything
                if cache.len() >= MAX_CACHE_ENTRIES {
                    cache.clear();
                }
            }
            cache.insert(cache_key, rendered.clone());
        }

        Ok(rendered)
    }

    pub fn render_thumbnail(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        max_width: u32,
    ) -> Result<RenderedPage> {
        doc_manager.with_document(|doc| {
            if page_index >= doc.metadata.page_count {
                return Err(PdfOffError::InvalidPage(
                    page_index,
                    doc.metadata.page_count,
                ));
            }

            let page = doc
                .document
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let bounds = page
                .bounds()
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let page_width = bounds.x1 - bounds.x0;
            let scale = max_width as f32 / page_width;
            let matrix = Matrix::new_scale(scale, scale);

            let pixmap = page
                .to_pixmap(&matrix, &Colorspace::device_rgb(), 1.0, true)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let png_data = pixmap
                .to_png()
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let base64_data = base64::engine::general_purpose::STANDARD.encode(&png_data);

            Ok(RenderedPage {
                page_index,
                image_data: base64_data,
                width: pixmap.width() as u32,
                height: pixmap.height() as u32,
                zoom: scale,
            })
        })
    }

    pub fn render_for_print(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        dpi: f32,
    ) -> Result<Vec<u8>> {
        doc_manager.with_document(|doc| {
            if page_index >= doc.metadata.page_count {
                return Err(PdfOffError::InvalidPage(
                    page_index,
                    doc.metadata.page_count,
                ));
            }

            let page = doc
                .document
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            let scale = dpi / PDF_DPI;
            let matrix = Matrix::new_scale(scale, scale);

            let pixmap = page
                .to_pixmap(&matrix, &Colorspace::device_rgb(), 1.0, true)
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;

            pixmap
                .to_png()
                .map_err(|e| PdfOffError::RenderFailed(e.to_string()))
        })
    }

    pub fn invalidate_cache(&self) {
        self.cache.lock().clear();
    }

    pub fn invalidate_page(&self, page_index: u32) {
        let mut cache = self.cache.lock();
        cache.retain(|k, _| k.page_index != page_index);
    }
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let renderer = Renderer::new();
        renderer.invalidate_cache();
    }

    #[test]
    fn test_cache_key_equality() {
        let k1 = CacheKey { page_index: 0, zoom_percent: 100 };
        let k2 = CacheKey { page_index: 0, zoom_percent: 100 };
        let k3 = CacheKey { page_index: 1, zoom_percent: 100 };
        assert_eq!(k1, k2);
        assert_ne!(k1, k3);
    }

    #[test]
    fn test_render_no_document() {
        let renderer = Renderer::new();
        let doc_mgr = DocumentManager::new();
        assert!(renderer.render_page(&doc_mgr, 0, 1.0).is_err());
    }
}
