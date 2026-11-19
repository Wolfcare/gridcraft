//! macOS: the Save panel with a File Format menu (Excel Workbook, CSV, Web Page, PDF), so Save As
//! can write a PDF directly. `rfd` gives the panel no format menu on macOS: a PDF could only be had
//! by typing `.pdf` at the end of the name, which nothing in the panel said.

use std::path::PathBuf;

use objc2::MainThreadMarker;
use objc2_app_kit::{NSModalResponseOK, NSSavePanel};
use objc2_foundation::{NSArray, NSString};
use objc2_uniform_type_identifiers::UTType;

/// Shows the Save panel suggesting `name`, with `extensions` in its File Format menu (the first
/// is chosen). `Some(None)` when the panel was cancelled; `None` when it could not be shown here
/// (off the main thread, or no known type), so the caller shows its plain panel instead.
pub fn pick(name: &str, extensions: &[&str]) -> Option<Option<PathBuf>> {
    let mtm = MainThreadMarker::new()?;
    let types: Vec<_> = extensions.iter().filter_map(|ext| UTType::typeWithFilenameExtension(&NSString::from_str(ext))).collect();
    if types.is_empty() {
        return None;
    }
    let panel = NSSavePanel::savePanel(mtm);
    panel.setAllowedContentTypes(&NSArray::from_retained_slice(&types));
    // The File Format menu is macOS 13's; earlier systems show the panel without it.
    if objc2::available!(macos = 13.0) {
        panel.setShowsContentTypes(true);
    }
    panel.setCanCreateDirectories(true);
    panel.setNameFieldStringValue(&NSString::from_str(name));
    if panel.runModal() != NSModalResponseOK {
        return Some(None);
    }
    Some(panel.URL().and_then(|url| url.path()).map(|path| PathBuf::from(path.to_string())))
}
