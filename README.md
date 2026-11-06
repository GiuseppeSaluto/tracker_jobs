# tracker_jobs

A terminal dashboard (TUI) in Rust for a job-application tracker kept in a spreadsheet (`.ods`, `.xlsx`, `.xls`).
The spreadsheet is the single source of truth: you edit it in LibreOffice Calc or Excel, tracker_jobs **only reads it**.

<!-- screenshot: e.g. `vhs demo.tape` or `asciinema rec` -->

## What it shows

- **Summary**: total, open, interviews, offers.
- **To do**: open applications with a next action due within 3 days (overdue in red, today in yellow).
- **By status**: one bar per status.
- **Table**: all applications sorted by next action date, with a details popup for the selected row.

## Usage

```bash
cargo run -- examples/sample.ods                # fake data, to try it out
cargo run -- examples/sample.xlsx               # same data, Excel format
cargo run --release -- ~/path/to/your_file.ods   # your real file
```

| Key | Action |
|---|---|
| `↑` `↓` | move the selection |
| `Enter` | row details (`Esc` closes) |
| `f` | filter: all → open → closed |
| `r` | reload the file from disk |
| `q` / `Esc` | quit |

## Spreadsheet format

The sheet and column names are in Italian, as in the original tracker.

Sheet `Candidature`, row 1 holds the headers. Columns are looked up by name, so their order doesn't matter:

| Header | Meaning |
|---|---|
| `Azienda` | Company (required: rows without it are skipped) |
| `Ruolo` | Role |
| `Settore` | Sector |
| `Tipo ruolo` | Role type |
| `Sede` | Location |
| `Stato` | Status: `Da inviare`, `Inviata`, `In selezione`, `Colloquio`, `Offerta`, `Rifiutata`, `Ritirata`, `Nessuna risposta` |
| `Prossima azione` | Next action |
| `Data prossima azione` | Next action date |
| `Data invio` | Date sent |

Any other column is ignored. An application is *open* unless its status is `Offerta`, `Rifiutata` or `Ritirata`.

## Privacy

Real data stays out of the repository: `.gitignore` excludes every spreadsheet except those in `examples/`,
which only contain made-up companies and contacts. Pass your real file on the command line.

## Layout

```
src/
  main.rs   # arguments, event loop
  data.rs   # spreadsheet reading, Status, parse_date, "to do" logic
  ui.rs     # drawing
  tests.rs  # cargo test
examples/
  sample.ods   # fake data, same structure as the real file
  sample.xlsx  # the same, in Excel format
```

Built with [ratatui](https://ratatui.rs), [calamine](https://github.com/tafia/calamine) and [chrono](https://github.com/chronotope/chrono).
