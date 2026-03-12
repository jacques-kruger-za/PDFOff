mod annotations;
mod document;
mod error;
mod forms;
mod navigator;
mod page_editor;
mod printer;
mod renderer;

use annotations::{AnnotationHandler, CreateAnnotationRequest, Annotation};
use document::{DocumentManager, DocumentMetadata, PageInfo, ViewState};
use error::PdfOffError;
use forms::{FormField, FormFieldUpdate, FormHandler};
use navigator::{FitMode, NavigationState, Navigator};
use page_editor::PageEditor;
use printer::{PrintSettings, Printer};
use renderer::{RenderedPage, Renderer};

use base64::Engine;
use tauri::State;

pub struct AppState {
    doc_manager: DocumentManager,
    renderer: Renderer,
    navigator: Navigator,
    printer: Printer,
    form_handler: FormHandler,
    annotation_handler: AnnotationHandler,
    page_editor: PageEditor,
}

// ── Phase 1: Document Management ──

#[tauri::command]
fn open_document(state: State<'_, AppState>, path: String) -> Result<DocumentMetadata, PdfOffError> {
    state.renderer.invalidate_cache();
    state.navigator.reset();
    state.page_editor.reset();
    state.annotation_handler.clear_local_cache();

    let metadata = state.doc_manager.open(&path)?;
    state
        .navigator
        .restore_state(metadata.page_count.min(1) - 1, 1.0);
    Ok(metadata)
}

#[tauri::command]
fn close_document(state: State<'_, AppState>) -> Result<(), PdfOffError> {
    state.renderer.invalidate_cache();
    state.navigator.reset();
    state.page_editor.reset();
    state.doc_manager.close()
}

#[tauri::command]
fn get_metadata(state: State<'_, AppState>) -> Result<DocumentMetadata, PdfOffError> {
    state.doc_manager.get_metadata()
}

#[tauri::command]
fn get_page_info(state: State<'_, AppState>, page_index: u32) -> Result<PageInfo, PdfOffError> {
    state.doc_manager.get_page_info(page_index)
}

#[tauri::command]
fn get_all_page_info(state: State<'_, AppState>) -> Result<Vec<PageInfo>, PdfOffError> {
    state.doc_manager.get_all_page_info()
}

#[tauri::command]
fn is_dirty(state: State<'_, AppState>) -> Result<bool, PdfOffError> {
    state.doc_manager.is_dirty()
}

// ── Phase 1: Rendering ──

#[tauri::command]
fn render_page(
    state: State<'_, AppState>,
    page_index: u32,
    zoom: f32,
) -> Result<RenderedPage, PdfOffError> {
    state.renderer.render_page(&state.doc_manager, page_index, zoom)
}

#[tauri::command]
fn render_thumbnail(
    state: State<'_, AppState>,
    page_index: u32,
    max_width: u32,
) -> Result<RenderedPage, PdfOffError> {
    state
        .renderer
        .render_thumbnail(&state.doc_manager, page_index, max_width)
}

// ── Phase 1: Navigation ──

#[tauri::command]
fn get_nav_state(state: State<'_, AppState>) -> Result<NavigationState, PdfOffError> {
    state.navigator.get_state(&state.doc_manager)
}

#[tauri::command]
fn go_to_page(state: State<'_, AppState>, page: u32) -> Result<u32, PdfOffError> {
    state.navigator.go_to_page(&state.doc_manager, page)
}

#[tauri::command]
fn next_page(state: State<'_, AppState>) -> Result<u32, PdfOffError> {
    state.navigator.next_page(&state.doc_manager)
}

#[tauri::command]
fn prev_page(state: State<'_, AppState>) -> Result<u32, PdfOffError> {
    state.navigator.prev_page(&state.doc_manager)
}

#[tauri::command]
fn first_page(state: State<'_, AppState>) -> Result<u32, PdfOffError> {
    state.navigator.first_page(&state.doc_manager)
}

#[tauri::command]
fn last_page(state: State<'_, AppState>) -> Result<u32, PdfOffError> {
    state.navigator.last_page(&state.doc_manager)
}

#[tauri::command]
fn zoom_in(state: State<'_, AppState>) -> f32 {
    state.navigator.zoom_in()
}

#[tauri::command]
fn zoom_out(state: State<'_, AppState>) -> f32 {
    state.navigator.zoom_out()
}

