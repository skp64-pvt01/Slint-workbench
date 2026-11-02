//! The application state, independent of the toolkit.
//!
//! The Slint file lays out properties; this module owns the settings, the
//! schema and the plant, so `cargo test` (and `--test`) can exercise the
//! behaviour without opening a window.

use crate::config::{self, Config};
use crate::guide::{self, Page};
use crate::plant::{Plant, PANEL_INTERLOCKS, PANEL_PUMP, PANEL_VALVE};
use crate::schema::{self, Schema, Setting};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Must match the `slint` entry in Cargo.lock; `self_test` checks it.
pub const SLINT_VERSION: &str = "1.18.1";
pub const DEFAULT_CONFIG: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/workbench.jsonc");
pub const SCHEMA_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/workbench.schema.json");

pub const PANE_APPLICATION: i32 = 0;
pub const PANE_PANEL: i32 = 1;
pub const PANE_HELP: i32 = 2;
pub const PANE_PREFERENCES: i32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct AppModel {
    pub config: Config,
    pub schema: Schema,
    pub plant: Plant,
    pub pane: i32,
    pub filter: String,
    pub status: String,
    pub guide_page: usize,
}

impl Default for AppModel {
    fn default() -> Self {
        Self::new(DEFAULT_CONFIG)
    }
}

impl AppModel {
    pub fn new(config_path: &str) -> Self {
        let mut model = Self {
            config: config::load(config_path),
            schema: schema::load(SCHEMA_PATH),
            plant: Plant::default(),
            pane: PANE_APPLICATION,
            filter: String::new(),
            status: String::new(),
            guide_page: 0,
        };
        model.apply_config();
        model
    }

    pub fn with_config(config: Config) -> Self {
        let mut model = Self { config, schema: schema::load(SCHEMA_PATH), ..Self::default() };
        model.apply_config();
        model
    }

    fn apply_config(&mut self) {
        self.plant.set_alarm_limit(self.config.alarm_limit);
        self.plant.set_window(self.config.chart_seconds);
        self.plant.set_switch(PANEL_PUMP, self.config.pump);
        self.plant.set_switch(PANEL_VALVE, self.config.valve);
        self.plant.set_switch(PANEL_INTERLOCKS, self.config.armed);
        self.pane = match self.config.pane {
            1 => PANE_PANEL,
            2 => PANE_HELP,
            3 => PANE_PREFERENCES,
            _ => PANE_APPLICATION,
        };
        self.status = self.config.status();
    }

    pub fn dark(&self) -> bool {
        self.config.theme != "light"
    }

    pub fn set_pane(&mut self, pane: i32) {
        self.pane = if (PANE_PANEL..=PANE_PREFERENCES).contains(&pane) { pane } else { PANE_APPLICATION };
        self.status = match self.pane {
            PANE_PANEL => "Instrument panel opened.",
            PANE_HELP => "User guide.",
            PANE_PREFERENCES => "Preferences.",
            _ => "Application information.",
        }
        .into();
    }

    pub fn toggle_running(&mut self) {
        self.plant.running = !self.plant.running;
        self.status = if self.plant.running {
            "Simulation running.".into()
        } else {
            "Simulation paused.".into()
        };
    }

    pub fn reset_plant(&mut self) {
        self.plant.reset();
        self.plant.running = true;
        self.status = "Plant reset to a cold start.".into();
    }

    pub fn set_theme(&mut self, dark: bool) {
        self.config.theme = if dark { "dark" } else { "light" }.into();
        self.status = if dark {
            "Dark theme. Save in Preferences to keep it.".into()
        } else {
            "Light theme. Save in Preferences to keep it.".into()
        };
    }

    pub fn apply_live_settings(&mut self) {
        self.plant.set_alarm_limit(self.config.alarm_limit);
        self.plant.set_window(self.config.chart_seconds);
        self.plant.set_switch(PANEL_PUMP, self.config.pump);
        self.plant.set_switch(PANEL_VALVE, self.config.valve);
        self.plant.set_switch(PANEL_INTERLOCKS, self.config.armed);
    }

    pub fn setting(&self, path: &str) -> Option<&Setting> {
        self.schema
            .groups
            .iter()
            .flat_map(|group| group.settings.iter())
            .find(|setting| setting.path == path)
    }

