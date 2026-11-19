//! macOS: the native menu bar (File, Edit, View, Insert, Format, Tools, Data, Window, Help), as
//! a desktop spreadsheet has on the Mac. Items run the same commands as the ribbon.

use std::str::FromStr;
use std::sync::mpsc::{Receiver, channel};

use gridcraft_ui_egui::{OpenWindow, SheetApp};
use muda::accelerator::Accelerator;
use muda::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;
use serde_json::{Value, json};

/// A menu bar whose Edit menu sends AppKit's standard text actions (`undo:`, `cut:`, `copy:`,
/// `paste:`, `selectAll:`) to the first responder.
fn text_editing_menu() -> Menu {
    let menu = Menu::new();
    let app_menu = Submenu::new("GridCraft", true);
    let _ = app_menu.append_items(&[&PredefinedMenuItem::hide(None), &PredefinedMenuItem::hide_others(None), &PredefinedMenuItem::show_all(None)]);
    let edit = Submenu::new("Edit", true);
    let _ = edit.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::select_all(None),
    ]);
    let _ = menu.append_items(&[&app_menu, &edit]);
    menu
}

/// Runs `show`, which presents a native Open or Save panel, with a standard Edit menu in the menu
/// bar, then puts the previous menu bar back.
///
/// A panel's file name field takes ⌘V, ⌘C, ⌘X, ⌘A and ⌘Z only through the main menu's key
/// equivalents for `paste:` and the other text actions. GridCraft's own Edit items run worksheet
/// commands instead, so without this a name could not be pasted into the Save panel.
pub fn with_text_editing_menu<T>(show: impl FnOnce() -> T) -> T {
    let Some(mtm) = MainThreadMarker::new() else {
        return show();
    };
    let nsapp = NSApplication::sharedApplication(mtm);
    let previous = nsapp.mainMenu();
    let menu = text_editing_menu();
    menu.init_for_nsapp();
    let picked = show();
    nsapp.setMainMenu(previous.as_deref());
    picked
}

