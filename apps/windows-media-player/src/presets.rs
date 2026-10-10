//! Graphic EQ preset domain. Safe curves stay within +/- 12 dB and use
//! conservative headroom. All DSP coefficients are calculated on audio control.
use crate::dsp::BAND_HZ;

#[derive(Clone, Debug, PartialEq)]
pub struct EqPreset {
    pub name: String,
    pub bands: [f32; 31],
    pub preamp: f32,
}

impl EqPreset {
    pub fn flat() -> Self {
        Self { name: "Рівний".into(), bands: [0.; 31], preamp: -6. }
    }

    pub fn sanitize(mut self) -> Self {
        self.name = self.name.trim().chars().take(48).collect();
        if self.name.is_empty() { self.name = "Мій пресет".into(); }
        for value in &mut self.bands {
            *value = if value.is_finite() { value.clamp(-12., 12.) } else { 0. };
        }
        self.preamp = if self.preamp.is_finite() { self.preamp.clamp(-18., 6.) } else { -6. };
        self
    }

    pub fn named_curve(name: &str, f: impl Fn(f32) -> f32) -> Self {
        let bands = std::array::from_fn(|i| f(BAND_HZ[i]).clamp(-12., 12.));
        let maximum = bands.iter().copied().fold(0_f32, f32::max);
        Self { name: name.to_owned(), bands, preamp: (-maximum - 2.).clamp(-12., -3.) }
    }
}

fn bell(frequency: f32, center: f32, width: f32) -> f32 {
    let x = (frequency / center).log2() / width;
    (-x * x * 0.5).exp()
}

pub fn factory() -> Vec<EqPreset> {
    vec![
        EqPreset::flat(),
        EqPreset::named_curve("Бас", |hz| 6. * bell(hz, 75., 1.3)),
        EqPreset::named_curve("Рок", |hz| 4. * bell(hz, 90., 1.7) - 1.8 * bell(hz, 650., 1.8) + 3. * bell(hz, 6800., 1.5)),
        EqPreset::named_curve("Поп", |hz| 2.5 * bell(hz, 100., 1.6) + 2.2 * bell(hz, 2600., 1.6)),
        EqPreset::named_curve("Джаз", |hz| 2. * bell(hz, 180., 1.8) + 2.5 * bell(hz, 4200., 2.)),
        EqPreset::named_curve("Вокал", |hz| -1.5 * bell(hz, 90., 1.6) + 4. * bell(hz, 2100., 1.2)),
        EqPreset::named_curve("Електронна", |hz| 5. * bell(hz, 65., 1.3) - 1.7 * bell(hz, 550., 1.6) + 3.5 * bell(hz, 9000., 1.4)),
        EqPreset::named_curve("Класика", |hz| 1.5 * bell(hz, 140., 2.0) + 1.3 * bell(hz, 7500., 2.)),
    ]
}

pub fn from_json(value: &serde_json::Value) -> Option<EqPreset> {
    let name = value.get("name")?.as_str()?;
    let entries = value.get("bands")?.as_array()?;
    if entries.len() != 31 { return None; }
    let mut bands = [0.; 31];
    for (index, entry) in entries.iter().enumerate() {
        let v = entry.as_f64()? as f32;
        if !v.is_finite() { return None; }
        bands[index] = v;
    }
    let preamp = value.get("preamp")?.as_f64()? as f32;
    Some(EqPreset { name: name.into(), bands, preamp }.sanitize())
}

pub fn to_json(preset: &EqPreset) -> serde_json::Value {
    serde_json::json!({
        "name": preset.name,
        "bands": preset.bands.to_vec(),
        "preamp": preset.preamp,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_factory_presets_have_31_finite_bands_and_headroom() {
        for preset in factory() {
            assert_eq!(preset.bands.len(), 31);
            assert!(preset.bands.iter().all(|v| v.is_finite() && v.abs() <= 12.));
            assert!(preset.preamp <= -3.);
        }
    }
    #[test]
    fn preset_json_roundtrip_and_corruption_checks() {
        let preset = factory()[2].clone();
        assert_eq!(from_json(&to_json(&preset)), Some(preset));
        assert_eq!(from_json(&serde_json::json!({"name":"test","bands":[2,3]})), None);
    }
    #[test]
    fn sanitized_presets_never_produce_nan() {
        let custom = EqPreset { name: " ".into(), bands: [f32::NAN;31], preamp: f32::INFINITY };
        let clean = custom.sanitize();
        assert!(clean.bands.iter().all(|v| *v == 0.));
        assert_eq!(clean.preamp, -6.);
    }
}
