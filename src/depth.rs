//! Depth units and the chart depths derived from draft and clearance.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Units {
    Feet,
    Meters,
    Fathoms,
}

impl Units {
    pub fn parse(s: &str) -> Option<Units> {
        match s.trim().to_ascii_lowercase().as_str() {
            "feet" | "ft" | "f" => Some(Units::Feet),
            "meters" | "metres" | "m" => Some(Units::Meters),
            "fathoms" | "fm" => Some(Units::Fathoms),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Units::Feet => "feet",
            Units::Meters => "meters",
            Units::Fathoms => "fathoms",
        }
    }

    fn meters(self) -> f64 {
        match self {
            Units::Feet => 0.3048,
            Units::Meters => 1.0,
            Units::Fathoms => 1.8288,
        }
    }

    /// `v` of these units in `to`, rounded the way depths are saved.
    pub fn convert(self, v: f64, to: Units) -> f64 {
        round2(v * self.meters() / to.meters())
    }

    /// 30 feet in these units, rounded up to a whole unit.
    fn thirty_feet(self) -> u32 {
        match self {
            Units::Feet => 30,
            Units::Meters => 10,
            Units::Fathoms => 5,
        }
    }

    /// The clearance offered when the sailor has none in mind.
    pub fn default_clearance(self) -> f64 {
        match self {
            Units::Feet => 2.0,
            Units::Meters => 0.6,
            Units::Fathoms => 0.5,
        }
    }
}

/// omahelm's four depths, in whole units.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Depths {
    pub safety_depth: u32,
    pub shallow_contour: u32,
    pub safety_contour: u32,
    pub deep_contour: u32,
}

/// Safety depth is draft plus clearance, rounded up to a whole unit. The
/// safety contour is the same. Shallow is half of it, rounded down; deep is
/// the larger of 30 ft and twice the safety depth. That keeps omahelm's
/// shallow ≤ safety ≤ deep, without which it reverts all three.
pub fn depths(units: Units, draft: f64, clearance: f64) -> Depths {
    let wanted = draft + clearance;
    let safety = (wanted - 1e-9).ceil().max(1.0) as u32;
    Depths {
        safety_depth: safety,
        shallow_contour: safety / 2,
        safety_contour: safety,
        deep_contour: units.thirty_feet().max(safety * 2),
    }
}

/// A depth to the two decimals boat.toml keeps. Every depth is rounded
/// this way when it comes in, so the first apply and every later one
/// work from the same number.
pub fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

/// A depth as the sailor typed it: `5.5`, never more than two decimals.
pub fn show(v: f64) -> String {
    crate::place::deg(round2(v))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(safety: u32, shallow: u32, deep: u32) -> Depths {
        Depths {
            safety_depth: safety,
            shallow_contour: shallow,
            safety_contour: safety,
            deep_contour: deep,
        }
    }

    #[test]
    fn depths_from_draft_and_clearance() {
        assert_eq!(depths(Units::Feet, 5.5, 2.0), d(8, 4, 30));
        assert_eq!(depths(Units::Feet, 4.0, 2.0), d(6, 3, 30));
        assert_eq!(depths(Units::Feet, 12.0, 6.5), d(19, 9, 38));
        assert_eq!(depths(Units::Meters, 1.7, 0.6), d(3, 1, 10));
        assert_eq!(
            depths(Units::Meters, 1.1, 0.9),
            d(2, 1, 10),
            "1.1 + 0.9 is 2, not 3"
        );
        assert_eq!(depths(Units::Fathoms, 1.0, 0.5), d(2, 1, 5));
        assert_eq!(depths(Units::Meters, 0.3, 0.2), d(1, 0, 10));
    }

    #[test]
    fn contours_always_run_shallow_safety_deep() {
        for units in [Units::Feet, Units::Meters, Units::Fathoms] {
            for tenths in 1..400 {
                let x = depths(units, f64::from(tenths) / 10.0, 0.5);
                assert!(x.shallow_contour <= x.safety_contour, "{x:?}");
                assert!(x.safety_contour <= x.deep_contour, "{x:?}");
            }
        }
    }

    #[test]
    fn depths_convert_between_units_to_two_decimals() {
        assert_eq!(Units::Meters.convert(1.7, Units::Feet), 5.58);
        assert_eq!(Units::Feet.convert(5.5, Units::Meters), 1.68);
        assert_eq!(Units::Fathoms.convert(1.0, Units::Feet), 6.0);
        assert_eq!(Units::Feet.convert(2.0, Units::Feet), 2.0);
    }

    #[test]
    fn units_parse_their_names_and_short_forms() {
        assert_eq!(Units::parse("Feet"), Some(Units::Feet));
        assert_eq!(Units::parse("m"), Some(Units::Meters));
        assert_eq!(Units::parse("metres"), Some(Units::Meters));
        assert_eq!(Units::parse("fm"), Some(Units::Fathoms));
        assert_eq!(Units::parse("cubits"), None);
        assert_eq!(show(5.5), "5.5");
        assert_eq!(show(2.0), "2");
    }
}
