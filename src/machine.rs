//! What is on this machine, read once. Facts only; `plan` decides.

use crate::apps::App;
use crate::boat::Boat;
use crate::place::Area;
use crate::sys::{Cmd, Dirs, Runner};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Build {
    NotBuilt,
    /// The binary is older than the checkout's last commit.
    Stale,
    Fresh,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Checkout {
    pub dir: PathBuf,
    /// A developer's own checkout, linked in by link-plugin.sh or on
    /// PATH: built where it is, never replaced.
    pub dev: bool,
    /// None for omalookout, which has no binary.
    pub build: Option<Build>,
    /// The checkout ships `dist/<unit>`.
    pub unit_file: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum OnDisk {
    Absent,
    Checkout(Checkout),
}

/// A link in ~/.local/bin.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Link {
    Missing,
    Ours,
    /// A dangling link, or a link to another build of the same binary.
    Repoint,
    /// A file, or a link to something else: never replaced.
    Conflict(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Unit {
    /// Link in ~/.config/systemd/user; `Ours` when it points at the checkout's dist/.
    pub file: Link,
    pub enabled: bool,
    pub active: bool,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Vault {
    pub repo: bool,
    pub name: bool,
    pub email: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Charts,
    Tides,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Fetched {
    /// The last area each kind was fetched for, as `S,W,N,E`.
    pub charts: Option<String>,
    pub tides: Option<String>,
}

impl Fetched {
    pub fn parse(text: &str) -> Fetched {
        let get = |k: &str| crate::conf::values(text, k).into_iter().last();
        Fetched {
            charts: get("charts"),
            tides: get("tides"),
        }
    }

    pub fn get(&self, k: Kind) -> Option<&str> {
        match k {
            Kind::Charts => self.charts.as_deref(),
            Kind::Tides => self.tides.as_deref(),
        }
    }

    pub fn with(mut self, k: Kind, area: Area) -> Fetched {
        let v = Some(area.to_config());
        match k {
            Kind::Charts => self.charts = v,
            Kind::Tides => self.tides = v,
        }
        self
    }

    pub fn render(&self) -> String {
        let mut out = String::from("# The last area omahoy fetched each kind of data for.\n");
        for (k, v) in [("charts", &self.charts), ("tides", &self.tides)] {
            if let Some(v) = v {
                out += &format!("{k} = {v}\n");
            }
        }
        out
    }
}

/// One source from `omakeel watch --once`.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SourceStatus {
    pub name: String,
    pub status: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Machine {
    pub dirs: Dirs,
    pub missing_packages: Vec<String>,
    pub uucp_db: bool,
    pub uucp_session: bool,
    pub apps: BTreeMap<App, OnDisk>,
    pub bins: BTreeMap<App, Link>,
    /// This checkout's bootstrap, and the state of ~/.local/bin/omahoy.
    pub omahoy: Option<(PathBuf, Link)>,
    /// Plugin id to enabled; None when the shell didn't answer.
    pub plugins: Option<BTreeMap<String, bool>>,
    pub configs: BTreeMap<App, String>,
    pub units: BTreeMap<App, Unit>,
    pub fetched: Fetched,
    pub charts_on_disk: bool,
    pub tides_on_disk: bool,
    pub vault: Vault,
    /// The running hub's sources, or why it didn't answer. None when the
    /// hub isn't meant to be running yet.
    pub hub: Option<Result<Vec<SourceStatus>, String>>,
}

pub const PACKAGES: [&str; 3] = ["git", "curl", "libarchive"];

impl Machine {
    pub fn checkout(&self, app: App) -> Option<&Checkout> {
        match self.apps.get(&app) {
            Some(OnDisk::Checkout(c)) => Some(c),
            _ => None,
        }
    }

    /// Where a new checkout goes: the plugin folder, or omahoy's own
    /// source folder for omakeel, which is not a plugin.
    pub fn home_of(dirs: &Dirs, app: App) -> PathBuf {
        match app.plugin_id() {
            Some(id) => dirs.plugins.join(id),
            None => dirs.data.join("omahoy/src").join(app.name()),
        }
    }

    pub fn binary_path(&self, app: App) -> Option<PathBuf> {
        let bin = app.binary()?;
        let dir = match self.checkout(app) {
            Some(c) => c.dir.clone(),
            None => Machine::home_of(&self.dirs, app),
        };
        Some(dir.join("target/release").join(bin))
    }

    pub fn survey(r: &mut dyn Runner, dirs: &Dirs, boat: &Boat, own: Option<&Path>) -> Machine {
        let missing_packages = PACKAGES
            .iter()
            .filter(|p| r.read(&Cmd::new("pacman", &["-Q", p])).is_err())
            .map(|p| (*p).to_string())
            .collect();
        let (uucp_db, uucp_session) = uucp(r, &dirs.user);
        let mut apps = BTreeMap::new();
        let mut bins = BTreeMap::new();
        let mut configs = BTreeMap::new();
        let mut units = BTreeMap::new();
        for app in App::ALL {
            let on_disk = find(r, dirs, app);
            if let (Some(bin), OnDisk::Checkout(c)) = (app.binary(), &on_disk) {
                let target = c.dir.join("target/release").join(bin);
                bins.insert(app, link_state(&dirs.bin.join(bin), &target, Some(bin)));
            } else if let Some(bin) = app.binary() {
                let target = Machine::home_of(dirs, app).join("target/release").join(bin);
                bins.insert(app, link_state(&dirs.bin.join(bin), &target, Some(bin)));
            }
            if let Some(p) = app.config(dirs) {
                configs.insert(app, std::fs::read_to_string(p).unwrap_or_default());
            }
            if let Some(unit) = app.unit() {
                let dir = match &on_disk {
                    OnDisk::Checkout(c) => c.dir.clone(),
                    OnDisk::Absent => Machine::home_of(dirs, app),
                };
                let file = link_state(&dirs.units.join(unit), &dir.join("dist").join(unit), None);
                let ask = |r: &mut dyn Runner, verb: &str| {
                    r.read(&Cmd::new("systemctl", &["--user", verb, unit]))
                        .is_ok()
                };
                units.insert(
                    app,
                    Unit {
                        file,
                        enabled: ask(r, "is-enabled"),
                        active: ask(r, "is-active"),
                    },
                );
            }
            apps.insert(app, on_disk);
        }
        let omahoy = own.map(|script| {
            let link = link_state(&dirs.bin.join("omahoy"), script, None);
            (script.to_path_buf(), link)
        });
        let plugins = r
            .read(&Cmd::new("omarchy", &["plugin", "list", "--json"]))
            .ok()
            .and_then(|json| plugin_states(&json));
        let fetched = std::fs::read_to_string(dirs.fetched())
            .map(|t| Fetched::parse(&t))
            .unwrap_or_default();
        let charts_on_disk = std::fs::read_dir(dirs.data.join("omahelm/charts"))
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);
        let tides_on_disk = dirs.data.join("omatide/stations.json").is_file();
        let vault_dir = dirs.expand(&boat.vault);
        let id_dir = if vault_dir.is_dir() {
            vault_dir.clone()
        } else {
            dirs.home.clone()
        };
        let ident = |r: &mut dyn Runner, key: &str| {
            r.read(&Cmd::new("git", &["config", "--get", key]).in_dir(&id_dir))
                .is_ok_and(|v| !v.trim().is_empty())
        };
        let vault = Vault {
            repo: vault_dir.join(".git").exists(),
            name: ident(r, "user.name"),
            email: ident(r, "user.email"),
        };
        let keel_active = units.get(&App::Keel).is_some_and(|u| u.active);
        let mut m = Machine {
            dirs: dirs.clone(),
            missing_packages,
            uucp_db,
            uucp_session,
            apps,
            bins,
            omahoy,
            plugins,
            configs,
            units,
            fetched,
            charts_on_disk,
            tides_on_disk,
            vault,
            hub: None,
        };
        if keel_active && let Some(bin) = m.binary_path(App::Keel) {
            m.hub = Some(
                r.read(&Cmd::new(bin.display().to_string(), &["watch", "--once"]))
                    .map(|out| parse_watch(&out)),
            );
        }
        m
    }
}

/// In the uucp group in /etc/group, and in this login's groups.
pub fn uucp(r: &mut dyn Runner, user: &str) -> (bool, bool) {
    let db = r
        .read(&Cmd::new("getent", &["group", "uucp"]))
        .is_ok_and(|line| {
            line.trim()
                .rsplit(':')
                .next()
                .is_some_and(|members| members.split(',').any(|m| m == user))
        });
    let session = r
        .read(&Cmd::new("id", &["-nG"]))
        .is_ok_and(|g| g.split_whitespace().any(|g| g == "uucp"));
    (db, session)
}

pub fn find(r: &mut dyn Runner, dirs: &Dirs, app: App) -> OnDisk {
    let (dir, dev) = match app.plugin_id() {
        Some(id) => {
            let at = dirs.plugins.join(id);
            match std::fs::symlink_metadata(&at) {
                Ok(m) if m.file_type().is_symlink() => match std::fs::canonicalize(&at) {
                    Ok(real) => (real, true),
                    Err(_) => return OnDisk::Absent,
                },
                Ok(m) if m.is_dir() => (at, false),
                _ => return OnDisk::Absent,
            }
        }
        None => match dev_checkout(&dirs.bin.join(app.name()), app.name()) {
            Some(dir) => (dir, true),
            None => {
                let at = Machine::home_of(dirs, app);
                if !at.is_dir() {
                    return OnDisk::Absent;
                }
                (at, false)
            }
        },
    };
    let build = app.binary().map(|bin| {
        let built = std::fs::metadata(dir.join("target/release").join(bin))
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let committed = r
            .read(&Cmd::new("git", &["log", "-1", "--format=%ct"]).in_dir(&dir))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok());
        match (built, committed) {
            (None, _) => Build::NotBuilt,
            (Some(b), Some(c)) if b < c => Build::Stale,
            _ => Build::Fresh,
        }
    });
    let unit_file = app
        .unit()
        .is_some_and(|u| dir.join("dist").join(u).is_file());
    OnDisk::Checkout(Checkout {
        dir,
        dev,
        build,
        unit_file,
    })
}

/// A ~/.local/bin link into some checkout's target/release is that
/// developer's checkout.
fn dev_checkout(link: &Path, bin: &str) -> Option<PathBuf> {
    if !std::fs::symlink_metadata(link)
        .ok()?
        .file_type()
        .is_symlink()
    {
        return None;
    }
    let to = std::fs::read_link(link).ok()?;
    let to = if to.is_absolute() {
        to
    } else {
        link.parent()?.join(to)
    };
    if to.file_name()?.to_str()? != bin {
        return None;
    }
    let release = to.parent()?;
    let target = release.parent()?;
    if release.file_name()?.to_str()? != "release" || target.file_name()?.to_str()? != "target" {
        return None;
    }
    let dir = target.parent()?;
    dir.join("Cargo.toml").is_file().then(|| dir.to_path_buf())
}

/// What a link at `link` is, against the `want`ed target. `bin` lets a
/// link into another build of the same binary be repointed.
pub fn link_state(link: &Path, want: &Path, bin: Option<&str>) -> Link {
    let Ok(meta) = std::fs::symlink_metadata(link) else {
        return Link::Missing;
    };
    if !meta.file_type().is_symlink() {
        return Link::Conflict(format!("{} is a file, not a link", link.display()));
    }
    let Ok(to) = std::fs::read_link(link) else {
        return Link::Missing;
    };
    if to == want {
        return Link::Ours;
    }
    let real = std::fs::canonicalize(link);
    if let (Ok(a), Ok(b)) = (&real, std::fs::canonicalize(want))
        && *a == b
    {
        return Link::Ours;
    }
    let Ok(real) = real else {
        return Link::Repoint;
    };
    let other_build = bin.is_some_and(|b| {
        real.file_name().is_some_and(|n| n == b)
            && real.parent().is_some_and(|p| p.ends_with("target/release"))
    });
    if other_build {
        Link::Repoint
    } else {
        Link::Conflict(format!("{} links to {}", link.display(), real.display()))
    }
}

/// Id to enabled, from `omarchy plugin list --json`: an array of plugin
/// objects, each with `id` and `enabled` at its top level.
pub fn plugin_states(json: &str) -> Option<BTreeMap<String, bool>> {
    let mut out = BTreeMap::new();
    let mut depth = 0usize;
    let mut chars = json.chars().peekable();
    let mut id: Option<String> = None;
    let mut enabled: Option<bool> = None;
    let mut key: Option<String> = None;
    let mut started = false;
    while let Some(c) = chars.next() {
        match c {
            '[' | '{' => {
                started = true;
                depth += 1;
                if depth == 2 {
                    id = None;
                    enabled = None;
                }
            }
            ']' | '}' => {
                if depth == 2
                    && let (Some(i), Some(e)) = (id.take(), enabled.take())
                {
                    out.insert(i, e);
                }
                depth = depth.checked_sub(1)?;
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next()? {
                        '\\' => {
                            s.push(chars.next()?);
                        }
                        '"' => break,
                        ch => s.push(ch),
                    }
                }
                if depth != 2 {
                    continue;
                }
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
                if chars.peek() == Some(&':') {
                    key = Some(s);
                } else if key.as_deref() == Some("id") {
                    id = Some(s);
                    key = None;
                } else {
                    key = None;
                }
            }
            't' | 'f' if depth == 2 && key.as_deref() == Some("enabled") => {
                enabled = Some(c == 't');
                key = None;
            }
            ',' if depth == 2 => key = None,
            _ => {}
        }
    }
    (started && depth == 0).then_some(out)
}

/// `omakeel watch --once`: a `fix STATUS ...` line, then
/// `source NAME STATUS [MESSAGE]` for each source.
pub fn parse_watch(text: &str) -> Vec<SourceStatus> {
    text.lines()
        .filter_map(|line| {
            let mut w = line.strip_prefix("source ")?.split_whitespace();
            Some(SourceStatus {
                name: w.next()?.to_string(),
                status: w.next()?.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plugin_states_read_the_top_level_of_each_plugin() {
        let json = r#"[
          {"id":"org.omahoy.helm","enabled":true,"firstParty":false,"kinds":["panel"],
           "barWidget":{"id":"nested","enabled":false}},
          {"name":"Wind \"live\"","enabled":false,"id":"org.omahoy.wind"},
          {"id":"no.state"}
        ]"#;
        let got = plugin_states(json).unwrap();
        assert_eq!(
            got.into_iter().collect::<Vec<_>>(),
            vec![
                ("org.omahoy.helm".to_string(), true),
                ("org.omahoy.wind".to_string(), false)
            ]
        );
        assert_eq!(plugin_states(""), None);
        assert_eq!(plugin_states("[{\"id\":"), None);
    }

    #[test]
    fn watch_once_names_each_source_and_its_status() {
        let out = "fix ok 37.8650000,-122.3200000 0s\n\
source serial:/dev/serial/by-id/usb-GPS:4800 ok\n\
source serial:/dev/serial/by-id/usb-dAISy:38400 error permission denied\n";
        assert_eq!(
            parse_watch(out),
            vec![
                SourceStatus {
                    name: "serial:/dev/serial/by-id/usb-GPS:4800".into(),
                    status: "ok".into()
                },
                SourceStatus {
                    name: "serial:/dev/serial/by-id/usb-dAISy:38400".into(),
                    status: "error".into()
                },
            ]
        );
        assert_eq!(parse_watch("fix none\n"), vec![]);
    }

    #[test]
    fn the_fetch_record_keeps_one_area_per_kind() {
        let a = Area::parse("1,2,3,4").unwrap();
        let b = Area::parse("5,6,7,8").unwrap();
        let f = Fetched::default()
            .with(Kind::Charts, a)
            .with(Kind::Tides, a);
        let f = Fetched::parse(&f.render()).with(Kind::Charts, b);
        assert_eq!(f.get(Kind::Charts), Some("5,6,7,8"));
        assert_eq!(f.get(Kind::Tides), Some("1,2,3,4"));
        let back = Fetched::parse(&f.with(Kind::Charts, a).render());
        assert_eq!(
            back.get(Kind::Charts),
            Some("1,2,3,4"),
            "A, B, then A again is A"
        );
    }

    #[test]
    fn links_are_ours_repointable_or_left_alone() {
        let tmp = std::env::temp_dir().join(format!("omahoy-links-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let rel = tmp.join("a/target/release");
        std::fs::create_dir_all(&rel).unwrap();
        std::fs::create_dir_all(tmp.join("b/target/release")).unwrap();
        std::fs::write(rel.join("omatide"), "").unwrap();
        std::fs::write(tmp.join("b/target/release/omatide"), "").unwrap();
        std::fs::write(tmp.join("wrapper"), "").unwrap();
        std::fs::write(tmp.join("a/Cargo.toml"), "").unwrap();
        let bin = tmp.join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let want = rel.join("omatide");
        let link = |name: &str, to: &Path| {
            std::os::unix::fs::symlink(to, bin.join(name)).unwrap();
            bin.join(name)
        };
        assert_eq!(
            link_state(&bin.join("none"), &want, Some("omatide")),
            Link::Missing
        );
        let ours = link("ours", &want);
        assert_eq!(link_state(&ours, &want, Some("omatide")), Link::Ours);
        let other = link("other", &tmp.join("b/target/release/omatide"));
        assert_eq!(link_state(&other, &want, Some("omatide")), Link::Repoint);
        let dangling = link("dangling", &tmp.join("gone"));
        assert_eq!(link_state(&dangling, &want, Some("omatide")), Link::Repoint);
        let wrapper = link("wrapper", &tmp.join("wrapper"));
        assert!(matches!(
            link_state(&wrapper, &want, Some("omatide")),
            Link::Conflict(_)
        ));
        std::fs::write(bin.join("file"), "").unwrap();
        assert!(matches!(
            link_state(&bin.join("file"), &want, Some("omatide")),
            Link::Conflict(_)
        ));
        let dev = link("omatide", &want);
        assert_eq!(dev_checkout(&dev, "omatide"), Some(tmp.join("a")));
        assert_eq!(dev_checkout(&wrapper, "wrapper"), None);
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
