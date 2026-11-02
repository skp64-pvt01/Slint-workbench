//! Slint Workbench - a desktop application shell on Slint.
//!
//! The same application as Raven Workbench (and its Dioxus port): a menu bar, a
//! toolbar, a side bar, a status bar, four panes (Application, Instrument panel,
//! User guide, Preferences), a JSONC configuration file with a JSON Schema that
//! drives the Preferences form, and a simulated instrument panel.
//!
//! ```sh
//! cargo run --release                  # the application
//! cargo test                           # the model and the UI wiring
//! cargo run --release -- --test        # the same model checks, no window
//! cargo run --release -- --config mine.jsonc
//! ```

mod config;
mod guide;
mod json;
mod model;
mod plant;
mod schema;

slint::include_modules!();

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

use slint::{ModelRc, SharedString, Timer, TimerMode, VecModel};

use model::{AppModel, PANE_PANEL, DEFAULT_CONFIG, VERSION};
use plant::Plant;

fn config_path(arguments: &[String]) -> String {
    if let Some(index) = arguments.iter().position(|a| a == "--config") {
        if let Some(path) = arguments.get(index + 1) {
            return path.clone();
        }
    }
    if let Ok(path) = std::env::var("WORKBENCH_CONFIG") {
        return path;
    }
    DEFAULT_CONFIG.to_string()
}

/* ---------------------------------------------------------------------------
 *  Drawing data: the gauge and the trend are Path commands
 * ------------------------------------------------------------------------- */

fn gauge_paths(plant: &Plant) -> (String, String) {
    let (cx, cy, radius) = (120.0f32, 105.0f32, 78.0f32);
    let (start, sweep) = (2.35619449f32, 4.71238898f32);
    let mut ticks = String::new();
    for i in 0..=10 {
        let angle = start + sweep * i as f32 / 10.0;
        let major = i % 5 == 0;
        let inner = radius * if major { 0.80 } else { 0.88 };
        ticks.push_str(&format!(
            "M {:.1} {:.1} L {:.1} {:.1} ",
            cx + angle.cos() * inner,
            cy + angle.sin() * inner,
            cx + angle.cos() * radius,
            cy + angle.sin() * radius
        ));
    }
    let value = (plant.pressure / 100.0).clamp(0.0, 1.0);
    let angle = start + sweep * value;
    let needle = format!(
        "M {cx:.1} {cy:.1} L {:.1} {:.1} ",
        cx + angle.cos() * radius * 0.78,
        cy + angle.sin() * radius * 0.78
    );
    (ticks, needle)
}

fn chart_paths(plant: &Plant) -> (String, String, String, String) {
    let (left, right, top, bottom) = (40.0f32, 470.0f32, 10.0f32, 150.0f32);
    let width = right - left;
    let height = bottom - top;
    let mut grid = String::new();
    for step in 0..=4 {
        let y = top + height * step as f32 / 4.0;
        grid.push_str(&format!("M {left:.1} {y:.1} L {right:.1} {y:.1} "));
    }
    for step in 0..=10 {
        let x = left + width * step as f32 / 10.0;
        grid.push_str(&format!("M {x:.1} {top:.1} L {x:.1} {bottom:.1} "));
    }

    let trend = plant.trend();
    let mut pressure = String::new();
    let mut flow = String::new();
    if trend.len() >= 2 {
        for (index, (p, f)) in trend.iter().enumerate() {
            let x = left + width * index as f32 / (trend.len() - 1) as f32;
            let py = top + height * (1.0 - (p / 100.0).clamp(0.0, 1.0));
            let fy = top + height * (1.0 - (f / 100.0).clamp(0.0, 1.0));
            if index == 0 {
                pressure.push_str(&format!("M {x:.1} {py:.1} "));
                flow.push_str(&format!("M {x:.1} {fy:.1} "));
            } else {
                pressure.push_str(&format!("L {x:.1} {py:.1} "));
                flow.push_str(&format!("L {x:.1} {fy:.1} "));
            }
        }
    }

    let alarm = if plant.armed {
        let y = top + height * (1.0 - plant.alarm_limit as f32 / 100.0);
        format!("M {left:.1} {y:.1} L {right:.1} {y:.1} ")
    } else {
        String::new()
    };
    (grid, pressure, flow, alarm)
}

