//! `~/.config/omahoy/boat.toml`: the one file the sailor owns. Parsed
//! strictly, because a wrong value here is copied into five app configs.

use crate::apps::App;
use crate::conf;
use crate::depth::{self, Units};
use crate::hub::{Hub, Source};
use crate::place::{Area, Position, coverage};
use crate::sys::Dirs;
use std::collections::BTreeSet;
use std::fmt;

/// The apps the sailor chose, without omakeel, which always comes. Built
/// only through `choose`, so it never holds an app that can't work in
/// the boat's area.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Apps(BTreeSet<App>);

impl Apps {
    pub fn choose(wanted: &[App], area: Area) -> Result<Apps, Vec<String>> {
        let c = coverage(area);
        let refused: Vec<String> = wanted
            .iter()
            .filter_map(|a| {
                a.refusal(c)
                    .map(|why| format!("{} can't work here: {why}", a.name()))
            })
            .collect();
        if !refused.is_empty() {
            return Err(refused);
        }
        Ok(Apps(
            wanted.iter().copied().filter(|a| *a != App::Keel).collect(),
        ))
    }

    pub fn has(&self, app: App) -> bool {
        app == App::Keel || self.0.contains(&app)
    }

    /// omakeel first, then the chosen apps in their usual order.
    pub fn with_keel(&self) -> Vec<App> {
        App::ALL.into_iter().filter(|a| self.has(*a)).collect()
    }

    pub fn chosen(&self) -> Vec<App> {
        self.0.iter().copied().collect()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Boat {
    pub name: String,
    pub home: Position,
    /// Contains `home`.
    pub area: Area,
    pub units: Units,
    /// In `units`, above zero.
    pub draft: f64,
    /// In `units`, zero or more.
    pub clearance: f64,
    pub hub: Hub,
    pub apps: Apps,
    /// As written: `~/Logbook` or an absolute path.
    pub vault: String,
    pub git: bool,
}

/// One thing wrong with boat.toml. Line 0 is the file as a whole.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Problem {
    pub line: usize,
    pub message: String,
}

impl fmt::Display for Problem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}", self.message)
        } else {
            write!(f, "line {}: {}", self.line, self.message)
        }
    }
}

/// Answers so far. Setup fills the gaps; only `finish` makes a Boat, so a
/// half-answered boat never reaches the plan.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct Partial {
    pub name: Option<String>,
    pub home: Option<Position>,
    pub area: Option<Area>,
    pub units: Option<Units>,
    pub draft: Option<f64>,
    pub clearance: Option<f64>,
    pub hub: Option<Hub>,
    pub apps: Option<Vec<App>>,
    pub vault: Option<String>,
    pub git: Option<bool>,
}

fn keep<T>(problems: &mut Vec<String>, r: Result<T, String>) -> Option<T> {
    r.map_err(|e| problems.push(e)).ok()
}

fn check_name(v: &str) -> Result<String, String> {
    let v = v.trim();
    if v.is_empty() {
        return Err("the boat needs a name".into());
    }
    if v.contains(['"', '#']) || v.chars().any(char::is_control) {
        return Err("a boat name can't hold \" or # (the app configs read them as syntax)".into());
    }
    Ok(v.to_string())
}

fn check_vault(v: &str) -> Result<String, String> {
    let v = v.trim();
    if !(v.starts_with('/') || v.starts_with("~/")) {
        return Err(format!("{v}: the logbook folder starts with / or ~/"));
    }
    if v.contains(['"', '#']) {
        return Err("a folder name can't hold \" or #".into());
    }
    Ok(v.trim_end_matches('/').to_string())
}

fn check_depth(v: f64, what: &str, zero_ok: bool) -> Result<f64, String> {
    let low_ok = if zero_ok { v >= 0.0 } else { v > 0.0 };
    if v.is_finite() && low_ok && v < 100.0 {
        Ok(v)
    } else {
        Err(format!("{what} {v} is not a depth"))
    }
}

