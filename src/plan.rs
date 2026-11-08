//! `plan`: the boat and the machine in, the steps that make them agree
//! out. Pure. `apply` runs the steps, `doctor` prints them, and an empty
//! plan means the machine matches boat.toml.

use crate::apps::{App, managed};
use crate::boat::Boat;
use crate::conf::{Diff, KeyState, compare};
use crate::machine::{Build, Kind, Link, Machine, OnDisk};
use crate::place::{Area, coverage};
use crate::sys::Dirs;
use std::path::PathBuf;

#[derive(Clone, PartialEq, Debug)]
pub enum Step {
    /// `omarchy pkg add`.
    Packages(Vec<String>),
    /// `sudo gpasswd -a USER uucp`.
    JoinUucp(String),
    /// A plugin through `omarchy plugin add`; omakeel by `git clone`.
    Get {
        app: App,
        dir: PathBuf,
    },
    /// `cargo build --release --locked` under the checkout's own pinned Rust.
    Build {
        app: App,
        dir: PathBuf,
    },
    /// `~/.local/bin/<name>` pointed at a build, or at this checkout's bootstrap.
    Link {
        link: PathBuf,
        target: PathBuf,
    },
    EnablePlugin(App),
    /// The managed keys that don't match, put right.
    Write {
        app: App,
        path: PathBuf,
        want: Vec<(&'static str, String)>,
        diffs: Vec<Diff>,
    },
    /// The checkout's `dist/<unit>` linked into ~/.config/systemd/user.
    InstallUnit {
        app: App,
        link: PathBuf,
        target: PathBuf,
    },
    /// `systemctl --user enable --now`.
    Start(App),
    /// Planned only when a running daemon's managed keys changed.
    Restart(App),
    /// `git init`, only when boat.toml says git and the vault isn't a repo.
    InitVault(PathBuf),
    Fetch {
        kind: Kind,
        area: Area,
        bin: PathBuf,
    },
}

impl Step {
    fn phase(&self) -> u8 {
        match self {
            Step::Packages(_) => 0,
            Step::JoinUucp(_) => 1,
            Step::Get { .. } => 2,
            Step::Build { .. } => 3,
            Step::Link { .. } => 4,
            Step::EnablePlugin(_) => 5,
            Step::Write { .. } => 6,
            Step::InstallUnit { .. } => 7,
            Step::Start(_) => 8,
            Step::Restart(_) => 9,
            Step::InitVault(_) => 10,
            Step::Fetch { .. } => 11,
        }
    }

