use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum PdfOffError {
    #[error("No document is currently open")]
    NoDocument,

    #[error("Failed to open PDF: {0}")]
    OpenFailed(String),

    #[error("Failed to render page: {0}")]
    RenderFailed(String),

    #[error("Invalid page number: {0} (document has {1} pages)")]
    InvalidPage(u32, u32),

    #[error("Failed to save PDF: {0}")]
    SaveFailed(String),

    #[error("Print failed: {0}")]
    PrintFailed(String),

    #[error("Form operation failed: {0}")]
    FormError(String),

    #[error("Annotation operation failed: {0}")]
    AnnotationError(String),

    #[error("Page editing operation failed: {0}")]
    PageEditError(String),

    #[error("MuPDF error: {0}")]
    MuPdf(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<mupdf::Error> for PdfOffError {
    fn from(e: mupdf::Error) -> Self {
        PdfOffError::MuPdf(e.to_string())
    }
}

impl Serialize for PdfOffError {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, PdfOffError>;
