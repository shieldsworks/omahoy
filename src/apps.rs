//! Every app setup knows, and every fact about each. This is the only app
//! list: adding one is a variant, and the compiler walks through the rest.

use crate::boat::Boat;
use crate::depth::depths;
use crate::place::Coverage;
use crate::sys::Dirs;
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum App {
    Keel,
    Helm,
    Lookout,
    Logbook,
    Tide,
    Wind,
}

impl App {
    pub const ALL: [App; 6] = [
        App::Keel,
        App::Helm,
        App::Lookout,
        App::Logbook,
        App::Tide,
        App::Wind,
    ];

    /// The apps a sailor picks. omakeel is the hub and always comes.
    pub const CHOSEN: [App; 5] = [App::Helm, App::Lookout, App::Logbook, App::Tide, App::Wind];

    /// The repo name, which is also the binary and the config directory.
    pub fn name(self) -> &'static str {
        match self {
            App::Keel => "omakeel",
            App::Helm => "omahelm",
            App::Lookout => "omalookout",
            App::Logbook => "omalogbook",
            App::Tide => "omatide",
            App::Wind => "omawind",
        }
    }

    /// The word in boat.toml's `apps` line.
    pub fn short(self) -> &'static str {
        match self {
            App::Keel => "keel",
            App::Helm => "helm",
            App::Lookout => "lookout",
            App::Logbook => "logbook",
            App::Tide => "tide",
            App::Wind => "wind",
        }
    }

    pub fn from_short(s: &str) -> Option<App> {
        App::CHOSEN.into_iter().find(|a| a.short() == s)
    }

    pub fn what(self) -> &'static str {
        match self {
            App::Keel => "the hub: GPS and AIS for every app",
            App::Helm => "chartplotter",
            App::Lookout => "AIS targets and collision alarms",
            App::Logbook => "ship's log",
            App::Tide => "tides and currents",
            App::Wind => "wind forecasts",
        }
    }

    pub fn repo(self) -> String {
        format!("https://github.com/shieldsworks/{}.git", self.name())
    }

    pub fn plugin_id(self) -> Option<&'static str> {
        match self {
            App::Keel => None,
            App::Helm => Some("org.omahoy.helm"),
            App::Lookout => Some("org.omahoy.lookout"),
            App::Logbook => Some("org.omahoy.logbook"),
            App::Tide => Some("org.omahoy.tide"),
            App::Wind => Some("org.omahoy.wind"),
        }
    }

    /// The Rust binary, built to `target/release/<binary>`. omalookout is QML only.
    pub fn binary(self) -> Option<&'static str> {
        match self {
            App::Lookout => None,
            other => Some(other.name()),
        }
    }

    /// The systemd user unit, shipped in the app's repo as `dist/<unit>`.
    pub fn unit(self) -> Option<&'static str> {
        match self {
            App::Keel => Some("omakeel.service"),
            App::Logbook => Some("omalogbook.service"),
            _ => None,
        }
    }

    pub fn config(self, dirs: &Dirs) -> Option<PathBuf> {
        match self {
            App::Lookout => None,
            other => Some(dirs.config.join(other.name()).join("config.toml")),
        }
    }

    /// Why this app can't work where the boat sails, if it can't.
    pub fn refusal(self, c: Coverage) -> Option<&'static str> {
        match self {
            App::Tide if !c.noaa => Some("NOAA tide stations cover US waters only"),
            App::Wind if !c.hrrr => Some("HRRR forecasts cover the lower 48 and its coasts only"),
            _ => None,
        }
    }

    /// What works differently where the boat sails, for an app that is
    /// still offered.
    pub fn caveat(self, c: Coverage) -> Option<&'static str> {
        match self {
            App::Helm if !c.noaa => {
                Some("no NOAA charts here; import your own ENC: omahelm import ZIP|DIR")
            }
            _ => None,
        }
    }
}

/// The keys omahoy owns in an app's config, as `(key, value)` with the
/// value written the way the line will read. `source` repeats. Every
/// other key in those files is the sailor's.
pub fn managed(app: App, boat: &Boat) -> Vec<(&'static str, String)> {
    let quoted = |s: &str| format!("\"{s}\"");
    match app {
        App::Keel => boat
            .hub
            .specs()
            .into_iter()
            .map(|s| ("source", s))
            .collect(),
        App::Helm => {
            let d = depths(boat.units, boat.draft, boat.clearance);
            vec![
                ("units", boat.units.name().into()),
                ("safety_depth", d.safety_depth.to_string()),
                ("shallow_contour", d.shallow_contour.to_string()),
                ("safety_contour", d.safety_contour.to_string()),
                ("deep_contour", d.deep_contour.to_string()),
                ("home", boat.home.to_config()),
            ]
        }
        App::Logbook => vec![
            ("boat", quoted(&boat.name)),
            ("vault", quoted(&boat.vault)),
            ("git", boat.git.to_string()),
        ],
        App::Tide | App::Wind => vec![
            ("region", boat.area.to_config()),
            ("home", boat.home.to_config()),
        ],
        App::Lookout => vec![],
    }
}