/* ---------------------------------------------------------------------------
 *  Refreshing the UI from the model
 * ------------------------------------------------------------------------- */

fn refresh_shell(ui: &Workbench, app: &AppModel) {
    ui.set_pane(app.pane);
    ui.set_dark(app.dark());
    ui.set_toolbar_visible(app.config.toolbar);
    ui.set_channel(app.config.channel.clone().into());
    ui.set_config_path(app.config.path.clone().into());
    ui.set_config_state(app.config.status().into());
    ui.set_status_text(app.status.clone().into());
    ui.set_status_right(app.status_right().into());
    ui.set_version_text(VERSION.into());
    ui.set_toolkit_label(format!("Slint {}", model::SLINT_VERSION).into());
    ui.set_about_text(app.about_text().into());
}

fn refresh_plant(ui: &Workbench, app: &AppModel) {
    let plant = &app.plant;
    let (ticks, needle) = gauge_paths(plant);
    let (grid, pressure, flow, alarm) = chart_paths(plant);
    ui.set_pressure(plant.pressure);
    ui.set_flow(plant.flow);
    ui.set_alarm_limit(plant.alarm_limit as f32);
    ui.set_window_seconds(plant.window_seconds as f32);
    ui.set_pump(plant.pump);
    ui.set_valve(plant.valve);
    ui.set_armed(plant.armed);
    ui.set_running(plant.running);
    ui.set_alarm(plant.alarm());
    ui.set_pressure_text(format!("{:.1} bar", plant.pressure).into());
    ui.set_flow_text(format!("{:5.1}", plant.flow).into());
    ui.set_window_label(format!("TREND  {:.0} s", plant.window_seconds).into());
    ui.set_samples(plant.samples() as i32);
    ui.set_gauge_ticks(ticks.into());
    ui.set_gauge_needle(needle.into());
    ui.set_chart_grid(grid.into());
    ui.set_chart_pressure(pressure.into());
    ui.set_chart_flow(flow.into());
    ui.set_chart_alarm(alarm.into());
    ui.set_status_right(app.status_right().into());
}

fn refresh_prefs(ui: &Workbench, app: &AppModel) {
    let mut settings = Vec::new();
    for group in &app.schema.groups {
        let mut first = true;
        for setting in &group.settings {
            if !app.setting_visible(setting) {
                continue;
            }
            let value = config::get(&app.config, &setting.path).unwrap_or_default();
            let choices = setting
                .choices
                .iter()
                .map(|choice| SharedString::from(choice.clone()))
                .collect::<Vec<_>>();
            settings.push(PrefSetting {
                path: setting.path.clone().into(),
                group: group.title.clone().into(),
                first_in_group: first,
                title: setting.title.clone().into(),
                description: setting.description.clone().into(),
                kind: setting.kind.clone().into(),
                choices: ModelRc::new(VecModel::from(choices)),
                value: value.into(),
            });
            first = false;
        }
    }
    ui.set_pref_settings(ModelRc::new(VecModel::from(settings)));
    ui.set_shown_settings(app.shown_settings() as i32);
    ui.set_total_settings(app.schema.setting_count() as i32);
    ui.set_meta_text(
        if app.schema.loaded {
            format!("config  {}\nschema  {}", app.config.path, app.schema.path)
        } else {
            format!("config  {}\nschema  {}  ({})", app.config.path, app.schema.path, app.schema.error)
        }
        .into(),
    );
}

fn refresh_guide(ui: &Workbench, app: &AppModel) {
    let page = app.guide();
    let sections = page
        .sections
        .iter()
        .map(|section| GuideSection {
            heading: section.heading.into(),
            body: section.body.into(),
        })
        .collect::<Vec<_>>();
    ui.set_guide_page(app.guide_page as i32);
    ui.set_guide_title(page.title.into());
    ui.set_guide_intro(page.intro.into());
    ui.set_guide_sections(ModelRc::new(VecModel::from(sections)));
}