#[tauri::command]
fn set_zoom(state: State<'_, AppState>, level: f32) -> f32 {
    state.navigator.set_zoom(level)
}

#[tauri::command]
fn set_fit_mode(state: State<'_, AppState>, mode: String) {
    let fit_mode = match mode.as_str() {
        "fit-width" => FitMode::FitWidth,
        "fit-page" => FitMode::FitPage,
        "actual-size" => FitMode::ActualSize,
        _ => FitMode::Custom,
    };
    state.navigator.set_fit_mode(fit_mode);
}

#[tauri::command]
fn update_view_state(state: State<'_, AppState>, view_state: ViewState) -> Result<(), PdfOffError> {
    state.doc_manager.update_view_state(view_state)
}

// ── Phase 2: Printing ──

#[tauri::command]
fn parse_print_range(range_str: String, total_pages: u32) -> Result<Vec<u32>, PdfOffError> {
    Printer::parse_page_range(&range_str, total_pages)
}

#[tauri::command]
fn prepare_print(
    state: State<'_, AppState>,
    settings: PrintSettings,
) -> Result<Vec<String>, PdfOffError> {
    let data = state
        .printer
        .prepare_print_data(&state.doc_manager, &state.renderer, &settings)?;
    let base64_pages: Vec<String> = data
        .iter()
        .map(|d| base64::engine::general_purpose::STANDARD.encode(d))
        .collect();
    Ok(base64_pages)
}

#[tauri::command]
fn get_printers() -> (Vec<String>, Option<String>) {
    Printer::enumerate_printers()
}

#[tauri::command]
fn execute_print(
    state: State<'_, AppState>,
    printer_name: String,
    pages: Vec<u32>,
    copies: u32,
    dpi: f32,
) -> Result<(), PdfOffError> {
    let title = state
        .doc_manager
        .with_document(|doc| Ok(doc.metadata.file_name.clone()))
        .unwrap_or_else(|_| "PDFOff Document".to_string());

    Printer::execute_print(
        &printer_name,
        &pages,
        copies,
        &state.doc_manager,
        &state.renderer,
        dpi,
        &title,
    )
}

// ── Phase 3: Forms ──

#[tauri::command]
fn get_form_fields(state: State<'_, AppState>) -> Result<Vec<FormField>, PdfOffError> {
    state.form_handler.get_form_fields(&state.doc_manager)
}

#[tauri::command]
fn set_form_field(
    state: State<'_, AppState>,
    update: FormFieldUpdate,
) -> Result<(), PdfOffError> {
    state.form_handler.set_field_value(&state.doc_manager, &update)?;
    state.renderer.invalidate_page(update.page_index);
    Ok(())
}

#[tauri::command]
fn has_forms(state: State<'_, AppState>) -> Result<bool, PdfOffError> {
    state.form_handler.has_forms(&state.doc_manager)
}

#[tauri::command]
fn clear_form_fields(state: State<'_, AppState>) -> Result<(), PdfOffError> {
    state.form_handler.clear_all_fields(&state.doc_manager)
}

// ── Phase 4: Annotations ──

#[tauri::command]
fn get_annotations(state: State<'_, AppState>) -> Result<Vec<Annotation>, PdfOffError> {
    state.annotation_handler.get_annotations(&state.doc_manager)
}

#[tauri::command]
fn get_page_annotations(
    state: State<'_, AppState>,
    page_index: u32,
) -> Result<Vec<Annotation>, PdfOffError> {
    state
        .annotation_handler
        .get_annotations_for_page(&state.doc_manager, page_index)
}

#[tauri::command]
fn create_annotation(
    state: State<'_, AppState>,
    request: CreateAnnotationRequest,
) -> Result<Annotation, PdfOffError> {
    let annot = state
        .annotation_handler
        .create_annotation(&state.doc_manager, &request)?;
    state.renderer.invalidate_page(request.page_index);
    Ok(annot)
}

#[tauri::command]
fn delete_annotation(
    state: State<'_, AppState>,
    page_index: u32,
    annotation_index: usize,
) -> Result<(), PdfOffError> {
    state
        .annotation_handler
        .delete_annotation(&state.doc_manager, page_index, annotation_index)?;
    state.renderer.invalidate_page(page_index);
    Ok(())
}

