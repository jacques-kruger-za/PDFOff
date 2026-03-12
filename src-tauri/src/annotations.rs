use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use mupdf::pdf::PdfPage;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: String,
    pub page_index: u32,
    pub annotation_type: AnnotationType,
    pub rect: AnnotationRect,
    pub content: String,
    pub color: AnnotationColor,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnnotationType {
    Highlight,
    Underline,
    Strikethrough,
    StickyNote,
    FreehandInk,
    TextBox,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl AnnotationColor {
    pub fn yellow() -> Self {
        Self { r: 1.0, g: 1.0, b: 0.0, a: 0.5 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InkStroke {
    pub points: Vec<(f32, f32)>,
    pub color: AnnotationColor,
    pub thickness: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAnnotationRequest {
    pub page_index: u32,
    pub annotation_type: AnnotationType,
    pub rect: AnnotationRect,
    pub content: String,
    pub color: AnnotationColor,
    pub ink_strokes: Option<Vec<InkStroke>>,
}

pub struct AnnotationHandler {
    local_annotations: Mutex<Vec<Annotation>>,
}

impl AnnotationHandler {
    pub fn new() -> Self {
        Self {
            local_annotations: Mutex::new(Vec::new()),
        }
    }

    pub fn get_annotations(&self, doc_manager: &DocumentManager) -> Result<Vec<Annotation>> {
        doc_manager.with_document(|doc| {
            let _pdf_doc = doc
                .pdf_doc()
                .ok_or_else(|| {
                    PdfOffError::AnnotationError("Not a PDF document".to_string())
                })?;

            let mut annotations = Vec::new();
            let page_count = doc.metadata.page_count;

            for page_idx in 0..page_count {
                let page = doc
                    .doc()
                    .load_page(page_idx as i32)
                    .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

                let pdf_page = PdfPage::from(page);

                for (idx, annot) in pdf_page.annotations().enumerate() {
                    let annot_type = match annot.r#type() {
                        Ok(t) => match t {
                            mupdf::pdf::PdfAnnotationType::Highlight => AnnotationType::Highlight,
                            mupdf::pdf::PdfAnnotationType::Underline => AnnotationType::Underline,
                            mupdf::pdf::PdfAnnotationType::StrikeOut => AnnotationType::Strikethrough,
                            mupdf::pdf::PdfAnnotationType::Text => AnnotationType::StickyNote,
                            mupdf::pdf::PdfAnnotationType::Ink => AnnotationType::FreehandInk,
                            mupdf::pdf::PdfAnnotationType::FreeText => AnnotationType::TextBox,
                            mupdf::pdf::PdfAnnotationType::Widget => continue,
                            _ => continue,
                        },
                        Err(_) => continue,
                    };

                    let content = annot.author()
                        .ok()
                        .and_then(|opt: Option<&str>| opt.map(|s| s.to_string()))
                        .unwrap_or_default();

                    // MuPDF 0.4 PdfAnnotation does not expose rect().
                    // Use page bounds as a fallback; the frontend tracks
                    // precise positions via the CreateAnnotationRequest rect.
                    let page_bounds = pdf_page.bounds().unwrap_or(mupdf::Rect {
                        x0: 0.0, y0: 0.0, x1: 612.0, y1: 792.0,
                    });

                    annotations.push(Annotation {
                        id: format!("annot_{}_{}", page_idx, idx),
                        page_index: page_idx,
                        annotation_type: annot_type,
                        rect: AnnotationRect {
                            x: page_bounds.x0,
                            y: page_bounds.y0,
                            width: page_bounds.x1 - page_bounds.x0,
                            height: page_bounds.y1 - page_bounds.y0,
                        },
                        content,
                        color: AnnotationColor::yellow(),
                        created_at: String::new(),
                    });
                }
            }

            // Merge in locally-tracked annotations
            let local = self.local_annotations.lock().unwrap();
            for annot in local.iter() {
                if !annotations.iter().any(|a| a.id == annot.id) {
                    annotations.push(annot.clone());
                }
            }

            Ok(annotations)
        })
    }

    pub fn get_annotations_for_page(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
    ) -> Result<Vec<Annotation>> {
        let all = self.get_annotations(doc_manager)?;
        Ok(all
            .into_iter()
            .filter(|a| a.page_index == page_index)
            .collect())
    }

    pub fn create_annotation(
        &self,
        doc_manager: &DocumentManager,
        request: &CreateAnnotationRequest,
    ) -> Result<Annotation> {
        doc_manager.with_document_mut(|doc| {
            let page = doc
                .doc()
                .load_page(request.page_index as i32)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            let mut pdf_page = PdfPage::from(page);

            let annot_type = match request.annotation_type {
                AnnotationType::Highlight => mupdf::pdf::PdfAnnotationType::Highlight,
                AnnotationType::Underline => mupdf::pdf::PdfAnnotationType::Underline,
                AnnotationType::Strikethrough => mupdf::pdf::PdfAnnotationType::StrikeOut,
                AnnotationType::StickyNote => mupdf::pdf::PdfAnnotationType::Text,
                AnnotationType::FreehandInk => mupdf::pdf::PdfAnnotationType::Ink,
                AnnotationType::TextBox => mupdf::pdf::PdfAnnotationType::FreeText,
            };

            let _annot = pdf_page
                .create_annotation(annot_type)
                .map_err(|e: mupdf::Error| PdfOffError::AnnotationError(e.to_string()))?;

            let id = uuid::Uuid::new_v4().to_string();
            doc.is_dirty = true;

            let annotation = Annotation {
                id,
                page_index: request.page_index,
                annotation_type: request.annotation_type.clone(),
                rect: request.rect.clone(),
                content: request.content.clone(),
                color: request.color.clone(),
                created_at: timestamp_now(),
            };

            // Store in local cache so we can return accurate rect info
            self.local_annotations.lock().unwrap().push(annotation.clone());

            Ok(annotation)
        })
    }

    pub fn delete_annotation(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        annotation_index: usize,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let page = doc
                .doc()
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            let mut pdf_page = PdfPage::from(page);

            let annot = pdf_page.annotations().nth(annotation_index).ok_or_else(|| {
                PdfOffError::AnnotationError(format!(
                    "Annotation index {} not found on page {}",
                    annotation_index, page_index
                ))
            })?;

            pdf_page.delete_annotation(&annot)
                .map_err(|e: mupdf::Error| PdfOffError::AnnotationError(e.to_string()))?;

            doc.is_dirty = true;
            Ok(())
        })
    }

    pub fn clear_local_cache(&self) {
        self.local_annotations.lock().unwrap().clear();
    }
}

fn timestamp_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}", now.as_secs())
}

impl Default for AnnotationHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentManager;

    #[test]
    fn test_annotation_colors() {
        let y = AnnotationColor::yellow();
        assert!((y.r - 1.0).abs() < f32::EPSILON);
        assert!((y.g - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_annotation_handler_no_doc() {
        let handler = AnnotationHandler::new();
        let mgr = DocumentManager::new();
        assert!(handler.get_annotations(&mgr).is_err());
    }

    #[test]
    fn test_timestamp_now() {
        let ts = timestamp_now();
        assert!(!ts.is_empty());
        let _: u64 = ts.parse().expect("should be a numeric timestamp");
    }
}
