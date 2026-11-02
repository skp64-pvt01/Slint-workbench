//! The simulated plant behind the instrument pane.
//!
//! The same model as the C++, Rust and Python Workbench implementations: two
//! instruments (pressure and flow), a trend, three switches and an interlock
//! threshold. It contains no UI so it can be tested without a window.

pub const SAMPLES: usize = 300; // ten seconds at 30 Hz
pub const SAMPLE_INTERVAL: f64 = 1.0 / 30.0;

pub const PANEL_PUMP: i32 = 0;
pub const PANEL_VALVE: i32 = 1;
pub const PANEL_INTERLOCKS: i32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct Plant {
    pub running: bool,
    pub pump: bool,
    pub valve: bool,
    pub armed: bool,
    pub time: f64,
    pub pressure: f32,
    pub flow: f32,
    pub alarm_limit: f64,
    pub window_seconds: f64,
    pub visible_samples: usize,

    pressure_history: [f32; SAMPLES],
    flow_history: [f32; SAMPLES],
    sample_count: usize,
    sample_head: usize,
    last_sample: f64,
}

impl Default for Plant {
    fn default() -> Self {
        Self {
            running: true,
            pump: false,
            valve: false,
            armed: false,
            time: 0.0,
            pressure: 2.0,
            flow: 0.0,
            alarm_limit: 75.0,
            window_seconds: 10.0,
            visible_samples: SAMPLES,
            pressure_history: [0.0; SAMPLES],
            flow_history: [0.0; SAMPLES],
            sample_count: 0,
            sample_head: 0,
            last_sample: 0.0,
        }
    }
}

impl Plant {
    pub fn simulate(&mut self, delta: f64) {
        if !self.running || delta <= 0.0 {
            return;
        }
        self.time += delta;
        let t = self.time as f32;

        let mut pressure_target = if self.pump { 68.0 } else { 2.0 };
        if self.pump {
            pressure_target += 2.5 * (t * 1.7).sin();
        }
        let mut flow_target = if self.valve { if self.pump { 44.0 } else { 12.0 } } else { 0.0 };
        if self.valve {
            flow_target += 3.0 * (t * 3.1).sin();
        }

        self.pressure += (pressure_target - self.pressure) * (1.0 - (-(delta as f32) / 0.6).exp());
        self.flow += (flow_target - self.flow) * (1.0 - (-(delta as f32) / 0.35).exp());
        self.pressure = self.pressure.max(0.0);
        self.flow = self.flow.max(0.0);

        if self.time - self.last_sample >= SAMPLE_INTERVAL {
            self.last_sample = self.time;
            self.pressure_history[self.sample_head] = self.pressure;
            self.flow_history[self.sample_head] = self.flow;
            self.sample_head = (self.sample_head + 1) % SAMPLES;
            if self.sample_count < SAMPLES {
                self.sample_count += 1;
            }
        }
    }

    /// Back to a cold plant and an empty trend; the switches keep their position.
    pub fn reset(&mut self) {
        self.time = 0.0;
        self.last_sample = 0.0;
        self.pressure = 2.0;
        self.flow = 0.0;
        self.sample_count = 0;
        self.sample_head = 0;
        self.pressure_history = [0.0; SAMPLES];
        self.flow_history = [0.0; SAMPLES];
    }

    pub fn alarm(&self) -> bool {
        self.armed && self.pressure > self.alarm_limit as f32
    }

    pub fn samples(&self) -> usize {
        self.sample_count
    }

    pub fn set_alarm_limit(&mut self, bar: f64) {
        self.alarm_limit = bar.clamp(10.0, 100.0);
    }

    pub fn set_window(&mut self, seconds: f64) {
        self.window_seconds = seconds.clamp(1.0, 10.0);
        let samples = (self.window_seconds * 30.0 + 0.5) as usize;
        self.visible_samples = samples.clamp(30, SAMPLES);
    }

    pub fn set_switch(&mut self, which: i32, on: bool) {
        match which {
            PANEL_PUMP => self.pump = on,
            PANEL_VALVE => self.valve = on,
            PANEL_INTERLOCKS => self.armed = on,
            _ => {}
        }
    }

    pub fn switch(&self, which: i32) -> bool {
        match which {
            PANEL_PUMP => self.pump,
            PANEL_VALVE => self.valve,
            PANEL_INTERLOCKS => self.armed,
            _ => false,
        }
    }

    /// The visible trend as `(pressure, flow)` pairs, oldest first.
    pub fn trend(&self) -> Vec<(f32, f32)> {
        let visible = self.visible_samples.min(self.sample_count);
        (0..visible)
            .map(|i| {
                let index = (self.sample_head + SAMPLES - visible + i) % SAMPLES;
                (self.pressure_history[index], self.flow_history[index])
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pump_raises_the_pressure() {
        let mut plant = Plant::default();
        let start = plant.pressure;
        plant.set_switch(PANEL_PUMP, true);
        for _ in 0..180 {
            plant.simulate(1.0 / 60.0);
        }
        assert!(plant.pressure > start + 20.0);
        assert!(plant.samples() > 0);
    }

    #[test]
    fn the_valve_lets_the_flow_through() {
        let mut plant = Plant::default();
        plant.set_switch(PANEL_PUMP, true);
        plant.set_switch(PANEL_VALVE, true);
        for _ in 0..240 {
            plant.simulate(1.0 / 60.0);
        }
        assert!(plant.flow > 20.0);
    }

    #[test]
    fn the_interlock_trips_at_the_limit() {
        let mut plant = Plant::default();
        plant.set_switch(PANEL_INTERLOCKS, true);
        plant.pressure = 80.0;
        assert!(plant.alarm());
        plant.set_alarm_limit(90.0);
        assert!(!plant.alarm());
    }

    #[test]
    fn reset_empties_the_trend_and_keeps_the_switches() {
        let mut plant = Plant::default();
        plant.set_switch(PANEL_PUMP, true);
        plant.simulate(1.0);
        assert!(plant.samples() > 0);
        plant.reset();
        assert_eq!(plant.samples(), 0);
        assert!(plant.pump);
        assert_eq!(plant.pressure, 2.0);
    }

    #[test]
    fn a_paused_plant_does_not_advance() {
        let mut plant = Plant::default();
        plant.running = false;
        let frozen = plant.time;
        plant.simulate(1.0);
        assert_eq!(plant.time, frozen);
    }

    #[test]
    fn the_trend_window_limits_the_visible_samples() {
        let mut plant = Plant::default();
        plant.set_window(5.0);
        assert_eq!(plant.visible_samples, 150);
        for _ in 0..300 {
            plant.simulate(1.0 / 30.0);
        }
        assert_eq!(plant.trend().len(), 150);
    }
}
