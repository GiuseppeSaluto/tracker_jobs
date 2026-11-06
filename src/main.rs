mod data;
mod db;
mod form;
#[cfg(test)]
mod tests;
mod ui;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;
use rusqlite::Connection;

use data::Application;
use form::{Action, Form};

const USAGE: &str = "usage:
  tracker_jobs [--db FILE]                             open the dashboard
  tracker_jobs import SHEET [--replace] [--db FILE]    load a .ods/.xlsx/.xls into the database
  tracker_jobs export FILE.xlsx [--db FILE]            write the database to a new .xlsx";

#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Open,
    Closed,
}

pub enum Mode {
    Table,
    Details,
    Form(Box<Form>),
    ConfirmDelete,
}

pub struct App {
    pub db_path: PathBuf,
    conn: Connection,
    pub applications: Vec<Application>,
    pub selected: usize,
    pub filter: Filter,
    pub mode: Mode,
    pub error: Option<String>,
}

impl App {
    pub fn new(db_path: PathBuf, conn: Connection) -> Result<App> {
        let applications = db::all(&conn)?;
        Ok(App {
            db_path,
            conn,
            applications,
            selected: 0,
            filter: Filter::All,
            mode: Mode::Table,
            error: None,
        })
    }

    /// Filtered applications, sorted by next action date (no date last).
    pub fn visible(&self) -> Vec<&Application> {
        let mut v: Vec<_> = self
            .applications
            .iter()
            .filter(|a| match self.filter {
                Filter::All => true,
                Filter::Open => a.status.is_open(),
                Filter::Closed => !a.status.is_open(),
            })
            .collect();
        v.sort_by_key(|a| (a.next_action_date.is_none(), a.next_action_date));
        v
    }

    pub fn selected_app(&self) -> Option<&Application> {
        self.visible().get(self.selected).copied()
    }

    /// Re-reads the database, keeping the selection on `select` when given.
    fn reload(&mut self, select: Option<i64>) -> Result<()> {
        self.applications = db::all(&self.conn)?;
        let visible = self.visible();
        let pos = select.and_then(|id| visible.iter().position(|a| a.id == Some(id)));
        let last = visible.len().saturating_sub(1);
        self.selected = pos.unwrap_or(self.selected).min(last);
        Ok(())
    }

    /// Inserts or updates, then reloads with the saved row selected.
    fn save(&mut self, a: &Application) -> Result<()> {
        let id = match a.id {
            Some(id) => db::update(&self.conn, id, a).map(|_| id)?,
            None => db::insert(&self.conn, a)?,
        };
        self.reload(Some(id))
    }

    fn delete_selected(&mut self) -> Result<()> {
        if let Some(id) = self.selected_app().and_then(|a| a.id) {
            db::delete(&self.conn, id)?;
            self.reload(None)?;
        }
        Ok(())
    }

