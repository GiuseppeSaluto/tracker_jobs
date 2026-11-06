use chrono::{Local, NaiveDate};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style, Stylize};
use ratatui::text::Line;
use ratatui::widgets::{
    Bar, BarChart, BarGroup, Block, Clear, Paragraph, Row, Table, TableState, Wrap,
};

use crate::data::{self, Application, Status};
use crate::form::{Form, LABELS, STATUS};
use crate::{App, Filter, Mode};

pub fn draw(f: &mut Frame, app: &App) {
    let today = Local::now().date_naive();
    let file_name = app
        .db_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy();
    let outer = Block::bordered().title(format!(
        " tracker_jobs ── {file_name} ── {} applications ",
        app.applications.len()
    ));
    let area = outer.inner(f.area());
    f.render_widget(outer, f.area());

    let [summary_area, middle, table_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(10),
        Constraint::Min(4),
        Constraint::Length(1),
    ])
    .areas(area);
    let [due_area, status_area] = Layout::horizontal([Constraint::Percentage(50); 2]).areas(middle);

    summary(f, summary_area, &app.applications);
    due(f, due_area, &app.applications, today);
    by_status(f, status_area, &app.applications);
    let visible = app.visible();
    table(f, table_area, &visible, app);
    footer(f, footer_area, app);

    match &app.mode {
        Mode::Table => {}
        Mode::Details => {
            if let Some(a) = app.selected_app() {
                let form = Form::edit(a);
                popup(f, " Details ", " e edit  Esc close ", fields(&form, None));
            }
        }
        Mode::Form(form) => {
            let title = if form.id.is_some() {
                " Edit application "
            } else {
                " New application "
            };
            let mut lines = fields(form, Some(form.focus));
            if let Some(e) = &form.error {
                lines.push(Line::default());
                lines.push(Line::from(e.as_str()).red());
            }
            popup(
                f,
                title,
                " ↑↓ field  ←→ status  Enter save  Esc cancel ",
                lines,
            );
        }
        Mode::ConfirmDelete => {
            if let Some(a) = app.selected_app() {
                let lines = vec![Line::from(format!("Delete {}?", a.company)).bold()];
                popup(f, " Delete ", " y delete  any other key cancels ", lines);
            }
        }
    }
}

fn short_date(d: Option<NaiveDate>) -> String {
    d.map(|d| d.format("%d/%m").to_string()).unwrap_or_default()
}

/// Column widths as `Table` lays them out (default flex and spacing).
fn column_widths(constraints: &[Constraint], width: u16) -> Vec<u16> {
    Layout::horizontal(constraints.iter().copied())
        .flex(Flex::Start)
        .spacing(1)
        .split(Rect::new(0, 0, width, 1))
        .iter()
        .map(|r| r.width)
        .collect()
}

