use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use mupdf::pdf::PdfPage;
use mupdf::Size;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageOperation {
    Delete { page_index: u32 },
    InsertBlank { after_page: u32, width: f32, height: f32 },
    Rotate { page_index: u32, degrees: i32 },
    Move { from_index: u32, to_index: u32 },
    Extract { pages: Vec<u32>, output_path: String },
    MergeFrom { source_path: String, insert_at: u32 },
}

#[derive(Debug, Clone)]
struct UndoEntry {
    operation: PageOperation,
    reverse: PageOperation,
}

pub struct PageEditor {
    undo_stack: Mutex<VecDeque<UndoEntry>>,
    redo_stack: Mutex<VecDeque<UndoEntry>>,
}

const MAX_UNDO_ENTRIES: usize = 50;

impl PageEditor {
    pub fn new() -> Self {
        Self {
            undo_stack: Mutex::new(VecDeque::new()),
            redo_stack: Mutex::new(VecDeque::new()),
        }
    }

    pub fn delete_page(&self, doc_manager: &DocumentManager, page_index: u32) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let page_count = doc.metadata.page_count;
            if page_index >= page_count {
                return Err(PdfOffError::InvalidPage(page_index, page_count));
            }
            if page_count <= 1 {
                return Err(PdfOffError::PageEditError(
                    "Cannot delete the last page".to_string(),
                ));
            }

            let pdf_doc = doc.pdf_doc_mut().ok_or_else(|| {
                PdfOffError::PageEditError("Not a PDF document".to_string())
            })?;

            pdf_doc
                .delete_page(page_index as i32)
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;

