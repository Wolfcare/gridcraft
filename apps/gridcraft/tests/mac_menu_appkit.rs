//! The macOS menu bar as AppKit sees it, on the main thread (`harness = false`): install it through
//! the real muda backend (`src/native_menu.rs`) and read NSApp back. The Window menu must be NSApp's
//! `windowsMenu`: that is where macOS adds Move & Resize, Fill and Center, the targets of the system
//! window tiling shortcuts (Fn+Ctrl+arrows, Fn+F), and lists the open windows (#716).

// The app's own module, built into this test (cargo builds harness = false tests with cfg(test), so its
// unit tests compile here too without a harness to run them).
#[cfg(target_os = "macos")]
#[path = "../src/native_menu.rs"]
mod native_menu;

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let mtm = MainThreadMarker::new().expect("harness = false runs on the main thread");
    let nsapp = NSApplication::sharedApplication(mtm);
    nsapp.finishLaunching();

    let ctx = egui::Context::default();
    let mut menu = native_menu::NativeMenu::install(&ctx);

    let bar = nsapp.mainMenu().expect("a main menu");
    let tops: Vec<String> = (0..bar.numberOfItems()).filter_map(|i| bar.itemAtIndex(i)?.submenu()).map(|m| m.title().to_string()).collect();
    assert_eq!(tops, ["GridCraft", "File", "Edit", "View", "Insert", "Format", "Tools", "Data", "Window", "Help"]);
    let windows_menu = nsapp.windowsMenu().expect("AppKit knows the Window menu");
    assert_eq!(windows_menu.title().to_string(), "Window");

    // A poll lists the open workbook windows at the end of the Window menu (#716).
    let mut session = gridcraft_engine::Session::new();
    session.new_workbook();
    let mut app = gridcraft_ui_egui::SheetApp::new(session, Default::default());
    menu.poll(&mut app, &ctx, egui::ViewportId::ROOT);
    let open = app.open_windows();
    assert_eq!(open.len(), 1, "one workbook window");
    let last = windows_menu.itemAtIndex(windows_menu.numberOfItems() - 1).expect("a Window menu item");
    assert_eq!(last.title().to_string(), open[0].label(), "the Window menu lists the open workbook");

    // File › Open Recent lists the recent workbooks, newest first, and follows the list.
    let file = (0..bar.numberOfItems()).filter_map(|i| bar.itemAtIndex(i)?.submenu()).find(|m| m.title().to_string() == "File").expect("a File menu");
    let titles = |m: &objc2_app_kit::NSMenu| -> Vec<String> {
        (0..m.numberOfItems()).filter_map(|i| m.itemAtIndex(i)).map(|it| it.title().to_string()).collect()
    };
    let recent = file.itemWithTitle(&objc2_foundation::NSString::from_str("Open Recent")).and_then(|it| it.submenu()).expect("File › Open Recent");
    assert_eq!(titles(&recent), ["Clear Menu"], "nothing recent yet");
    app.ui.recent = vec!["/books/Rechnung.xlsx".into(), "/books/Budget.xlsx".into()];
    menu.poll(&mut app, &ctx, egui::ViewportId::ROOT);
    assert_eq!(titles(&recent), ["Rechnung.xlsx", "Budget.xlsx", "", "Clear Menu"]);
    app.ui.recent.clear();
    menu.poll(&mut app, &ctx, egui::ViewportId::ROOT);
    assert_eq!(titles(&recent), ["Clear Menu"], "Open Recent follows the cleared list");

    // A native Open or Save panel gets a standard Edit menu, so its name field takes ⌘V; the
    // GridCraft menu bar and its Window menu come back afterwards.
    let edit_keys = native_menu::with_text_editing_menu(|| {
        let bar = nsapp.mainMenu().expect("a main menu while the panel is open");
        let edit =
            (0..bar.numberOfItems()).filter_map(|i| bar.itemAtIndex(i)?.submenu()).find(|m| m.title().to_string() == "Edit").expect("an Edit menu");
        (0..edit.numberOfItems()).filter_map(|i| edit.itemAtIndex(i)).map(|item| item.keyEquivalent().to_string()).collect::<Vec<_>>()
    });
    for key in ["v", "c", "x", "a", "z"] {
        assert!(edit_keys.iter().any(|k| k == key), "the panel's Edit menu has ⌘{key}: {edit_keys:?}");
    }
    let bar = nsapp.mainMenu().expect("the main menu is back");
    let restored: Vec<String> = (0..bar.numberOfItems()).filter_map(|i| bar.itemAtIndex(i)?.submenu()).map(|m| m.title().to_string()).collect();
    assert_eq!(restored, tops, "the GridCraft menu bar is restored after the panel");
    assert_eq!(nsapp.windowsMenu().expect("the Window menu is still AppKit's").title().to_string(), "Window");

    println!("mac_menu_appkit: ok");
}

#[cfg(not(target_os = "macos"))]
fn main() {}
