//! Where the boat sails: positions, the sailing area, and what the free
//! US data reaches there. Pure; the only geography in omahoy.

/// Degrees, WGS84. Built only through `new`, so it is always on the globe.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Position {
    lat: f64,
    lon: f64,
}

impl Position {
    pub fn new(lat: f64, lon: f64) -> Result<Position, String> {
        if !(lat.is_finite() && (-90.0..=90.0).contains(&lat)) {
            return Err(format!("latitude {lat} is not between -90 and 90"));
        }
        if !(lon.is_finite() && (-180.0..=180.0).contains(&lon)) {
            return Err(format!("longitude {lon} is not between -180 and 180"));
        }
        Ok(Position { lat, lon })
    }

    /// `47.687, -122.413`, or degrees and minutes with hemispheres:
    /// `47°41.2'N 122°24.8'W`, `47 41.2 N, 122 24.8 W`.
    pub fn parse(text: &str) -> Result<Position, String> {
        let text = text.trim();
        if text.contains(['N', 'S', 'E', 'W', 'n', 's', 'e', 'w']) {
            return parse_hemispheres(text);
        }
        let Some((a, b)) = text.split_once(',') else {
            return Err("expected latitude, longitude, like 47.687, -122.413".into());
        };
        let num = |s: &str| {
            s.trim()
                .parse::<f64>()
                .map_err(|_| format!("{} is not a number of degrees", s.trim()))
        };
        Position::new(num(a)?, num(b)?)
    }

    /// The form every app config uses: `47.687, -122.413`.
    pub fn to_config(self) -> String {
        format!("{}, {}", deg(self.lat), deg(self.lon))
    }

    /// `47°41.2′N 122°24.8′W`, for the sailor to read.
    pub fn show(self) -> String {
        let dm = |v: f64, pos: char, neg: char| {
            let h = if v < 0.0 { neg } else { pos };
            let v = v.abs();
            let mut d = v.trunc();
            let mut m = ((v - d) * 60.0 * 10.0).round() / 10.0;
            if m >= 60.0 {
                d += 1.0;
                m -= 60.0;
            }
            format!("{d:.0}°{m:.1}′{h}")
        };
        format!("{} {}", dm(self.lat, 'N', 'S'), dm(self.lon, 'E', 'W'))
    }
}

fn parse_hemispheres(text: &str) -> Result<Position, String> {
    let bad = || "expected 47°41.2'N 122°24.8'W, or 47.687, -122.413".to_string();
    let mut parts: Vec<(f64, char)> = Vec::new();
    let mut nums: Vec<f64> = Vec::new();
    let mut word = String::new();
    let flush = |word: &mut String, nums: &mut Vec<f64>| -> Result<(), String> {
        if !word.is_empty() {
            nums.push(word.parse::<f64>().map_err(|_| bad())?);
            word.clear();
        }
        Ok(())
    };
    for c in text.chars() {
        match c {
            '0'..='9' | '.' => word.push(c),
            'N' | 'S' | 'E' | 'W' | 'n' | 's' | 'e' | 'w' => {
                flush(&mut word, &mut nums)?;
                let value = match nums.as_slice() {
                    [d] => *d,
                    [d, m] if (0.0..60.0).contains(m) => d + m / 60.0,
                    [d, m, s] if (0.0..60.0).contains(m) && (0.0..60.0).contains(s) => {
                        d + m / 60.0 + s / 3600.0
                    }
                    _ => return Err(bad()),
                };
                nums.clear();
                parts.push((value, c.to_ascii_uppercase()));
            }
            _ => flush(&mut word, &mut nums)?,
        }
    }
    flush(&mut word, &mut nums)?;
    if !nums.is_empty() {
        return Err(bad());
    }
    match parts.as_slice() {
        [(lat, a), (lon, b)] if matches!(a, 'N' | 'S') && matches!(b, 'E' | 'W') => {
            let lat = if *a == 'S' { -lat } else { *lat };
            let lon = if *b == 'W' { -lon } else { *lon };
            Position::new(lat, lon)
        }
        _ => Err(bad()),
    }
}