type Entry = (&'static str, &'static str, Option<&'static str>);

/// (label, command or `dialog:name`, accelerator). `-` is a separator.
fn tree() -> Vec<(&'static str, Vec<Entry>)> {
    vec![
        (
            "File",
            vec![
                ("New Workbook", "file.new", Some("CMD+N")),
                ("New from Sample…", "dialog:start", None),
                ("Open…", "file.open", Some("CMD+O")),
                ("Open Recent", RECENT_MENU, None),
                ("-", "", None),
                ("Close", "file.close", Some("CMD+W")),
                ("Save", "file.save", Some("CMD+S")),
                ("Save As…", "file.saveAs", Some("CMD+SHIFT+S")),
                ("Export as PDF…", "file.exportPdf", None),
                ("Export as CSV…", "dialog:saveCopy:csv", None),
                ("Save as Web Page…", "dialog:saveCopy:html", None),
                ("-", "", None),
                ("Page Setup…", "dialog:pageSetup", None),
                ("Print…", "file.print", Some("CMD+P")),
                ("-", "", None),
                ("Properties…", "dialog:backstage:info", None),
            ],
        ),
        (
            "Edit",
            vec![
                ("Undo", "edit.undo", None),
                ("Redo", "edit.redo", None),
                ("-", "", None),
                ("Cut", "edit.cut", None),
                ("Copy", "edit.copy", None),
                ("Paste", "edit.paste", None),
                ("Paste Special…", "dialog:pasteSpecial", Some("CTRL+CMD+V")),
                ("-", "", None),
                ("Fill Down", "edit.fillDown", None),
                ("Fill Right", "edit.fillRight", None),
                ("Series…", "dialog:series", None),
                ("Flash Fill", "edit.flashFill", None),
                ("-", "", None),
                ("Clear All", "edit.clearAll", None),
                ("Clear Formats", "edit.clearFormats", None),
                ("Clear Contents", "edit.clearContents", None),
                ("-", "", None),
                ("Delete…", "dialog:deleteCells", None),
                ("Delete Sheet", "home.deleteSheet", None),
                ("-", "", None),
                ("Find…", "dialog:find", None),
                ("Replace…", "dialog:find:replace", Some("CTRL+H")),
                ("Go To…", "dialog:goTo", Some("CTRL+G")),
            ],
        ),
        (
            "View",
            vec![
                ("Normal", "view.normal", None),
                ("Page Layout", "view.pageLayout", None),
                ("Page Break Preview", "view.pageBreakPreview", None),
                ("-", "", None),
                ("Formula Bar", "view.formulaBar", None),
                ("Gridlines", "view.gridlines", None),
                ("Headings", "view.headings", None),
                ("-", "", None),
                ("Zoom…", "dialog:zoom", None),
                ("Zoom to Selection", "view.zoomToSelection", None),
                ("Freeze Panes", "view.freezePanes", None),
                ("Freeze Top Row", "view.freezeTopRow", None),
                ("Unfreeze Panes", "view.unfreezePanes", None),
                ("-", "", None),
                ("Display Theme", "view.theme", None),
                ("Collapse Ribbon", "view.collapseRibbon", Some("CMD+ALT+R")),
            ],
        ),
        (
            "Insert",
            vec![
                ("Cells…", "dialog:insertCells", None),
                ("Rows", "home.insertRows", None),
                ("Columns", "home.insertColumns", None),
                ("Sheet", "home.insertSheet", Some("SHIFT+F11")),
                ("-", "", None),
                ("Chart", "insert.recommendedCharts", None),
                ("Sparklines…", "dialog:sparkline", None),
                ("PivotTable", "insert.pivotTable", None),
                ("Table", "insert.table", None),
                ("-", "", None),
                ("Function…", "dialog:insertFunction", Some("SHIFT+F3")),
                ("Name…", "dialog:defineName", None),
                ("New Comment", "dialog:comment", None),
                ("New Note", "dialog:note", None),
                ("Picture…", "dialog:insertPicture", None),
                ("Text Box", "insert.textBox", None),
                ("Link…", "dialog:insertLink", None),
                ("Checkbox", "insert.checkbox", None),
            ],
        ),
        (
            "Format",
            vec![
                ("Cells…", "dialog:formatCells", None),
                ("Row Height…", "dialog:rowHeight", None),
                ("AutoFit Row Height", "home.autofitRowHeight", None),
                ("Column Width…", "dialog:columnWidth", None),
                ("AutoFit Column Width", "home.autofitColumnWidth", None),
                ("-", "", None),
                ("Hide Rows", "home.hideRows", None),
                ("Unhide Rows", "home.unhideRows", None),
                ("Hide Columns", "home.hideColumns", None),
                ("Unhide Columns", "home.unhideColumns", None),
                ("-", "", None),
                ("Rename Sheet…", "dialog:renameSheet", None),
                ("Hide Sheet", "sheet.hide", None),
                ("Unhide Sheet…", "sheet.unhide", None),
                ("-", "", None),
                ("Conditional Formatting…", "dialog:manageRules", None),
                ("Format as Table", "home.formatAsTable", None),
            ],
        ),
        (
            "Tools",
            vec![
                ("Spelling…", "dialog:spelling", None),
                ("Workbook Statistics", "dialog:statistics", None),
                ("Check Accessibility", "dialog:accessibility", None),
                ("-", "", None),
                ("Protect Sheet…", "dialog:protectSheet", None),
                ("Protect Workbook", "review.protectWorkbook", None),
                ("-", "", None),
                ("Goal Seek…", "dialog:goalSeek", None),
                ("Error Checking…", "dialog:errorChecking", None),
                ("Evaluate Formula…", "dialog:evaluateFormula", None),
                ("-", "", None),
                ("Record Actions", "automate.recordActions", None),
                ("Command Palette…", "dialog:commandSearch", Some("CMD+SHIFT+P")),
            ],
        ),
        (
            "Data",
            vec![
                ("Sort…", "dialog:sort", None),
                ("Sort A to Z", "data.sortAscending", None),
                ("Sort Z to A", "data.sortDescending", None),
                ("Filter", "data.filter", None),
                ("Clear Filter", "data.clearFilter", None),
                ("-", "", None),
                ("Text to Columns…", "dialog:textToColumns", None),
                ("Remove Duplicates…", "dialog:removeDuplicates", None),
                ("Data Validation…", "dialog:dataValidation", None),
                ("-", "", None),
                ("Group", "data.group", None),
                ("Ungroup", "data.ungroup", None),
                ("Show Detail", "data.showDetail", None),
                ("Hide Detail", "data.hideDetail", None),
                ("Subtotal…", "dialog:subtotal", None),
                ("-", "", None),
                ("Calculate Now", "formulas.calculateNow", None),
            ],
        ),
        // The open workbooks follow the separator (`NativeMenu::sync_windows`).
        (
            "Window",
            vec![
                ("New Window", "view.newWindow", None),
                ("Next Sheet", "sheet.next", None),
                ("Previous Sheet", "sheet.previous", None),
                ("-", "", None),
            ],
        ),
        (
            "Help",
            vec![
                ("Check for Updates…", "app.checkForUpdates", None),
                ("Agent Control (MCP)…", "dialog:agents", None),
                ("Contributors…", "dialog:about:Contributors", None),
                ("GridCraft on getartcraft.com", "url:https://getartcraft.com/apps/gridcraft", None),
                ("Join the ArtCraft Discord…", "url:https://discord.gg/artcraft", None),
            ],
        ),
    ]
}

pub struct NativeMenu {
    _menu: Menu,
    events: Receiver<MenuEvent>,
    theme_items: Vec<(&'static str, CheckMenuItem)>,
    window_menu: Option<Submenu>,
    /// The Window menu's window items and what they last showed.
    window_items: Vec<CheckMenuItem>,
    windows: Vec<OpenWindow>,
    /// File › Open Recent, and the paths it last listed.
    recent_menu: Option<Submenu>,
    recent: Option<Vec<String>>,
}

/// Marks where File › Open Recent goes in [`tree`].
const RECENT_MENU: &str = "menu:openRecent";
/// Menu id of the Open Recent item that opens recent workbook `n` (0 = the most recent).
const RECENT_ITEM: &str = "recent:";
/// Open Recent › Clear Menu.
const RECENT_CLEAR: &str = "recent:clear";
/// How many workbooks Open Recent lists.
const RECENT_SHOWN: usize = 10;

/// The Open Recent labels for `paths`: the file name, and its folder too where two listed
/// workbooks share a name.
fn recent_labels(paths: &[String]) -> Vec<String> {
    let split = |p: &str| -> (String, String) {
        let path = std::path::Path::new(p);
        let name = path.file_name().map_or_else(|| p.to_string(), |n| n.to_string_lossy().into_owned());
        let folder = path.parent().and_then(|f| f.file_name()).map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
        (name, folder)
    };
    let parts: Vec<(String, String)> = paths.iter().map(|p| split(p)).collect();
    parts
        .iter()
        .map(|(name, folder)| {
            let shared = parts.iter().filter(|(other, _)| other == name).count() > 1;
            if shared && !folder.is_empty() { format!("{name} — {folder}") } else { name.clone() }
        })
        .collect()
}

/// Menu id of the Window menu item that brings workbook `doc` forward.
const WINDOW_ITEM: &str = "window:id:";

/// The Quit item: asks about each unsaved workbook in its window first
/// (`SheetApp::request_quit`), which AppKit's own Quit (terminate:) would skip.
const QUIT: &str = "app.quit";

fn select_theme(app: &mut SheetApp, id: &str) -> bool {
    let mode = match id {
        "theme:system" => "system",
        "theme:light" => "light",
        "theme:dark" => "dark",
        _ => return false,
    };
    app.run_or_alert("view.theme", json!({"mode": mode}));
    true
}

impl NativeMenu {
    pub fn install(ctx: &egui::Context) -> NativeMenu {
        let menu = Menu::new();
        let app_menu = Submenu::new("GridCraft", true);
        let about = MenuItem::with_id("dialog:about", "About GridCraft", true, None);
        let _ = app_menu.append_items(&[
            &about,
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::services(None),
            &PredefinedMenuItem::separator(),
            &PredefinedMenuItem::hide(None),
            &PredefinedMenuItem::hide_others(None),
            &PredefinedMenuItem::show_all(None),
            &PredefinedMenuItem::separator(),
            &MenuItem::with_id(QUIT, "Quit GridCraft", true, Accelerator::from_str("CMD+Q").ok()),
        ]);
        let _ = menu.append(&app_menu);
        let mut theme_items = Vec::new();
        let mut window_menu = None;
        let mut recent_menu = None;
        for (title, entries) in tree() {
            let sub = Submenu::new(title, true);
            for (label, id, acc) in entries {
                if label == "-" {
                    let _ = sub.append(&PredefinedMenuItem::separator());
                    continue;
                }
                if id == RECENT_MENU {
                    let recent = Submenu::new(label, true);
                    let _ = sub.append(&recent);
                    recent_menu = Some(recent);
                    continue;
                }
                if id == "view.theme" {
                    let themes = Submenu::new(label, true);
                    for (mode, label) in [("system", "System"), ("light", "Light"), ("dark", "Dark")] {
                        let item = CheckMenuItem::with_id(format!("theme:{mode}"), label, true, false, None);
                        let _ = themes.append(&item);
                        theme_items.push((mode, item));
                    }
                    let _ = sub.append(&themes);
                    continue;
                }
                let accel = acc.and_then(|a| Accelerator::from_str(a).ok());
                let item = MenuItem::with_id(id, label, true, accel);
                let _ = sub.append(&item);
            }
            let _ = menu.append(&sub);
            if title == "Window" {
                window_menu = Some(sub);
            }
        }
        menu.init_for_nsapp();
        // NSApp's windowsMenu is where macOS adds Move & Resize, Fill and Center, the targets of the
        // system tiling shortcuts (Fn+Ctrl+arrows, Fn+F). Only once the menu is NSApp's: muda looks
        // the submenu up in the installed main menu.
        if let Some(window) = &window_menu {
            window.set_as_windows_menu_for_nsapp();
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        MenuEvent::set_event_handler(Some(move |e: MenuEvent| {
            let _ = tx.send(e);
            ctx.request_repaint();
        }));
        NativeMenu { _menu: menu, events: rx, theme_items, window_menu, window_items: Vec::new(), windows: Vec::new(), recent_menu, recent: None }
    }

    /// Keeps the Window menu's list of open windows (title, unsaved bullet, active check) in
    /// step with the session.
    fn sync_windows(&mut self, app: &SheetApp) {
        let windows = app.open_windows();
        if windows != self.windows
            && let Some(menu) = &self.window_menu
        {
            let same_windows = windows.len() == self.windows.len() && windows.iter().zip(&self.windows).all(|(a, b)| a.window == b.window);
            if same_windows {
                for (item, w) in self.window_items.iter().zip(&windows) {
                    item.set_text(w.label());
                }
            } else {
                for item in self.window_items.drain(..) {
                    let _ = menu.remove(&item);
                }
                for w in &windows {
                    let item = CheckMenuItem::with_id(format!("{WINDOW_ITEM}{}", w.window), w.label(), true, w.active, None);
                    let _ = menu.append(&item);
                    self.window_items.push(item);
                }
            }
            self.windows = windows;
        }
        // AppKit flips a check item when it is chosen, so the active check is put back every poll.
        for (item, w) in self.window_items.iter().zip(&self.windows) {
            if item.is_checked() != w.active {
                item.set_checked(w.active);
            }
        }
    }

    /// Keeps File › Open Recent in step with the recent workbooks (the Backstage's list).
    fn sync_recent(&mut self, app: &SheetApp) {
        let Some(menu) = &self.recent_menu else { return };
        let paths: Vec<String> = app.ui.recent.iter().take(RECENT_SHOWN).cloned().collect();
        if self.recent.as_ref() == Some(&paths) {
            return;
        }
        for _ in 0..menu.items().len() {
            let _ = menu.remove_at(0);
        }
        for (i, label) in recent_labels(&paths).into_iter().enumerate() {
            let _ = menu.append(&MenuItem::with_id(format!("{RECENT_ITEM}{i}"), label, true, None));
        }
        if !paths.is_empty() {
            let _ = menu.append(&PredefinedMenuItem::separator());
        }
        let _ = menu.append(&MenuItem::with_id(RECENT_CLEAR, "Clear Menu", !paths.is_empty(), None));
        self.recent = Some(paths);
    }

    /// Runs an Open Recent item; false when `id` is not one.
    fn run_recent(&mut self, app: &mut SheetApp, id: &str) -> bool {
        if id == RECENT_CLEAR {
            app.ui.recent.clear();
            return true;
        }
        let Some(n) = id.strip_prefix(RECENT_ITEM).and_then(|n| n.parse::<usize>().ok()) else { return false };
        // The item opens the path it showed, even if the list has changed since.
        if let Some(path) = self.recent.as_ref().and_then(|r| r.get(n)).cloned() {
            app.open_path(&path);
        }
        true
    }

    /// Runs the chosen menu items. Commands go to the active window, the most recently focused;
    /// `viewport` is its egui viewport, for the items that act on its input.
    pub fn poll(&mut self, app: &mut SheetApp, ctx: &egui::Context, viewport: egui::ViewportId) {
        while let Ok(ev) = self.events.try_recv() {
            let id = ev.id.as_ref().to_string();
            if id == QUIT {
                app.request_quit(ctx);
                continue;
            }
            if let Some(window) = id.strip_prefix(WINDOW_ITEM).and_then(|w| w.parse::<u64>().ok()) {
                app.run_or_alert("view.switchWindows", json!({"window": window}));
                continue;
            }
            // Text selection belongs to the inline editor, not the worksheet clipboard/undo.
            if app.text_box_editor.is_some() {
                let clipboard = match id.as_str() {
                    "edit.copy" => Some(egui::ViewportCommand::RequestCopy),
                    "edit.cut" => Some(egui::ViewportCommand::RequestCut),
                    "edit.paste" => Some(egui::ViewportCommand::RequestPaste),
                    _ => None,
                };
                if let Some(command) = clipboard {
                    ctx.send_viewport_cmd_to(viewport, command);
                    continue;
                }
                if id == "edit.undo" || id == "edit.redo" {
                    ctx.input_mut_for(viewport, |i| {
                        i.events.push(egui::Event::Key {
                            key: egui::Key::Z,
                            physical_key: None,
                            pressed: true,
                            repeat: false,
                            modifiers: egui::Modifiers { command: true, mac_cmd: true, shift: id == "edit.redo", ..Default::default() },
                        });
                    });
                    continue;
                }
            }
            if self.run_recent(app, &id) {
                continue;
            }
            if select_theme(app, &id) {
                continue;
            }
            if let Some(url) = id.strip_prefix("url:") {
                if let Some(open) = &app.services.open_url {
                    open(url);
                }
            } else if let Some(rest) = id.strip_prefix("dialog:") {
                let (name, arg) = rest.split_once(':').unwrap_or((rest, ""));
                let params: Value = match (name, arg) {
                    ("saveCopy", fmt) => json!({"format": fmt}),
                    ("find", "replace") => json!({"replace": true}),
                    ("note", _) => json!({"threaded": false}),
                    ("comment", _) => json!({"threaded": true}),
                    ("about", tab) => json!({"tab": tab}),
                    ("backstage", page) => json!({"page": page}),
                    _ => json!({}),
                };
                let name = if name == "note" { "comment" } else { name };
                app.open_dialog(name, params);
            } else if id == "edit.copy" || id == "edit.cut" {
                app.copy_to_clipboard(ctx, &id);
            } else if id == "edit.paste" {
                app.paste_from_clipboard();
            } else {
                app.run_or_alert(&id, json!({}));
            }
        }
        for (mode, item) in &self.theme_items {
            let checked = app.ui.theme_mode() == *mode;
            if item.is_checked() != checked {
                item.set_checked(checked);
            }
        }
        self.sync_windows(app);
        self.sync_recent(app);
    }
}

#[cfg(test)]
mod tests {
    // No `use super::*`: tests/mac_menu_appkit.rs builds this file with cfg(test) but no test
    // harness, so the #[test] functions are left out and a module-level import would be unused.

    #[test]
    fn file_menu_has_export_as_pdf_dispatching_the_command() {
        let tree = super::tree();
        let (_, file) = tree.iter().find(|(title, _)| *title == "File").expect("File menu");
        let item = file.iter().find(|e| e.0 == "Export as PDF…").expect("Export as PDF… in the File menu");
        assert_eq!(item.1, "file.exportPdf");
        assert!(gridcraft_engine::find_command("file.exportPdf").is_some());
    }

    #[test]
    fn file_menu_lists_recent_workbooks_after_open() {
        let tree = super::tree();
        let (_, file) = tree.iter().find(|(title, _)| *title == "File").expect("File menu");
        let open = file.iter().position(|e| e.1 == "file.open").expect("Open… in the File menu");
        assert_eq!(file.get(open + 1).map(|e| e.1), Some(super::RECENT_MENU), "Open Recent follows Open…");
    }

    #[test]
    fn recent_labels_name_the_folder_only_where_names_repeat() {
        let paths = ["/a/Invoices/Rechnung.xlsx", "/b/Old/Rechnung.xlsx", "/a/Budget.xlsx", "Loose.csv"].map(String::from);
        assert_eq!(super::recent_labels(&paths), ["Rechnung.xlsx — Invoices", "Rechnung.xlsx — Old", "Budget.xlsx", "Loose.csv"]);
    }

    #[test]
    fn help_menu_checks_for_updates_through_the_registered_command() {
        let tree = super::tree();
        let (_, help) = tree.iter().find(|(title, _)| *title == "Help").expect("Help menu");
        let item = help.iter().find(|e| e.0 == "Check for Updates…").expect("Check for Updates… in the Help menu");
        assert_eq!(item.1, "app.checkForUpdates");
        let mut app = super::SheetApp::new(gridcraft_engine::Session::new(), Default::default());
        assert!(app.run(item.1, serde_json::json!({})).is_ok());
    }

    #[test]
    fn theme_choices_use_the_shared_preference_command() {
        let mut app = super::SheetApp::new(gridcraft_engine::Session::new(), Default::default());
        for mode in ["dark", "system", "light"] {
            assert!(super::select_theme(&mut app, &format!("theme:{mode}")));
            assert_eq!(app.ui.theme_mode(), mode);
            assert!(app.message.is_none());
        }
        assert!(!super::select_theme(&mut app, "view.formulaBar"));
        assert_eq!(app.ui.theme_mode(), "light");
    }
}