            doc.metadata.page_count -= 1;
            doc.is_dirty = true;
            Ok(())
        })?;

        let entry = UndoEntry {
            operation: PageOperation::Delete { page_index },
            reverse: PageOperation::InsertBlank {
                after_page: page_index.saturating_sub(1),
                width: 612.0,
                height: 792.0,
            },
        };
        self.push_undo(entry);
        Ok(())
    }

    pub fn insert_blank_page(
        &self,
        doc_manager: &DocumentManager,
        after_page: u32,
        width: f32,
        height: f32,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let pdf_doc = doc.pdf_doc_mut().ok_or_else(|| {
                PdfOffError::PageEditError("Not a PDF document".to_string())
            })?;

            let insert_at = (after_page + 1) as i32;
            let _new_page = pdf_doc
                .new_page_at(insert_at, Size { width, height })
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;

            doc.metadata.page_count += 1;
            doc.is_dirty = true;
            Ok(())
        })?;

        let entry = UndoEntry {
            operation: PageOperation::InsertBlank {
                after_page,
                width,
                height,
            },
            reverse: PageOperation::Delete {
                page_index: after_page + 1,
            },
        };
        self.push_undo(entry);
        Ok(())
    }

    pub fn rotate_page(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        degrees: i32,
    ) -> Result<()> {
        let valid_degrees = match degrees % 360 {
            d if d == 0 || d == 90 || d == 180 || d == 270 => d,
            d if d == -90 || d == -180 || d == -270 => d + 360,
            _ => {
                return Err(PdfOffError::PageEditError(format!(
                    "Invalid rotation: {} degrees. Must be a multiple of 90.",
                    degrees
                )));
            }
        };

        doc_manager.with_document_mut(|doc| {
            if page_index >= doc.metadata.page_count {
                return Err(PdfOffError::InvalidPage(page_index, doc.metadata.page_count));
            }

            let page = doc
                .doc()
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;

            let mut pdf_page = PdfPage::try_from(page)
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;

            let current_rotation = pdf_page
                .rotation()
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
            let new_rotation = (current_rotation + valid_degrees) % 360;

            pdf_page.set_rotation(new_rotation)
                .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;

            doc.is_dirty = true;
            Ok(())
        })?;

        let reverse_degrees = (360 - valid_degrees) % 360;
        let entry = UndoEntry {
            operation: PageOperation::Rotate {
                page_index,
                degrees: valid_degrees,
            },
            reverse: PageOperation::Rotate {
                page_index,
                degrees: reverse_degrees,
            },
        };
        self.push_undo(entry);
        Ok(())
    }

    pub fn extract_pages(
        &self,
        doc_manager: &DocumentManager,
        pages: &[u32],
        output_path: &str,
    ) -> Result<()> {
        doc_manager.with_document(|doc| {
            let pdf_doc = doc.pdf_doc().ok_or_else(|| {
                PdfOffError::PageEditError("Not a PDF document".to_string())
            })?;

            for &p in pages {
                if p >= doc.metadata.page_count {
                    return Err(PdfOffError::InvalidPage(p, doc.metadata.page_count));
                }
            }

            let mut new_doc = mupdf::pdf::PdfDocument::new();

            let mut graft_map = new_doc
                .new_graft_map()
                .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;

            for &page_idx in pages {
                let page_obj = pdf_doc
                    .find_page(page_idx as i32)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
                let grafted = graft_map
                    .graft_object(&page_obj)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
                let page_count = new_doc
                    .page_count()
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
                new_doc
                    .insert_page(page_count, &grafted)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
            }

            new_doc
                .save(output_path)
                .map_err(|e: mupdf::Error| PdfOffError::SaveFailed(e.to_string()))?;

            Ok(())
        })
    }

    pub fn merge_document(
        &self,
        doc_manager: &DocumentManager,
        source_path: &str,
        insert_at: u32,
    ) -> Result<u32> {
        doc_manager.with_document_mut(|doc| {
            let source = mupdf::pdf::PdfDocument::open(source_path)
                .map_err(|e: mupdf::Error| PdfOffError::OpenFailed(e.to_string()))?;

            let source_pages = source
                .page_count()
                .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))? as u32;

            let pdf_doc = doc.pdf_doc_mut().ok_or_else(|| {
                PdfOffError::PageEditError("Not a PDF document".to_string())
            })?;

            let mut graft_map = pdf_doc
                .new_graft_map()
                .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;

            for i in 0..source_pages {
                let page_obj = source
                    .find_page(i as i32)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
                let grafted = graft_map
                    .graft_object(&page_obj)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
                let target_pos = insert_at as i32 + i as i32;
                pdf_doc
                    .insert_page(target_pos, &grafted)
                    .map_err(|e: mupdf::Error| PdfOffError::PageEditError(e.to_string()))?;
            }

            doc.metadata.page_count += source_pages;
            doc.is_dirty = true;

            Ok(source_pages)
        })
    }

    pub fn save_document(&self, doc_manager: &DocumentManager) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let path = doc.file_path.to_string_lossy().to_string();
            let pdf_doc = doc.pdf_doc().ok_or_else(|| {
                PdfOffError::SaveFailed("Not a PDF document".to_string())
            })?;
            pdf_doc
                .save(&path)
                .map_err(|e: mupdf::Error| PdfOffError::SaveFailed(e.to_string()))?;
            doc.is_dirty = false;
            Ok(())
        })
    }

    pub fn save_document_as(
        &self,
        doc_manager: &DocumentManager,
        output_path: &str,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            {
                let pdf_doc = doc.pdf_doc().ok_or_else(|| {
                    PdfOffError::SaveFailed("Not a PDF document".to_string())
                })?;
                pdf_doc
                    .save(output_path)
                    .map_err(|e: mupdf::Error| PdfOffError::SaveFailed(e.to_string()))?;
            }
            doc.file_path = std::path::PathBuf::from(output_path);
            doc.metadata.file_path = output_path.to_string();
            doc.metadata.file_name = std::path::Path::new(output_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());
            doc.is_dirty = false;
            Ok(())
        })
    }

    pub fn undo(&self, doc_manager: &DocumentManager) -> Result<bool> {
        let entry = self.undo_stack.lock().unwrap().pop_back();
        if let Some(entry) = entry {
            self.apply_operation(doc_manager, &entry.reverse)?;
            self.redo_stack.lock().unwrap().push_back(entry);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn redo(&self, doc_manager: &DocumentManager) -> Result<bool> {
        let entry = self.redo_stack.lock().unwrap().pop_back();
        if let Some(entry) = entry {
            self.apply_operation(doc_manager, &entry.operation)?;
            self.undo_stack.lock().unwrap().push_back(entry);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.lock().unwrap().is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.lock().unwrap().is_empty()
    }

    fn push_undo(&self, entry: UndoEntry) {
        let mut stack = self.undo_stack.lock().unwrap();
        if stack.len() >= MAX_UNDO_ENTRIES {
            stack.pop_front();
        }
        stack.push_back(entry);
        self.redo_stack.lock().unwrap().clear();
    }

    fn apply_operation(
        &self,
        doc_manager: &DocumentManager,
        operation: &PageOperation,
    ) -> Result<()> {
        match operation {
            PageOperation::Delete { page_index } => {
                doc_manager.with_document_mut(|doc| {
                    let pdf_doc = doc.pdf_doc_mut().ok_or_else(|| {
                        PdfOffError::PageEditError("Not a PDF document".to_string())
                    })?;
                    pdf_doc
                        .delete_page(*page_index as i32)
                        .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
                    doc.metadata.page_count -= 1;
                    doc.is_dirty = true;
                    Ok(())
                })
            }
            PageOperation::Rotate { page_index, degrees } => {
                doc_manager.with_document_mut(|doc| {
                    let page = doc
                        .doc()
                        .load_page(*page_index as i32)
                        .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
                    let mut pdf_page = PdfPage::try_from(page)
                        .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
                    let current = pdf_page
                        .rotation()
                        .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
                    pdf_page.set_rotation((current + degrees) % 360)
                        .map_err(|e| PdfOffError::PageEditError(e.to_string()))?;
                    doc.is_dirty = true;
                    Ok(())
                })
            }
            _ => Ok(()),
        }
    }

    pub fn reset(&self) {
        self.undo_stack.lock().unwrap().clear();
        self.redo_stack.lock().unwrap().clear();
    }
}

impl Default for PageEditor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentManager;

    #[test]
    fn test_page_editor_creation() {
        let editor = PageEditor::new();
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }

    #[test]
    fn test_undo_redo_no_doc() {
        let editor = PageEditor::new();
        let mgr = DocumentManager::new();
        assert!(!editor.undo(&mgr).unwrap());
        assert!(!editor.redo(&mgr).unwrap());
    }

    #[test]
    fn test_rotation_validation() {
        let editor = PageEditor::new();
        let mgr = DocumentManager::new();
        assert!(editor.rotate_page(&mgr, 0, 90).is_err());
    }

    #[test]
    fn test_delete_no_doc() {
        let editor = PageEditor::new();
        let mgr = DocumentManager::new();
        assert!(editor.delete_page(&mgr, 0).is_err());
    }

    #[test]
    fn test_reset() {
        let editor = PageEditor::new();
        editor.reset();
        assert!(!editor.can_undo());
        assert!(!editor.can_redo());
    }
}
