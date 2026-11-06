# tracker_jobs

A terminal app (TUI) in Rust to track job applications: add, edit and delete them from the
terminal, see at a glance what to do today, and move data in and out of spreadsheets
(`.ods`, `.xlsx`, `.xls`).

Data lives in a local SQLite database (schema in [`schema.sql`](schema.sql)). Spreadsheets are for **import** (start from an existing
tracker) and **export** (view or share it in LibreOffice Calc or Excel).

<!-- screenshot: e.g. `vhs demo.tape` or `asciinema rec` -->

## What it shows

- **Summary**: total, open, interviews, offers.
- **To do**: open applications with a next action due within 3 days (overdue in red, today in yellow).
- **By status**: one bar per status.
- **Table**: all applications sorted by next action date, with a details popup for the selected row.

## Usage

```bash
cargo run -- import examples/sample.ods      # load a spreadsheet into the database
cargo run                                    # open the dashboard
cargo run -- export ~/applications.xlsx      # write the database to a new .xlsx
```

| Command | What it does |
|---|---|
| `tracker_jobs` | open the dashboard |
| `tracker_jobs import SHEET` | load a `.ods`/`.xlsx`/`.xls` into the database. Refuses if the database isn't empty |
| `tracker_jobs import SHEET --replace` | the same, replacing everything; the old database is saved as `tracker.db.bak` first |
| `tracker_jobs export FILE.xlsx` | write a new `.xlsx`, never overwriting an existing file. It can be imported again |
| `--db FILE` | use another database (default: `~/.local/share/tracker_jobs/tracker.db`) |

### Keys

| Key | Dashboard | Form |
|---|---|---|
| `↑` `↓` | move the selection | previous / next field (also `Tab`) |
| `Enter` | row details | save |
| `a` | add an application | |
| `e` | edit the selected one (also from details) | |
| `d` | delete it (asks for confirmation) | |
| `f` | filter: all → open → closed | |
| `←` `→` | | change the status |
| `Esc` | close / quit (`q` also quits) | cancel |

Dates are typed as `dd/mm/yyyy` (or `yyyy-mm-dd`); the company is the only required field.

## Spreadsheet format

Import and export use the layout of the original (Italian) tracker: sheet `Candidature`,
headers in row 1. On import, columns are looked up by name, so their order doesn't matter
and extra columns are ignored.

| Header | Field | | Header | Field |
|---|---|---|---|---|
| `Azienda` | Company (required) | | `Stato` | Status |
| `Ruolo` | Role | | `Prossima azione` | Next action |
| `Settore` | Sector | | `Data prossima azione` | Next action date |
| `Tipo ruolo` | Role type | | `RAL / condizioni` | Salary / terms |
| `Sede` | Location | | `CV usato` | CV used |
| `Canale` | Channel | | `Link annuncio` | Job link |
| `Referente` | Contact | | `Note` | Notes |
| `Data invio` | Date sent | | | |

Status values: `Da inviare` (to send), `Inviata` (sent), `In selezione` (in selection),
`Colloquio` (interview), `Offerta` (offer), `Rifiutata` (rejected), `Ritirata` (withdrawn),
`Nessuna risposta` (no response). Unknown values are kept as they are. An application is
*open* unless it's an offer, rejected or withdrawn.

## Privacy

Real data stays out of the repository: `.gitignore` excludes databases and every spreadsheet
except those in `examples/`, which only contain made-up companies and contacts.

## Layout

```
src/
  main.rs   # commands, app state, key handling
  data.rs   # Application, Status, spreadsheet import/export, "to do" logic
  db.rs     # SQLite storage
  form.rs   # add/edit form
  ui.rs     # drawing
  tests.rs  # cargo test
schema.sql  # SQLite schema, embedded in the binary
examples/
  sample.ods   # fake data, same structure as the real tracker
  sample.xlsx  # the same, in Excel format
```

Built with [ratatui](https://ratatui.rs), [rusqlite](https://github.com/rusqlite/rusqlite),
[calamine](https://github.com/tafia/calamine), [rust_xlsxwriter](https://github.com/jmcnamara/rust_xlsxwriter)
and [chrono](https://github.com/chronotope/chrono).
