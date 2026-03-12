use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use mupdf::pdf::PdfDocument;
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
                .pdf_document
                .as_ref()
                .ok_or_else(|| PdfOffError::FormError("Not a PDF document".to_string()))?;

            let mut fields = Vec::new();
            let page_count = doc.metadata.page_count;

            for page_idx in 0..page_count {
                let page = pdf_doc
                    .load_page(page_idx as i32)
                    .map_err(|e| PdfOffError::FormError(e.to_string()))?;

                let mut widget = page.first_widget();
                while let Some(w) = widget {
                    let field_type = match w.field_type() {
                        Ok(mupdf::pdf::PdfWidgetType::Text) => FormFieldType::Text,
                        Ok(mupdf::pdf::PdfWidgetType::CheckBox) => FormFieldType::Checkbox,
                        Ok(mupdf::pdf::PdfWidgetType::RadioButton) => FormFieldType::RadioButton,
                        Ok(mupdf::pdf::PdfWidgetType::ComboBox) => FormFieldType::ComboBox,
                        Ok(mupdf::pdf::PdfWidgetType::ListBox) => FormFieldType::Dropdown,
                        Ok(mupdf::pdf::PdfWidgetType::Signature) => FormFieldType::Signature,
                        Ok(mupdf::pdf::PdfWidgetType::PushButton) => FormFieldType::Button,
                        _ => FormFieldType::Unknown,
                    };

                    let rect = w.rect();
                    let name = w.field_label().unwrap_or_default();
                    let value = w.value().unwrap_or_default();
                    let max_length = {
                        let ml = w.max_length();
                        if ml > 0 { Some(ml as u32) } else { None }
                    };

                    let field_flags = w.field_flags();
                    let is_read_only = field_flags & 1 != 0;
                    let is_required = field_flags & 2 != 0;

                    let options = w.options()
                        .map(|opts| opts.into_iter().map(|o| o.to_string()).collect())
                        .unwrap_or_default();

                    fields.push(FormField {
                        id: format!("field_{}_{}", page_idx, fields.len()),
                        page_index: page_idx,
                        field_type,
                        name,
                        value,
                        rect: FieldRect {
                            x: rect.x0,
                            y: rect.y0,
                            width: rect.x1 - rect.x0,
                            height: rect.y1 - rect.y0,
                        },
                        max_length,
                        options,
                        is_read_only,
                        is_required,
                    });

                    widget = w.next_widget();
                }
            }

            Ok(fields)
        })
    }

    pub fn set_field_value(
        &self,
        doc_manager: &DocumentManager,
        update: &FormFieldUpdate,
    ) -> Result<()> {
        doc_manager.with_document_mut(|doc| {
            let pdf_doc = doc
                .pdf_document
                .as_ref()
                .ok_or_else(|| PdfOffError::FormError("Not a PDF document".to_string()))?;

            let page = pdf_doc
                .load_page(update.page_index as i32)
                .map_err(|e| PdfOffError::FormError(e.to_string()))?;

            let mut widget = page.first_widget();
            while let Some(mut w) = widget {
                let name = w.field_label().unwrap_or_default();
                if name == update.field_name {
                    w.set_value(&update.value)
                        .map_err(|e| PdfOffError::FormError(e.to_string()))?;
                    doc.is_dirty = true;
                    return Ok(());
                }
                widget = w.next_widget();
            }

            Err(PdfOffError::FormError(format!(
                "Field '{}' not found on page {}",
                update.field_name, update.page_index
            )))
        })
    }

    pub fn has_forms(&self, doc_manager: &DocumentManager) -> Result<bool> {
        match self.get_form_fields(doc_manager) {
            Ok(fields) => Ok(!fields.is_empty()),
            Err(_) => Ok(false),
        }
    }

    pub fn clear_all_fields(&self, doc_manager: &DocumentManager) -> Result<()> {
        let fields = self.get_form_fields(doc_manager)?;
        for field in fields {
            if !field.is_read_only {
                let update = FormFieldUpdate {
                    page_index: field.page_index,
                    field_name: field.name.clone(),
                    value: String::new(),
                };
                let _ = self.set_field_value(doc_manager, &update);
            }
        }
        Ok(())
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
        assert!(!handler.has_forms(&mgr).unwrap_or(false));
    }
}