    /// One line (two for a drifted config) for the plan and doctor.
    pub fn describe(&self, dirs: &Dirs) -> String {
        let show = |p: &PathBuf| dirs.show(p);
        match self {
            Step::Packages(p) => format!("install packages: {}", p.join(" ")),
            Step::JoinUucp(u) => format!("add {u} to the uucp group, for serial ports (sudo)"),
            Step::Get { app, dir } => match app.plugin_id() {
                Some(_) => format!("install the {} plugin into {}", app.name(), show(dir)),
                None => format!("clone {} into {}", app.name(), show(dir)),
            },
            Step::Build { app, dir } => format!("build {} in {}", app.name(), show(dir)),
            Step::Link { link, target } => format!("link {} -> {}", show(link), show(target)),
            Step::EnablePlugin(app) => {
                format!("enable plugin {}", app.plugin_id().unwrap_or(app.name()))
            }
            Step::Write { path, diffs, .. } => {
                let mut out = format!("write {}:", show(path));
                for d in diffs {
                    match &d.state {
                        KeyState::Matches => {}
                        KeyState::Missing => out += &format!("\n      set {} = {}", d.key, d.want),
                        KeyState::Differs { found } => {
                            out +=
                                &format!("\n      {} is {found}; boat.toml says {}", d.key, d.want)
                        }
                    }
                }
                out
            }
            Step::InstallUnit { link, .. } => format!("install {}", show(link)),
            Step::Start(app) => format!(
                "start {} now and at every login",
                app.unit().unwrap_or(app.name())
            ),
            Step::Restart(app) => format!(
                "restart {} for its new settings",
                app.unit().unwrap_or(app.name())
            ),
            Step::InitVault(p) => format!("start a git repo in {}", show(p)),
            Step::Fetch { kind, area, .. } => match kind {
                Kind::Charts => format!(
                    "fetch NOAA charts for {} (omahelm fetch --region)",
                    area.to_config()
                ),
                Kind::Tides => format!(
                    "fetch tide stations for {} (omatide fetch; a few minutes)",
                    area.to_config()
                ),
            },
        }
    }
}

pub fn plan(boat: &Boat, m: &Machine) -> Vec<Step> {
    let dirs = &m.dirs;
    let mut steps = Vec::new();
    let apps = boat.apps.with_keel();

    let mut need = vec!["git"];
    if [App::Helm, App::Tide, App::Wind]
        .iter()
        .any(|a| boat.apps.has(*a))
    {
        need.push("curl");
    }
    if boat.apps.has(App::Helm) {
        need.push("libarchive");
    }
    let missing: Vec<String> = m
        .missing_packages
        .iter()
        .filter(|p| need.contains(&p.as_str()))
        .cloned()
        .collect();
    if !missing.is_empty() {
        steps.push(Step::Packages(missing));
    }

    if boat.hub.sources().iter().any(|s| s.is_serial()) && !m.uucp_db {
        steps.push(Step::JoinUucp(dirs.user.clone()));
    }

    for app in apps {
        let (dir, build, fresh) = match m.apps.get(&app) {
            Some(OnDisk::Checkout(c)) => (c.dir.clone(), c.build, false),
            _ => {
                let dir = Machine::home_of(dirs, app);
                steps.push(Step::Get {
                    app,
                    dir: dir.clone(),
                });
                (dir, app.binary().map(|_| Build::NotBuilt), true)
            }
        };
        if matches!(build, Some(Build::NotBuilt | Build::Stale)) {
            steps.push(Step::Build {
                app,
                dir: dir.clone(),
            });
        }
        if let Some(bin) = app.binary()
            && matches!(m.bins.get(&app), Some(Link::Missing | Link::Repoint) | None)
        {
            steps.push(Step::Link {
                link: dirs.bin.join(bin),
                target: dir.join("target/release").join(bin),
            });
        }
        if let Some(id) = app.plugin_id() {
            let enabled = m.plugins.as_ref().and_then(|p| p.get(id).copied());
            if fresh || (m.plugins.is_some() && enabled != Some(true)) {
                steps.push(Step::EnablePlugin(app));
            }
        }
        let mut wrote = false;
        if let Some(path) = app.config(dirs) {
            let want = managed(app, boat);
            let text = m.configs.get(&app).map(String::as_str).unwrap_or("");
            let diffs = compare(text, &want);
            if diffs.iter().any(|d| d.state != KeyState::Matches) {
                wrote = true;
                steps.push(Step::Write {
                    app,
                    path,
                    want,
                    diffs,
                });
            }
        }
        if let Some(unit) = app.unit() {
            let state = m.units.get(&app);
            let file = state.map(|u| &u.file);
            if matches!(file, Some(Link::Missing | Link::Repoint) | None) {
                steps.push(Step::InstallUnit {
                    app,
                    link: dirs.units.join(unit),
                    target: dir.join("dist").join(unit),
                });
            }
            let (enabled, active) = state.map_or((false, false), |u| (u.enabled, u.active));
            if !(enabled && active) {
                steps.push(Step::Start(app));
            } else if wrote {
                steps.push(Step::Restart(app));
            }
        }
    }

    if let Some((script, Link::Missing | Link::Repoint)) = &m.omahoy {
        steps.push(Step::Link {
            link: dirs.bin.join("omahoy"),
            target: script.clone(),
        });
    }

    if boat.apps.has(App::Logbook) && boat.git && !m.vault.repo {
        steps.push(Step::InitVault(dirs.expand(&boat.vault)));
    }

    let c = coverage(boat.area);
    let area = boat.area.to_config();
    let wants = [
        (App::Helm, Kind::Charts, c.noaa, m.charts_on_disk),
        (App::Tide, Kind::Tides, true, m.tides_on_disk),
    ];
    for (app, kind, covered, on_disk) in wants {
        if boat.apps.has(app)
            && covered
            && (m.fetched.get(kind) != Some(area.as_str()) || !on_disk)
            && let Some(bin) = m.binary_path(app)
        {
            steps.push(Step::Fetch {
                kind,
                area: boat.area,
                bin,
            });
        }
    }

    steps.sort_by_key(Step::phase);
    steps
}

/// What `apply` can't put right: things for the sailor to do or know.
pub fn findings(boat: &Boat, m: &Machine) -> Vec<String> {
    let dirs = &m.dirs;
    let mut out = Vec::new();
    for app in boat.apps.with_keel() {
        if let Some(Link::Conflict(why)) = m.bins.get(&app) {
            out.push(format!(
                "{why}. Setup never replaces it; move it aside to let omahoy link {}.",
                app.name()
            ));
        }
        if let (Some(unit), Some(u)) = (app.unit(), m.units.get(&app)) {
            if let Link::Conflict(why) = &u.file {
                out.push(format!("{why}. Setup leaves your own {unit} alone."));
            }
            if let Some(c) = m.checkout(app)
                && !c.unit_file
            {
                out.push(format!(
                    "{} has no dist/{unit}. Update that checkout (git pull) so it can start at login.",
                    dirs.show(&c.dir)
                ));
            }
        }
    }
    if let Some((_, Link::Conflict(why))) = &m.omahoy {
        out.push(format!("{why}. Setup never replaces it."));
    }
    if m.plugins.is_none() {
        out.push(
            "The Omarchy shell didn't list its plugins, so their state is unknown. Run omahoy doctor once the shell is up."
                .into(),
        );
    }
    if boat.hub.sources().iter().any(|s| s.is_serial()) && m.uucp_db && !m.uucp_session {
        out.push(
            "You're in the uucp group, but this login isn't yet. Log out and back in once so the hub can open the serial ports."
                .into(),
        );
    }
    if boat.apps.has(App::Logbook) && boat.git && !(m.vault.name && m.vault.email) {
        out.push(
            "git has no user.name or user.email, so logbook commits have no author. Set them: git config --global user.name \"Your Name\"; git config --global user.email you@example.com"
                .into(),
        );
    }
    if boat.apps.has(App::Helm) && !coverage(boat.area).noaa && !m.charts_on_disk {
        out.push(
            "No charts yet, and NOAA doesn't cover this area. Import your own ENC: omahelm import ZIP|DIR"
                .into(),
        );
    }
    match &m.hub {
        Some(Ok(sources)) => {
            for s in sources.iter().filter(|s| s.status == "error") {
                out.push(format!(
                    "The hub can't read {}. Is it unplugged? The hub retries every 2 s; to drop it for good, run omahoy hardware.",
                    s.name
                ));
            }
        }
        Some(Err(why)) => out.push(format!("The hub isn't answering: {why}")),
        None => {}
    }
    out
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::boat::tests::wavelength;
    use crate::hub::Hub;
    use crate::machine::{Checkout, Fetched, Unit, Vault};
    use std::collections::BTreeMap;

    pub fn dirs() -> Dirs {
        Dirs::resolve(Some("/h".into()), None, None, None, Some("sailor".into())).unwrap()
    }

    /// A machine where everything already matches Wavelength's boat.toml.
    pub fn healthy(boat: &Boat) -> Machine {
        let dirs = dirs();
        let mut apps = BTreeMap::new();
        let mut bins = BTreeMap::new();
        let mut configs = BTreeMap::new();
        let mut units = BTreeMap::new();
        let mut plugins = BTreeMap::new();
        for app in App::ALL {
            let dir = Machine::home_of(&dirs, app);
            apps.insert(
                app,
                OnDisk::Checkout(Checkout {
                    dir,
                    dev: false,
                    build: app.binary().map(|_| Build::Fresh),
                    unit_file: app.unit().is_some(),
                }),
            );
            if app.binary().is_some() {
                bins.insert(app, Link::Ours);
            }
            if let Some(id) = app.plugin_id() {
                plugins.insert(id.to_string(), true);
            }
            if app.config(&dirs).is_some() {
                configs.insert(app, crate::conf::edit("", &managed(app, boat)));
            }
            if app.unit().is_some() {
                units.insert(
                    app,
                    Unit {
                        file: Link::Ours,
                        enabled: true,
                        active: true,
                    },
                );
            }
        }
        let area = boat.area;
        Machine {
            dirs,
            missing_packages: vec![],
            uucp_db: true,
            uucp_session: true,
            apps,
            bins,
            omahoy: None,
            plugins: Some(plugins),
            configs,
            units,
            fetched: Fetched::default()
                .with(Kind::Charts, area)
                .with(Kind::Tides, area),
            charts_on_disk: true,
            tides_on_disk: true,
            vault: Vault {
                repo: true,
                name: true,
                email: true,
            },
            hub: Some(Ok(vec![])),
        }
    }

    fn lines(boat: &Boat, m: &Machine) -> Vec<String> {
        plan(boat, m).iter().map(|s| s.describe(&m.dirs)).collect()
    }

    #[test]
    fn a_healthy_machine_has_an_empty_plan() {
        let b = wavelength();
        let m = healthy(&b);
        assert_eq!(plan(&b, &m), vec![]);
        assert_eq!(findings(&b, &m), Vec::<String>::new());
    }

    #[test]
    fn a_new_machine_gets_everything_in_order() {
        let b = wavelength();
        let mut m = healthy(&b);
        for app in App::ALL {
            m.apps.insert(app, OnDisk::Absent);
            if app.binary().is_some() {
                m.bins.insert(app, Link::Missing);
            }
            if let Some(u) = m.units.get_mut(&app) {
                *u = Unit {
                    file: Link::Missing,
                    enabled: false,
                    active: false,
                };
            }
        }
        m.configs.clear();
        m.plugins = Some(BTreeMap::new());
        m.missing_packages = vec!["libarchive".into()];
        m.uucp_db = false;
        m.uucp_session = false;
        m.fetched = Fetched::default();
        m.charts_on_disk = false;
        m.tides_on_disk = false;
        m.vault.repo = false;
        let got = lines(&b, &m);
        let want = [
            "install packages: libarchive",
            "add sailor to the uucp group, for serial ports (sudo)",
            "clone omakeel into ~/.local/share/omahoy/src/omakeel",
            "install the omahelm plugin into ~/.config/omarchy/plugins/org.omahoy.helm",
            "install the omalookout plugin into ~/.config/omarchy/plugins/org.omahoy.lookout",
            "install the omalogbook plugin into ~/.config/omarchy/plugins/org.omahoy.logbook",
            "install the omatide plugin into ~/.config/omarchy/plugins/org.omahoy.tide",
            "install the omawind plugin into ~/.config/omarchy/plugins/org.omahoy.wind",
            "build omakeel in ~/.local/share/omahoy/src/omakeel",
            "build omahelm in ~/.config/omarchy/plugins/org.omahoy.helm",
            "build omalogbook in ~/.config/omarchy/plugins/org.omahoy.logbook",
            "build omatide in ~/.config/omarchy/plugins/org.omahoy.tide",
            "build omawind in ~/.config/omarchy/plugins/org.omahoy.wind",
            "link ~/.local/bin/omakeel -> ~/.local/share/omahoy/src/omakeel/target/release/omakeel",
            "link ~/.local/bin/omahelm -> ~/.config/omarchy/plugins/org.omahoy.helm/target/release/omahelm",
            "link ~/.local/bin/omalogbook -> ~/.config/omarchy/plugins/org.omahoy.logbook/target/release/omalogbook",
            "link ~/.local/bin/omatide -> ~/.config/omarchy/plugins/org.omahoy.tide/target/release/omatide",
            "link ~/.local/bin/omawind -> ~/.config/omarchy/plugins/org.omahoy.wind/target/release/omawind",
            "enable plugin org.omahoy.helm",
            "enable plugin org.omahoy.lookout",
            "enable plugin org.omahoy.logbook",
            "enable plugin org.omahoy.tide",
            "enable plugin org.omahoy.wind",
        ];
        assert_eq!(got[..want.len()], want);
        let rest: Vec<&str> = got[want.len()..]
            .iter()
            .map(|l| l.lines().next().unwrap_or(""))
            .collect();
        assert_eq!(
            rest,
            vec![
                "write ~/.config/omakeel/config.toml:",
                "write ~/.config/omahelm/config.toml:",
                "write ~/.config/omalogbook/config.toml:",
                "write ~/.config/omatide/config.toml:",
                "write ~/.config/omawind/config.toml:",
                "install ~/.config/systemd/user/omakeel.service",
                "install ~/.config/systemd/user/omalogbook.service",
                "start omakeel.service now and at every login",
                "start omalogbook.service now and at every login",
                "start a git repo in ~/Logbook",
                "fetch NOAA charts for 46.687,-123.513,48.687,-121.313 (omahelm fetch --region)",
                "fetch tide stations for 46.687,-123.513,48.687,-121.313 (omatide fetch; a few minutes)",
            ]
        );
    }

    #[test]
    fn drift_is_shown_with_both_values_and_restarts_a_running_daemon() {
        let b = wavelength();
        let mut m = healthy(&b);
        m.configs.insert(
            App::Tide,
            "region = 36.8,-123.8,38.8,-121.6\nhome = 47.687, -122.413\n".into(),
        );
        m.configs
            .insert(App::Keel, "source = serial:/dev/ttyACM0:38400\n".into());
        assert_eq!(
            lines(&b, &m),
            vec![
                "write ~/.config/omakeel/config.toml:\n      source is serial:/dev/ttyACM0:38400; boat.toml says serial:/dev/serial/by-id/usb-Prolific_Technology_Inc._USB-Serial_Controller-if00-port0:4800, serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver_0001-if00:38400",
                "write ~/.config/omatide/config.toml:\n      region is 36.8,-123.8,38.8,-121.6; boat.toml says 46.687,-123.513,48.687,-121.313",
                "restart omakeel.service for its new settings",
            ]
        );
    }

    #[test]
    fn a_new_area_fetches_again_and_missing_data_too() {
        let b = wavelength();
        let mut m = healthy(&b);
        m.fetched.charts = Some("36.8,-123.8,38.8,-121.6".into());
        m.tides_on_disk = false;
        let kinds: Vec<Kind> = plan(&b, &m)
            .into_iter()
            .filter_map(|s| match s {
                Step::Fetch { kind, .. } => Some(kind),
                _ => None,
            })
            .collect();
        assert_eq!(kinds, vec![Kind::Charts, Kind::Tides]);
    }

    #[test]
    fn dev_checkouts_are_built_in_place_and_conflicts_are_left_alone() {
        let b = wavelength();
        let mut m = healthy(&b);
        m.apps.insert(
            App::Helm,
            OnDisk::Checkout(Checkout {
                dir: "/work/omahelm".into(),
                dev: true,
                build: Some(Build::Stale),
                unit_file: false,
            }),
        );
        m.bins.insert(
            App::Logbook,
            Link::Conflict("/h/.local/bin/omalogbook links to /work/bin/omalogbook".into()),
        );
        assert_eq!(lines(&b, &m), vec!["build omahelm in /work/omahelm"]);
        assert_eq!(
            findings(&b, &m),
            vec![
                "/h/.local/bin/omalogbook links to /work/bin/omalogbook. Setup never replaces it; move it aside to let omahoy link omalogbook."
            ]
        );
    }

    #[test]
    fn the_demo_never_asks_for_uucp_and_a_disabled_plugin_is_enabled() {
        let mut b = wavelength();
        b.hub = Hub::Demo;
        let mut m = healthy(&b);
        m.uucp_db = false;
        m.uucp_session = false;
        if let Some(p) = m.plugins.as_mut() {
            p.insert("org.omahoy.wind".into(), false);
        }
        assert_eq!(lines(&b, &m), vec!["enable plugin org.omahoy.wind"]);
        assert_eq!(findings(&b, &m), Vec::<String>::new());
    }

    #[test]
    fn what_apply_cannot_fix_is_reported() {
        let b = wavelength();
        let mut m = healthy(&b);
        m.uucp_session = false;
        m.vault.email = false;
        m.plugins = None;
        m.hub = Some(Ok(vec![crate::machine::SourceStatus {
            name: "serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver_0001-if00:38400".into(),
            status: "error".into(),
        }]));
        assert_eq!(plan(&b, &m), vec![]);
        let f = findings(&b, &m);
        assert_eq!(f.len(), 4, "{f:#?}");
        assert!(f[0].starts_with("The Omarchy shell didn't list"));
        assert!(f[1].contains("Log out and back in once"));
        assert!(f[2].starts_with("git has no user.name or user.email"));
        assert!(f[3].starts_with("The hub can't read serial:/dev/serial/by-id/usb-dAISy"));
    }

    #[test]
    fn outside_noaa_helm_gets_no_fetch_and_an_import_hint() {
        let text = crate::boat::tests::WAVELENGTH
            .replace("home      = 47.687, -122.413", "home = 39.47, 2.73")
            .replace(
                "area      = 46.687,-123.513,48.687,-121.313",
                "area = 38.47,1.63,40.47,3.83",
            )
            .replace(
                "helm, lookout, logbook, tide, wind",
                "helm, lookout, logbook",
            );
        let b = Boat::parse(&text).unwrap();
        let mut m = healthy(&b);
        m.charts_on_disk = false;
        m.fetched = Fetched::default();
        assert_eq!(lines(&b, &m), Vec::<String>::new());
        assert_eq!(
            findings(&b, &m),
            vec![
                "No charts yet, and NOAA doesn't cover this area. Import your own ENC: omahelm import ZIP|DIR"
            ]
        );
    }
}
