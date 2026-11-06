use calamine::Data;
use chrono::NaiveDate;
use crossterm::event::KeyCode;
use rusqlite::Connection;

use crate::data::{Application, Status, due, export, load, parse_date};
use crate::form::Form;
use crate::{App, db};

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn app(company: &str, status: Status, date: Option<NaiveDate>) -> Application {
    Application {
        company: company.into(),
        status,
        next_action_date: date,
        ..Default::default()
    }
}

fn memory_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(db::SCHEMA).unwrap();
    conn
}

#[test]
fn due_filters_and_sorts() {
    let today = d("2026-11-01");
    let apps = [
        app("in 10 days", Status::Sent, Some(d("2026-11-11"))),
        app("in 2 days", Status::Sent, Some(d("2026-11-03"))),
        app("overdue", Status::Sent, Some(d("2026-10-28"))),
        app("rejected", Status::Rejected, Some(d("2026-10-28"))),
        app("no date", Status::Sent, None),
    ];
    let names: Vec<_> = due(&apps, today)
        .iter()
        .map(|a| a.company.as_str())
        .collect();
    assert_eq!(names, ["overdue", "in 2 days"]);
}

#[test]
fn unknown_status_does_not_panic() {
    assert_eq!(Status::from("Boh"), Status::Unknown("Boh".into()));
    assert_eq!(Status::from("In selezione"), Status::InSelection);
}

#[test]
fn parse_date_from_text() {
    assert_eq!(
        parse_date(&Data::String("2026-11-06".into())),
        Some(d("2026-11-06"))
    );
    assert_eq!(parse_date(&Data::String("".into())), None);
    assert_eq!(parse_date(&Data::Empty), None);
}

#[test]
fn loads_example_files() {
    for path in ["examples/sample.ods", "examples/sample.xlsx"] {
        let apps = load(path.as_ref()).unwrap();
        assert_eq!(apps.len(), 10, "{path}");
        let lumen = apps.iter().find(|a| a.company == "Lumen Sistemi").unwrap();
        assert_eq!(lumen.status, Status::Sent, "{path}");
        assert_eq!(lumen.next_action_date, Some(d("2026-11-20")), "{path}"); // date stored as text
        assert_eq!(lumen.salary, "40k", "{path}");
        assert_eq!(apps[0].sent_date, Some(d("2026-10-12")), "{path}"); // real date cell
    }
}

#[test]
fn fit_adds_ellipsis() {
    assert_eq!(crate::ui::fit("Candidatura", 6), "Candi…");
    assert_eq!(crate::ui::fit("Sent", 6), "Sent");
}

#[test]
fn visible_is_sorted_by_date() {
    let conn = memory_db();
    for a in [
        app("no date", Status::Sent, None),
        app("later", Status::Sent, Some(d("2026-11-10"))),
        app("sooner", Status::Sent, Some(d("2026-11-03"))),
    ] {
        db::insert(&conn, &a).unwrap();
    }
    let app = App::new(Default::default(), conn).unwrap();
    let names: Vec<_> = app.visible().iter().map(|a| a.company.as_str()).collect();
    assert_eq!(names, ["sooner", "later", "no date"]);
}

#[test]
fn db_crud() {
    let conn = memory_db();
    let mut a = app("Acme", Status::Unknown("Boh".into()), Some(d("2026-11-03")));
    a.notes = "note".into();
    let id = db::insert(&conn, &a).unwrap();
    a.id = Some(id);
    assert_eq!(db::all(&conn).unwrap(), [a.clone()]); // unknown status survives

    a.status = Status::Interview;
    a.next_action_date = None;
    db::update(&conn, id, &a).unwrap();
    assert_eq!(db::all(&conn).unwrap(), [a]);

    db::delete(&conn, id).unwrap();
    assert_eq!(db::count(&conn).unwrap(), 0);
}

#[test]
fn import_then_export_round_trip() {
    let sheet = load("examples/sample.ods".as_ref()).unwrap();
    let mut conn = memory_db();
    db::replace_all(&mut conn, &sheet).unwrap();
    db::replace_all(&mut conn, &sheet).unwrap(); // replacing again doesn't duplicate
    let stored = db::all(&conn).unwrap();
    assert_eq!(stored.len(), sheet.len());

    let out = std::env::temp_dir().join(format!("tracker_jobs_test_{}.xlsx", std::process::id()));
    export(&out, &stored).unwrap();
    let reloaded = load(&out);
    std::fs::remove_file(&out).unwrap();
    assert_eq!(reloaded.unwrap(), sheet); // every field, dates included
}

#[test]
fn form_validates_and_round_trips() {
    let mut a = app("Acme", Status::Sent, Some(d("2026-11-03")));
    a.id = Some(7);
    a.link = "https://example.com".into();
    assert_eq!(Form::edit(&a).application(), Ok(a));

    let mut form = Form::edit(&Application::default());
    assert!(form.application().is_err()); // company is required
    for c in "Acme".chars() {
        form.key(KeyCode::Char(c));
    }
    form.focus = 7; // "Sent on"
    for c in "31/02/2026".chars() {
        form.key(KeyCode::Char(c));
    }
    assert!(form.application().unwrap_err().contains("Sent on"));
    form.focus = crate::form::STATUS;
    form.key(KeyCode::Right);
    assert_eq!(form.status, Status::Sent);
}
