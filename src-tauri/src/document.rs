use crate::error::{PdfOffError, Result};
use mupdf::{Document, Matrix, Colorspace, pdf::PdfDocument};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub page_count: u32,
    pub file_path: String,
    pub file_name: String,
    pub file_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct PageInfo {
    pub index: u32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewState {
    pub current_page: u32,
    pub zoom_level: f32,
    pub scroll_x: f32,
    pub scroll_y: f32,
}

impl Default for ViewState {
    fn default() -> Self {
        Self {
            current_page: 0,
            zoom_level: 1.0,
            scroll_x: 0.0,
            scroll_y: 0.0,
        }
    }
}

pub struct OpenDocument {
    pub document: Document,
    pub pdf_document: Option<PdfDocument>,
    pub metadata: DocumentMetadata,
    pub view_state: ViewState,
    pub is_dirty: bool,
    pub file_path: PathBuf,
}

pub struct DocumentManager {
    current: Mutex<Option<OpenDocument>>,
    position_store: Mutex<HashMap<String, ViewState>>,
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            current: Mutex::new(None),
            position_store: Mutex::new(HashMap::new()),
        }
    }

    pub fn open(&self, path: &str) -> Result<DocumentMetadata> {
        let file_path = PathBuf::from(path);
        if !file_path.exists() {
            return Err(PdfOffError::OpenFailed(format!(
                "File not found: {}",
                path
            )));
        }

        let file_size = std::fs::metadata(&file_path)
            .map(|m| m.len())
            .unwrap_or(0);

        let document = Document::open(path)
            .map_err(|e| PdfOffError::OpenFailed(e.to_string()))?;

        let page_count = document.page_count()
            .map_err(|e| PdfOffError::OpenFailed(e.to_string()))? as u32;

        let title = document.metadata("info:Title").ok().flatten();
        let author = document.metadata("info:Author").ok().flatten();

        let file_name = file_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let metadata = DocumentMetadata {
            title: title.clone(),
            author,
            page_count,
            file_path: path.to_string(),
            file_name: file_name.clone(),
            file_size_bytes: file_size,
        };

        let pdf_document = PdfDocument::open(path).ok();

        let view_state = self
            .position_store
            .lock()
            .get(path)
            .cloned()
            .unwrap_or_default();

        let open_doc = OpenDocument {
            document,
            pdf_document,
            metadata: metadata.clone(),
            view_state,
            is_dirty: false,
            file_path,
        };

        *self.current.lock() = Some(open_doc);
        Ok(metadata)
    }

    pub fn close(&self) -> Result<()> {
        let mut current = self.current.lock();
        if let Some(doc) = current.as_ref() {
            let path = doc.metadata.file_path.clone();
            let view_state = doc.view_state.clone();
            self.position_store
                .lock()
                .insert(path, view_state);
        }
        *current = None;
        Ok(())
    }

    pub fn with_document<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&OpenDocument) -> Result<T>,
    {
        let current = self.current.lock();
        let doc = current.as_ref().ok_or(PdfOffError::NoDocument)?;
        f(doc)
    }

    pub fn with_document_mut<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut OpenDocument) -> Result<T>,
    {
        let mut current = self.current.lock();
        let doc = current.as_mut().ok_or(PdfOffError::NoDocument)?;
        f(doc)
    }

    pub fn get_metadata(&self) -> Result<DocumentMetadata> {
        self.with_document(|doc| Ok(doc.metadata.clone()))
    }

    pub fn get_page_info(&self, page_index: u32) -> Result<PageInfo> {
        self.with_document(|doc| {
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
            Ok(PageInfo {
                index: page_index,
                width: bounds.x1 - bounds.x0,
                height: bounds.y1 - bounds.y0,
            })
        })
    }

    pub fn get_all_page_info(&self) -> Result<Vec<PageInfo>> {
        self.with_document(|doc| {
            let mut pages = Vec::new();
            for i in 0..doc.metadata.page_count {
                let page = doc
                    .document
                    .load_page(i as i32)
                    .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;
                let bounds = page
                    .bounds()
                    .map_err(|e| PdfOffError::RenderFailed(e.to_string()))?;
                pages.push(PageInfo {
                    index: i,
                    width: bounds.x1 - bounds.x0,
                    height: bounds.y1 - bounds.y0,
                });
            }
            Ok(pages)
        })
    }

    pub fn update_view_state(&self, state: ViewState) -> Result<()> {
        self.with_document_mut(|doc| {
            doc.view_state = state;
            Ok(())
        })
    }

    pub fn is_dirty(&self) -> Result<bool> {
        self.with_document(|doc| Ok(doc.is_dirty))
    }

    pub fn set_dirty(&self, dirty: bool) -> Result<()> {
        self.with_document_mut(|doc| {
            doc.is_dirty = dirty;
            Ok(())
        })
    }

    pub fn get_file_path(&self) -> Result<String> {
        self.with_document(|doc| Ok(doc.metadata.file_path.clone()))
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_manager_no_document() {
        let mgr = DocumentManager::new();
        assert!(mgr.get_metadata().is_err());
        assert!(mgr.is_dirty().is_err());
    }

    #[test]
    fn test_view_state_default() {
        let vs = ViewState::default();
        assert_eq!(vs.current_page, 0);
        assert!((vs.zoom_level - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_open_nonexistent() {
        let mgr = DocumentManager::new();
        assert!(mgr.open("/nonexistent/file.pdf").is_err());
    }

    #[test]
    fn test_close_without_open() {
        let mgr = DocumentManager::new();
        assert!(mgr.close().is_ok());
    }
}