// ── Phase 5: Page Editing ──

#[tauri::command]
fn delete_pdf_page(state: State<'_, AppState>, page_index: u32) -> Result<(), PdfOffError> {
    state.page_editor.delete_page(&state.doc_manager, page_index)?;
    state.renderer.invalidate_cache();
    Ok(())
}

#[tauri::command]
fn insert_blank_page(
    state: State<'_, AppState>,
    after_page: u32,
    width: f32,
    height: f32,
) -> Result<(), PdfOffError> {
    state
        .page_editor
        .insert_blank_page(&state.doc_manager, after_page, width, height)?;
    state.renderer.invalidate_cache();
    Ok(())
}

#[tauri::command]
fn rotate_page(
    state: State<'_, AppState>,
    page_index: u32,
    degrees: i32,
) -> Result<(), PdfOffError> {
    state
        .page_editor
        .rotate_page(&state.doc_manager, page_index, degrees)?;
    state.renderer.invalidate_page(page_index);
    Ok(())
}

#[tauri::command]
fn extract_pages(
    state: State<'_, AppState>,
    pages: Vec<u32>,
    output_path: String,
) -> Result<(), PdfOffError> {
    state
        .page_editor
        .extract_pages(&state.doc_manager, &pages, &output_path)
}

#[tauri::command]
fn merge_document(
    state: State<'_, AppState>,
    source_path: String,
    insert_at: u32,
) -> Result<u32, PdfOffError> {
    let count = state
        .page_editor
        .merge_document(&state.doc_manager, &source_path, insert_at)?;
    state.renderer.invalidate_cache();
    Ok(count)
}

#[tauri::command]
fn save_document(state: State<'_, AppState>) -> Result<(), PdfOffError> {
    state.page_editor.save_document(&state.doc_manager)
}

#[tauri::command]
fn save_document_as(
    state: State<'_, AppState>,
    output_path: String,
) -> Result<(), PdfOffError> {
    state
        .page_editor
        .save_document_as(&state.doc_manager, &output_path)
}

#[tauri::command]
fn undo_page_edit(state: State<'_, AppState>) -> Result<bool, PdfOffError> {
    let result = state.page_editor.undo(&state.doc_manager)?;
    if result {
        state.renderer.invalidate_cache();
    }
    Ok(result)
}

#[tauri::command]
fn redo_page_edit(state: State<'_, AppState>) -> Result<bool, PdfOffError> {
    let result = state.page_editor.redo(&state.doc_manager)?;
    if result {
        state.renderer.invalidate_cache();
    }
    Ok(result)
}

#[tauri::command]
fn can_undo(state: State<'_, AppState>) -> bool {
    state.page_editor.can_undo()
}

#[tauri::command]
fn can_redo(state: State<'_, AppState>) -> bool {
    state.page_editor.can_redo()
}

// ── App entry point ──

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(AppState {
            doc_manager: DocumentManager::new(),
            renderer: Renderer::new(),
            navigator: Navigator::new(),
            printer: Printer::new(),
            form_handler: FormHandler::new(),
            annotation_handler: AnnotationHandler::new(),
            page_editor: PageEditor::new(),
        })
        .invoke_handler(tauri::generate_handler![
            // Phase 1: Document
            open_document,
            close_document,
            get_metadata,
            get_page_info,
            get_all_page_info,
            is_dirty,
            // Phase 1: Rendering
            render_page,
            render_thumbnail,
            // Phase 1: Navigation
            get_nav_state,
            go_to_page,
            next_page,
            prev_page,
            first_page,
            last_page,
            zoom_in,
            zoom_out,
            set_zoom,
            set_fit_mode,
            update_view_state,
            // Phase 2: Printing
            parse_print_range,
            prepare_print,
            get_printers,
            execute_print,
            // Phase 3: Forms
            get_form_fields,
            set_form_field,
            has_forms,
            clear_form_fields,
            // Phase 4: Annotations
            get_annotations,
            get_page_annotations,
            create_annotation,
            delete_annotation,
            // Phase 5: Page Editing
            delete_pdf_page,
            insert_blank_page,
            rotate_page,
            extract_pages,
            merge_document,
            save_document,
            save_document_as,
            undo_page_edit,
            redo_page_edit,
            can_undo,
            can_redo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PDFOff");
}
