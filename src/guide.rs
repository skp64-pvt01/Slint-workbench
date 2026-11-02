//! The user guide, as native text.
//!
//! The Raven Workbench embeds the HTML in `help/` in a web view. A Slint or
//! Dioxus application is already the UI toolkit, so the same pages are shown
//! as text: a title, an introduction and sections. The HTML copy stays in the
//! repository for the other implementations and for reading outside the app.

pub struct Section {
    pub heading: &'static str,
    pub body: &'static str,
}

pub struct Page {
    pub title: &'static str,
    pub intro: &'static str,
    pub sections: &'static [Section],
}

pub const PAGES: &[Page] = &[
    Page {
        title: "User guide",
        intro: "A desktop shell with an application page, a simulated instrument \
                panel, this guide and a preferences form. The menu bar, the toolbar \
                and the side bar all reach the same four panes.",
        sections: &[
            Section {
                heading: "Panes",
                body: "Application shows the version, the release channel, the toolkit \
                       and the configuration file in use. Instrument panel runs a small \
                       simulated plant: pressure and flow meters, three lamps and a \
                       trend. User guide is this text. Preferences edits the settings.",
            },
            Section {
                heading: "Shortcuts",
                body: "Ctrl+0  Application\nCtrl+I  Instrument panel\nCtrl+,  Preferences\n\
                       F1      User guide\nCtrl+Q  Quit",
            },
            Section {
                heading: "The settings file",
                body: "workbench.jsonc sits next to this project. Every key is optional. \
                       A missing key, a wrong type or a missing file keeps the default; a \
                       syntax error is reported in the status bar and the defaults are used.\n\n\
                       Start with another file:  dioxus-workbench --config mine.jsonc\n\
                       The WORKBENCH_CONFIG variable works too.",
            },
            Section {
                heading: "Changing a setting",
                body: "Edit > Preferences opens the form generated from \
                       workbench.schema.json. Changes apply at once where that is possible; \
                       Save writes the file in place, keeping its comments and layout. \
                       Toolbar visibility and the first pane apply at the next start.",
            },
        ],
    },
    Page {
        title: "Configuration",
        intro: "One JSONC file, described by a JSON Schema, drives the theme, the \
                toolbar, the first pane, the plant and the preferences form.",
        sections: &[
            Section {
                heading: "release.channel",
                body: "A string shown in the application page, the status bar and About. \
                       Default: dev.",
            },
            Section {
                heading: "ui.theme  /  ui.toolbar",
                body: "\"dark\" or \"light\" selects the theme before the window is built. \
                       false on ui.toolbar removes the toolbar; the View menu still has \
                       Pause and Reset.",
            },
            Section {
                heading: "startup.pane  /  startup.pump / valve / armed",
                body: "The pane shown at startup (\"application\" or \"instrument-panel\") \
                       and the panel's three switches, so the plant can run from the first \
                       frame.",
            },
            Section {
                heading: "simulation.alarm_limit  /  chart_seconds",
                body: "The interlock threshold in bar (10..100) and how much history the \
                       trend shows in seconds (1..10). Both apply at once when the \
                       Preferences form changes them.",
            },
        ],
    },
    Page {
        title: "About the ports",
        intro: "Dioxus Workbench is a standalone project modelled on Raven Workbench. \
                The layout, panes, actions, settings file, schema and checks are the \
                same; the toolkit underneath is Dioxus.",
        sections: &[
            Section {
                heading: "Why a port",
                body: "The Workbench is a realistic shell, not a toy: menus, a toolbar, \
                       a switcher, a generated preferences form, a JSONC file with \
                       comments, and a simulated instrument panel. Porting it shows what \
                       each Rust UI toolkit makes easy and what it makes you write.",
            },
            Section {
                heading: "What is shared",
                body: "workbench.jsonc, workbench.schema.json, help/ and the checks are \
                       shared with the Raven, Slint and other ports. Only the UI layer \
                       differs. The JSONC reader, the settings, the schema reader and the \
                       plant are plain Rust and can be compared line by line.",
            },
            Section {
                heading: "Building and testing",
                body: "cargo run --release       the application\n\
                       cargo test                the model and the rendered UI\n\
                       cargo run --release -- --test   the same checks, with a PASS line",
            },
        ],
    },
];

pub fn page(index: usize) -> &'static Page {
    &PAGES[index.min(PAGES.len() - 1)]
}
