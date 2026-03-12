use crate::document::DocumentManager;
use crate::error::{PdfOffError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavigationState {
    pub current_page: u32,
    pub total_pages: u32,
    pub zoom_level: f32,
    pub fit_mode: FitMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FitMode {
    FitWidth,
    FitPage,
    ActualSize,
    Custom,
}

const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;
const ZOOM_STEP: f32 = 0.10;

pub struct Navigator {
    current_page: std::sync::Mutex<u32>,
    zoom_level: std::sync::Mutex<f32>,
    fit_mode: std::sync::Mutex<FitMode>,
}

impl Navigator {
    pub fn new() -> Self {
        Self {
            current_page: std::sync::Mutex::new(0),
            zoom_level: std::sync::Mutex::new(1.0),
            fit_mode: std::sync::Mutex::new(FitMode::FitWidth),
        }
    }

    pub fn get_state(&self, doc_manager: &DocumentManager) -> Result<NavigationState> {
        let total_pages = doc_manager.with_document(|doc| Ok(doc.metadata.page_count))?;
        Ok(NavigationState {
            current_page: *self.current_page.lock().unwrap(),
            total_pages,
            zoom_level: *self.zoom_level.lock().unwrap(),
            fit_mode: self.fit_mode.lock().unwrap().clone(),
        })
    }

    pub fn go_to_page(&self, doc_manager: &DocumentManager, page: u32) -> Result<u32> {
        let total = doc_manager.with_document(|doc| Ok(doc.metadata.page_count))?;
        if page >= total {
            return Err(PdfOffError::InvalidPage(page, total));
        }
        *self.current_page.lock().unwrap() = page;
        doc_manager.with_document_mut(|doc| {
            doc.view_state.current_page = page;
            Ok(())
        })?;
        Ok(page)
    }

    pub fn next_page(&self, doc_manager: &DocumentManager) -> Result<u32> {
        let total = doc_manager.with_document(|doc| Ok(doc.metadata.page_count))?;
        let mut current = self.current_page.lock().unwrap();
        if *current + 1 < total {
            *current += 1;
            let page = *current;
            drop(current);
            doc_manager.with_document_mut(|doc| {
                doc.view_state.current_page = page;
                Ok(())
            })?;
            Ok(page)
        } else {
            Ok(*current)
        }
    }

    pub fn prev_page(&self, doc_manager: &DocumentManager) -> Result<u32> {
        let mut current = self.current_page.lock().unwrap();
        if *current > 0 {
            *current -= 1;
            let page = *current;
            drop(current);
            doc_manager.with_document_mut(|doc| {
                doc.view_state.current_page = page;
                Ok(())
            })?;
            Ok(page)
        } else {
            Ok(0)
        }
    }

    pub fn first_page(&self, doc_manager: &DocumentManager) -> Result<u32> {
        self.go_to_page(doc_manager, 0)
    }

    pub fn last_page(&self, doc_manager: &DocumentManager) -> Result<u32> {
        let total = doc_manager.with_document(|doc| Ok(doc.metadata.page_count))?;
        self.go_to_page(doc_manager, total.saturating_sub(1))
    }

    pub fn zoom_in(&self) -> f32 {
        let mut zoom = self.zoom_level.lock().unwrap();
        *zoom = (*zoom + ZOOM_STEP).min(MAX_ZOOM);
        *self.fit_mode.lock().unwrap() = FitMode::Custom;
        *zoom
    }

    pub fn zoom_out(&self) -> f32 {
        let mut zoom = self.zoom_level.lock().unwrap();
        *zoom = (*zoom - ZOOM_STEP).max(MIN_ZOOM);
        *self.fit_mode.lock().unwrap() = FitMode::Custom;
        *zoom
    }

    pub fn set_zoom(&self, level: f32) -> f32 {
        let clamped = level.clamp(MIN_ZOOM, MAX_ZOOM);
        *self.zoom_level.lock().unwrap() = clamped;
        *self.fit_mode.lock().unwrap() = FitMode::Custom;
        clamped
    }

    pub fn set_fit_mode(&self, mode: FitMode) {
        *self.fit_mode.lock().unwrap() = mode;
    }

    pub fn get_zoom(&self) -> f32 {
        *self.zoom_level.lock().unwrap()
    }

    pub fn get_current_page(&self) -> u32 {
        *self.current_page.lock().unwrap()
    }

    pub fn reset(&self) {
        *self.current_page.lock().unwrap() = 0;
        *self.zoom_level.lock().unwrap() = 1.0;
        *self.fit_mode.lock().unwrap() = FitMode::FitWidth;
    }

    pub fn restore_state(&self, page: u32, zoom: f32) {
        *self.current_page.lock().unwrap() = page;
        *self.zoom_level.lock().unwrap() = zoom;
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigator_defaults() {
        let nav = Navigator::new();
        assert_eq!(nav.get_current_page(), 0);
        assert!((nav.get_zoom() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zoom_in_out() {
        let nav = Navigator::new();
        let z = nav.zoom_in();
        assert!(z > 1.0);
        let z2 = nav.zoom_out();
        assert!((z2 - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_zoom_clamp() {
        let nav = Navigator::new();
        let z = nav.set_zoom(100.0);
        assert!((z - MAX_ZOOM).abs() < f32::EPSILON);
        let z = nav.set_zoom(-1.0);
        assert!((z - MIN_ZOOM).abs() < f32::EPSILON);
    }

    #[test]
    fn test_reset() {
        let nav = Navigator::new();
        nav.zoom_in();
        nav.reset();
        assert_eq!(nav.get_current_page(), 0);
        assert!((nav.get_zoom() - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_restore_state() {
        let nav = Navigator::new();
        nav.restore_state(5, 2.0);
        assert_eq!(nav.get_current_page(), 5);
        assert!((nav.get_zoom() - 2.0).abs() < f32::EPSILON);
    }
}