/// Degrees to at most four decimals (about 10 m), without trailing zeros.
pub fn deg(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" { "0".into() } else { s.into() }
}

/// South, west, north, east, with south below north and west below east.
/// It never crosses the antimeridian.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Area {
    south: f64,
    west: f64,
    north: f64,
    east: f64,
}

impl Area {
    pub fn new(south: f64, west: f64, north: f64, east: f64) -> Result<Area, String> {
        let sw = Position::new(south, west)?;
        let ne = Position::new(north, east)?;
        if sw.lat >= ne.lat {
            return Err("south must be below north".into());
        }
        if sw.lon >= ne.lon {
            return Err(
                "west must be below east (an area across the antimeridian is not supported)".into(),
            );
        }
        Ok(Area {
            south,
            west,
            north,
            east,
        })
    }

    /// `S,W,N,E` in degrees.
    pub fn parse(text: &str) -> Result<Area, String> {
        let n: Vec<f64> = text
            .split(',')
            .map(|p| p.trim().parse::<f64>().ok().filter(|v| v.is_finite()))
            .collect::<Option<_>>()
            .ok_or("expected south, west, north, east in degrees")?;
        let &[s, w, n, e] = n.as_slice() else {
            return Err("expected four numbers: south, west, north, east".into());
        };
        Area::new(s, w, n, e)
    }

    /// The default sailing area: 1° of latitude and 1.1° of longitude
    /// each side of home, about 60 nm. Near a pole or the antimeridian it
    /// is cut short rather than wrapped.
    pub fn around(home: Position) -> Area {
        let round = |v: f64| (v * 1000.0).round() / 1000.0;
        Area {
            south: round((home.lat - 1.0).max(-90.0)),
            north: round((home.lat + 1.0).min(90.0)),
            west: round((home.lon - 1.1).max(-180.0)),
            east: round((home.lon + 1.1).min(180.0)),
        }
    }

    pub fn contains(&self, p: Position) -> bool {
        (self.south..=self.north).contains(&p.lat) && (self.west..=self.east).contains(&p.lon)
    }

    fn intersects(&self, o: &Area) -> bool {
        self.south < o.north && o.south < self.north && self.west < o.east && o.west < self.east
    }

    fn inside(&self, o: &Area) -> bool {
        self.south >= o.south && self.north <= o.north && self.west >= o.west && self.east <= o.east
    }

    /// The form every app config uses: `S,W,N,E`.
    pub fn to_config(self) -> String {
        format!(
            "{},{},{},{}",
            deg(self.south),
            deg(self.west),
            deg(self.north),
            deg(self.east)
        )
    }
}

/// What the free US data reaches. Derived, never stored.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Coverage {
    /// NOAA ENC charts and CO-OPS tides.
    pub noaa: bool,
    /// HRRR wind forecasts.
    pub hrrr: bool,
}

const fn bx(south: f64, west: f64, north: f64, east: f64) -> Area {
    Area {
        south,
        west,
        north,
        east,
    }
}

/// US waters, coarsely: the coasts of the lower 48 and the Great Lakes,
/// Alaska and the Aleutians, Hawaii, Puerto Rico and the US Virgin
/// Islands, Guam and the Northern Marianas. It only decides what setup
/// offers. omahelm and omatide stay the authority on what they fetch.
pub const NOAA: [Area; 10] = [
    bx(32.5, -125.0, 49.0, -117.0),
    bx(25.0, -97.9, 31.0, -82.0),
    bx(24.4, -82.0, 31.0, -80.0),
    bx(31.0, -81.5, 45.3, -66.9),
    bx(41.3, -92.3, 49.1, -75.9),
    bx(51.0, -180.0, 71.5, -129.9),
    bx(51.0, 172.0, 55.0, 180.0),
    bx(18.5, -160.8, 22.5, -154.5),
    bx(17.6, -67.5, 18.6, -64.5),
    bx(13.2, 144.5, 20.6, 146.1),
];

