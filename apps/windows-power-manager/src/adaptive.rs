//! Opt-in, bounded adaptive CPU ceiling. This reacts to CPU load, NOT to actual UPS watts.
//! Consecutive samples and cooldown prevent frequent powercfg calls.
#[derive(Debug, Clone)]
pub struct AdaptiveCpu {
    active: bool,
    cap: u8,
    high: u8,
    low: u8,
    cooldown: u8,
}

impl Default for AdaptiveCpu {
    fn default() -> Self {
        Self {
            active: false,
            cap: 55,
            high: 0,
            low: 0,
            cooldown: 0,
        }
    }
}

impl AdaptiveCpu {
    pub fn active(&self) -> bool {
        self.active
    }

    pub fn enable(&mut self, initial: u8) {
        self.cap = initial.clamp(35, 95);
        self.active = true;
        self.reset_counters();
    }

    pub fn disable(&mut self) {
        self.active = false;
        self.reset_counters();
    }

    fn reset_counters(&mut self) {
        self.high = 0;
        self.low = 0;
        self.cooldown = 0;
    }

    /// Called every 3 seconds. Raise the cap after 9s high CPU utilization;
    /// lower only after 30s low utilization, with a 12s cooldown.
    /// Bounded between 35% and 95% so manual Hard (25%) stays a separate mode.
    pub fn observe(&mut self, cpu_usage_pct: f32) -> Option<u8> {
        if !self.active || !cpu_usage_pct.is_finite()
            || !(0.0..=100.0).contains(&cpu_usage_pct)
        {
            return None;
        }
        if self.cooldown > 0 {
            self.cooldown -= 1;
            self.high = 0;
            self.low = 0;
            return None;
        }
        if cpu_usage_pct >= 85.0 {
            self.high = self.high.saturating_add(1);
            self.low = 0;
            if self.high >= 3 && self.cap < 95 {
                self.cap = self.cap.saturating_add(10).min(95);
                self.high = 0;
                self.cooldown = 4;
                return Some(self.cap);
            }
        } else if cpu_usage_pct <= 20.0 {
            self.low = self.low.saturating_add(1);
            self.high = 0;
            if self.low >= 10 && self.cap > 35 {
                self.cap = self.cap.saturating_sub(5).max(35);
                self.low = 0;
                self.cooldown = 4;
                return Some(self.cap);
            }
        } else {
            self.high = 0;
            self.low = 0;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_disabled_by_default() {
        let mut adaptive = AdaptiveCpu::default();
        for _ in 0..20 {
            assert_eq!(adaptive.observe(99.0), None);
        }
    }

    #[test]
    fn increases_only_after_sustained_load() {
        let mut adaptive = AdaptiveCpu::default();
        adaptive.enable(55);
        assert_eq!(adaptive.observe(90.0), None);
        assert_eq!(adaptive.observe(90.0), None);
        assert_eq!(adaptive.observe(90.0), Some(65));
    }

    #[test]
    fn decreases_only_after_sustained_idle_and_cooldown() {
        let mut adaptive = AdaptiveCpu::default();
        adaptive.enable(65);
        for _ in 0..9 {
            assert_eq!(adaptive.observe(10.0), None);
        }
        assert_eq!(adaptive.observe(10.0), Some(60));
        for _ in 0..4 {
            assert_eq!(adaptive.observe(10.0), None);
        }
    }

    #[test]
    fn no_oscillation_on_mixed_load_or_invalid_readings() {
        let mut adaptive = AdaptiveCpu::default();
        adaptive.enable(50);
        for usage in [90.0, 10.0, 70.0, f32::NAN, 90.0, 10.0] {
            assert_eq!(adaptive.observe(usage), None);
        }
        adaptive.disable();
        assert!(!adaptive.active());
    }

    #[test]
    fn cap_is_bounded() {
        let mut adaptive = AdaptiveCpu::default();
        adaptive.enable(250);
        for _ in 0..100 {
            if let Some(cap) = adaptive.observe(99.0) {
                assert!(cap <= 95);
            }
        }
        adaptive.enable(1);
        for _ in 0..200 {
            if let Some(cap) = adaptive.observe(0.0) {
                assert!(cap >= 35);
            }
        }
    }
}
