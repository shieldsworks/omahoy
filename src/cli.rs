//! The four commands: setup, doctor, apply, hardware.

use crate::apps::App;
use crate::ask::Ask;
use crate::boat::{Boat, Partial};
use crate::machine::{Build, Machine, OnDisk, find};
use crate::plan::{Step, findings, plan};
use crate::sys::{Dirs, Runner, Terminal, confirm};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    Setup,
    Doctor,
    Apply { yes: bool },
    Hardware,
    Help,
}

pub const USAGE: &str = "\
usage: omahoy                set up, or change boat.toml and apply it
       omahoy doctor         read-only: what's wrong, and what apply would do
       omahoy apply [--yes]  make the machine match boat.toml
       omahoy hardware       ask only the hardware question again";

pub fn parse_args(args: &[String]) -> Result<Command, String> {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    Ok(match words.as_slice() {
        [] | ["setup"] => Command::Setup,
        ["doctor"] => Command::Doctor,
        ["apply"] => Command::Apply { yes: false },
        ["apply", "--yes" | "-y"] => Command::Apply { yes: true },
        ["hardware"] => Command::Hardware,
        ["help" | "--help" | "-h"] => Command::Help,
        _ => return Err(format!("unknown command: {}\n{USAGE}", words.join(" "))),
    })
}

pub struct Env<'a> {
    pub t: &'a mut dyn Terminal,
    pub r: &'a mut dyn Runner,
    pub dirs: &'a Dirs,
    /// This checkout's `omahoy` bootstrap, when running from one.
    pub own: Option<PathBuf>,
}

pub fn run(cmd: Command, e: &mut Env<'_>) -> Result<i32, String> {
    match cmd {
        Command::Help => {
            e.t.say(USAGE);
            Ok(0)
        }
        Command::Doctor => doctor(e),
        Command::Apply { yes } => {
            let boat = required(e.dirs)?;
            converge(e, &boat, yes)
        }
        Command::Setup => {
            let existing = Boat::load(e.dirs)?;
            let omakeel = hub_ready(e)?;
            let mut ask = Ask {
                t: &mut *e.t,
                r: &mut *e.r,
                dirs: e.dirs,
                omakeel,
            };
            let boat = match &existing {
                Some(b) => ask.revise(b)?,
                None => ask.first_run(Partial::from_existing(e.dirs))?,
            };
            save(e, &boat)?;
            converge(e, &boat, false)
        }
        Command::Hardware => {
            let boat = required(e.dirs)?;
            let omakeel = hub_ready(e)?;
            let boat = Ask {
                t: &mut *e.t,
                r: &mut *e.r,
                dirs: e.dirs,
                omakeel,
            }
            .hardware_only(&boat)?;
            save(e, &boat)?;
            converge(e, &boat, false)
        }
    }
}

fn required(dirs: &Dirs) -> Result<Boat, String> {
    Boat::load(dirs)?
        .ok_or_else(|| format!("No {} yet. Run omahoy to set up.", dirs.show(&dirs.boat())))
}

fn save(e: &mut Env<'_>, boat: &Boat) -> Result<(), String> {
    boat.save(e.dirs)?;
    e.t.say(&format!("\nSaved {}.", e.dirs.show(&e.dirs.boat())));
    Ok(())
}

/// The probe lives in omakeel, so omakeel is fetched and built before the
/// hardware question, when it isn't already. A stale build is rebuilt too:
/// a binary from before `probe` existed would fail the question.
fn hub_ready(e: &mut Env<'_>) -> Result<PathBuf, String> {
    let (dir, build) = match find(e.r, e.dirs, App::Keel) {
        OnDisk::Checkout(c) => (c.dir, c.build),
        OnDisk::Absent => (Machine::home_of(e.dirs, App::Keel), None),
    };
    let bin = dir.join("target/release/omakeel");
    let mut steps = Vec::new();
    if !dir.exists() {
        steps.push(Step::Get {
            app: App::Keel,
            dir: dir.clone(),
        });
    }
    if build != Some(Build::Fresh) {
        steps.push(Step::Build {
            app: App::Keel,
            dir: dir.clone(),
        });
    }
    if !steps.is_empty() {
        e.t.say("Getting the hub ready first, so setup can listen for your GPS. This builds omakeel once (a minute or two).");
        for s in &steps {
            e.t.say(&format!("→ {}", s.describe(e.dirs)));
            s.run(e.r, e.dirs)?;
        }
        e.t.say("");
    }
    Ok(bin)
}