/// HRRR's reach, mirrored from omawind's `Region::check`, along with its
/// limit of 8° of latitude by 10° of longitude.
pub const HRRR: Area = bx(21.2, -134.0, 52.6, -61.0);

pub fn coverage(area: Area) -> Coverage {
    Coverage {
        noaa: NOAA.iter().any(|b| area.intersects(b)),
        hrrr: area.inside(&HRRR) && area.north - area.south <= 8.0 && area.east - area.west <= 10.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(lat: f64, lon: f64) -> Position {
        Position::new(lat, lon).unwrap()
    }

    #[test]
    fn positions_parse_in_decimal_and_degrees_minutes() {
        assert_eq!(Position::parse("47.687, -122.413"), Ok(p(47.687, -122.413)));
        let dm = Position::parse("47°41.22'N 122°24.78'W").unwrap();
        assert_eq!(
            (deg(dm.lat), deg(dm.lon)),
            ("47.687".into(), "-122.413".into())
        );
        let spaced = Position::parse("37 52 N, 122 18.9 W").unwrap();
        assert_eq!(spaced.to_config(), "37.8667, -122.315");
        assert!(Position::parse("91, 0").is_err());
        assert!(Position::parse("47.6").is_err());
        assert!(Position::parse("47 N 61 N").is_err());
    }

    #[test]
    fn positions_show_in_degrees_and_minutes() {
        assert_eq!(p(47.687, -122.413).show(), "47°41.2′N 122°24.8′W");
        assert_eq!(p(-33.8599, 151.2).show(), "33°51.6′S 151°12.0′E");
    }

    #[test]
    fn the_default_area_is_a_degree_around_home() {
        let a = Area::around(p(47.687, -122.413));
        assert_eq!(a.to_config(), "46.687,-123.513,48.687,-121.313");
        assert!(a.contains(p(47.687, -122.413)));
    }

    #[test]
    fn the_area_never_wraps_the_antimeridian_or_a_pole() {
        let a = Area::around(p(89.5, 179.5));
        assert_eq!(a.to_config(), "88.5,178.4,90,180");
        assert!(Area::parse("10,170,20,-170").is_err());
        assert!(Area::parse("20,0,10,1").is_err());
        assert!(Area::parse("1,2,3").is_err());
    }

    #[test]
    fn coverage_where_people_sail() {
        let at = |lat, lon| coverage(Area::around(p(lat, lon)));
        let both = Coverage {
            noaa: true,
            hrrr: true,
        };
        assert_eq!(at(37.8663, -122.3148), both, "Berkeley");
        assert_eq!(at(47.687, -122.413), both, "Seattle");
        assert_eq!(at(25.77, -80.19), both, "Miami");
        assert_eq!(at(42.36, -71.05), both, "Boston");
        assert_eq!(at(41.88, -87.6), both, "Chicago");
        let noaa_only = Coverage {
            noaa: true,
            hrrr: false,
        };
        assert_eq!(at(21.3, -157.86), noaa_only, "Honolulu");
        assert_eq!(at(61.2, -149.9), noaa_only, "Anchorage");
        assert_eq!(at(18.45, -66.1), noaa_only, "San Juan");
        let none = Coverage {
            noaa: false,
            hrrr: false,
        };
        assert_eq!(at(39.47, 2.73), none, "Mallorca");
        assert_eq!(at(-33.86, 151.2), none, "Sydney");
        assert_eq!(at(50.8, -1.3), none, "Solent");
        assert!(!at(25.0, -77.3).noaa, "Nassau");
    }

    #[test]
    fn hrrr_bounds_match_omawind() {
        assert_eq!(HRRR.to_config(), "21.2,-134,52.6,-61");
        let big = Area::parse("30,-125,39,-115").unwrap();
        assert!(!coverage(big).hrrr, "taller than 8°");
    }
}