fn refresh_all(ui: &Workbench, app: &AppModel) {
    refresh_shell(ui, app);
    refresh_plant(ui, app);
    refresh_prefs(ui, app);
    refresh_guide(ui, app);
}

fn copy_to_clipboard(ui: &Workbench, app: &mut AppModel, kind: &str) {
    let text = match kind {
        "version" => format!("Slint Workbench {} ({})", VERSION, app.config.channel),
        "config" => app.config.path.clone(),
        _ => format!("P-101 {:.1} bar   F-201 {:.1} L/min", app.plant.pressure, app.plant.flow),
    };
    match arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
        Ok(()) => app.status = "Copied to the clipboard.".into(),
        Err(_) => app.status = "The clipboard is not available in this session.".into(),
    }
    ui.set_status_text(app.status.clone().into());
}

/* ---------------------------------------------------------------------------
 *  Wiring
 * ------------------------------------------------------------------------- */

fn wire(ui: &Workbench, app: Rc<RefCell<AppModel>>) {
    let weak = ui.as_weak();

    ui.on_set_pane({
        let app = app.clone();
        let weak = weak.clone();
        move |pane| {
            let mut model = app.borrow_mut();
            model.set_pane(pane);
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_plant(&ui, &model);
                refresh_prefs(&ui, &model);
                refresh_guide(&ui, &model);
            }
        }
    });

    ui.on_toggle_running({
        let app = app.clone();
        let weak = weak.clone();
        move || {
            let mut model = app.borrow_mut();
            model.toggle_running();
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_plant(&ui, &model);
            }
        }
    });

    ui.on_reset_plant({
        let app = app.clone();
        let weak = weak.clone();
        move || {
            let mut model = app.borrow_mut();
            model.reset_plant();
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_plant(&ui, &model);
            }
        }
    });

    ui.on_toggle_switch({
        let app = app.clone();
        let weak = weak.clone();
        move |which| {
            let mut model = app.borrow_mut();
            let on = model.plant.switch(which);
            model.plant.set_switch(which, !on);
            if let Some(ui) = weak.upgrade() {
                refresh_plant(&ui, &model);
                refresh_shell(&ui, &model);
            }
        }
    });

    ui.on_set_theme({
        let app = app.clone();
        let weak = weak.clone();
        move |dark| {
            let mut model = app.borrow_mut();
            model.set_theme(dark);
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_plant(&ui, &model);
            }
        }
    });

    ui.on_set_filter({
        let app = app.clone();
        let weak = weak.clone();
        move |text| {
            let mut model = app.borrow_mut();
            model.filter = text.to_string();
            if let Some(ui) = weak.upgrade() {
                refresh_prefs(&ui, &model);
            }
        }
    });

    ui.on_change_setting({
        let app = app.clone();
        let weak = weak.clone();
        move |path, value| {
            let mut model = app.borrow_mut();
            if model.set_setting(&path, &value) {
                if let Some(ui) = weak.upgrade() {
                    refresh_shell(&ui, &model);
                    refresh_plant(&ui, &model);
                }
            }
        }
    });

    ui.on_save_settings({
        let app = app.clone();
        let weak = weak.clone();
        move || {
            let mut model = app.borrow_mut();
            model.save();
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_prefs(&ui, &model);
            }
        }
    });

    ui.on_reset_settings({
        let app = app.clone();
        let weak = weak.clone();
        move || {
            let mut model = app.borrow_mut();
            model.reset_settings();
            if let Some(ui) = weak.upgrade() {
                refresh_all(&ui, &model);
            }
        }
    });

    ui.on_change_guide_page({
        let app = app.clone();
        let weak = weak.clone();
        move |page| {
            let mut model = app.borrow_mut();
            model.set_guide_page(page.max(0) as usize);
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
                refresh_guide(&ui, &model);
            }
        }
    });

    ui.on_copy({
        let app = app.clone();
        let weak = weak.clone();
        move |kind| {
            let mut model = app.borrow_mut();
            if let Some(ui) = weak.upgrade() {
                copy_to_clipboard(&ui, &mut model, &kind);
            }
        }
    });

    ui.on_placeholder({
        let app = app.clone();
        let weak = weak.clone();
        move || {
            let mut model = app.borrow_mut();
            model.status = "Alarms is a placeholder: the panel's alarm lamp is the real one.".into();
            if let Some(ui) = weak.upgrade() {
                refresh_shell(&ui, &model);
            }
        }
    });

    ui.on_quit_app(|| {
        let _ = slint::quit_event_loop();
    });
}