    /// Handles one key. Returns `true` to quit.
    fn key(&mut self, code: KeyCode) -> bool {
        match &mut self.mode {
            Mode::Details => match code {
                KeyCode::Esc | KeyCode::Enter => self.mode = Mode::Table,
                KeyCode::Char('e') => {
                    if let Some(a) = self.selected_app() {
                        self.mode = Mode::Form(Box::new(Form::edit(a)));
                    }
                }
                _ => {}
            },
            Mode::ConfirmDelete => {
                if code == KeyCode::Char('y') {
                    self.error = self.delete_selected().err().map(|e| format!("{e:#}"));
                }
                self.mode = Mode::Table;
            }
            Mode::Form(form) => match form.key(code) {
                Action::Continue => {}
                Action::Cancel => self.mode = Mode::Table,
                Action::Save => match form.application() {
                    Err(e) => form.error = Some(e),
                    Ok(a) => match self.save(&a) {
                        Ok(()) => self.mode = Mode::Table,
                        Err(e) => {
                            if let Mode::Form(form) = &mut self.mode {
                                form.error = Some(format!("{e:#}"));
                            }
                        }
                    },
                },
            },
            Mode::Table => {
                let n = self.visible().len();
                match code {
                    KeyCode::Char('q') | KeyCode::Esc => return true,
                    KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                    KeyCode::Down if self.selected + 1 < n => self.selected += 1,
                    KeyCode::Enter if n > 0 => self.mode = Mode::Details,
                    KeyCode::Char('a') => {
                        self.mode = Mode::Form(Box::new(Form::edit(&Application::default())))
                    }
                    KeyCode::Char('e') => {
                        if let Some(a) = self.selected_app() {
                            self.mode = Mode::Form(Box::new(Form::edit(a)));
                        }
                    }
                    KeyCode::Char('d') if n > 0 => self.mode = Mode::ConfirmDelete,
                    KeyCode::Char('f') => {
                        self.filter = match self.filter {
                            Filter::All => Filter::Open,
                            Filter::Open => Filter::Closed,
                            Filter::Closed => Filter::All,
                        };
                        self.selected = 0;
                    }
                    _ => {}
                }
            }
        }
        false
    }

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| ui::draw(f, self))?;
            if let Event::Key(k) = event::read()?
                && k.kind == KeyEventKind::Press
                && self.key(k.code)
            {
                return Ok(());
            }
        }
    }
}

fn default_db() -> PathBuf {
    // ponytail: same folder on every OS. Upgrade path: the `dirs` crate for OS-native data dirs.
    std::env::home_dir()
        .unwrap_or_default()
        .join(".local/share/tracker_jobs/tracker.db")
}

fn import(sheet: &Path, db_path: &Path, replace: bool) -> Result<()> {
    let apps = data::load(sheet)?;
    let mut conn = db::open(db_path)?;
    let n = db::count(&conn)?;
    if n > 0 {
        if !replace {
            bail!(
                "{} already has {n} applications: use --replace to overwrite them (a backup is kept)",
                db_path.display()
            );
        }
        let mut backup = db_path.as_os_str().to_owned();
        backup.push(".bak");
        std::fs::copy(db_path, &backup).context("cannot back up the database")?;
        println!("backup saved to {}", Path::new(&backup).display());
    }
    db::replace_all(&mut conn, &apps)?;
    println!(
        "imported {} applications into {}",
        apps.len(),
        db_path.display()
    );
    Ok(())
}

fn export(out: &Path, db_path: &Path) -> Result<()> {
    if !out
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("xlsx"))
    {
        bail!("export writes .xlsx files: use a name ending in .xlsx");
    }
    // Never overwrite an existing spreadsheet: it could be the user's real tracker.
    if out.exists() {
        bail!("{} already exists: pick another name", out.display());
    }
    if !db_path.exists() {
        bail!(
            "no database at {}: run `tracker_jobs import` first",
            db_path.display()
        );
    }
    let apps = db::all(&db::open(db_path)?)?;
    data::export(out, &apps)?;
    println!("exported {} applications to {}", apps.len(), out.display());
    Ok(())
}

fn main() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let db_path = match args.iter().position(|a| a == "--db") {
        Some(i) if i + 1 < args.len() => PathBuf::from(args.drain(i..=i + 1).nth(1).unwrap()),
        Some(_) => bail!(USAGE),
        None => default_db(),
    };
    let replace = args
        .iter()
        .position(|a| a == "--replace")
        .map(|i| args.remove(i))
        .is_some();

    match args.as_slice() {
        [] if !replace => {
            let mut app = App::new(db_path.clone(), db::open(&db_path)?)?;
            // ratatui::run restores the terminal on exit, even after an error or a panic.
            ratatui::run(|t| app.run(t))
        }
        [cmd, sheet] if cmd == "import" => import(sheet.as_ref(), &db_path, replace),
        [cmd, out] if cmd == "export" && !replace => export(out.as_ref(), &db_path),
        _ => bail!(USAGE),
    }
}
