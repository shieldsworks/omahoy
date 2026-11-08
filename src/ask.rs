//! The questions. In order: the boat, its hardware, home and the sailing
//! area, the apps, the logbook. Hardware comes before home, so a GPS fix
//! can be the home port.

use crate::apps::App;
use crate::boat::{Boat, Partial, parse_depth};
use crate::depth::{self, Units};
use crate::hub::{Baud, Heard, Hub, Source, Verdict, parse_probe};
use crate::machine::uucp;
use crate::place::{Area, Position, coverage};
use crate::sys::{Cmd, Dirs, Runner, Terminal, confirm};
use std::path::PathBuf;

pub const STOPPED: &str = "Setup stopped. Nothing changed.";

pub struct Ask<'a> {
    pub t: &'a mut dyn Terminal,
    pub r: &'a mut dyn Runner,
    pub dirs: &'a Dirs,
    /// The omakeel binary that probes the ports.
    pub omakeel: PathBuf,
}

/// The sections a sailor can change later, in the order first asked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Boat,
    Hardware,
    Home,
    Apps,
    Logbook,
}

impl Section {
    pub const ALL: [Section; 5] = [
        Section::Boat,
        Section::Hardware,
        Section::Home,
        Section::Apps,
        Section::Logbook,
    ];

    fn title(self) -> &'static str {
        match self {
            Section::Boat => "boat: name, units, draft, clearance",
            Section::Hardware => "hardware: GPS, AIS and network sources",
            Section::Home => "home port and sailing area",
            Section::Apps => "apps",
            Section::Logbook => "logbook folder and git",
        }
    }
}