/// A depth as typed or read from boat.toml, rounded to the two decimals
/// the file keeps. A nonzero value that rounds to zero is refused.
pub fn parse_depth(s: &str, what: &str, zero_ok: bool) -> Result<f64, String> {
    let v = s
        .trim()
        .parse::<f64>()
        .map_err(|_| format!("{what}: {} is not a number", s.trim()))?;
    let r = depth::round2(v);
    if v != 0.0 && r == 0.0 {
        return Err(format!(
            "{what} {} rounds to zero; depths keep two decimals",
            s.trim()
        ));
    }
    check_depth(r, what, zero_ok)
}

impl Partial {
    pub fn from_boat(b: &Boat) -> Partial {
        Partial {
            name: Some(b.name.clone()),
            home: Some(b.home),
            area: Some(b.area),
            units: Some(b.units),
            draft: Some(b.draft),
            clearance: Some(b.clearance),
            hub: Some(b.hub.clone()),
            apps: Some(b.apps.chosen()),
            vault: Some(b.vault.clone()),
            git: Some(b.git),
        }
    }

    /// Every rule boat.toml has, applied to whole answers.
    pub fn finish(self) -> Result<Boat, Vec<String>> {
        let mut missing = Vec::new();
        macro_rules! need {
            ($f:ident) => {
                match self.$f {
                    Some(v) => Some(v),
                    None => {
                        missing.push(format!("missing {}", stringify!($f)));
                        None
                    }
                }
            };
        }
        let (name, home, area, units, draft, clearance, hub, apps, vault, git) = (
            need!(name),
            need!(home),
            need!(area),
            need!(units),
            need!(draft),
            need!(clearance),
            need!(hub),
            need!(apps),
            need!(vault),
            need!(git),
        );
        let (
            Some(name),
            Some(home),
            Some(area),
            Some(units),
            Some(draft),
            Some(clearance),
            Some(hub),
            Some(apps),
            Some(vault),
            Some(git),
        ) = (
            name, home, area, units, draft, clearance, hub, apps, vault, git,
        )
        else {
            return Err(missing);
        };
        let mut problems = Vec::new();
        let name = keep(&mut problems, check_name(&name));
        let vault = keep(&mut problems, check_vault(&vault));
        let draft = keep(&mut problems, check_depth(draft, "draft", false));
        let clearance = keep(&mut problems, check_depth(clearance, "clearance", true));
        if !area.contains(home) {
            problems.push(format!(
                "area {} doesn't contain home {}",
                area.to_config(),
                home.to_config()
            ));
        }
        let apps = Apps::choose(&apps, area)
            .map_err(|e| problems.extend(e))
            .ok();
        match (name, vault, draft, clearance, apps) {
            (Some(name), Some(vault), Some(draft), Some(clearance), Some(apps))
                if problems.is_empty() =>
            {
                Ok(Boat {
                    name,
                    home,
                    area,
                    units,
                    draft,
                    clearance,
                    hub,
                    apps,
                    vault,
                    git,
                })
            }
            _ => Err(problems),
        }
    }

    /// Defaults from the app configs already on this machine, so a sailor
    /// who set the apps up by hand keeps what they chose.
    pub fn from_existing(dirs: &Dirs) -> Partial {
        let read = |app: App| {
            app.config(dirs)
                .and_then(|p| std::fs::read_to_string(p).ok())
                .unwrap_or_default()
        };
        let first = |text: &str, key: &str| conf::values(text, key).into_iter().next();
        let mut p = Partial::default();

        let logbook = read(App::Logbook);
        let has_logbook_config = App::Logbook
            .config(dirs)
            .is_some_and(|c| std::fs::metadata(c).is_ok());
        p.name = match first(&logbook, "boat") {
            Some(v) => check_name(&v).ok(),
            // omalogbook's runtime default for a config with no `boat`
            // key was "Dash" until it changed to "Boat", so existing log
            // entries carry that name.
            None if has_logbook_config => Some("Dash".into()),
            None => None,
        };
        p.vault = first(&logbook, "vault").and_then(|v| check_vault(&v).ok());
        p.git = first(&logbook, "git").and_then(|v| v.parse().ok());

        p.units = first(&read(App::Helm), "units").and_then(|v| Units::parse(&v));

        for app in [App::Tide, App::Wind] {
            let text = read(app);
            if p.home.is_none() {
                p.home = first(&text, "home").and_then(|v| Position::parse(&v).ok());
            }
            if p.area.is_none() {
                p.area = first(&text, "region").and_then(|v| Area::parse(&v).ok());
            }
        }

        let specs = conf::values(&read(App::Keel), "source");
        p.hub = if specs.iter().any(|s| s == "demo") {
            Some(Hub::Demo)
        } else {
            specs
                .iter()
                .map(|s| Source::parse(s))
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .filter(|s| !s.is_empty())
                .map(Hub::from_sources)
        };

        let installed: Vec<App> = App::CHOSEN
            .into_iter()
            .filter(|a| {
                a.plugin_id()
                    .is_some_and(|id| std::fs::symlink_metadata(dirs.plugins.join(id)).is_ok())
            })
            .collect();
        if !installed.is_empty() {
            p.apps = Some(installed);
        }
        p
    }
}