/// Cuts `s` to `width` columns, ending with "…" when it doesn't fit.
/// ponytail: counts chars, not display width; wide chars (CJK, emoji) would overflow.
/// Upgrade path: unicode-width.
pub fn fit(s: &str, width: u16) -> String {
    let w = width as usize;
    if s.chars().count() <= w {
        return s.to_string();
    }
    let mut out: String = s.chars().take(w.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Same colors as the spreadsheet.
fn status_style(s: &Status) -> Style {
    match s {
        Status::Offer => Style::new().green(),
        Status::Interview => Style::new().yellow(),
        Status::InSelection => Style::new().cyan(),
        s if !s.is_open() => Style::new().dark_gray(),
        _ => Style::new(),
    }
}

fn summary(f: &mut Frame, area: Rect, apps: &[Application]) {
    let count = |p: fn(&Application) -> bool| apps.iter().filter(|a| p(a)).count();
    let text = format!(
        " Total {}   Open {}   Interviews {}   Offers {}",
        apps.len(),
        count(|a| a.status.is_open()),
        count(|a| a.status == Status::Interview),
        count(|a| a.status == Status::Offer),
    );
    f.render_widget(Paragraph::new(text).bold(), area);
}

fn due(f: &mut Frame, area: Rect, apps: &[Application], today: NaiveDate) {
    let constraints = [
        Constraint::Percentage(40),
        Constraint::Length(6),
        Constraint::Fill(1),
    ];
    let w = column_widths(&constraints, area.width.saturating_sub(2));
    let rows = data::due(apps, today).into_iter().map(|a| {
        let d = a.next_action_date.unwrap_or(today);
        let (style, when) = if d < today {
            (Style::new().red(), short_date(Some(d)))
        } else if d == today {
            (Style::new().yellow(), "today".to_string())
        } else {
            (Style::new(), short_date(Some(d)))
        };
        let mark = if d <= today { "!" } else { " " };
        Row::new([
            fit(&format!("{mark} {}", a.company), w[0]),
            when,
            fit(&a.next_action, w[2]),
        ])
        .style(style)
    });
    let t = Table::new(rows, constraints).block(Block::bordered().title(" To do "));
    f.render_widget(t, area);
}

fn by_status(f: &mut Frame, area: Rect, apps: &[Application]) {
    let mut statuses: Vec<Status> = Status::ALL.to_vec();
    for a in apps {
        if !statuses.contains(&a.status) {
            statuses.push(a.status.clone());
        }
    }
    let bars: Vec<Bar> = statuses
        .iter()
        .map(|s| (s, apps.iter().filter(|a| &a.status == s).count() as u64))
        .filter(|(_, n)| *n > 0)
        .map(|(s, n)| {
            Bar::default()
                .label(Line::from(format!("{:<16}", s.name())))
                .value(n)
                .style(status_style(s))
        })
        .collect();
    let chart = BarChart::default()
        .block(Block::bordered().title(" By status "))
        .direction(Direction::Horizontal)
        .bar_width(1)
        .bar_gap(0)
        .data(BarGroup::default().bars(&bars));
    f.render_widget(chart, area);
}

fn table(f: &mut Frame, area: Rect, visible: &[&Application], app: &App) {
    let constraints = [
        Constraint::Fill(2),
        Constraint::Fill(2),
        Constraint::Length(16),
        Constraint::Fill(3),
        Constraint::Length(6),
    ];
    // Borders (2) and the "> " highlight symbol (2).
    let w = column_widths(&constraints, area.width.saturating_sub(4));
    let rows = visible.iter().map(|a| {
        Row::new([
            fit(&a.company, w[0]),
            fit(&a.role, w[1]),
            fit(a.status.name(), w[2]),
            fit(&a.next_action, w[3]),
            short_date(a.next_action_date),
        ])
        .style(status_style(&a.status))
    });
    let filter = match app.filter {
        Filter::All => "all",
        Filter::Open => "open only",
        Filter::Closed => "closed only",
    };
    let t = Table::new(rows, constraints)
        .header(Row::new(["Company", "Role", "Status", "Next action", "Date"]).bold())
        .block(Block::bordered().title(format!(" Applications ({filter}) ")))
        .row_highlight_style(Modifier::REVERSED)
        .highlight_symbol("> ");
    if app.applications.is_empty() {
        let hint =
            " No applications yet: press a to add one, or run `tracker_jobs import <sheet>`.";
        f.render_widget(Paragraph::new(hint).block(Block::bordered()), area);
        return;
    }
    let mut state = TableState::default().with_selected(Some(app.selected));
    f.render_stateful_widget(t, area, &mut state);
}

fn footer(f: &mut Frame, area: Rect, app: &App) {
    let p = match &app.error {
        Some(e) => Paragraph::new(format!(" Error: {e}")).red(),
        None => {
            Paragraph::new(" ↑↓ move  Enter details  a add  e edit  d delete  f filter  q quit")
                .dark_gray()
        }
    };
    f.render_widget(p, area);
}

/// One line per field; `focus` highlights the field being edited.
fn fields(form: &Form, focus: Option<usize>) -> Vec<Line<'_>> {
    LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let label = format!("{label:<16}").bold();
            if focus != Some(i) {
                return Line::from(vec![label, form.value(i).into()]);
            }
            let value = if i == STATUS {
                format!("◀ {} ▶", form.value(i))
            } else {
                format!("{}▏", form.value(i))
            };
            Line::from(vec![label, value.reversed()])
        })
        .collect()
}

fn popup(f: &mut Frame, title: &str, help: &str, lines: Vec<Line>) {
    let p = Paragraph::new(lines)
        .wrap(Wrap { trim: true })
        .block(Block::bordered().title(title).title_bottom(help));
    // Long values wrap: size the popup to the wrapped height (inner width = width - borders).
    let width = f
        .area()
        .centered(Constraint::Percentage(60), Constraint::Length(1))
        .width;
    let height = p.line_count(width.saturating_sub(2)) as u16;
    let area = f
        .area()
        .centered(Constraint::Percentage(60), Constraint::Length(height));
    f.render_widget(Clear, area);
    f.render_widget(p, area);
}
