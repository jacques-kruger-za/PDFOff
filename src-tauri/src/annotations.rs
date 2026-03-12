use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use serde::{Deserialize, Serialize};

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
    pub fn green() -> Self {
        Self { r: 0.0, g: 1.0, b: 0.0, a: 0.5 }
    }
    pub fn blue() -> Self {
        Self { r: 0.0, g: 0.5, b: 1.0, a: 0.5 }
    }
    pub fn pink() -> Self {
        Self { r: 1.0, g: 0.4, b: 0.7, a: 0.5 }
    }
    pub fn red() -> Self {
        Self { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }
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
    annotations: parking_lot::Mutex<Vec<Annotation>>,
}

impl AnnotationHandler {
    pub fn new() -> Self {
        Self {
            annotations: parking_lot::Mutex::new(Vec::new()),
        }
    }

    pub fn get_annotations(&self, doc_manager: &DocumentManager) -> Result<Vec<Annotation>> {
        doc_manager.with_document(|doc| {
            let pdf_doc = doc
                .pdf_document
                .as_ref()
                .ok_or_else(|| {
                    PdfOffError::AnnotationError("Not a PDF document".to_string())
                })?;

            let mut annotations = Vec::new();
            let page_count = doc.metadata.page_count;

            for page_idx in 0..page_count {
                let page = pdf_doc
                    .load_page(page_idx as i32)
                    .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

                for (idx, annot) in page.annotations().enumerate() {
                    let rect = annot.rect()
                        .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;
                    let annot_type = annot.subtype()
                        .map(|st| match st {
                            mupdf::pdf::PdfAnnotationType::Highlight => AnnotationType::Highlight,
                            mupdf::pdf::PdfAnnotationType::Underline => AnnotationType::Underline,
                            mupdf::pdf::PdfAnnotationType::StrikeOut => AnnotationType::Strikethrough,
                            mupdf::pdf::PdfAnnotationType::Text => AnnotationType::StickyNote,
                            mupdf::pdf::PdfAnnotationType::Ink => AnnotationType::FreehandInk,
                            mupdf::pdf::PdfAnnotationType::FreeText => AnnotationType::TextBox,
                            _ => AnnotationType::StickyNote,
                        })
                        .unwrap_or(AnnotationType::StickyNote);

                    let content = annot.contents()
                        .unwrap_or_default();

                    annotations.push(Annotation {
                        id: format!("annot_{}_{}", page_idx, idx),
                        page_index: page_idx,
                        annotation_type: annot_type,
                        rect: AnnotationRect {
                            x: rect.x0,
                            y: rect.y0,
                            width: rect.x1 - rect.x0,
                            height: rect.y1 - rect.y0,
                        },
                        content,
                        color: AnnotationColor::yellow(),
                        created_at: String::new(),
                    });
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
            let pdf_doc = doc
                .pdf_document
                .as_ref()
                .ok_or_else(|| {
                    PdfOffError::AnnotationError("Not a PDF document".to_string())
                })?;

            let page = pdf_doc
                .load_page(request.page_index as i32)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            let annot_type = match request.annotation_type {
                AnnotationType::Highlight => mupdf::pdf::PdfAnnotationType::Highlight,
                AnnotationType::Underline => mupdf::pdf::PdfAnnotationType::Underline,
                AnnotationType::Strikethrough => mupdf::pdf::PdfAnnotationType::StrikeOut,
                AnnotationType::StickyNote => mupdf::pdf::PdfAnnotationType::Text,
                AnnotationType::FreehandInk => mupdf::pdf::PdfAnnotationType::Ink,
                AnnotationType::TextBox => mupdf::pdf::PdfAnnotationType::FreeText,
            };

            let mut annot = page
                .create_annotation(annot_type)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            annot
                .set_rect(mupdf::Rect {
                    x0: request.rect.x,
                    y0: request.rect.y,
                    x1: request.rect.x + request.rect.width,
                    y1: request.rect.y + request.rect.height,
                })
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            if !request.content.is_empty() {
                annot
                    .set_contents(&request.content)
                    .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;
            }

            annot
                .set_color(request.color.r, request.color.g, request.color.b)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            let id = uuid::Uuid::new_v4().to_string();
            doc.is_dirty = true;

            Ok(Annotation {
                id,
                page_index: request.page_index,
                annotation_type: request.annotation_type.clone(),
                rect: request.rect.clone(),
                content: request.content.clone(),
                color: request.color.clone(),
                created_at: chrono_now(),
            })
        })
    }

    pub fn delete_annotation(
        &self,
        doc_manager: &DocumentManager,
        page_index: u32,
        annotation_index: usize,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let pdf_doc = doc
                .pdf_document
                .as_ref()
                .ok_or_else(|| {
                    PdfOffError::AnnotationError("Not a PDF document".to_string())
                })?;

            let page = pdf_doc
                .load_page(page_index as i32)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            let annot = page.annotations().nth(annotation_index).ok_or_else(|| {
                PdfOffError::AnnotationError(format!(
                    "Annotation index {} not found on page {}",
                    annotation_index, page_index
                ))
            })?;

            page.delete_annotation(annot)
                .map_err(|e| PdfOffError::AnnotationError(e.to_string()))?;

            doc.is_dirty = true;
            Ok(())
        })
    }

    pub fn clear_local_cache(&self) {
        self.annotations.lock().clear();
    }
}

fn chrono_now() -> String {
    // Simple ISO-ish timestamp without chrono dependency
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

    #[test]
    fn test_annotation_colors() {
        let y = AnnotationColor::yellow();
        assert!((y.r - 1.0).abs() < f32::EPSILON);
        assert!((y.g - 1.0).abs() < f32::EPSILON);
        assert!((y.b - 0.0).abs() < f32::EPSILON);

        let r = AnnotationColor::red();
        assert!((r.a - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_annotation_handler_no_doc() {
        let handler = AnnotationHandler::new();
        let mgr = DocumentManager::new();
        assert!(handler.get_annotations(&mgr).is_err());
    }

    #[test]
    fn test_chrono_now() {
        let ts = chrono_now();
        assert!(!ts.is_empty());
        let _: u64 = ts.parse().expect("should be a numeric timestamp");
    }
}
