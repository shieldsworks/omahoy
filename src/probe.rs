//! Listening for hardware with `omakeel probe`, for the terminal's hardware
//! question and for `omahoy gui probe`.

use crate::hub::{Heard, parse_probe};
use crate::sys::{Cmd, Runner, Terminal};
use std::path::Path;

/// Runs `omakeel probe`. A probe that fails to run is an error, never an
/// empty scan: an omakeel from before `probe` existed would otherwise
/// read as "no hardware" and set the hub to the demo.
pub fn probe(
    r: &mut dyn Runner,
    omakeel: &Path,
    targets: &[String],
    via_uucp: bool,
) -> Result<Vec<Result<Heard, String>>, String> {
    let bin = omakeel.display().to_string();
    let cmd = if via_uucp {
        let mut inner = format!("{} probe", shell_quote(&bin));
        for t in targets {
            inner += &format!(" {}", shell_quote(t));
        }
        Cmd::new("sg", &["uucp", "-c", &inner])
    } else {
        Cmd::new(bin, &["probe"]).args(targets.iter().cloned())
    };
    let out = r
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

/// omakeel.service holds the ports it reads, so it is stopped while the
/// probe listens and started again after.
pub fn probe_with_hub_stopped(
    r: &mut dyn Runner,
    t: &mut dyn Terminal,
    omakeel: &Path,
    targets: &[String],
    via_uucp: bool,
) -> Result<Vec<Result<Heard, String>>, String> {
    let unit = ["--user", "is-active", "--quiet", "omakeel.service"];
    let running = r.read(&Cmd::new("systemctl", &unit)).is_ok();
    let mut stopped = Ok(());
    if running {
        t.say("  Stopping the hub while setup listens to the ports.");
        stopped = r.run(&Cmd::new(
            "systemctl",
            &["--user", "stop", "omakeel.service"],
        ));
    }
    let heard = match stopped {
        Ok(()) => probe(r, omakeel, targets, via_uucp),
        Err(e) => Err(format!(
            "couldn't stop omakeel.service to listen to the ports: {e}"
        )),
    };
    if !running {
        return heard;
    }
    let started = r.run(&Cmd::new(
        "systemctl",
        &["--user", "start", "omakeel.service"],
    ));
    match (heard, started) {
        (heard, Ok(())) => heard,
        (Ok(_), Err(e)) => Err(restart_failed(&e)),
        (Err(probe_err), Err(e)) => Err(format!("{probe_err}\nAlso {}", restart_failed(&e))),
    }
}

/// The hub was stopped for the probe, so GPS and AIS stay offline until it
/// starts.
fn restart_failed(e: &str) -> String {
    format!("couldn't start omakeel.service again ({e}); GPS and AIS stay offline until it runs")
}

/// One word for `sh -c`, in single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::fake::{Commands, Script};

    const PROBE: &str = "/k/omakeel probe";
    const ACTIVE: &str = "systemctl --user is-active --quiet omakeel.service";
    const STOP: &str = "systemctl --user stop omakeel.service";
    const START: &str = "systemctl --user start omakeel.service";

    fn go(r: &mut Commands) -> Result<Vec<Result<Heard, String>>, String> {
        let mut t = Script::new(&[]);
        probe_with_hub_stopped(r, &mut t, Path::new("/k/omakeel"), &[], false)
    }

    #[test]
    fn a_hub_that_will_not_start_again_fails_the_probe() {
        let mut r = Commands::default()
            .on(ACTIVE, Ok(""))
            .on(STOP, Ok(""))
            .on(PROBE, Ok("serial:/dev/ttyUSB0  busy\n"))
            .on(START, Err("unit is masked"));
        let err = go(&mut r).unwrap_err();
        assert!(
            err.contains("couldn't start omakeel.service again"),
            "{err}"
        );
        assert!(err.contains("unit is masked"), "{err}");
    }

    #[test]
    fn both_failures_are_named_when_the_probe_and_the_restart_fail() {
        let mut r = Commands::default()
            .on(ACTIVE, Ok(""))
            .on(STOP, Ok(""))
            .on(PROBE, Err("exit 2"))
            .on(START, Err("unit is masked"));
        let err = go(&mut r).unwrap_err();
        assert!(err.contains("couldn't run omakeel probe"), "{err}");
        assert!(
            err.contains("couldn't start omakeel.service again"),
            "{err}"
        );
    }

    #[test]
    fn a_hub_that_will_not_stop_is_not_probed_and_is_started_again() {
        let mut r = Commands::default()
            .on(ACTIVE, Ok(""))
            .on(STOP, Err("busy"))
            .on(START, Err("unit is masked"));
        let err = go(&mut r).unwrap_err();
        assert!(err.contains("couldn't stop omakeel.service"), "{err}");
        assert!(
            err.contains("couldn't start omakeel.service again"),
            "{err}"
        );
        assert_eq!(r.log, vec![ACTIVE, STOP, START]);
        let mut r = Commands::default()
            .on(ACTIVE, Ok(""))
            .on(STOP, Err("busy"))
            .on(START, Ok(""));
        assert!(go(&mut r).unwrap_err().contains("busy"));
        assert!(!r.log.iter().any(|l| l == PROBE));
    }

    #[test]
    fn an_inactive_hub_is_never_stopped_or_started() {
        let mut r = Commands::default()
            .on(ACTIVE, Err("inactive"))
            .on(PROBE, Ok("serial:/dev/ttyUSB0  busy\n"));
        assert!(go(&mut r).is_ok());
        assert_eq!(r.log, vec![ACTIVE, PROBE]);
    }
}
