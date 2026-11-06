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

    /// Value used in the spreadsheet dropdown (Italian). Also how it's stored in the database.
    pub fn sheet_value(&self) -> &str {
        match self {
            Status::ToSend => "Da inviare",
            Status::Sent => "Inviata",
            Status::InSelection => "In selezione",
            Status::Interview => "Colloquio",
            Status::Offer => "Offerta",
            Status::Rejected => "Rifiutata",
            Status::Withdrawn => "Ritirata",
            Status::NoResponse => "Nessuna risposta",
            Status::Unknown(s) => s,
        }
    }

    /// Name shown in the UI.
    pub fn name(&self) -> &str {
        match self {
            Status::ToSend => "To send",
            Status::Sent => "Sent",
            Status::InSelection => "In selection",
            Status::Interview => "Interview",
            Status::Offer => "Offer",
            Status::Rejected => "Rejected",
            Status::Withdrawn => "Withdrawn",
            Status::NoResponse => "No response",
            Status::Unknown(s) => s,
        }
    }
}

impl From<&str> for Status {
    fn from(s: &str) -> Self {
        let s = s.trim();
        Status::ALL
            .into_iter()
            .find(|st| st.sheet_value().eq_ignore_ascii_case(s))
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

/// Spreadsheet headers (Italian), in the order they are exported.
pub const HEADERS: [&str; 15] = [
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
];

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

/// Reads the `Candidature` sheet. Columns are looked up by header name, not position.
pub fn load(path: &Path) -> Result<Vec<Application>> {
    // Format chosen from the extension: .ods, .xlsx, .xlsm, .xlsb, .xls.
    let mut wb =
        open_workbook_auto(path).with_context(|| format!("cannot open {}", path.display()))?;
    let range = wb
        .worksheet_range("Candidature")
        .context("sheet 'Candidature' not found")?;
    let mut rows = range.rows();
    let Some(headers) = rows.next() else {
        bail!("sheet 'Candidature' is empty");
    };
    let col = |name: &str| headers.iter().position(|c| c.to_string().trim() == name);
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
    ] = HEADERS.map(col);
    if company.is_none() {
        bail!("column 'Azienda' not found");
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
    ws.set_name("Candidature")?;
    let bold = Format::new().set_bold();
    let date = Format::new().set_num_format("dd/mm/yyyy");
    for (c, h) in HEADERS.iter().enumerate() {
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
        ws.write(r, 8, a.status.sheet_value())?;
        for (c, d) in [(7, a.sent_date), (10, a.next_action_date)] {
            if let Some(d) = d {
                ws.write_with_format(r, c, &d, &date)?;
            }
        }
    }
    // Status dropdown, as in the original tracker.
    let values: Vec<&str> = Status::ALL.iter().map(Status::sheet_value).collect();
    ws.add_data_validation(
        1,
        8,
        1000,
        8,
        &DataValidation::new().allow_list_strings(&values)?,
    )?;
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
