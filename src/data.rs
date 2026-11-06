use std::path::Path;

use anyhow::{Context, Result, bail};
use calamine::{Data, DataType, Reader, open_workbook_auto};
use chrono::{Days, NaiveDate};
use rust_xlsxwriter::{DataValidation, Format, Workbook};

#[derive(Debug, Clone, PartialEq, Default)]
pub enum Status {
    #[default]
    ToSend,
    Sent,
    InSelection,
    Interview,
    Offer,
    Rejected,
    Withdrawn,
    NoResponse,
    Unknown(String),
}

impl Status {
    pub const ALL: [Status; 8] = [
        Status::ToSend,
        Status::Sent,
        Status::InSelection,
        Status::Interview,
        Status::Offer,
        Status::Rejected,
        Status::Withdrawn,
        Status::NoResponse,
    ];

    pub fn is_open(&self) -> bool {
        !matches!(self, Status::Offer | Status::Rejected | Status::Withdrawn)
    }

    /// Canonical name: shown in the UI, stored in the database, written on export.
    pub fn name(&self) -> &str {
        match self {
            Status::Unknown(s) => s,
            known => {
                let i = Status::ALL.iter().position(|s| s == known);
                ENGLISH.statuses[i.expect("ALL lists every known status")]
            }
        }
    }
}

