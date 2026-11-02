use calamine::Data;
use chrono::NaiveDate;

use crate::data::{Application, Status, due, load, parse_date};

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

fn app(company: &str, status: Status, date: Option<NaiveDate>) -> Application {
    Application {
        company: company.into(),
        role: String::new(),
        sector: String::new(),
        role_type: String::new(),
        location: String::new(),
        status,
        next_action: String::new(),
        next_action_date: date,
        sent_date: None,
    }
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
fn loads_example_file() {
    let apps = load("examples/sample.ods".as_ref()).unwrap();
    assert_eq!(apps.len(), 10);
    let lumen = apps.iter().find(|a| a.company == "Lumen Sistemi").unwrap();
    assert_eq!(lumen.status, Status::Sent);
    assert_eq!(lumen.next_action_date, Some(d("2026-11-20"))); // date stored as text
    assert_eq!(apps[0].sent_date, Some(d("2026-10-12"))); // real date cell
}

#[test]
fn fit_adds_ellipsis() {
    assert_eq!(crate::ui::fit("Candidatura", 6), "Candi…");
    assert_eq!(crate::ui::fit("Sent", 6), "Sent");
}

#[test]
fn visible_is_sorted_by_date() {
    let app = crate::App {
        path: Default::default(),
        applications: vec![
            app("no date", Status::Sent, None),
            app("later", Status::Sent, Some(d("2026-11-10"))),
            app("sooner", Status::Sent, Some(d("2026-11-03"))),
        ],
        selected: 0,
        filter: crate::Filter::All,
        popup: false,
        error: None,
    };
    let names: Vec<_> = app.visible().iter().map(|a| a.company.as_str()).collect();
    assert_eq!(names, ["sooner", "later", "no date"]);
}
