# DESIGN.md — Slint Workbench

A desktop application shell on top of [Slint](https://slint.dev/). This
document describes how the port is put together and how to extend it. It stands
alone: the folder is meant to be copied into the next project.

Raven Workbench is the reference: the same four panes, actions, settings file,
schema and checks, with Slint replacing the native toolkit.

---

## 1. Goals and non-goals

**Goals**

* the same shell as Raven Workbench, in Slint;
* a realistic desktop app: menus, toolbar, side bar, status bar, switcher;
* configuration a user can edit by hand *and* through a form;
* an instrument panel that actually simulates and draws;
* every behaviour checked without a display or a GPU;
* a folder that builds with plain Cargo and can be copied.

**Non-goals**

* a component library or a framework;
* networking, multi-window, persistence beyond the settings file;
* an embedded Chromium guide — the guide is native text here;
* packaging.

---

## 2. Layout

```
Slint-workbench/
├── Cargo.toml              slint + arboard; testing backend for tests
├── build.rs                compile_with_config(..., with_debug_info(true))
├── ui/workbench.slint      the whole interface
├── workbench.jsonc         settings, comments
├── workbench.schema.json   JSON Schema 2020-12
└── src/
    ├── main.rs             launch, callbacks, refresh, timer, --test, UI test
    ├── model.rs            AppModel + self-test
    ├── plant.rs            simulation + trend
    ├── guide.rs            guide pages as text
    ├── json.rs             JSONC reader with spans
    ├── config.rs           Config, load/get/set/save
    └── schema.rs           schema reader
```

---

## 3. Architecture

```
                    Workbench (ui/workbench.slint)
      properties in, callbacks out · MenuBar · ContextMenuArea
                            │
              ┌─────────────┼──────────────┐
              ▼             ▼              ▼
          properties     callbacks      Path commands
        (pane, texts)   (set-pane,     (gauge, trend)
                         save...)
              │             │
              └──────┬──────┘
                     ▼
                AppModel (Rust)
         config · schema · plant · guide
                     │
       ┌─────────────┼─────────────┐
       ▼             ▼             ▼
   config.rs      schema.rs     plant.rs
       │
     json.rs ── workbench.jsonc
```

| Module | Responsibility |
|---|---|
| `ui/workbench.slint` | layout, properties, callbacks, menus, `Path` elements |
| `main.rs` | wiring the callbacks, refreshing properties, the timer, CLI, UI test |
| `model.rs` | all state and behaviour; panes, settings, status, checks |
| `plant.rs` | the simulated process and the trend buffer |
| `guide.rs` | the three guide pages |
| `config.rs` / `json.rs` / `schema.rs` | the same modules as the Raven Rust port |

The Slint file is declarative. Every displayed string is a property that Rust
sets (`pressure-text`, `status-right`, `meta-text`, `guide-title`); every user
action is a callback that Rust handles. That keeps formatting and logic out of
the UI language and makes the generated Rust API the only integration point.

---

## 4. Data flow

1. **Startup** — `main` resolves `--config` / `WORKBENCH_CONFIG`, creates
   `AppModel::new(path)`, creates `Workbench::new()`, wires the callbacks and
   calls `refresh_all`.
2. **Refreshing** — `refresh_shell`, `refresh_plant`, `refresh_prefs` and
   `refresh_guide` push the model into properties. `refresh_plant` also builds
   the `Path` commands for the gauge and the trend.
3. **Actions** — Slint callbacks call `AppModel` methods; the same methods back
   the menu bar, toolbar, side bar and context menus.
4. **Settings** — `change-setting(path, value)` calls `AppModel::set_setting`,
   which updates the config, applies live settings and updates the status.
   `set-filter` rebuilds the list; Save writes the file; Reset restores the
   defaults.
5. **Plant** — a `slint::Timer` wakes every 33 ms; only on the panel pane it
   simulates and refreshes the plant properties.
6. **Guide** — `set-pane` / `change-guide-page` choose a static `Page`; Rust
   pushes its title, intro and sections.

---

## 5. Decisions

**D1 — Layout in Slint, logic in Rust.** The `.slint` owns geometry and
styling; `AppModel` owns state. *Why:* the file stays readable, and the model is
testable without Slint. *Cost:* a property and a callback for every interaction.

**D2 — Display strings come from Rust.** No float formatting or branching in
the UI. *Why:* the same model drives the UI test, the menu labels and the status
bar. *Cost:* more properties.

**D3 — The gauge and trend are `Path` commands.** Rust generates the tick,
needle and polyline commands from the plant. *Why:* Slint has no custom paint
callback in the public API, and `Path` is vector-correct. *Cost:* a fixed
480×160 coordinate system for the trend, centred in its tile.

**D4 — Numbers use `LineEdit`.** Slint's `SpinBox` is integer-only and the
settings are floats. The line edit carries the text; `config::set` parses and
clamps it. *Cost:* no spinner arrows, and invalid text keeps the old value.

**D5 — Native menus and context menus.** `MenuBar` on the `Window` and
`ContextMenuArea` around the side bar and each pane. *Why:* keyboard support,
accessibility and platform placement come for free. *Cost:* menu items are not
checkable in this port (the side bar shows the active pane).

**D6 — The testing backend is the UI test.** `i-slint-backend-testing` creates
the real component, finds a button by accessible label and clicks it.
*Why:* it catches a broken property or callback binding without a display.
*Cost:* the build needs debug info, and the backend can only be initialised once
per process.

**D7 — Save keeps the file as written.** The JSONC reader records value spans;
the writer replaces only the values, inserts missing keys, and re-parses before
writing. *Why:* the configuration is the user's text. *Cost:* inserted keys get
canonical indentation, and an invalid file is refused.

---

## 6. Configuration and schema

Format, lookup order and schema subset are identical to Raven Workbench: JSONC
with `//`, block comments and trailing commas; `--config`, then
`WORKBENCH_CONFIG`, then the shipped file; nine settings in four groups.

Adding a setting: the `Config` field, `config::get`/`config::set`, the schema
entry, `workbench.jsonc`, and — if it applies without a restart —
`apply_live_settings` and `set_setting`. Add checks in `model.rs`.

---

## 7. The instrument panel

`Plant` exposes `simulate`, `reset`, `set_switch`, `switch`, `set_alarm_limit`,
`set_window`, `alarm`, `samples` and `trend`. `main.rs` turns it into:

* `gauge-ticks` and `gauge-needle` (`Path` commands, 240×190);
* `chart-grid`, `chart-pressure`, `chart-flow`, `chart-alarm` (`Path` commands,
  480×160);
* scalar properties for the flow meter, the lamps and the switches.

Adding an instrument: state in `Plant`, a getter, a property, and a Rust
refresh; a configurable one also needs a setter and a live apply.

---

## 8. Extension recipes

* **A new pane** — a `PANE_*` constant, a branch of `root.pane` in the
  `.slint`, a side-bar button, a menu entry, and a model test.
* **A new menu or context entry** — add the item in `MenuBar` /
  `ContextMenuArea` and the Rust callback.
* **A new setting** — see §6.
* **A new guide page** — add a `Page` in `guide.rs`; update the Forward limit in
  the `.slint` (or expose `guide-page-count` from Rust).
* **A new theme** — the `dark` property selects colours; add a palette there.

---

## 9. Testing

`cargo test` runs:

* `plant.rs`: pump/valve/interlock/reset/pause/trend-window;
* `model.rs` self-test: configuration, schema, panes, settings, save/reload and
  comment preservation;
* `ui_tests`: creates `Workbench`, wires the callbacks, changes pane, filter,
  a setting, resets, changes the guide page, and clicks a side-bar button found
  by its accessible label.

`cargo run -- --test` runs the model self-test in the binary and prints a
`PASS`/`FAIL` line. Visual review is done in the window; there is no capture
mode.

---

## 10. Limits and future work

* The trend uses a fixed 480×160 coordinate system, centred in the tile.
* Menu items are not checkable; the side bar indicates the pane.
* Numbers are line edits, not spin boxes.
* The clipboard uses `arboard`; a headless session can fail, and the status bar
  says so.
* The toolbar and first pane apply at the next start.
* Slint's `MenuBar` placement depends on the platform (native on macOS).
* Packaging is out of scope; the Slint license option must be chosen for
  distribution.

---

## 11. File map

| File | Lines (approx.) | Contents |
|---|---|---|
| `ui/workbench.slint` | 700 | all layout, properties, callbacks, menus, paths |
| `src/main.rs` | 450 | wiring, refresh, paths, timer, CLI, UI test |
| `src/model.rs` | 300 | state, actions, self-test |
| `src/plant.rs` | 200 | plant and trend |
| `src/guide.rs` | 110 | guide text |
| `src/config.rs` | 370 | load, get/set, comment-preserving save |
| `src/json.rs` | 300 | JSONC reader with spans |
| `src/schema.rs` | 140 | schema reader |
| `workbench.schema.json` | 100 | settings schema |
| `workbench.jsonc` | 30 | shipped configuration |
