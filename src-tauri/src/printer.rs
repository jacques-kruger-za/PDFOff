use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use crate::renderer::Renderer;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrintSettings {
    pub page_range: PageRange,
    pub copies: u32,
    pub dpi: f32,
    pub scaling: PrintScaling,
    pub orientation: PrintOrientation,
    pub collate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PageRange {
    All,
    Current(u32),
    Custom(Vec<u32>),
    Range(u32, u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrintScaling {
    FitToPage,
    ActualSize,
    Custom(f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrintOrientation {
    Auto,
    Portrait,
    Landscape,
}

impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            page_range: PageRange::All,
            copies: 1,
            dpi: 300.0,
            scaling: PrintScaling::FitToPage,
            orientation: PrintOrientation::Auto,
            collate: true,
        }
    }
}

pub struct Printer;

impl Printer {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_page_range(range_str: &str, total_pages: u32) -> Result<Vec<u32>> {
        let mut pages = Vec::new();
        let parts: Vec<&str> = range_str.split(',').collect();

        for part in parts {
            let part = part.trim();
            if part.contains('-') {
                let range_parts: Vec<&str> = part.split('-').collect();
                if range_parts.len() != 2 {
                    return Err(PdfOffError::PrintFailed(format!(
                        "Invalid range: {}",
                        part
                    )));
                }
                let start: u32 = range_parts[0]
                    .trim()
                    .parse()
                    .map_err(|_| PdfOffError::PrintFailed(format!("Invalid number: {}", range_parts[0])))?;
                let end: u32 = range_parts[1]
                    .trim()
                    .parse()
                    .map_err(|_| PdfOffError::PrintFailed(format!("Invalid number: {}", range_parts[1])))?;

                if start == 0 || end == 0 || start > end || end > total_pages {
                    return Err(PdfOffError::PrintFailed(format!(
                        "Invalid range: {}-{} (document has {} pages)",
                        start, end, total_pages
                    )));
                }

                for p in start..=end {
                    pages.push(p - 1); // Convert to 0-indexed
                }
            } else {
                let page: u32 = part
                    .parse()
                    .map_err(|_| PdfOffError::PrintFailed(format!("Invalid page: {}", part)))?;
                if page == 0 || page > total_pages {
                    return Err(PdfOffError::PrintFailed(format!(
                        "Page {} out of range (document has {} pages)",
                        page, total_pages
                    )));
                }
                pages.push(page - 1);
            }
        }

        pages.sort_unstable();
        pages.dedup();
        Ok(pages)
    }

    pub fn get_pages_for_range(
        &self,
        doc_manager: &DocumentManager,
        range: &PageRange,
    ) -> Result<Vec<u32>> {
        let total = doc_manager.with_document(|doc| Ok(doc.metadata.page_count))?;
        match range {
            PageRange::All => Ok((0..total).collect()),
            PageRange::Current(page) => {
                if *page >= total {
                    return Err(PdfOffError::InvalidPage(*page, total));
                }
                Ok(vec![*page])
            }
            PageRange::Custom(pages) => {
                for &p in pages {
                    if p >= total {
                        return Err(PdfOffError::InvalidPage(p, total));
                    }
                }
                Ok(pages.clone())
            }
            PageRange::Range(start, end) => {
                if *start >= total || *end >= total || start > end {
                    return Err(PdfOffError::PrintFailed(format!(
                        "Invalid range: {}-{} (document has {} pages)",
                        start, end, total
                    )));
                }
                Ok((*start..=*end).collect())
            }
        }
    }

    pub fn prepare_print_data(
        &self,
        doc_manager: &DocumentManager,
        renderer: &Renderer,
        settings: &PrintSettings,
    ) -> Result<Vec<Vec<u8>>> {
        let pages = self.get_pages_for_range(doc_manager, &settings.page_range)?;
        let mut print_data = Vec::new();

        for page_index in pages {
            let data = renderer.render_for_print(doc_manager, page_index, settings.dpi)?;
            print_data.push(data);
        }

        Ok(print_data)
    }
}

impl Default for Printer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_page() {
        let pages = Printer::parse_page_range("3", 10).unwrap();
        assert_eq!(pages, vec![2]);
    }

    #[test]
    fn test_parse_page_range() {
        let pages = Printer::parse_page_range("1-3", 10).unwrap();
        assert_eq!(pages, vec![0, 1, 2]);
    }

    #[test]
    fn test_parse_complex_range() {
        let pages = Printer::parse_page_range("1-3, 7, 9-10", 10).unwrap();
        assert_eq!(pages, vec![0, 1, 2, 6, 8, 9]);
    }

    #[test]
    fn test_parse_invalid_range() {
        assert!(Printer::parse_page_range("0", 10).is_err());
        assert!(Printer::parse_page_range("11", 10).is_err());
        assert!(Printer::parse_page_range("5-3", 10).is_err());
        assert!(Printer::parse_page_range("abc", 10).is_err());
    }

    #[test]
    fn test_parse_dedup() {
        let pages = Printer::parse_page_range("1, 1, 2, 2", 10).unwrap();
        assert_eq!(pages, vec![0, 1]);
    }

    #[test]
    fn test_default_settings() {
        let settings = PrintSettings::default();
        assert_eq!(settings.copies, 1);
        assert!((settings.dpi - 300.0).abs() < f32::EPSILON);
        assert!(settings.collate);
    }
}