fn first_line(seen: &[(&str, usize)], key: &str) -> Option<usize> {
    seen.iter().find(|(k, _)| *k == key).map(|(_, n)| *n)
}

const KEYS: [&str; 10] = [
    "name",
    "home",
    "area",
    "units",
    "draft",
    "clearance",
    "source",
    "apps",
    "vault",
    "git",
];

impl Boat {
    /// Strict: every problem, each with its line number. No defaults.
    pub fn parse(text: &str) -> Result<Boat, Vec<Problem>> {
        let mut problems = Vec::new();
        let mut seen: Vec<(&str, usize)> = Vec::new();
        let mut p = Partial::default();
        let mut sources: Vec<(usize, String)> = Vec::new();
        let mut lines: Vec<(usize, &str, &str)> = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let n = i + 1;
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                problems.push(Problem {
                    line: n,
                    message: "expected key = value".into(),
                });
                continue;
            };
            let (k, v) = (k.trim(), v.trim());
            if v.starts_with('"') != v.ends_with('"') || v == "\"" {
                problems.push(Problem {
                    line: n,
                    message: format!("{k}: unclosed quote (a # starts a comment)"),
                });
                continue;
            }
            let Some(key) = KEYS.iter().find(|x| **x == k) else {
                problems.push(Problem {
                    line: n,
                    message: format!("unknown key {k}"),
                });
                continue;
            };
            if *key != "source" {
                if let Some(first) = first_line(&seen, key) {
                    problems.push(Problem {
                        line: n,
                        message: format!("{key} is set again (first at line {first})"),
                    });
                    continue;
                }
                seen.push((key, n));
            }
            lines.push((n, key, v));
        }
        let unquote = |v: &str| v.trim_matches('"').to_string();
        for (n, key, v) in lines {
            let mut bad = |m: String| {
                problems.push(Problem {
                    line: n,
                    message: m,
                })
            };
            match key {
                "name" => match check_name(&unquote(v)) {
                    Ok(x) => p.name = Some(x),
                    Err(e) => bad(e),
                },
                "home" => match Position::parse(v) {
                    Ok(x) => p.home = Some(x),
                    Err(e) => bad(format!("home: {e}")),
                },
                "area" => match Area::parse(v) {
                    Ok(x) => p.area = Some(x),
                    Err(e) => bad(format!("area: {e}")),
                },
                "units" => match Units::parse(v) {
                    Some(x) => p.units = Some(x),
                    None => bad(format!("units: {v} is not feet, meters or fathoms")),
                },
                "draft" => match parse_depth(v, "draft", false) {
                    Ok(x) => p.draft = Some(x),
                    Err(e) => bad(e),
                },
                "clearance" => match parse_depth(v, "clearance", true) {
                    Ok(x) => p.clearance = Some(x),
                    Err(e) => bad(e),
                },
                "source" => sources.push((n, v.to_string())),
                "apps" => {
                    let mut apps = Vec::new();
                    for w in v.split(',').map(str::trim).filter(|w| !w.is_empty()) {
                        match App::from_short(w) {
                            Some(a) => apps.push(a),
                            None if w == "keel" => {
                                bad("keel always comes; leave it out of apps".into())
                            }
                            None => bad(format!(
                                "apps: {w} is not one of helm, lookout, logbook, tide, wind"
                            )),
                        }
                    }
                    p.apps = Some(apps);
                }
                "vault" => match check_vault(&unquote(v)) {
                    Ok(x) => p.vault = Some(x),
                    Err(e) => bad(e),
                },
                "git" => match v {
                    "true" | "false" => p.git = Some(v == "true"),
                    _ => bad(format!("git: {v} is not true or false")),
                },
                _ => {}
            }
        }
        let demo: Vec<usize> = sources
            .iter()
            .filter(|(_, s)| s == "demo")
            .map(|(n, _)| *n)
            .collect();
        if let Some(n) = demo.first() {
            if sources.len() > 1 {
                problems.push(Problem {
                    line: *n,
                    message: "source = demo must be the only source".into(),
                });
            } else {
                p.hub = Some(Hub::Demo);
            }
        } else {
            let mut good = Vec::new();
            for (n, s) in &sources {
                match Source::parse(s) {
                    Ok(x) => good.push(x),
                    Err(e) => problems.push(Problem {
                        line: *n,
                        message: e,
                    }),
                }
            }
            if !good.is_empty() && good.len() == sources.len() {
                p.hub = Some(Hub::from_sources(good));
            }
        }
        if sources.is_empty() {
            problems.push(Problem {
                line: 0,
                message: "missing source (source = demo when there is no hardware)".into(),
            });
        }
        if !problems.is_empty() {
            return Err(problems);
        }
        let line_of = |key: &str| first_line(&seen, key).unwrap_or(0);
        p.finish().map_err(|errs| {
            errs.into_iter()
                .map(|message| {
                    let line = if message.starts_with("area ") {
                        line_of("area")
                    } else if message.contains("can't work here") {
                        line_of("apps")
                    } else {
                        0
                    };
                    Problem { line, message }
                })
                .collect()
        })
    }

    pub fn render(&self) -> String {
        let mut out = String::from(
            "# Your boat. `omahoy apply` makes every app match this file.\n\
             # area is south,west,north,east; draft and clearance are in units.\n",
        );
        let mut line = |k: &str, v: &str| out.push_str(&format!("{k:<9} = {v}\n"));
        line("name", &format!("\"{}\"", self.name));
        line("home", &self.home.to_config());
        line("area", &self.area.to_config());
        line("units", self.units.name());
        line("draft", &depth::show(self.draft));
        line("clearance", &depth::show(self.clearance));
        for s in self.hub.specs() {
            line("source", &s);
        }
        let apps: Vec<&str> = self.apps.chosen().iter().map(|a| a.short()).collect();
        line("apps", &apps.join(", "));
        line("vault", &self.vault);
        line("git", if self.git { "true" } else { "false" });
        out
    }

    /// Missing file: Ok(None).
    pub fn load(dirs: &Dirs) -> Result<Option<Boat>, String> {
        let path = dirs.boat();
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("{}: {e}", path.display())),
        };
        Boat::parse(&text).map(Some).map_err(|problems| {
            let mut out = format!("{} has problems:", dirs.show(&path));
            for p in problems {
                out += &format!("\n  {p}");
            }
            out
        })
    }

    pub fn save(&self, dirs: &Dirs) -> Result<(), String> {
        crate::sys::write_atomic(&dirs.boat(), &self.render())
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub const WAVELENGTH: &str = "\
# Your boat. `omahoy apply` makes every app match this file.
# area is south,west,north,east; draft and clearance are in units.
name      = \"Wavelength\"
home      = 47.687, -122.413
area      = 46.687,-123.513,48.687,-121.313
units     = feet
draft     = 5.5
clearance = 2
source    = serial:/dev/serial/by-id/usb-Prolific_Technology_Inc._USB-Serial_Controller-if00-port0:4800
source    = serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver_0001-if00:38400
apps      = helm, lookout, logbook, tide, wind
vault     = ~/Logbook
git       = true
";

    pub fn wavelength() -> Boat {
        Boat::parse(WAVELENGTH).unwrap()
    }

    fn problems(text: &str) -> Vec<String> {
        Boat::parse(text)
            .unwrap_err()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn render_and_parse_round_trip_byte_for_byte() {
        let b = wavelength();
        assert_eq!(b.render(), WAVELENGTH);
        assert_eq!(Boat::parse(&b.render()), Ok(b.clone()));
        assert_eq!(b.name, "Wavelength");
        assert_eq!(b.hub.sources().len(), 2);
        let mut demo = b;
        demo.hub = Hub::Demo;
        assert_eq!(Boat::parse(&demo.render()), Ok(demo));
    }

    #[test]
    fn every_bad_line_is_named_by_number() {
        let text = WAVELENGTH
            .replace("units     = feet", "units     = cubits")
            .replace("draft     = 5.5", "draft     = deep")
            .replace(
                "git       = true",
                "git       = yes\nname = \"Again\"\ncolour = blue\njunk",
            );
        assert_eq!(
            problems(&text),
            vec![
                "line 14: name is set again (first at line 3)",
                "line 15: unknown key colour",
                "line 16: expected key = value",
                "line 6: units: cubits is not feet, meters or fathoms",
                "line 7: draft: deep is not a number",
                "line 13: git: yes is not true or false",
            ]
        );
    }

    #[test]
    fn demo_stands_alone() {
        let text = WAVELENGTH.replace(
            "source    = serial:/dev/serial/by-id/usb-Prolific",
            "source = demo\n# serial:/dev/serial/by-id/usb-Prolific",
        );
        assert_eq!(
            problems(&text),
            vec!["line 9: source = demo must be the only source"]
        );
        let none: String = WAVELENGTH
            .lines()
            .filter(|l| !l.starts_with("source"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(
            problems(&none),
            vec!["missing source (source = demo when there is no hardware)"]
        );
    }

    #[test]
    fn the_area_must_hold_home_and_suit_the_apps() {
        let away = WAVELENGTH.replace("home      = 47.687, -122.413", "home = 37.8, -122.3");
        assert_eq!(
            problems(&away),
            vec!["line 5: area 46.687,-123.513,48.687,-121.313 doesn't contain home 37.8, -122.3"]
        );
        let mallorca = WAVELENGTH
            .replace("home      = 47.687, -122.413", "home = 39.47, 2.73")
            .replace(
                "area      = 46.687,-123.513,48.687,-121.313",
                "area = 38.47,1.63,40.47,3.83",
            );
        assert_eq!(
            problems(&mallorca),
            vec![
                "line 11: omatide can't work here: NOAA tide stations cover US waters only",
                "line 11: omawind can't work here: HRRR forecasts cover the lower 48 and its coasts only",
            ]
        );
    }

    #[test]
    fn depths_are_rounded_once_where_they_come_in() {
        assert_eq!(parse_depth("5.001", "draft", false), Ok(5.0));
        assert_eq!(parse_depth("1.7", "draft", false), Ok(1.7));
        assert_eq!(parse_depth("0", "clearance", true), Ok(0.0));
        assert_eq!(
            parse_depth("0.004", "draft", false),
            Err("draft 0.004 rounds to zero; depths keep two decimals".into())
        );
        assert!(parse_depth("0.004", "clearance", true).is_err());
        assert!(parse_depth("0", "draft", false).is_err());
        let b = Boat::parse(&WAVELENGTH.replace("draft     = 5.5", "draft     = 5.001")).unwrap();
        assert_eq!(b.draft, 5.0, "the file is read as it would be saved");
        assert_eq!(Boat::parse(&b.render()), Ok(b));
    }

    #[test]
    fn names_and_folders_that_would_break_an_app_config_are_refused() {
        let text = WAVELENGTH
            .replace("\"Wavelength\"", "\"Wave#1\"")
            .replace("~/Logbook", "Logbook");
        assert_eq!(
            problems(&text),
            vec![
                "line 3: name: unclosed quote (a # starts a comment)",
                "line 12: Logbook: the logbook folder starts with / or ~/",
            ]
        );
    }
}
