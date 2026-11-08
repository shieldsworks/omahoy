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
/// hardware question, when it isn't already.
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
    if !matches!(build, Some(Build::Fresh | Build::Stale)) {
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
        needs_you(e.t, &findings(boat, &m));
        return Ok(0);
    }
    e.t.say("Here is what setup will do:");
    for s in &steps {
        e.t.say(&format!("  {}", s.describe(e.dirs)));
    }
    if !yes && !confirm(e.t, "Go?", true) {
        e.t.say("Nothing changed. Run omahoy apply when you're ready.");
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
    e.t.say("\nDone.");
    if boat.apps.has(App::Helm) {
        e.t.say("Open the chart: omarchy-shell shell toggle org.omahoy.helm {}");
    }
    e.t.say("Run omahoy doctor any time to check on things.");
    let after = Machine::survey(e.r, e.dirs, boat, e.own.as_deref());
    needs_you(e.t, &findings(boat, &after));
    Ok(0)
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
