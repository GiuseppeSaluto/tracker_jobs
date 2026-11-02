mod data;
#[cfg(test)]
mod tests;
mod ui;

use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::DefaultTerminal;

use data::Application;

#[derive(Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Open,
    Closed,
}

pub struct App {
    pub path: PathBuf,
    pub applications: Vec<Application>,
    pub selected: usize,
    pub filter: Filter,
    pub popup: bool,
    pub error: Option<String>,
}

impl App {
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

    fn run(&mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|f| ui::draw(f, self))?;
            let Event::Key(k) = event::read()? else {
                continue;
            };
            if k.kind != KeyEventKind::Press {
                continue;
            }
            if self.popup {
                self.popup = k.code != KeyCode::Esc;
                continue;
            }
            let n = self.visible().len();
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Up => self.selected = self.selected.saturating_sub(1),
                KeyCode::Down if self.selected + 1 < n => self.selected += 1,
                KeyCode::Enter => self.popup = n > 0,
                KeyCode::Char('f') => {
                    self.filter = match self.filter {
                        Filter::All => Filter::Open,
                        Filter::Open => Filter::Closed,
                        Filter::Closed => Filter::All,
                    };
                    self.selected = 0;
                }
                // If reloading fails, keep the old data.
                KeyCode::Char('r') => match data::load(&self.path) {
                    Ok(apps) => {
                        self.applications = apps;
                        self.error = None;
                        let n = self.visible().len();
                        self.selected = self.selected.min(n.saturating_sub(1));
                    }
                    Err(e) => self.error = Some(format!("{e:#}")),
                },
                _ => {}
            }
        }
    }
}

fn main() -> Result<()> {
    let path: PathBuf = std::env::args_os()
        .nth(1)
        .context("usage: tracker_jobs <path/to/applications.ods>")?
        .into();
    // Read errors surface here, before the UI opens.
    let applications = data::load(&path)?;
    let mut app = App {
        path,
        applications,
        selected: 0,
        filter: Filter::All,
        popup: false,
        error: None,
    };
    // ratatui::run restores the terminal on exit, even after an error or a panic.
    ratatui::run(|t| app.run(t))
}