    pub fn set_setting(&mut self, path: &str, value: &str) -> bool {
        if !config::set(&mut self.config, path, value) {
            return false;
        }
        self.apply_live_settings();
        let title = self.setting(path).map(|setting| setting.title.as_str()).unwrap_or(path);
        self.status = format!("Changed \"{}\" - not saved yet.", title);
        true
    }

    pub fn save(&mut self) -> bool {
        match config::save(&self.config.path.clone(), &self.config) {
            Ok(()) => {
                self.config.loaded = true;
                self.config.error.clear();
                self.config.settings = self.schema.setting_count() as i32;
                self.status =
                    format!("Saved {}. Toolbar and first pane apply at the next start.", self.config.path);
                true
            }
            Err(error) => {
                self.status = error;
                false
            }
        }
    }

    pub fn reset_settings(&mut self) {
        let path = self.config.path.clone();
        self.config = config::defaults(&path);
        self.apply_live_settings();
        self.status = "Defaults restored in memory - Save writes them.".into();
    }

    pub fn setting_visible(&self, setting: &Setting) -> bool {
        let needle = self.filter.to_lowercase();
        needle.is_empty()
            || setting.title.to_lowercase().contains(&needle)
            || setting.description.to_lowercase().contains(&needle)
            || setting.path.to_lowercase().contains(&needle)
    }

    pub fn shown_settings(&self) -> usize {
        self.schema
            .groups
            .iter()
            .flat_map(|group| group.settings.iter())
            .filter(|setting| self.setting_visible(setting))
            .count()
    }

    pub fn guide(&self) -> &'static Page {
        guide::page(self.guide_page)
    }

    pub fn set_guide_page(&mut self, page: usize) {
        self.guide_page = page.min(guide::PAGES.len() - 1);
    }

    pub fn status_right(&self) -> String {
        if self.pane == PANE_APPLICATION {
            format!("Slint Workbench {}   {}", VERSION, self.config.channel)
        } else {
            format!(
                "P-101 {:5.1} bar   F-201 {:5.1} L/min   {}{}",
                self.plant.pressure,
                self.plant.flow,
                if self.plant.running { "running" } else { "paused" },
                if self.plant.alarm() { "   ALARM" } else { "" }
            )
        }
    }

    pub fn about_text(&self) -> String {
        format!(
            "Slint Workbench {} ({})\n\nSlint {}\n\nConfiguration:\n  {}\n  {}",
            VERSION,
            self.config.channel,
            SLINT_VERSION,
            self.config.path,
            self.config.status()
        )
    }
}

fn locked_version(crate_name: &str) -> Option<String> {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).ok()?;
    let mut lines = lock.lines();
    while let Some(line) = lines.next() {
        if line == format!("name = \"{}\"", crate_name) {
            let version = lines.next()?.trim();
            return version.strip_prefix("version = \"")?.strip_suffix('"').map(str::to_string);
        }
    }
    None
}

