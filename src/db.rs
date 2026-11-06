use std::path::Path;

use anyhow::{Context, Result};
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};
use rusqlite::{Connection, Row, params};

use crate::data::{Application, Status};

/// Embedded at compile time: the binary doesn't need the .sql file at runtime.
pub const SCHEMA: &str = include_str!("../schema.sql");

const COLUMNS: &str = "company, role, sector, role_type, location, channel, contact, sent_date, \
                       status, next_action, next_action_date, salary, cv, link, notes";

// Statuses are stored as their spreadsheet value, so unknown ones survive untouched.
impl ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(self.sheet_value().into())
    }
}

impl FromSql for Status {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        value.as_str().map(Status::from)
    }
}

/// Opens (or creates) the database file and its table.
pub fn open(path: &Path) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let conn = Connection::open(path).with_context(|| format!("cannot open {}", path.display()))?;
    conn.execute_batch(SCHEMA)?;
    Ok(conn)
}

fn from_row(r: &Row) -> rusqlite::Result<Application> {
    Ok(Application {
        id: Some(r.get(0)?),
        company: r.get(1)?,
        role: r.get(2)?,
        sector: r.get(3)?,
        role_type: r.get(4)?,
        location: r.get(5)?,
        channel: r.get(6)?,
        contact: r.get(7)?,
        sent_date: r.get(8)?,
        status: r.get(9)?,
        next_action: r.get(10)?,
        next_action_date: r.get(11)?,
        salary: r.get(12)?,
        cv: r.get(13)?,
        link: r.get(14)?,
        notes: r.get(15)?,
    })
}

pub fn all(conn: &Connection) -> Result<Vec<Application>> {
    let mut stmt = conn.prepare(&format!(
        "SELECT id, {COLUMNS} FROM applications ORDER BY id"
    ))?;
    let rows = stmt.query_map([], from_row)?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM applications", [], |r| r.get(0))?)
}

/// Inserts a new application and returns its id.
pub fn insert(conn: &Connection, a: &Application) -> Result<i64> {
    conn.execute(
        &format!(
            "INSERT INTO applications ({COLUMNS}) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)"
        ),
        params![
            a.company,
            a.role,
            a.sector,
            a.role_type,
            a.location,
            a.channel,
            a.contact,
            a.sent_date,
            a.status,
            a.next_action,
            a.next_action_date,
            a.salary,
            a.cv,
            a.link,
            a.notes
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update(conn: &Connection, id: i64, a: &Application) -> Result<()> {
    conn.execute(
        "UPDATE applications SET company = ?1, role = ?2, sector = ?3, role_type = ?4, \
         location = ?5, channel = ?6, contact = ?7, sent_date = ?8, status = ?9, \
         next_action = ?10, next_action_date = ?11, salary = ?12, cv = ?13, link = ?14, \
         notes = ?15 WHERE id = ?16",
        params![
            a.company,
            a.role,
            a.sector,
            a.role_type,
            a.location,
            a.channel,
            a.contact,
            a.sent_date,
            a.status,
            a.next_action,
            a.next_action_date,
            a.salary,
            a.cv,
            a.link,
            a.notes,
            id
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM applications WHERE id = ?1", [id])?;
    Ok(())
}

/// Replaces every application in one transaction: all or nothing.
pub fn replace_all(conn: &mut Connection, apps: &[Application]) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM applications", [])?;
    for a in apps {
        insert(&tx, a)?;
    }
    tx.commit()?;
    Ok(())
}
