use std::path::Path;

use anyhow::{Context, Result, bail};
use calamine::{Data, DataType, Reader, open_workbook_auto};
use chrono::{Days, NaiveDate};

#[derive(Debug, Clone, PartialEq)]
pub enum Status {
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

    /// Value used in the spreadsheet dropdown (Italian).
    fn sheet_value(&self) -> &str {
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

#[derive(Debug, Clone)]
pub struct Application {
    pub company: String,
    pub role: String,
    pub sector: String,
    pub role_type: String,
    pub location: String,
    pub status: Status,
    pub next_action: String,
    pub next_action_date: Option<NaiveDate>,
    pub sent_date: Option<NaiveDate>,
}

/// ODS dates: date/time, ISO, serial number (handled by calamine) or plain text.
pub fn parse_date(cell: &Data) -> Option<NaiveDate> {
    cell.as_date().or_else(|| {
        let s = cell.get_string()?.trim();
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(s, "%d/%m/%Y"))
            .ok()
    })
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
    let Some(company) = col("Azienda") else {
        bail!("column 'Azienda' not found");
    };
    let (role, sector, role_type, location, status, action, action_date, sent_date) = (
        col("Ruolo"),
        col("Settore"),
        col("Tipo ruolo"),
        col("Sede"),
        col("Stato"),
        col("Prossima azione"),
        col("Data prossima azione"),
        col("Data invio"),
    );

    Ok(rows
        .filter_map(|r| {
            let cell = |i: Option<usize>| i.and_then(|i| r.get(i)).unwrap_or(&Data::Empty);
            let text = |i| cell(i).to_string().trim().to_string();
            let company = text(Some(company));
            // Rows without a company are skipped.
            (!company.is_empty()).then(|| Application {
                company,
                role: text(role),
                sector: text(sector),
                role_type: text(role_type),
                location: text(location),
                status: Status::from(text(status).as_str()),
                next_action: text(action),
                next_action_date: parse_date(cell(action_date)),
                sent_date: parse_date(cell(sent_date)),
            })
        })
        .collect())
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