/// The checks `--test` runs, and the ones `cargo test` runs through the model.
pub fn self_test() -> Vec<String> {
    let mut failures = Vec::new();
    macro_rules! check {
        ($condition:expr, $message:expr) => {
            if !$condition {
                failures.push($message.to_string());
            }
        };
    }

    let path = "/tmp/slint_workbench_test.jsonc";
    std::fs::write(
        path,
        "{\n  // a hand-written configuration\n  \"release\": { \"channel\": \"beta\" },\n  \"ui\": { \"theme\": \"light\", \"toolbar\": false, },\n  /* the pane to open at startup */\n  \"startup\": { \"pane\": \"instrument-panel\", \"pump\": true, \"valve\": true },\n  \"simulation\": { \"alarm_limit\": 50, \"chart_seconds\": 5 },\n}\n",
    )
    .unwrap();
    let custom = config::load(path);
    check!(custom.loaded, "the JSONC file loads (comments and trailing commas)");
    check!(custom.channel == "beta", "release.channel");
    check!(custom.theme == "light", "ui.theme");
    check!(!custom.toolbar, "ui.toolbar");
    check!(custom.pane == 1, "startup.pane");
    check!(custom.pump && custom.valve && !custom.armed, "startup switches");
    check!(custom.alarm_limit == 50.0, "simulation.alarm_limit");
    check!(custom.chart_seconds == 5.0, "simulation.chart_seconds");
    check!(custom.settings == 8, "eight settings applied");

    std::fs::write(path, "{ \"ui\": { \"theme\": } }").unwrap();
    let broken = config::load(path);
    check!(!broken.loaded && !broken.error.is_empty(), "a syntax error is reported");
    check!(broken.channel == "dev", "a broken file keeps the defaults");
    check!(
        config::load("/tmp/slint_workbench_absent.jsonc").error.is_empty(),
        "a missing file is not an error"
    );

    let mut model = AppModel::default();
    check!(model.pane == PANE_APPLICATION, "the application pane is the default");
    check!(model.config.toolbar, "the toolbar is shown by default");
    check!(model.plant.window_seconds == 10.0, "the chart window defaults to 10 s");
    check!(model.plant.alarm_limit == 75.0, "the alarm defaults to 75 bar");
    check!(model.schema.loaded, "the schema loads");
    check!(model.schema.groups.len() == 4, "four groups in the schema");
    check!(model.schema.setting_count() == 9, "nine settings in the schema");

    model.set_pane(PANE_PANEL);
    for _ in 0..90 {
        model.plant.simulate(1.0 / 60.0);
    }
    let samples = model.plant.samples();
    check!(samples > 0, "the panel collected samples");
    model.set_pane(PANE_APPLICATION);
    let frozen = model.plant.samples();
    check!(model.plant.samples() == frozen, "a hidden pane does not simulate");

    model.set_pane(PANE_PANEL);
    model.toggle_running();
    check!(!model.plant.running, "Pause stops the plant");
    model.toggle_running();
    model.reset_plant();
    check!(model.plant.samples() == 0, "Reset empties the trend");

    check!(model.shown_settings() == 9, "every setting has a row");
    model.filter = "trend".into();
    check!(model.shown_settings() == 1, "the search filters the rows");
    model.filter.clear();
    check!(model.shown_settings() == 9, "clearing the search shows them all");
    check!(model.set_setting("ui.toolbar", "false"), "the form changes a setting");
    check!(!model.config.toolbar, "the setting reached the model");
    model.reset_settings();
    check!(model.config.toolbar, "Reset restores the defaults");

    let path = "/tmp/slint_workbench_comments.jsonc";
    std::fs::write(
        path,
        "{\n  // keep this comment\n  \"release\": { \"channel\": \"dev\" },\n  \"ui\": {\n    \"theme\": \"dark\", // inline comment\n    \"toolbar\": true\n  },\n  \"startup\": { \"pane\": \"application\", \"pump\": false, \"valve\": false },\n  \"simulation\": { \"alarm_limit\": 75, \"chart_seconds\": 10 }\n}\n",
    )
    .unwrap();
    let mut edited = config::load(path);
    edited.theme = "light".into();
    edited.armed = true;
    check!(config::save(path, &edited).is_ok(), "save edits the file in place");
    let text = std::fs::read_to_string(path).unwrap();
    check!(text.contains("// keep this comment"), "comments survive a save");
    check!(text.contains("// inline comment"), "an inline comment survives a save");
    check!(text.contains("\"theme\": \"light\""), "the value was replaced");
    check!(text.contains("\"armed\": true"), "a missing key was added");
    let reread = config::load(path);
    check!(reread.theme == "light" && reread.armed, "the edited file reads back");
    let _ = std::fs::remove_file(path);

    let mut configured = AppModel::with_config(custom);
    check!(configured.pane == PANE_PANEL, "startup.pane opens the instrument panel");
    check!(configured.plant.alarm_limit == 50.0, "the alarm limit reaches the plant");
    check!(configured.plant.window_seconds == 5.0, "the chart window reaches the trend");
    check!(configured.plant.switch(PANEL_PUMP) && configured.plant.switch(PANEL_VALVE), "the switches are on");
    check!(!configured.dark(), "ui.theme = light");
    let start = configured.plant.pressure;
    for _ in 0..180 {
        configured.plant.simulate(1.0 / 60.0);
    }
    check!(configured.plant.pressure > start + 20.0, "the configured pump runs the plant");

    check!(
        locked_version("slint").as_deref() == Some(SLINT_VERSION),
        "the locked Slint version matches the one the application shows"
    );
    check!(guide::PAGES.len() >= 3, "the guide has pages");
    check!(!guide::page(0).sections.is_empty(), "the guide has sections");

    failures
}
