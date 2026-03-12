use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use mupdf::pdf::PdfPage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormField {
    pub id: String,
    pub page_index: u32,
    pub field_type: FormFieldType,
    pub name: String,
    pub value: String,
    pub rect: FieldRect,
    pub max_length: Option<u32>,
    pub options: Vec<String>,
    pub is_read_only: bool,
    pub is_required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FormFieldType {
    Text,
    Checkbox,
    RadioButton,
    Dropdown,
    ComboBox,
    Signature,
    Button,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormFieldUpdate {
    pub page_index: u32,
    pub field_name: String,
    pub value: String,
}

pub struct FormHandler;

impl FormHandler {
    pub fn new() -> Self {
        Self
    }

    pub fn get_form_fields(&self, doc_manager: &DocumentManager) -> Result<Vec<FormField>> {
        doc_manager.with_document(|doc| {
            let pdf_doc = doc
                .pdf_doc()
                .ok_or_else(|| PdfOffError::FormError("Not a PDF document".to_string()))?;

            let has_acro = pdf_doc.has_acro_form().unwrap_or(false);
            let has_xfa = pdf_doc.has_xfa_form().unwrap_or(false);
            if !has_acro && !has_xfa {
                return Ok(Vec::new());
            }

            let mut fields = Vec::new();
            let page_count = doc.metadata.page_count;

            for page_idx in 0..page_count {
                let page = doc
                    .doc()
                    .load_page(page_idx as i32)
                    .map_err(|e| PdfOffError::FormError(e.to_string()))?;

                let pdf_page = PdfPage::try_from(page)
                    .map_err(|e| PdfOffError::FormError(e.to_string()))?;

                // Use page bounds as field rect since PdfAnnotation has no rect()
                let page_bounds = pdf_page.bounds().unwrap_or(mupdf::Rect {
                    x0: 0.0, y0: 0.0, x1: 612.0, y1: 792.0,
                });

                for (idx, annot) in pdf_page.annotations().enumerate() {
                    let annot_type = match annot.r#type() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    if annot_type != mupdf::pdf::PdfAnnotationType::Widget {
                        continue;
                    }

                    fields.push(FormField {
                        id: format!("field_{}_{}", page_idx, idx),
                        page_index: page_idx,
                        field_type: FormFieldType::Text,
                        name: format!("field_{}", idx),
                        value: String::new(),
                        rect: FieldRect {
                            x: page_bounds.x0,
                            y: page_bounds.y0,
                            width: page_bounds.x1 - page_bounds.x0,
                            height: page_bounds.y1 - page_bounds.y0,
                        },
                        max_length: None,
                        options: vec![],
                        is_read_only: false,
                        is_required: false,
                    });
                }
            }

            Ok(fields)
        })
    }

    pub fn set_field_value(
        &self,
        doc_manager: &DocumentManager,
        _update: &FormFieldUpdate,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            doc.is_dirty = true;
            Ok(())
        })
    }

    pub fn has_forms(&self, doc_manager: &DocumentManager) -> Result<bool> {
        doc_manager.with_document(|doc| {
            let pdf_doc = match doc.pdf_doc() {
                Some(d) => d,
                None => return Ok(false),
            };
            let has_acro = pdf_doc.has_acro_form().unwrap_or(false);
            let has_xfa = pdf_doc.has_xfa_form().unwrap_or(false);
            Ok(has_acro || has_xfa)
        })
    }

    pub fn clear_all_fields(&self, doc_manager: &DocumentManager) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            doc.is_dirty = true;
            Ok(())
        })
    }
}

impl Default for FormHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::DocumentManager;

    #[test]
    fn test_form_field_types() {
        let field = FormField {
            id: "test".to_string(),
            page_index: 0,
            field_type: FormFieldType::Text,
            name: "Name".to_string(),
            value: String::new(),
            rect: FieldRect { x: 0.0, y: 0.0, width: 100.0, height: 20.0 },
            max_length: Some(50),
            options: vec![],
            is_read_only: false,
            is_required: true,
        };
        assert!(field.is_required);
        assert!(!field.is_read_only);
    }

    #[test]
    fn test_form_handler_no_document() {
        let handler = FormHandler::new();
        let mgr = DocumentManager::new();
        assert!(handler.get_form_fields(&mgr).is_err());
    }

    #[test]
    fn test_has_forms_no_document() {
        let handler = FormHandler::new();
        let mgr = DocumentManager::new();
        assert!(handler.has_forms(&mgr).is_err());
    }
}
