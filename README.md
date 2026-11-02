# Slint Workbench

A desktop application shell written with [Slint](https://slint.dev/): a menu
bar, a toolbar, a side bar, a status bar, four panes (Application, Instrument
panel, User guide, Preferences), a JSONC configuration file with a JSON Schema
that drives the Preferences form, a native user guide and right-click menus.

It is a standalone port of [Raven Workbench](https://github.com/skp64-pvt01/Raven-workbench):
the same layout, actions, settings file, schema and checks, with Slint instead
of Raven. It is meant to be **copied** as the template of the next desktop
project.

---

## What it demonstrates

| Piece | How |
|---|---|
| **Menu bar** | Slint's native `MenuBar` with File / Edit / View / Help and `@keys(...)` shortcuts |
| **Toolbar** | optional row of `Button`s; `"ui": { "toolbar": false }` removes it |
| **Side bar** | the four panes plus a placeholder, with the active one highlighted |
| **Central pane** | `if root.pane == N` sections; the plant keeps its state when hidden |
| **Application page** | version, channel, toolkit, configuration file and its state, shortcuts |
| **Instrument panel** | gauge and trend as `Path` commands generated from the plant, a digital meter, lamps, switches and a 30 Hz timer |
| **User guide** | native text pages (Back / Forward / Index) driven by Rust |
| **Preferences** | a form generated from the schema: search, check boxes, combo boxes, text fields for numbers and strings, Save and Reset |
| **Configuration** | JSONC, applied at startup: theme, toolbar, pane, switches, alarm threshold, trend window |
| **Comment-preserving save** | the reader records value spans; Save replaces only values and keeps comments and layout |
| **Context menus** | Slint's built-in `ContextMenuArea` on the side bar and every pane, with the same actions |
| **Headless tests** | the model tests plus a UI test on Slint's testing backend |

---

## Layout

```
Slint-workbench/
├── Cargo.toml
├── build.rs               compiles the .slint with debug info for tests
├── ui/workbench.slint     the whole interface
├── workbench.jsonc        the settings, shared with the other ports
├── workbench.schema.json  JSON Schema: the same settings, described
├── src/
│   ├── main.rs            launch, callbacks, timer, --config, --test, UI test
│   ├── model.rs           AppModel and the self-test
│   ├── plant.rs           the simulated plant and trend (no UI)
│   ├── guide.rs           the user guide as native text
│   ├── json.rs            JSONC reader with value spans
│   ├── config.rs          settings, defaults, get/set, comment-preserving save
│   └── schema.rs          the JSON Schema reader
├── AGENTS.md              build/test hand-off and template steps
├── DESIGN.md              architecture, decisions, extension recipes
├── NOTICE
└── LICENSE                GPL-3.0-only (the Slint option this project uses)
```

---

## Build and run

Rust 1.92+ (the Slint 1.18 requirement).

```bash
cargo run --release                        # the window
cargo run --release -- --test              # model checks, no window
cargo run --release -- --config mine.jsonc # another settings file
```

`WORKBENCH_CONFIG` is read when `--config` is absent; otherwise the
`workbench.jsonc` next to `Cargo.toml` is used.

The default Slint backend is selected at run time (winit/Femtovg/software under
the `slint` crate's default features). A headless machine can still run
`cargo test`, which uses the testing backend.

## Test

```bash
cargo test
```

* model tests: JSONC, settings, schema, the plant (pump, valve, interlock,
  reset, pause, trend window), save/reload and comment preservation;
* a UI test on `i-slint-backend-testing`: it creates the real `Workbench`, wires
  the callbacks, changes panes/settings/guide and clicks a side-bar button found
  by its accessible label;
* `cargo run -- --test` runs the model checks and prints `PASS`.

---

## Configuration

JSONC: `//` and block comments, trailing commas. Every key is optional; a
missing key, wrong type or missing file keeps the default, and a syntax error is
reported in the status bar while the defaults are used.

| Key | Type | Default | Effect |
|---|---|---|---|
| `release.channel` | string | `dev` | shown in the application page and status bar |
| `ui.theme` | `dark` \| `light` | `dark` | colours of the shell |
| `ui.toolbar` | boolean | `true` | `false` builds the window without the toolbar |
| `startup.pane` | `application` \| `instrument-panel` | `application` | the page shown at startup |
| `startup.pump` / `valve` / `armed` | boolean | `false` | the plant's switches at startup |
| `simulation.alarm_limit` | number | `75` | interlock threshold (10..100) |
| `simulation.chart_seconds` | number | `10` | trend window (1..10) |

`workbench.schema.json` is JSON Schema 2020-12; the preferences form reads
`properties`, `title`, `description`, `type`, `enum`, `minimum`, `maximum` and
`default`, and ignores the rest.

---

## Differences from the Raven Workbench

* The layout is `.slint`; the logic and drawing data are Rust.
* Numbers in the form use `LineEdit` (Slint's `SpinBox` is integer-only);
  `config::set` parses and clamps them.
* The user guide is native text (`src/guide.rs`), not an embedded Chromium page.
* Clipboard entries use the `arboard` crate (Slint has no public clipboard API
  in 1.18).
* Menu items are not checkable; the active pane is shown by the side bar.
* There is no `--capture`; the UI test runs on the testing backend.

---

## License

GPL-3.0-only, the Slint option this project uses. Slint is also available under
its royalty-free license; see [`NOTICE`](NOTICE) and Slint's own license terms.