fn converge(e: &mut Env<'_>, boat: &Boat, yes: bool) -> Result<i32, String> {
    let m = Machine::survey(e.r, e.dirs, boat, e.own.as_deref());
    let steps = plan(boat, &m);
    if steps.is_empty() {
        e.t.say("Everything matches boat.toml.");
        let found = findings(boat, &m);
        needs_you(e.t, &found);
        return Ok(i32::from(!found.is_empty()));
    }
    e.t.say("Here is what setup will do:");
    for s in &steps {
        e.t.say(&format!("  {}", s.describe(e.dirs)));
    }
    if !yes && !confirm(e.t, "Go?", true) {
        e.t.say("Stopped before applying. Run omahoy apply when you're ready.");
        return Ok(0);
    }
    for s in &steps {
        let line = s.describe(e.dirs);
        e.t.say(&format!("→ {}", line.lines().next().unwrap_or("")));
        s.run(e.r, e.dirs).map_err(|err| {
            format!(
                "{err}\nThe steps before this one are done. Put it right, then run omahoy apply."
            )
        })?;
    }
    let after = Machine::survey(e.r, e.dirs, boat, e.own.as_deref());
    let left = plan(boat, &after);
    let found = findings(boat, &after);
    if left.is_empty() && found.is_empty() {
        e.t.say("\nDone.");
    } else if left.is_empty() {
        e.t.say("\nApplied.");
    } else {
        e.t.say("\nNot finished. Still to do:");
        for s in &left {
            e.t.say(&format!("  {}", s.describe(e.dirs)));
        }
    }
    if left.is_empty() && boat.apps.has(App::Helm) {
        e.t.say("Open the chart: omarchy-shell shell toggle org.omahoy.helm {}");
    }
    needs_you(e.t, &found);
    if left.is_empty() {
        e.t.say("Run omahoy doctor any time to check on things.");
    } else {
        e.t.say("Run omahoy apply to try again, or omahoy doctor to see why.");
    }
    Ok(i32::from(!(left.is_empty() && found.is_empty())))
}

fn needs_you(t: &mut dyn Terminal, found: &[String]) {
    if found.is_empty() {
        return;
    }
    t.say("\nNeeds you:");
    for f in found {
        t.say(&format!("  - {f}"));
    }
}