/// Recognizes a status written in any of `LANGUAGES`, ignoring case.
impl From<&str> for Status {
    fn from(s: &str) -> Self {
        let s = s.trim();
        LANGUAGES
            .iter()
            .find_map(|l| l.statuses.iter().position(|v| v.eq_ignore_ascii_case(s)))
            .map(|i| Status::ALL[i].clone())
            .unwrap_or_else(|| Status::Unknown(s.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Application {
    /// Database id; `None` until saved.
    pub id: Option<i64>,
    pub company: String,
    pub role: String,
    pub sector: String,
    pub role_type: String,
    pub location: String,
    pub channel: String,
    pub contact: String,
    pub sent_date: Option<NaiveDate>,
    pub status: Status,
    pub next_action: String,
    pub next_action_date: Option<NaiveDate>,
    pub salary: String,
    pub cv: String,
    pub link: String,
    pub notes: String,
}

/// Spreadsheet words of one language.
pub struct Language {
    pub sheet: &'static str,
    /// One per `Application` field, in export order.
    pub headers: [&'static str; 15],
    /// Same order as `Status::ALL`.
    pub statuses: [&'static str; 8],
}

/// Languages recognized on import. To support a new one, add its table here.
pub const LANGUAGES: [Language; 2] = [ENGLISH, ITALIAN];

/// Canonical language: UI, database and export.
pub const ENGLISH: Language = Language {
    sheet: "Applications",
    headers: [
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
    ],
    statuses: [
        "To send",
        "Sent",
        "In selection",
        "Interview",
        "Offer",
        "Rejected",
        "Withdrawn",
        "No response",
    ],
};

const ITALIAN: Language = Language {
    sheet: "Candidature",
    headers: [
        "Azienda",
        "Ruolo",
        "Settore",
        "Tipo ruolo",
        "Sede",
        "Canale",
        "Referente",
        "Data invio",
        "Stato",
        "Prossima azione",
        "Data prossima azione",
        "RAL / condizioni",
        "CV usato",
        "Link annuncio",
        "Note",
    ],
    statuses: [
        "Da inviare",
        "Inviata",
        "In selezione",
        "Colloquio",
        "Offerta",
        "Rifiutata",
        "Ritirata",
        "Nessuna risposta",
    ],
};

/// Text dates: ISO (`2026-11-06`) or Italian (`06/11/2026`).
pub fn parse_date_text(s: &str) -> Option<NaiveDate> {
    let s = s.trim();
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| NaiveDate::parse_from_str(s, "%d/%m/%Y"))
        .ok()
}

/// ODS/XLSX dates: date/time, ISO, serial number (handled by calamine) or plain text.
pub fn parse_date(cell: &Data) -> Option<NaiveDate> {
    cell.as_date()
        .or_else(|| parse_date_text(cell.get_string()?))
}

/// Reads the applications sheet. Sheet and columns are looked up by name in any of
/// `LANGUAGES` (each column on its own), not by position.
pub fn load(path: &Path) -> Result<Vec<Application>> {
    // Format chosen from the extension: .ods, .xlsx, .xlsm, .xlsb, .xls.
    let mut wb =
        open_workbook_auto(path).with_context(|| format!("cannot open {}", path.display()))?;
    let names = |pick: fn(&Language) -> &'static str| LANGUAGES.map(|l| pick(&l)).join("' or '");
    let range = LANGUAGES
        .iter()
        .find_map(|l| wb.worksheet_range(l.sheet).ok())
        .with_context(|| format!("no sheet named '{}'", names(|l| l.sheet)))?;
    let mut rows = range.rows();
    let Some(headers) = rows.next() else {
        bail!("the applications sheet is empty");
    };
    let col = |name: &str| headers.iter().position(|c| c.to_string().trim() == name);
    let cols: [Option<usize>; 15] =
        std::array::from_fn(|i| LANGUAGES.iter().find_map(|l| col(l.headers[i])));
    let [
        company,
        role,
        sector,
        role_type,
        location,
        channel,
        contact,
        sent_date,
        status,
        next_action,
        next_action_date,
        salary,
        cv,
        link,
        notes,
    ] = cols;
    if company.is_none() {
        bail!("column '{}' not found", names(|l| l.headers[0]));
    }

    Ok(rows
        .filter_map(|r| {
            let cell = |i: Option<usize>| i.and_then(|i| r.get(i)).unwrap_or(&Data::Empty);
            let text = |i| cell(i).to_string().trim().to_string();
            let company = text(company);
            // Rows without a company are skipped.
            (!company.is_empty()).then(|| Application {
                id: None,
                company,
                role: text(role),
                sector: text(sector),
                role_type: text(role_type),
                location: text(location),
                channel: text(channel),
                contact: text(contact),
                sent_date: parse_date(cell(sent_date)),
                status: Status::from(text(status).as_str()),
                next_action: text(next_action),
                next_action_date: parse_date(cell(next_action_date)),
                salary: text(salary),
                cv: text(cv),
                link: text(link),
                notes: text(notes),
            })
        })
        .collect())
}

/// Writes a new `.xlsx` with the same layout `load` reads, so an export can be imported again.
pub fn export(path: &Path, apps: &[Application]) -> Result<()> {
    let mut wb = Workbook::new();
    let ws = wb.add_worksheet();
    ws.set_name(ENGLISH.sheet)?;
    let bold = Format::new().set_bold();
    let date = Format::new().set_num_format("dd/mm/yyyy");
    for (c, h) in ENGLISH.headers.iter().enumerate() {
        ws.write_with_format(0, c as u16, *h, &bold)?;
    }
    for (i, a) in apps.iter().enumerate() {
        let r = i as u32 + 1;
        let texts = [
            (0, &a.company),
            (1, &a.role),
            (2, &a.sector),
            (3, &a.role_type),
            (4, &a.location),
            (5, &a.channel),
            (6, &a.contact),
            (9, &a.next_action),
            (11, &a.salary),
            (12, &a.cv),
            (13, &a.link),
            (14, &a.notes),
        ];
        for (c, v) in texts {
            ws.write(r, c, v.as_str())?;
        }
        ws.write(r, 8, a.status.name())?;
        for (c, d) in [(7, a.sent_date), (10, a.next_action_date)] {
            if let Some(d) = d {
                ws.write_with_format(r, c, &d, &date)?;
            }
        }
    }
    // Status dropdown, as in the original tracker.
    let dropdown = DataValidation::new().allow_list_strings(&ENGLISH.statuses)?;
    ws.add_data_validation(1, 8, 1000, 8, &dropdown)?;
    ws.set_freeze_panes(1, 0)?;
    ws.autofit();
    wb.save(path)
        .with_context(|| format!("cannot write {}", path.display()))
}

/// Open applications with a next action due within today + 3 days, sorted by date.
pub fn due(apps: &[Application], today: NaiveDate) -> Vec<&Application> {
    let limit = today + Days::new(3);
    let mut v: Vec<_> = apps
        .iter()
        .filter(|a| a.status.is_open() && a.next_action_date.is_some_and(|d| d <= limit))
        .collect();
    v.sort_by_key(|a| a.next_action_date);
    v
}
