//! File › New › My templates: the start template opens at New Workbook as an untitled copy, and a
//! workbook saved to a new file goes to the top of Recent.

use gridcraft_engine::{Session, core::CellRef, core::Value};
use gridcraft_ui_egui::{MAX_TEMPLATES, Services, SheetApp};
use serde_json::json;

/// A fresh temp dir named after `tag`.
fn dir(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gridcraft-templates-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// An .xlsx in `dir` named `name`, with `text` in A1.
fn workbook(dir: &std::path::Path, name: &str, text: &str) -> String {
    let mut s = Session::new();
    s.new_workbook();
    s.run("cell.set", json!({"cell": "A1", "input": text})).unwrap();
    let path = dir.join(name);
    std::fs::write(&path, gridcraft_engine::io::save_bytes(&s.active().unwrap().wb, name).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

fn app() -> SheetApp {
    let mut session = Session::new();
    session.new_workbook();
    SheetApp::new(session, Services::default())
}

fn a1(app: &SheetApp) -> Value {
    app.session.active().unwrap().wb.active().unwrap().value(CellRef::parse("A1").unwrap())
}

#[test]
fn new_workbook_opens_the_start_template_as_an_untitled_copy() {
    let dir = dir("start");
    let template = workbook(&dir, "Rechnung.xlsx", "Invoice");
    let before = std::fs::read(&template).unwrap();
    let mut app = app();
    assert!(app.add_template(&template));
    app.ui.start_template = Some(template.clone());

    app.run_or_alert("file.new", json!({}));
    assert!(app.message.is_none(), "{:?}", app.message);
    let doc = app.session.active().unwrap();
    assert_eq!(a1(&app), Value::Text("Invoice".into()), "the template's contents");
    assert_eq!(doc.path, None, "an untitled copy: Save asks where, the template is never overwritten");
    assert_eq!(doc.display_title(), "Rechnung.xlsx");
    assert!(app.ui.recent.is_empty(), "a template copy is not a recent file");

    // The Blank workbook tiles and programmatic callers still get a blank workbook.
    app.new_blank_workbook();
    assert_eq!(a1(&app), Value::Empty);
    app.run("file.new", json!({})).unwrap();
    assert_eq!(a1(&app), Value::Empty);
    assert_eq!(std::fs::read(&template).unwrap(), before, "the template file is untouched");
}

#[test]
fn without_a_start_template_new_workbook_is_blank() {
    let dir = dir("none");
    let template = workbook(&dir, "Quote.xlsx", "Quote");
    let mut app = app();
    assert!(app.add_template(&template));
    assert!(!app.open_start_template(), "no template chosen to open at start");
    // A start template that is no longer one of My templates is not opened either.
    app.ui.start_template = Some(dir.join("Gone.xlsx").to_string_lossy().into_owned());
    app.run_or_alert("file.new", json!({}));
    assert_eq!(a1(&app), Value::Empty);
    assert_eq!(app.session.active().unwrap().path, None);
}

#[test]
fn my_templates_keeps_six_and_removing_one_stops_it_opening_at_start() {
    let dir = dir("six");
    let mut app = app();
    let paths: Vec<String> = (0..=MAX_TEMPLATES).map(|i| workbook(&dir, &format!("T{i}.xlsx"), "x")).collect();
    for p in &paths[..MAX_TEMPLATES] {
        assert!(app.add_template(p));
    }
    assert!(!app.add_template(&paths[MAX_TEMPLATES]), "a seventh does not fit");
    assert!(app.add_template(&paths[0]), "one already there stays");
    assert_eq!(app.ui.templates, paths[..MAX_TEMPLATES]);
    app.ui.start_template = Some(paths[2].clone());
    app.remove_template(&paths[2]);
    assert_eq!(app.ui.templates.len(), MAX_TEMPLATES - 1);
    assert_eq!(app.ui.start_template, None);
}

#[test]
fn a_missing_template_says_so_instead_of_opening() {
    let dir = dir("missing");
    let mut app = app();
    let gone = dir.join("Moved.xlsx").to_string_lossy().into_owned();
    app.ui.templates = vec![gone.clone()];
    app.ui.start_template = Some(gone);
    app.run_or_alert("file.new", json!({}));
    let message = app.message.as_ref().map(|(_, m)| m.clone()).unwrap_or_default();
    assert!(message.contains("Moved.xlsx"), "{message}");
}

#[test]
fn a_workbook_saved_to_a_new_file_goes_to_the_top_of_recent() {
    let dir = dir("recent");
    let opened = workbook(&dir, "Rechnung.xlsx", "Invoice");
    let mut app = app();
    app.open_path(&opened);
    assert_eq!(app.ui.recent, std::slice::from_ref(&opened));

    // Save As: the new invoice is listed at once, above the file it was made from.
    let saved = dir.join("261119-03 Rechnung.xlsx").to_string_lossy().into_owned();
    app.run_or_alert("file.save", json!({"path": saved}));
    assert!(app.message.is_none(), "{:?}", app.message);
    assert_eq!(app.ui.recent, [saved.clone(), opened.clone()]);

    // A PDF saved from Save As lists the workbook, not the PDF.
    let pdf = dir.join("261119-03 Rechnung.pdf").to_string_lossy().into_owned();
    app.run_or_alert("file.save", json!({"path": pdf}));
    assert!(std::path::Path::new(&pdf).exists());
    assert_eq!(app.ui.recent, [saved, opened]);
}

#[test]
fn file_new_shows_my_templates_with_add_open_at_start_and_remove() {
    use egui_kittest::kittest::Queryable as _;
    let dir = dir("page");
    let template = workbook(&dir, "Rechnung.xlsx", "Invoice");
    let services = Services { pick_open: Some(Box::new(|| None)), ..Default::default() };
    let mut h = crate::support::services_harness(services, 2);
    h.state_mut().add_template(&template);
    h.state_mut().ui.start_template = Some(template.clone());
    h.state_mut().run("ui.backstage", json!({"page": "new"})).unwrap();
    h.run_steps(3);
    if let Ok(path) = std::env::var("GRIDCRAFT_MY_TEMPLATES_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    assert!(h.query_by_label("My templates").is_some());
    assert!(h.query_by_label("Add template…").is_some(), "a free slot offers to add one");
    assert!(h.query_by_label("Rechnung").is_some(), "the template's tile");

    // Unticking Open at start keeps the template but New Workbook goes back to blank.
    h.get_by_label("Open at start").click();
    h.run_steps(2);
    assert_eq!(h.state().ui.start_template, None);
    h.get_by_label("Remove").click();
    h.run_steps(2);
    assert!(h.state().ui.templates.is_empty());

    // Clicking a template's tile opens its copy and closes the File page.
    h.state_mut().add_template(&template);
    h.run_steps(2);
    h.get_by_label("Rechnung").click();
    h.run_steps(3);
    assert_eq!(a1(h.state()), Value::Text("Invoice".into()));
    assert_eq!(h.state().session.active().unwrap().path, None);
    assert!(h.state().backstage.is_none(), "the File page closed");
}

#[test]
fn file_home_shows_my_templates_between_new_and_recent() {
    use egui_kittest::kittest::Queryable as _;
    let dir = dir("home");
    let template = workbook(&dir, "Rechnung.xlsx", "Invoice");
    let services = Services { pick_open: Some(Box::new(|| None)), ..Default::default() };
    let mut h = crate::support::services_harness(services, 2);
    h.state_mut().add_template(&template);
    h.state_mut().run("ui.backstage", json!({"page": "home"})).unwrap();
    h.run_steps(3);
    if let Ok(path) = std::env::var("GRIDCRAFT_MY_TEMPLATES_HOME_SCREENSHOT") {
        h.render().unwrap().save(path).unwrap();
    }
    let top = |label: &str| h.get_by_label(label).rect().top();
    assert!(top("Blank workbook") < top("My templates"), "below New");
    assert!(top("My templates") < top("Recent"), "above Recent");
    h.get_by_label("Rechnung").click();
    h.run_steps(3);
    assert_eq!(a1(h.state()), Value::Text("Invoice".into()));
}
