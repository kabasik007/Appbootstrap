//! Battery autonomy is an estimate until real read-only JK BMS telemetry is added.
#[derive(Debug, Clone, Copy)]
pub struct BatteryModel {
    pub amp_hours: f64,
    pub nominal_volts: f64,
    pub reserve_pct: f64,
    pub efficiency: f64,
    pub inverter_idle_w: f64,
}
impl Default for BatteryModel {
    fn default() -> Self {
        Self {
            amp_hours: 334.0,
            nominal_volts: 12.8,
            reserve_pct: 10.0,
            efficiency: 0.85,
            inverter_idle_w: 15.0,
        }
    }
}
impl BatteryModel {
    /// AC load covers every device plugged into the UPS, not just CPU+GPU.
    pub fn hours(self, charge_pct: f64, ac_watts: f64) -> Option<f64> {
        if !charge_pct.is_finite() || !(0.0..=100.0).contains(&charge_pct)
            || !ac_watts.is_finite() || ac_watts <= 0.0
            || !self.amp_hours.is_finite() || self.amp_hours <= 0.0
            || !self.nominal_volts.is_finite() || self.nominal_volts <= 0.0
            || !self.efficiency.is_finite() || !(0.01..=1.0).contains(&self.efficiency)
            || !self.inverter_idle_w.is_finite() || self.inverter_idle_w < 0.0
            || !self.reserve_pct.is_finite() || !(0.0..100.0).contains(&self.reserve_pct)
        { return None; }
        let usable_wh = self.amp_hours * self.nominal_volts
            * ((charge_pct - self.reserve_pct).max(0.0) / 100.0);
        Some(usable_wh / (ac_watts / self.efficiency + self.inverter_idle_w))
    }
    pub fn label(self, soc: f64, watts: f64) -> String {
        match self.hours(soc, watts) {
            Some(v) if v < (1.0 / 60.0) => "< 1 хв".to_owned(),
            Some(v) => {
                let minutes = (v * 60.0).floor() as u64;
                format!("{} год {} хв", minutes / 60, minutes % 60)
            }
            None => "—".to_owned()
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn reserve_is_zero() { assert_eq!(BatteryModel::default().hours(10.0, 300.0), Some(0.0)); }
    #[test] fn full_has_energy() { assert!(BatteryModel::default().hours(100.0, 300.0).unwrap() > 8.0); }
    #[test] fn invalid_inputs() {
        for n in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(BatteryModel::default().hours(50.0, n).is_none());
        }
        assert!(BatteryModel::default().hours(50.0, 0.0).is_none());
        assert!(BatteryModel::default().hours(101.0, 500.0).is_none());
    }
    #[test] fn heavier_load_shortens_runtime() {
        let m = BatteryModel::default();
        assert!(m.hours(78.0, 200.0) > m.hours(78.0, 600.0));
    }
}
