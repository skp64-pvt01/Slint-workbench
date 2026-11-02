# AGENTS.md — Slint Workbench

A standalone desktop application shell on [Slint](https://slint.dev/), modelled
on Raven Workbench: four panes, menus, a toolbar, a side bar, a JSONC settings
file with a JSON Schema and a generated Preferences form, a simulated
instrument panel and a native user guide.

The aim is the template of the next desktop project, so it must build with
plain Cargo, run with `cargo run`, and be checked with `cargo test` and
`--test` without a display.

---

## What is here

| Path | What it is |
|---|---|
| `ui/workbench.slint` | the whole interface: menu bar, toolbar, side bar, four panes, status bar, overlays |
| `build.rs` | compiles the `.slint` (with debug info, so the test backend can find elements) |
| `src/main.rs` | launch, callbacks, timer, `--config`, `--test`, the UI test |
| `src/model.rs` | `AppModel`: panes, settings, status, self-test |
| `src/plant.rs` | the simulated plant and trend (no UI) |
| `src/guide.rs` | the user guide as native text |
| `src/json.rs` | JSONC reader with value spans |
| `src/config.rs` | `Config`, defaults, load, get/set, comment-preserving save |
| `src/schema.rs` | reads `workbench.schema.json` into groups of settings |
| `workbench.jsonc` | the shipped configuration (comments included) |
| `workbench.schema.json` | the JSON Schema that drives the form |
| `README.md` | what it does, how to build and run |
| `DESIGN.md` | architecture, decisions, extension recipes |

## Prerequisites

* Rust 1.92 or newer with **Slint 1.18.1**. The locked version is checked against the one the UI shows.
* A desktop session for the window; `cargo test` uses the testing backend.
* No extra native packages beyond what the default Slint backend pulls in on
  Linux (winit + X11/Wayland, or the software renderer).

## Build and test

```bash
cargo build
cargo run --release                        # the window
cargo test                                 # model + UI on the testing backend
cargo run --release -- --test              # model checks, no window
cargo run --release -- --config mine.jsonc
```

`cargo test` is the full check: it creates the real `Workbench`, wires the Rust
callbacks and clicks a button through accessibility, so a broken `.slint`
binding fails the build or the test. `--test` is the model checks in the binary,
with a `PASS` line for scripts.

Run the commands from this folder: the default `workbench.jsonc` and
`workbench.schema.json` paths are compiled from `CARGO_MANIFEST_DIR`.

## Rules

1. **The `.slint` is layout only.** State and behaviour live in `AppModel` /
   `Plant`; the Slint file exposes properties and callbacks. Do not add business
   logic to a callback in the UI file.
2. **Rust sets every display string.** The UI does not format floats or build
   sentences; Rust sets `pressure-text`, `status-right`, `meta-text` and so on.
   That keeps the two sides independent and the tests simple.
3. **Actions are one place.** Menu, toolbar, side bar and context menu call the
   same Rust callback or model method.
4. **A setting lives in four places**: the `Config` field, `config::get` /
   `config::set`, `workbench.schema.json`, and `workbench.jsonc`.
5. **Every behaviour gets a check**, in the model tests or the UI test.
6. **Write only to `/tmp` from tests.** The save tests point the config path at
   a temporary file; never touch the shipped `workbench.jsonc`.
7. **Do not commit `target/`.**
8. **Keep the shared contract.** `workbench.jsonc`, `workbench.schema.json` and
   the checks should stay aligned with the Raven and Dioxus Workbench ports.

## Pitfalls

| Symptom | Cause |
|---|---|
| `Cannot specify both 'width' and 'min-width'` | A `Window` sets its size one way; use `width`/`height` or the `min-*` pair, not both. |
| `@keys(Control + 0)` fails | A digit must be a string: `@keys(Control + "0")`. |
| The UI test cannot find an element | `ElementHandle` needs debug info: `build.rs` calls `compile_with_config(..., CompilerConfiguration::new().with_debug_info(true))`. |
| `Cannot specify both ...` inside a layout | A layout child with `width` also inherits stretch; remove one or wrap it. |
| The gauge or trend is blank | The `Path` `commands` are empty. Fill `gauge-ticks` / `chart-pressure` from Rust; a plant with fewer than two samples has no line. |
| The chart is tiny or clipped | The `Path` commands use a fixed 480×160 canvas, centred in the tile; keep the `Path` `width`/`height` and the Rust coordinates in step. |
| A number in Preferences cannot be edited | Slint's `SpinBox` is integer-only; numbers are `LineEdit`s whose text `config::set` parses and clamps. |
| `ElementHandle` panics after another test | The testing backend can be initialised once per process; keep the UI test a single `#[test]`. |
| The window does not appear | A headless session; use `--test` and `cargo test`. |

## Using this project as a template

1. Copy the folder.
2. Rename `slint-workbench` in `Cargo.toml`, the window title in
   `ui/workbench.slint`, and "Slint Workbench" in the UI/README.
3. Replace `workbench.jsonc`, `workbench.schema.json` and `guide.rs` with your
   own content.
4. Keep `plant.rs` and the panel section only if you want the instrument pane.
5. Keep `cargo test` and `--test` green; add checks for new panes and settings.
6. Choose the Slint license option you distribute under (GPL or royalty-free).

## Definition of done

* `cargo build` with **zero warnings**.
* `cargo test` passes (model and UI).
* `cargo run -- --test` prints `PASS`.
* README / schema / guide updated together.
* Nothing built or temporary is staged.
