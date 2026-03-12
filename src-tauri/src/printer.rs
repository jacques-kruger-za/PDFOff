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
                    pages.push(p - 1);
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

    /// Enumerate installed printers. Returns (names, default_printer_name).
    #[cfg(windows)]
    pub fn enumerate_printers() -> (Vec<String>, Option<String>) {
        use windows::Win32::Graphics::Printing::{
            EnumPrintersW, GetDefaultPrinterW, PRINTER_ENUM_CONNECTIONS, PRINTER_ENUM_LOCAL,
            PRINTER_INFO_4W,
        };
        use windows::core::PWSTR;

        // Get default printer
        let default = {
            let mut size: u32 = 512;
            let mut buf: Vec<u16> = vec![0u16; size as usize];
            unsafe {
                if GetDefaultPrinterW(PWSTR(buf.as_mut_ptr()), &mut size).as_bool() {
                    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
                    Some(String::from_utf16_lossy(&buf[..end]).to_string())
                } else {
                    None
                }
            }
        };

        // Enumerate printers
        let mut needed: u32 = 0;
        let mut returned: u32 = 0;
        unsafe {
            let _ = EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                PWSTR::null(),
                4,
                None,
                &mut needed,
                &mut returned,
            );
        }

        if needed == 0 {
            return (Vec::new(), default);
        }

        let mut buf: Vec<u8> = vec![0u8; needed as usize];
        unsafe {
            if EnumPrintersW(
                PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS,
                PWSTR::null(),
                4,
                Some(&mut buf),
                &mut needed,
                &mut returned,
            )
            .is_err()
            {
                return (Vec::new(), default);
            }
        }

        let info_size = std::mem::size_of::<PRINTER_INFO_4W>();
        let names: Vec<String> = (0..returned as usize)
            .filter_map(|i| {
                let info: &PRINTER_INFO_4W = unsafe {
                    &*(buf.as_ptr().add(i * info_size) as *const PRINTER_INFO_4W)
                };
                let name = unsafe { info.pPrinterName.to_string().ok()? };
                if name.is_empty() { None } else { Some(name) }
            })
            .collect();

        (names, default)
    }

    #[cfg(not(windows))]
    pub fn enumerate_printers() -> (Vec<String>, Option<String>) {
        (Vec::new(), None)
    }

    /// Submit a print job using native Windows GDI.
    #[cfg(windows)]
    pub fn execute_print(
        printer_name: &str,
        pages: &[u32],
        copies: u32,
        doc_manager: &DocumentManager,
        renderer: &Renderer,
        dpi: f32,
        doc_title: &str,
    ) -> Result<()> {
        use std::iter::once;
        use windows::Win32::Graphics::Gdi::{
            CreateDCW, DeleteDC, GetDeviceCaps, StretchDIBits, BITMAPINFOHEADER, BI_RGB,
            DIB_RGB_COLORS, PHYSICALHEIGHT, PHYSICALOFFSETX, PHYSICALOFFSETY, PHYSICALWIDTH,
            SRCCOPY,
        };
        use windows::Win32::Graphics::Gdi::HDC;
        use windows::core::PCWSTR;

        // GDI print functions — not exposed in windows 0.58 module structure,
        // declared directly via FFI (all in gdi32.dll, wingdi.h)
        #[repr(C)]
        struct DocInfoW {
            cb_size: i32,
            psz_doc_name: *const u16,
            psz_output: *const u16,
            psz_datatype: *const u16,
            fw_type: u32,
        }
        extern "system" {
            fn StartDocW(hdc: HDC, lpdi: *const DocInfoW) -> i32;
            fn StartPage(hdc: HDC) -> i32;
            fn EndPage(hdc: HDC) -> i32;
            fn EndDoc(hdc: HDC) -> i32;
        }

        let printer_wide: Vec<u16> = printer_name.encode_utf16().chain(once(0)).collect();
        let hdc = unsafe {
            CreateDCW(
                PCWSTR::null(),
                PCWSTR(printer_wide.as_ptr()),
                PCWSTR::null(),
                None,
            )
        };

        if hdc.is_invalid() {
            return Err(PdfOffError::PrintFailed(format!(
                "Failed to create DC for printer '{}'",
                printer_name
            )));
        }

        let (phys_w, phys_h, off_x, off_y) = unsafe {
            (
                GetDeviceCaps(hdc, PHYSICALWIDTH),
                GetDeviceCaps(hdc, PHYSICALHEIGHT),
                GetDeviceCaps(hdc, PHYSICALOFFSETX),
                GetDeviceCaps(hdc, PHYSICALOFFSETY),
            )
        };
        let print_w = (phys_w - 2 * off_x).max(1);
        let print_h = (phys_h - 2 * off_y).max(1);

        let title_wide: Vec<u16> = doc_title.encode_utf16().chain(once(0)).collect();
        let doc_info = DocInfoW {
            cb_size: std::mem::size_of::<DocInfoW>() as i32,
            psz_doc_name: title_wide.as_ptr(),
            psz_output: std::ptr::null(),
            psz_datatype: std::ptr::null(),
            fw_type: 0,
        };

        let job_id = unsafe { StartDocW(hdc, &doc_info as *const _) };
        if job_id <= 0 {
            unsafe { DeleteDC(hdc) };
            return Err(PdfOffError::PrintFailed("StartDoc failed".into()));
        }

        // Inline BITMAPINFO-compatible struct (header + empty color table for 24-bit)
        #[repr(C)]
        struct BitmapInfo {
            header: BITMAPINFOHEADER,
            _colors: u32,
        }

        for _copy in 0..copies {
            for &page_idx in pages {
                let (rgb_pixels, img_w, img_h) =
                    renderer.render_for_print_raw(doc_manager, page_idx, dpi)?;

                // RGB → BGR for Windows GDI 24-bit DIB
                let mut bgr: Vec<u8> = Vec::with_capacity(rgb_pixels.len());
                for px in rgb_pixels.chunks_exact(3) {
                    bgr.push(px[2]);
                    bgr.push(px[1]);
                    bgr.push(px[0]);
                }

                let binfo = BitmapInfo {
                    header: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: img_w as i32,
                        biHeight: -(img_h as i32), // top-down
                        biPlanes: 1,
                        biBitCount: 24,
                        biCompression: BI_RGB.0,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    },
                    _colors: 0,
                };

                unsafe {
                    StartPage(hdc);
                    StretchDIBits(
                        hdc,
                        0,
                        0,
                        print_w,
                        print_h,
                        0,
                        0,
                        img_w as i32,
                        img_h as i32,
                        Some(bgr.as_ptr() as *const _),
                        &binfo as *const _ as *const _,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    EndPage(hdc);
                }
            }
        }

        unsafe {
            EndDoc(hdc);
            DeleteDC(hdc);
        }

        Ok(())
    }

    #[cfg(not(windows))]
    pub fn execute_print(
        _printer_name: &str,
        _pages: &[u32],
        _copies: u32,
        _doc_manager: &DocumentManager,
        _renderer: &Renderer,
        _dpi: f32,
        _doc_title: &str,
    ) -> Result<()> {
        Err(PdfOffError::PrintFailed(
            "Printing not supported on this platform".into(),
        ))
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
