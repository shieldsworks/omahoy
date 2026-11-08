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
    if running {
        t.say("  Stopping the hub while setup listens to the ports.");
        let _ = r.run(&Cmd::new(
            "systemctl",
            &["--user", "stop", "omakeel.service"],
        ));
    }
    let heard = probe(r, omakeel, targets, via_uucp);
    if running {
        let _ = r.run(&Cmd::new(
            "systemctl",
            &["--user", "start", "omakeel.service"],
        ));
    }
    heard
}

/// One word for `sh -c`, in single quotes.
fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}