impl Ask<'_> {
    fn line(&mut self, prompt: &str, default: Option<&str>) -> Result<String, String> {
        let prompt = match default {
            Some(d) if !d.is_empty() => format!("{prompt} [{d}]:"),
            _ => format!("{prompt}:"),
        };
        loop {
            let a = self.t.ask(&prompt).ok_or(STOPPED)?;
            if !a.is_empty() {
                return Ok(a);
            }
            if let Some(d) = default {
                return Ok(d.to_string());
            }
        }
    }

    /// Asks until `parse` accepts, showing why it didn't.
    fn parsed<T>(
        &mut self,
        prompt: &str,
        default: Option<&str>,
        parse: impl Fn(&str) -> Result<T, String>,
    ) -> Result<T, String> {
        loop {
            let a = self.line(prompt, default)?;
            match parse(&a) {
                Ok(v) => return Ok(v),
                Err(e) => self.t.say(&format!("  {e}")),
            }
        }
    }

    fn yes(&mut self, prompt: &str, default: bool) -> bool {
        confirm(self.t, prompt, default)
    }

    /// Every section in order, from defaults to a whole boat.
    pub fn first_run(&mut self, mut p: Partial) -> Result<Boat, String> {
        self.t
            .say("Omahoy. A few questions about your boat; nothing changes until you say so.\n");
        self.boat(&mut p)?;
        self.t.say("");
        let fix = self.hardware(&mut p)?;
        self.t.say("");
        self.home(&mut p, fix)?;
        self.t.say("");
        self.apps(&mut p)?;
        self.logbook(&mut p)?;
        p.finish().map_err(|e| e.join("\n"))
    }

    /// boat.toml exists: show it, change sections until Enter.
    pub fn revise(&mut self, boat: &Boat) -> Result<Boat, String> {
        self.t.say(&format!(
            "Your boat, from {}:\n",
            self.dirs.show(&self.dirs.boat())
        ));
        self.t.say(boat.render().trim_end());
        let mut p = Partial::from_boat(boat);
        loop {
            self.t.say("");
            for (i, s) in Section::ALL.iter().enumerate() {
                self.t.say(&format!("  {} {}", i + 1, s.title()));
            }
            let a = self
                .t
                .ask("Change which? A number, or Enter to keep it and check the machine:")
                .ok_or(STOPPED)?;
            if a.is_empty() {
                break;
            }
            match a
                .parse::<usize>()
                .ok()
                .and_then(|n| Section::ALL.get(n.wrapping_sub(1)))
            {
                Some(s) => self.section(*s, &mut p)?,
                None => self.t.say("  Pick a number from the list."),
            }
        }
        p.finish().map_err(|e| e.join("\n"))
    }

    /// `omahoy hardware`: only the hardware question.
    pub fn hardware_only(&mut self, boat: &Boat) -> Result<Boat, String> {
        let mut p = Partial::from_boat(boat);
        self.section(Section::Hardware, &mut p)?;
        p.finish().map_err(|e| e.join("\n"))
    }

    /// One section of a boat that already has every answer.
    fn section(&mut self, s: Section, p: &mut Partial) -> Result<(), String> {
        match s {
            Section::Boat => self.boat(p),
            Section::Hardware => {
                let fix = self.hardware(p)?;
                if fix.is_some() && fix != p.home {
                    self.home(p, fix)?;
                }
                Ok(())
            }
            Section::Home => self.home(p, None),
            Section::Apps => self.apps(p),
            Section::Logbook => self.logbook(p),
        }
    }

    fn boat(&mut self, p: &mut Partial) -> Result<(), String> {
        let name = self.parsed("Boat name", p.name.as_deref(), |s| {
            if s.contains(['"', '#']) {
                Err("A boat name can't hold \" or #.".to_string())
            } else {
                Ok(s.trim().to_string())
            }
        })?;
        let units = self.parsed(
            "Depth units: feet, meters or fathoms",
            Some(p.units.unwrap_or(Units::Feet).name()),
            |s| Units::parse(s).ok_or_else(|| format!("{s} is not feet, meters or fathoms")),
        )?;
        let draft_default = p.draft.map(depth::show);
        let draft = self.parsed(
            &format!("Draft, in {}", units.name()),
            draft_default.as_deref(),
            |s| parse_depth(s, "draft", false),
        )?;
        let clearance_default = depth::show(p.clearance.unwrap_or(units.default_clearance()));
        let clearance = self.parsed(
            &format!("Clearance you want under the keel, in {}", units.name()),
            Some(&clearance_default),
            |s| parse_depth(s, "clearance", true),
        )?;
        let d = depth::depths(units, draft, clearance);
        self.t.say(&format!(
            "  Safety depth {s} {u}: the chart shades water shallower than {s} {u}.",
            s = d.safety_depth,
            u = units.name()
        ));
        p.name = Some(name);
        p.units = Some(units);
        p.draft = Some(draft);
        p.clearance = Some(clearance);
        Ok(())
    }

    /// Runs `omakeel probe`. A probe that fails to run is an error, never
    /// an empty scan: an omakeel from before `probe` existed would
    /// otherwise read as "no hardware" and set the hub to the demo.
    fn probe(
        &mut self,
        targets: &[String],
        via_uucp: bool,
    ) -> Result<Vec<Result<Heard, String>>, String> {
        let bin = self.omakeel.display().to_string();
        let cmd = if via_uucp {
            let mut inner = format!("{} probe", shell_quote(&bin));
            for t in targets {
                inner += &format!(" {}", shell_quote(t));
            }
            Cmd::new("sg", &["uucp", "-c", &inner])
        } else {
            Cmd::new(bin, &["probe"]).args(targets.iter().cloned())
        };
        let out = self
            .r
            .read(&cmd)
            .map_err(|e| format!("couldn't run omakeel probe: {e}"))?;
        let is_probe_line = |l: &str| {
            l.split_whitespace()
                .next()
                .is_some_and(|w| w.starts_with("serial:") || w.starts_with("tcp:"))
        };
        let lines = out.lines().filter(|l| !l.trim().is_empty());
        if lines.clone().next().is_some() && !lines.clone().any(is_probe_line) {
            let first = lines.clone().next().unwrap_or("").trim();
            return Err(format!(
                "couldn't run omakeel probe: it printed {first:?}, which is not a probe report. Update omakeel."
            ));
        }
        Ok(parse_probe(&out))
    }

    /// omakeel.service holds the ports it reads, so it is stopped while
    /// the probe listens and started again after.
    fn probe_with_hub_stopped(
        &mut self,
        targets: &[String],
        via_uucp: bool,
    ) -> Result<Vec<Result<Heard, String>>, String> {
        let unit = ["--user", "is-active", "--quiet", "omakeel.service"];
        let running = self.r.read(&Cmd::new("systemctl", &unit)).is_ok();
        if running {
            self.t
                .say("  Stopping the hub while setup listens to the ports.");
            let _ = self.r.run(&Cmd::new(
                "systemctl",
                &["--user", "stop", "omakeel.service"],
            ));
        }
        let heard = self.probe(targets, via_uucp);
        if running {
            let _ = self.r.run(&Cmd::new(
                "systemctl",
                &["--user", "start", "omakeel.service"],
            ));
        }
        heard
    }

    /// Sorts what the probe heard: kept sources, and the paths that need uucp.
    fn sort(&mut self, heard: Vec<Result<Heard, String>>, kept: &mut Vec<Source>) -> Vec<String> {
        let mut denied = Vec::new();
        for h in heard {
            let h = match h {
                Ok(h) => h,
                Err(e) => {
                    self.t.say(&format!("  ? {e}"));
                    continue;
                }
            };
            let name = h.path().map_or(h.spec.clone(), short_device);
            match &h.verdict {
                Verdict::Ok { source, .. } => {
                    self.t.say(&format!("  ✓ {name}: {}", h.describe()));
                    if !kept.contains(source) {
                        kept.push(source.clone());
                    }
                }
                Verdict::Denied => {
                    self.t.say(&format!("  ✗ {name}: permission denied"));
                    if let Some(p) = h.path() {
                        denied.push(p.to_string());
                    }
                }
                Verdict::Busy => self.t.say(&format!(
                    "  ✗ {name}: held by another program. Close it, then run omahoy hardware."
                )),
                Verdict::Silent => {
                    self.t.say(&format!("  ? {name}: silent at every baud."));
                    if let Some(path) = h.path()
                        && self.yes(
                            "    Keep it as AIS at 38400, the usual rate? A dAISy in a quiet anchorage can stay silent for minutes.",
                            false,
                        )
                    {
                        kept.push(Source::Serial {
                            path: path.to_string(),
                            baud: Baud::B38400,
                        });
                    }
                }
                Verdict::Missing => self.t.say(&format!("  ✗ {name}: not plugged in")),
                Verdict::Unreachable(why) => self.t.say(&format!("  ✗ {name}: {why}")),
            }
        }
        denied
    }

    /// The hardware question. Returns the GPS fix, if one was heard.
    fn hardware(&mut self, p: &mut Partial) -> Result<Option<Position>, String> {
        self.t
            .say("Hardware. Plug in your GPS and AIS receivers now, then press Enter.");
        self.t.ask("").ok_or(STOPPED)?;
        self.t
            .say("  Listening to each serial device, at each baud (up to 30 s a device).");
        let heard = self.probe_with_hub_stopped(&[], false)?;
        let fix = first_fix(&heard);
        let mut kept = Vec::new();
        if heard.is_empty() {
            self.t.say("  No serial devices found.");
        }
        let denied = self.sort(heard, &mut kept);
        let mut fix = fix;
        if !denied.is_empty() {
            let user = self.dirs.user.clone();
            let (db, session) = uucp(self.r, &user);
            let mut joined = db;
            if !db {
                self.t.say("  Serial ports need the uucp group.");
                if self.yes(
                    &format!("  Add {user} now? (sudo gpasswd -a {user} uucp)"),
                    true,
                ) {
                    match self
                        .r
                        .run(&Cmd::new("sudo", &["gpasswd", "-a", &user, "uucp"]))
                    {
                        Ok(()) => joined = true,
                        Err(e) => self.t.say(&format!("  That didn't work: {e}")),
                    }
                }
            }
            if joined && !session {
                self.t.say("  Listening again through sg uucp.");
                let again = self.probe_with_hub_stopped(&denied, true)?;
                fix = fix.or(first_fix(&again));
                let still = self.sort(again, &mut kept);
                if !still.is_empty() {
                    self.t
                        .say("  Still denied. Log out and back in, then run omahoy hardware.");
                }
                self.t
                    .say("  The hub gets the group at your next login. Log out and back in once.");
            } else if !joined {
                self.t
                    .say("  Skipped. Run omahoy hardware once you're in the uucp group.");
            } else {
                self.t.say("  Denied even with the uucp group. Check the device's group: ls -l /dev/serial/by-id/");
            }
        }
        for old in p
            .hub
            .as_ref()
            .map(|h| h.sources().to_vec())
            .unwrap_or_default()
        {
            if !kept.contains(&old)
                && !matches!(old, Source::Tcp { .. })
                && self.yes(
                    &format!(
                        "  Keep {} from before? It wasn't heard just now.",
                        old.spec()
                    ),
                    false,
                )
            {
                kept.push(old);
            }
        }
        let had_tcp: Vec<Source> = p
            .hub
            .as_ref()
            .map(|h| {
                h.sources()
                    .iter()
                    .filter(|s| matches!(s, Source::Tcp { .. }))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();
        let tcp_default = had_tcp
            .first()
            .map(|s| s.spec().trim_start_matches("tcp:").to_string());
        loop {
            let a = self
                .t
                .ask(&match &tcp_default {
                    Some(d) => format!("Network source (a phone app or NMEA multiplexer)? host:port, - for none [{d}]:"),
                    None => "Network source (a phone app or NMEA multiplexer)? host:port, or Enter for none:".into(),
                })
                .ok_or(STOPPED)?;
            let a = match (a.as_str(), &tcp_default) {
                ("", Some(d)) => d.clone(),
                ("" | "-", _) => break,
                _ => a,
            };
            let spec = if a.starts_with("tcp:") {
                a
            } else {
                format!("tcp:{a}")
            };
            let source = match Source::parse(&spec) {
                Ok(s) => s,
                Err(e) => {
                    self.t.say(&format!("  {e}"));
                    continue;
                }
            };
            let heard = self.probe(&[source.spec()], false)?;
            let reached = heard.iter().any(|h| {
                matches!(
                    h,
                    Ok(Heard {
                        verdict: Verdict::Ok { .. },
                        ..
                    })
                )
            });
            fix = fix.or(first_fix(&heard));
            self.sort(heard, &mut kept);
            if !reached
                && self.yes(
                    "  Keep it anyway? Phone apps share only while they're open.",
                    true,
                )
                && !kept.contains(&source)
            {
                kept.push(source);
            }
            break;
        }
        let hub = Hub::from_sources(kept);
        if hub == Hub::Demo {
            self.t.say(
                "  No hardware. The hub will play the demo sail, a recorded afternoon off Berkeley Marina,\n  so the apps have something to show. Run omahoy hardware when you plug something in.",
            );
        }
        p.hub = Some(hub);
        Ok(fix)
    }

    fn home(&mut self, p: &mut Partial, fix: Option<Position>) -> Result<(), String> {
        let default = fix.or(p.home).map(Position::to_config);
        let prompt = match fix {
            Some(f) => format!(
                "Home port. Enter for where the GPS is now ({}), or type lat, lon",
                f.show()
            ),
            None => "Home port, as lat, lon (47.687, -122.413 or 47°41.2'N 122°24.8'W)".into(),
        };
        let home = self.parsed(&prompt, default.as_deref(), Position::parse)?;
        let area = match p.area {
            Some(a) if a.contains(home) && p.home == Some(home) => a,
            _ => Area::around(home),
        };
        let c = coverage(area);
        let mark = |b: bool| if b { "✓" } else { "✗" };
        self.t.say(&format!(
            "  Sailing area {} (south, west, north, east), about 60 nm around home.",
            area.to_config()
        ));
        self.t.say(&format!(
            "  NOAA charts {}  NOAA tides and currents {}  HRRR wind {}",
            mark(c.noaa),
            mark(c.noaa),
            mark(c.hrrr)
        ));
        if !(c.noaa && c.hrrr) {
            self.t.say("  NOAA and HRRR cover US waters only.");
        }
        p.home = Some(home);
        p.area = Some(area);
        if let Some(apps) = &mut p.apps {
            let before = apps.len();
            apps.retain(|a| a.refusal(c).is_none());
            if apps.len() < before {
                self.t.say("  Apps that can't work here are now off.");
            }
        }
        Ok(())
    }

    fn apps(&mut self, p: &mut Partial) -> Result<(), String> {
        let Some(area) = p.area else {
            return Err("the apps question needs the home port first".into());
        };
        let c = coverage(area);
        let mut on: Vec<App> = p
            .apps
            .clone()
            .unwrap_or_else(|| App::CHOSEN.to_vec())
            .into_iter()
            .filter(|a| a.refusal(c).is_none())
            .collect();
        loop {
            self.t.say("Apps (omakeel, the hub, always comes):");
            for (i, app) in App::CHOSEN.iter().enumerate() {
                let line = match app.refusal(c) {
                    Some(why) => format!(
                        "  {} [-] {:<11} not available where you sail: {why}",
                        i + 1,
                        app.name()
                    ),
                    None => {
                        let mark = if on.contains(app) { "x" } else { " " };
                        let mut l =
                            format!("  {} [{mark}] {:<11} {}", i + 1, app.name(), app.what());
                        if let Some(c) = app.caveat(c) {
                            l += &format!("; {c}");
                        }
                        l
                    }
                };
                self.t.say(&line);
            }
            let a = self
                .t
                .ask("Toggle by number, Enter to accept:")
                .ok_or(STOPPED)?;
            if a.is_empty() {
                break;
            }
            for w in a.split([' ', ',']).filter(|w| !w.is_empty()) {
                let Some(app) = w
                    .parse::<usize>()
                    .ok()
                    .and_then(|n| App::CHOSEN.get(n.wrapping_sub(1)))
                else {
                    self.t.say(&format!("  {w} is not on the list."));
                    continue;
                };
                if let Some(why) = app.refusal(c) {
                    self.t
                        .say(&format!("  {} can't work here: {why}.", app.name()));
                } else if let Some(i) = on.iter().position(|a| a == app) {
                    on.remove(i);
                } else {
                    on.push(*app);
                }
            }
        }
        on.sort();
        p.apps = Some(on);
        Ok(())
    }

    fn logbook(&mut self, p: &mut Partial) -> Result<(), String> {
        let logging = p.apps.as_ref().is_some_and(|a| a.contains(&App::Logbook));
        if !logging {
            p.vault.get_or_insert_with(|| "~/Logbook".into());
            p.git.get_or_insert(true);
            return Ok(());
        }
        let default = p.vault.clone().unwrap_or_else(|| "~/Logbook".into());
        let vault = self.parsed("Logbook folder", Some(&default), |s| {
            let s = s.trim().trim_end_matches('/');
            if !(s.starts_with('/') || s.starts_with("~/")) {
                Err("The folder starts with / or ~/.".to_string())
            } else if s.contains(['"', '#']) {
                Err("A folder name can't hold \" or #.".to_string())
            } else {
                Ok(s.to_string())
            }
        })?;
        let git = self.yes("Keep the logbook in git?", p.git.unwrap_or(true));
        p.vault = Some(vault);
        p.git = Some(git);
        Ok(())
    }
}

fn first_fix(heard: &[Result<Heard, String>]) -> Option<Position> {
    heard.iter().find_map(|h| match h {
        Ok(Heard {
            verdict: Verdict::Ok { at: Some(p), .. },
            ..
        }) => Some(*p),
        _ => None,
    })
}

/// One word for `sh -c`, in single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// `usb-dAISy_AIS_Receiver_0001-if00` rather than the whole by-id path.
fn short_device(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::tests::dirs;
    use crate::sys::fake::{Commands, Script};

    const KEEL: &str = "/k/omakeel";

    fn ask(t: &mut Script, r: &mut Commands, p: Partial) -> Result<Boat, String> {
        let dirs = dirs();
        Ask {
            t,
            r,
            dirs: &dirs,
            omakeel: KEEL.into(),
        }
        .first_run(p)
    }

    #[test]
    fn no_hardware_is_the_demo_and_the_rest_are_defaults() {
        let mut t = Script::new(&[
            "Wavelength",
            "",
            "5.5",
            "", // boat, feet, draft, clearance 2
            "", // plug in, Enter
            "", // no network source
            "47.687, -122.413",
            "", // apps as offered
            "", // ~/Logbook
            "", // git yes
        ]);
        let mut r = Commands::default().on("/k/omakeel probe", Ok(""));
        let b = ask(&mut t, &mut r, Partial::default()).unwrap();
        assert_eq!(
            b.render(),
            crate::boat::tests::WAVELENGTH
                .lines()
                .filter(|l| !l.starts_with("source"))
                .map(|l| format!("{l}\n"))
                .collect::<String>()
                .replace("apps ", "source    = demo\napps ")
        );
        assert!(
            t.shown
                .contains("No hardware. The hub will play the demo sail")
        );
        assert!(t.shown.contains("Safety depth 8 feet"));
        assert!(
            t.shown
                .contains("NOAA charts ✓  NOAA tides and currents ✓  HRRR wind ✓")
        );
    }

    #[test]
    fn a_gps_fix_is_the_home_port_and_a_denied_port_offers_uucp() {
        let mut t = Script::new(&[
            "Dash",
            "feet",
            "4",
            "2",
            "",  // plug in
            "",  // add to uucp: yes
            "",  // no network source
            "",  // home: the GPS fix
            "2", // apps: lookout off
            "",  // accept
            "~/notes/Logbook",
            "n",
        ]);
        let probe = "\
serial:/dev/serial/by-id/usb-Prolific-port0:115200  ok  fix=41 ais=0 rejected=2 at=37.8663,-122.3148
serial:/dev/serial/by-id/usb-dAISy-if00  denied
";
        let mut r = Commands::default()
            .on("/k/omakeel probe", Ok(probe))
            .on("getent group uucp", Ok("uucp:x:987:\n"))
            .on("id -nG", Ok("sailor wheel\n"))
            .on("sudo gpasswd -a sailor uucp", Ok(""))
            .on(
                "sg uucp -c ''\\''/k/omakeel'\\'' probe '\\''/dev/serial/by-id/usb-dAISy-if00'\\'''",
                Ok("serial:/dev/serial/by-id/usb-dAISy-if00:38400  ok  fix=0 ais=6 rejected=0\n"),
            );
        let b = ask(&mut t, &mut r, Partial::default()).unwrap();
        assert_eq!(
            b.hub.specs(),
            vec![
                "serial:/dev/serial/by-id/usb-Prolific-port0:115200",
                "serial:/dev/serial/by-id/usb-dAISy-if00:38400"
            ]
        );
        assert_eq!(b.home.to_config(), "37.8663, -122.3148");
        assert_eq!(b.area.to_config(), "36.866,-123.415,38.866,-121.215");
        assert_eq!(
            b.apps.chosen(),
            vec![App::Helm, App::Logbook, App::Tide, App::Wind]
        );
        assert_eq!((b.vault.as_str(), b.git), ("~/notes/Logbook", false));
        assert!(r.log.contains(&"sudo gpasswd -a sailor uucp".to_string()));
        assert!(t.shown.contains("Log out and back in once"));
        assert!(
            t.shown
                .contains("Enter for where the GPS is now (37°52.0′N 122°18.9′W)")
        );
    }

    #[test]
    fn a_silent_device_can_be_kept_and_an_unreachable_phone_too() {
        let mut t = Script::new(&[
            "Dash",
            "",
            "4",
            "",
            "",
            "y",                  // keep silent as AIS
            "192.168.1.20:10110", // phone
            "",                   // keep anyway
            "37.8663, -122.3148",
            "",
            "",
            "",
        ]);
        let mut r = Commands::default()
            .on(
                "/k/omakeel probe",
                Ok("serial:/dev/serial/by-id/usb-dAISy-if00  silent\n"),
            )
            .on(
                "/k/omakeel probe tcp:192.168.1.20:10110",
                Ok("tcp:192.168.1.20:10110  unreachable  connection refused\n"),
            );
        let b = ask(&mut t, &mut r, Partial::default()).unwrap();
        assert_eq!(
            b.hub.specs(),
            vec![
                "serial:/dev/serial/by-id/usb-dAISy-if00:38400",
                "tcp:192.168.1.20:10110"
            ]
        );
        assert!(
            t.shown
                .contains("✗ tcp:192.168.1.20:10110: connection refused")
        );
    }

    #[test]
    fn a_running_hub_is_stopped_for_the_probe_and_started_after() {
        let mut t = Script::new(&[
            "Dash",
            "",
            "4",
            "",
            "",
            "",
            "37.8663, -122.3148",
            "",
            "",
            "",
        ]);
        let mut r = Commands::default()
            .on("systemctl --user is-active --quiet omakeel.service", Ok(""))
            .on("systemctl --user stop omakeel.service", Ok(""))
            .on("systemctl --user start omakeel.service", Ok(""))
            .on(
                "/k/omakeel probe",
                Ok("serial:/dev/serial/by-id/usb-GPS  busy\n"),
            );
        ask(&mut t, &mut r, Partial::default()).unwrap();
        let order: Vec<&str> = r
            .log
            .iter()
            .map(String::as_str)
            .filter(|l| !l.contains("is-active"))
            .collect();
        assert_eq!(
            order,
            vec![
                "systemctl --user stop omakeel.service",
                "/k/omakeel probe",
                "systemctl --user start omakeel.service"
            ]
        );
        assert!(t.shown.contains("usb-GPS: held by another program"));
    }

    #[test]
    fn mallorca_gets_no_tide_or_wind_and_the_chart_import_hint() {
        let mut t = Script::new(&[
            "Salt",
            "meters",
            "1.7",
            "",
            "",
            "",
            "39.47, 2.73",
            "4", // try to turn on omatide
            "",
            "",
            "",
        ]);
        let mut r = Commands::default().on("/k/omakeel probe", Ok(""));
        let b = ask(&mut t, &mut r, Partial::default()).unwrap();
        assert_eq!(b.apps.chosen(), vec![App::Helm, App::Lookout, App::Logbook]);
        assert_eq!(b.area.to_config(), "38.47,1.63,40.47,3.83");
        assert!(
            t.shown
                .contains("NOAA charts ✗  NOAA tides and currents ✗  HRRR wind ✗")
        );
        assert!(t.shown.contains("4 [-] omatide     not available where you sail: NOAA tide stations cover US waters only"));
        assert!(
            t.shown
                .contains("5 [-] omawind     not available where you sail: HRRR forecasts")
        );
        assert!(
            t.shown
                .contains("import your own ENC: omahelm import ZIP|DIR")
        );
        assert!(t.shown.contains("omatide can't work here"));
        assert!(t.shown.contains("Safety depth 3 meters"));
    }

    #[test]
    fn caseys_machine_keeps_his_vault_and_asks_his_name() {
        let tmp = std::env::temp_dir().join(format!("omahoy-adopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let dirs = Dirs::resolve(
            Some(tmp.clone().into()),
            None,
            None,
            None,
            Some("casey".into()),
        )
        .unwrap();
        std::fs::create_dir_all(dirs.config.join("omalogbook")).unwrap();
        std::fs::write(
            dirs.config.join("omalogbook/config.toml"),
            "vault = \"/home/casey/notes/Logbook\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(&dirs.plugins).unwrap();
        for id in [
            "org.omahoy.helm",
            "org.omahoy.logbook",
            "org.omahoy.lookout",
            "org.omahoy.wind",
        ] {
            std::os::unix::fs::symlink("/work", dirs.plugins.join(id)).unwrap();
        }
        let p = Partial::from_existing(&dirs);
        assert_eq!(p.name, None, "his logbook config has no boat line");
        assert_eq!(p.vault.as_deref(), Some("/home/casey/notes/Logbook"));
        assert_eq!(
            p.apps,
            Some(vec![App::Helm, App::Lookout, App::Logbook, App::Wind])
        );
        let mut t = Script::new(&[
            "Dash",
            "",
            "4",
            "",
            "",
            "",
            "37.8663, -122.3148",
            "",
            "",
            "",
        ]);
        let mut r = Commands::default().on("/k/omakeel probe", Ok(""));
        let b = Ask {
            t: &mut t,
            r: &mut r,
            dirs: &dirs,
            omakeel: KEEL.into(),
        }
        .first_run(p)
        .unwrap();
        std::fs::remove_dir_all(&tmp).unwrap();
        assert!(t.shown.contains("Boat name:"), "no default name to offer");
        assert!(
            t.shown
                .contains("Logbook folder [/home/casey/notes/Logbook]:")
        );
        assert_eq!(b.name, "Dash");
        assert_eq!(b.vault, "/home/casey/notes/Logbook");
        assert_eq!(
            b.apps.chosen(),
            vec![App::Helm, App::Lookout, App::Logbook, App::Wind],
            "omatide stays off: it isn't installed"
        );
    }

    #[test]
    fn a_probe_that_fails_to_run_is_an_error_not_no_hardware() {
        for (said, want) in [
            (
                Err("/k/omakeel probe failed (exit 2): unknown command: probe"),
                "couldn't run omakeel probe: /k/omakeel probe failed (exit 2): unknown command: probe",
            ),
            (
                Ok("usage: omakeel [run|watch] [--source SPEC]\n"),
                "couldn't run omakeel probe: it printed \"usage: omakeel [run|watch] [--source SPEC]\", which is not a probe report. Update omakeel.",
            ),
        ] {
            let mut t = Script::new(&["Dash", "", "4", "", ""]);
            let mut r = Commands::default().on("/k/omakeel probe", said);
            let got = ask(&mut t, &mut r, Partial::default());
            assert_eq!(got, Err(want.to_string()));
            assert!(!t.shown.contains("No hardware"), "never the demo");
        }
    }

    #[test]
    fn a_failed_probe_still_starts_the_hub_it_stopped() {
        let mut t = Script::new(&["Dash", "", "4", "", ""]);
        let mut r = Commands::default()
            .on("systemctl --user is-active --quiet omakeel.service", Ok(""))
            .on("systemctl --user stop omakeel.service", Ok(""))
            .on("systemctl --user start omakeel.service", Ok(""))
            .on("/k/omakeel probe", Err("exit 2"));
        assert!(ask(&mut t, &mut r, Partial::default()).is_err());
        assert_eq!(
            r.log.last().map(String::as_str),
            Some("systemctl --user start omakeel.service")
        );
    }

    #[test]
    fn end_of_input_stops_without_a_boat() {
        let mut t = Script::new(&["Dash"]);
        let mut r = Commands::default();
        assert_eq!(ask(&mut t, &mut r, Partial::default()), Err(STOPPED.into()));
    }
}