fn run() -> Result<(), slint::PlatformError> {
    let arguments: Vec<String> = std::env::args().collect();
    let app = Rc::new(RefCell::new(AppModel::new(&config_path(&arguments))));
    let ui = Workbench::new()?;
    wire(&ui, app.clone());
    refresh_all(&ui, &app.borrow());

    let weak = ui.as_weak();
    let timer = Timer::default();
    timer.start(TimerMode::Repeated, Duration::from_millis(33), move || {
        if let Some(ui) = weak.upgrade() {
            let mut model = app.borrow_mut();
            if model.pane == PANE_PANEL {
                model.plant.simulate(1.0 / 30.0);
                refresh_plant(&ui, &model);
            }
        }
    });

    ui.run()
}

fn main() -> Result<(), slint::PlatformError> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.iter().any(|argument| argument == "--test") {
        let mut model = AppModel::new(&config_path(&arguments));
        for _ in 0..90 {
            model.plant.simulate(1.0 / 60.0);
        }
        let failures = model::self_test();
        for failure in &failures {
            println!("  FAIL: {}", failure);
        }
        println!(
            "slint-workbench: {} checks failed, {} samples, config {}",
            failures.len(),
            model.plant.samples(),
            model.config.status()
        );
        println!("{}", if failures.is_empty() { "PASS" } else { "FAIL" });
        std::process::exit(if failures.is_empty() { 0 } else { 1 });
    }
    run()
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    use i_slint_backend_testing::ElementHandle;

    #[test]
    fn the_ui_is_wired_to_the_model() {
        i_slint_backend_testing::init_no_event_loop();
        let ui = Workbench::new().unwrap();
        let app = Rc::new(RefCell::new(AppModel::default()));
        wire(&ui, app.clone());
        refresh_all(&ui, &app.borrow());

        assert_eq!(ui.get_pane(), model::PANE_APPLICATION);
        assert_eq!(ui.get_total_settings(), 9);
        assert_eq!(ui.get_pane(), app.borrow().pane);
        assert!(!ui.get_gauge_ticks().is_empty());

        ui.invoke_set_pane(PANE_PANEL);
        assert_eq!(ui.get_pane(), PANE_PANEL);
        assert_eq!(app.borrow().pane, PANE_PANEL);

        ui.invoke_toggle_running();
        assert!(!app.borrow().plant.running);
        ui.invoke_toggle_running();
        assert!(app.borrow().plant.running);

        ui.invoke_set_pane(model::PANE_PREFERENCES);
        assert_eq!(app.borrow().pane, model::PANE_PREFERENCES);
        ui.invoke_set_filter(SharedString::from("trend"));
        assert_eq!(ui.get_shown_settings(), 1);
        ui.invoke_set_filter(SharedString::from(""));
        assert_eq!(ui.get_shown_settings(), 9);

        ui.invoke_change_setting(SharedString::from("ui.toolbar"), SharedString::from("false"));
        assert!(!app.borrow().config.toolbar);
        ui.invoke_reset_settings();
        assert!(app.borrow().config.toolbar);

        ui.invoke_change_guide_page(1);
        assert_eq!(ui.get_guide_page(), 1);
        assert!(ui.get_guide_title().contains("Configuration"));

        // A real button in the side bar, found through accessibility.
        let buttons: Vec<_> = ElementHandle::find_by_accessible_label(&ui, "User guide").collect();
        assert!(!buttons.is_empty());
        buttons[0].invoke_accessible_default_action();
        assert_eq!(app.borrow().pane, model::PANE_HELP);
    }
}