fn doctor(e: &mut Env<'_>) -> Result<i32, String> {
    let boat = required(e.dirs)?;
    let m = Machine::survey(e.r, e.dirs, &boat, e.own.as_deref());
    let steps = plan(&boat, &m);
    let found = findings(&boat, &m);
    e.t.say(&format!(
        "Checking this machine against {} ({}).",
        e.dirs.show(&e.dirs.boat()),
        boat.name
    ));
    if steps.is_empty() && found.is_empty() {
        e.t.say("Everything matches boat.toml.");
        return Ok(0);
    }
    if !steps.is_empty() {
        e.t.say("\nomahoy apply would:");
        for s in &steps {
            e.t.say(&format!("  {}", s.describe(e.dirs)));
        }
    }
    needs_you(e.t, &found);
    Ok(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::sys::fake::{Commands, Script};

    fn tmp_dirs(name: &str) -> Dirs {
        let tmp = std::env::temp_dir().join(format!("omahoy-cli-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        Dirs::resolve(Some(tmp.into()), None, None, None, Some("sailor".into())).unwrap()
    }

    #[test]
    fn a_stale_omakeel_is_rebuilt_before_the_probe() {
        let dirs = tmp_dirs("stale-keel");
        let keel = Machine::home_of(&dirs, App::Keel);
        std::fs::create_dir_all(keel.join("target/release")).unwrap();
        std::fs::write(keel.join("target/release/omakeel"), "").unwrap();
        let mut r = Commands::default()
            .on("git rev-parse HEAD", Ok("abc\n"))
            .on(&format!("mise trust --quiet {}", keel.display()), Ok(""))
            .on("mise install", Ok(""))
            .on("mise exec -- cargo build --release --locked", Ok(""));
        let mut t = Script::new(&[]);
        let bin = hub_ready(&mut Env {
            t: &mut t,
            r: &mut r,
            dirs: &dirs,
            own: None,
        });
        let _ = std::fs::remove_dir_all(&dirs.home);
        assert_eq!(bin, Ok(keel.join("target/release/omakeel")));
        assert!(
            r.log
                .contains(&"mise exec -- cargo build --release --locked".to_string()),
            "{:?}",
            r.log
        );
    }

    /// A tmp home with only omakeel on it, built and linked, for a boat
    /// that has the demo hub and no apps. Everything matches unless the
    /// unit is stopped.
    fn keel_only(name: &str, active: bool, plugins_listed: bool) -> (Dirs, Boat, Commands) {
        let dirs = tmp_dirs(name);
        let mut boat = crate::boat::tests::wavelength();
        boat.hub = crate::hub::Hub::Demo;
        boat.apps = crate::boat::Apps::choose(&[], boat.area).unwrap();
        let keel = Machine::home_of(&dirs, App::Keel);
        let bin = keel.join("target/release/omakeel");
        std::fs::create_dir_all(bin.parent().unwrap()).unwrap();
        std::fs::create_dir_all(keel.join("dist")).unwrap();
        std::fs::write(&bin, "").unwrap();
        std::fs::write(keel.join(crate::machine::BUILT), "abc\n").unwrap();
        std::fs::write(keel.join("dist/omakeel.service"), "").unwrap();
        std::fs::create_dir_all(&dirs.bin).unwrap();
        std::fs::create_dir_all(&dirs.units).unwrap();
        std::os::unix::fs::symlink(&bin, dirs.bin.join("omakeel")).unwrap();
        std::os::unix::fs::symlink(
            keel.join("dist/omakeel.service"),
            dirs.units.join("omakeel.service"),
        )
        .unwrap();
        let conf = App::Keel.config(&dirs).unwrap();
        std::fs::create_dir_all(conf.parent().unwrap()).unwrap();
        std::fs::write(
            &conf,
            crate::conf::edit("", &crate::apps::managed(App::Keel, &boat)),
        )
        .unwrap();
        let mut r = Commands::default()
            .on("pacman -Q git", Ok(""))
            .on("git rev-parse HEAD", Ok("abc\n"))
            .on("systemctl --user is-enabled omakeel.service", Ok(""));
        if plugins_listed {
            r = r.on("omarchy plugin list --json", Ok("[]"));
        }
        if active {
            r = r
                .on("systemctl --user is-active omakeel.service", Ok(""))
                .on(
                    "systemctl --user --timestamp=unix show -p ActiveEnterTimestamp --value omakeel.service",
                    Ok("@99999999999\n"),
                )
                .on(&format!("{} watch --once", bin.display()), Ok("fix none\n"));
        }
        (
            dirs,
            boat,
            r.on("systemctl --user enable --now omakeel.service", Ok("")),
        )
    }

    #[test]
    fn apply_that_leaves_steps_undone_says_so_and_exits_1() {
        let (dirs, boat, mut r) = keel_only("undone", false, true);
        let mut t = Script::new(&[]);
        let code = converge(
            &mut Env {
                t: &mut t,
                r: &mut r,
                dirs: &dirs,
                own: None,
            },
            &boat,
            true,
        );
        let _ = std::fs::remove_dir_all(&dirs.home);
        assert_eq!(code, Ok(1));
        assert!(
            t.shown.contains(
                "→ start omakeel.service now and at every login\n\nNot finished. Still to do:\n  start omakeel.service now and at every login\n"
            ),
            "{}",
            t.shown
        );
        assert!(!t.shown.contains("Done."));
    }

    #[test]
    fn a_machine_that_matches_exits_0_and_one_with_findings_exits_1() {
        let (dirs, boat, mut r) = keel_only("matches", true, true);
        let mut t = Script::new(&[]);
        let mut env = Env {
            t: &mut t,
            r: &mut r,
            dirs: &dirs,
            own: None,
        };
        assert_eq!(converge(&mut env, &boat, true), Ok(0));
        assert!(t.shown.contains("Everything matches boat.toml."));

        let (dirs2, boat2, mut r2) = keel_only("findings", true, false);
        let mut t2 = Script::new(&[]);
        let code = converge(
            &mut Env {
                t: &mut t2,
                r: &mut r2,
                dirs: &dirs2,
                own: None,
            },
            &boat2,
            true,
        );
        let _ = std::fs::remove_dir_all(&dirs.home);
        let _ = std::fs::remove_dir_all(&dirs2.home);
        assert_eq!(code, Ok(1));
        assert!(t2.shown.contains("Needs you:"), "{}", t2.shown);
    }

    #[test]
    fn arguments() {
        let p = |a: &[&str]| parse_args(&a.iter().map(|s| (*s).to_string()).collect::<Vec<_>>());
        assert_eq!(p(&[]), Ok(Command::Setup));
        assert_eq!(p(&["doctor"]), Ok(Command::Doctor));
        assert_eq!(p(&["apply", "--yes"]), Ok(Command::Apply { yes: true }));
        assert_eq!(p(&["hardware"]), Ok(Command::Hardware));
        assert!(p(&["apply", "--force"]).is_err());
    }
}
