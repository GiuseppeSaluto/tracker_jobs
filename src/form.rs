use crossterm::event::KeyCode;

use crate::data::{Application, Status, parse_date_text};

/// Field labels, in the same order as the spreadsheet columns.
pub const LABELS: [&str; 15] = [
    "Company",
    "Role",
    "Sector",
    "Role type",
    "Location",
    "Channel",
    "Contact",
    "Sent on",
    "Status",
    "Next action",
    "Next action on",
    "Salary / terms",
    "CV used",
    "Job link",
    "Notes",
];
pub const STATUS: usize = 8;
const SENT_DATE: usize = 7;
const NEXT_DATE: usize = 10;

/// Add/edit form. `values[STATUS]` is unused: the status is picked with ←→, not typed.
pub struct Form {
    pub id: Option<i64>,
    pub values: [String; 15],
    pub status: Status,
    pub focus: usize,
    pub error: Option<String>,
}

pub enum Action {
    Continue,
    Cancel,
    Save,
}

fn date_text(d: Option<chrono::NaiveDate>) -> String {
    d.map(|d| d.format("%d/%m/%Y").to_string())
        .unwrap_or_default()
}

impl Form {
    pub fn edit(a: &Application) -> Self {
        Form {
            id: a.id,
            values: [
                a.company.clone(),
                a.role.clone(),
                a.sector.clone(),
                a.role_type.clone(),
                a.location.clone(),
                a.channel.clone(),
                a.contact.clone(),
                date_text(a.sent_date),
                String::new(),
                a.next_action.clone(),
                date_text(a.next_action_date),
                a.salary.clone(),
                a.cv.clone(),
                a.link.clone(),
                a.notes.clone(),
            ],
            status: a.status.clone(),
            focus: 0,
            error: None,
        }
    }

    /// Text shown for field `i`.
    pub fn value(&self, i: usize) -> &str {
        if i == STATUS {
            self.status.name()
        } else {
            &self.values[i]
        }
    }

    // ponytail: typing only appends and Backspace removes the last char, no cursor movement
    // inside a field. Upgrade path: the tui-input crate.
    pub fn key(&mut self, code: KeyCode) -> Action {
        match code {
            KeyCode::Esc => return Action::Cancel,
            KeyCode::Enter => return Action::Save,
            KeyCode::Up | KeyCode::BackTab => self.focus = self.focus.saturating_sub(1),
            KeyCode::Down | KeyCode::Tab => self.focus = (self.focus + 1).min(LABELS.len() - 1),
            KeyCode::Left | KeyCode::Right if self.focus == STATUS => {
                let n = Status::ALL.len();
                // An unknown status (not in the list) starts from the first one.
                let i = Status::ALL.iter().position(|s| *s == self.status);
                let next = match (i, code) {
                    (Some(i), KeyCode::Right) => (i + 1) % n,
                    (Some(i), _) => (i + n - 1) % n,
                    (None, _) => 0,
                };
                self.status = Status::ALL[next].clone();
            }
            KeyCode::Backspace if self.focus != STATUS => {
                self.values[self.focus].pop();
            }
            KeyCode::Char(c) if self.focus != STATUS => self.values[self.focus].push(c),
            _ => {}
        }
        Action::Continue
    }

    /// Validates the form. Errors are messages for the user.
    pub fn application(&self) -> Result<Application, String> {
        let v = |i: usize| self.values[i].trim().to_string();
        let date = |i: usize| {
            let s = v(i);
            if s.is_empty() {
                return Ok(None);
            }
            parse_date_text(&s)
                .map(Some)
                .ok_or_else(|| format!("{}: invalid date, use dd/mm/yyyy", LABELS[i]))
        };
        if v(0).is_empty() {
            return Err("Company is required".into());
        }
        Ok(Application {
            id: self.id,
            company: v(0),
            role: v(1),
            sector: v(2),
            role_type: v(3),
            location: v(4),
            channel: v(5),
            contact: v(6),
            sent_date: date(SENT_DATE)?,
            status: self.status.clone(),
            next_action: v(9),
            next_action_date: date(NEXT_DATE)?,
            salary: v(11),
            cv: v(12),
            link: v(13),
            notes: v(14),
        })
    }
}
