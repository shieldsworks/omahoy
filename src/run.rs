//! `Step::run`: the only place omahoy changes the machine. Each step
//! re-reads what it touches, so running one that is already true is
//! harmless.

use crate::conf;
use crate::machine::{BUILT, Fetched, head_of};
use crate::plan::Step;
use crate::sys::{Cmd, Dirs, Runner, symlink_atomic, write_atomic};
use std::path::Path;

/// Refuses to replace anything at `link` that isn't a link.
fn link_over(target: &Path, link: &Path) -> Result<(), String> {
    if let Ok(m) = std::fs::symlink_metadata(link)
        && !m.file_type().is_symlink()
    {
        return Err(format!(
            "{} is a file, not a link; setup never replaces it",
            link.display()
        ));
    }
    symlink_atomic(target, link)
}

impl Step {
    pub fn run(&self, r: &mut dyn Runner, dirs: &Dirs) -> Result<(), String> {
        let systemctl = |verb: &[&str], unit: &str| {
            Cmd::new("systemctl", &["--user"])
                .args(verb.iter().map(|v| (*v).to_string()))
                .arg(unit)
        };
        match self {
            Step::Packages(p) => r.run(&Cmd::new("omarchy", &["pkg", "add"]).args(p.clone())),
            Step::JoinUucp(user) => r.run(&Cmd::new("sudo", &["gpasswd", "-a", user, "uucp"])),
            Step::Get { app, dir } => {
                if dir.exists() {
                    return Ok(());
                }
                match app.plugin_id() {
                    Some(_) => r.run(&Cmd::new(
                        "omarchy",
                        &["plugin", "add", &app.repo(), "--yes"],
                    )),
                    None => {
                        if let Some(parent) = dir.parent() {
                            std::fs::create_dir_all(parent)
                                .map_err(|e| format!("{}: {e}", parent.display()))?;
                        }
                        r.run(&Cmd::new("git", &["clone", &app.repo()]).path_arg(dir))
                    }
                }
            }
            Step::Build { dir, .. } => {
                let head = head_of(r, dir);
                r.run(&Cmd::new("mise", &["trust", "--quiet"]).path_arg(dir))?;
                r.run(&Cmd::new("mise", &["install"]).in_dir(dir))?;
                r.run(
                    &Cmd::new(
                        "mise",
                        &["exec", "--", "cargo", "build", "--release", "--locked"],
                    )
                    .in_dir(dir),
                )?;
                match head {
                    Some(h) => write_atomic(&dir.join(BUILT), &format!("{h}\n")),
                    None => Ok(()),
                }
            }
            Step::Link { link, target } => link_over(target, link),
            Step::EnablePlugin(app) => match app.plugin_id() {
                Some(id) => r.run(&Cmd::new("omarchy", &["plugin", "enable", id])),
                None => Ok(()),
            },
            Step::Write { path, want, .. } => {
                let text = match std::fs::read_to_string(path) {
                    Ok(t) => t,
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
                    Err(e) => return Err(format!("{}: {e}", path.display())),
                };
                let new = conf::edit(&text, want);
                if new == text {
                    return Ok(());
                }
                write_atomic(path, &new)
            }
            Step::InstallUnit { app, link, target } => {
                if !target.is_file() {
                    return Err(format!(
                        "{} is missing; update {}'s checkout",
                        target.display(),
                        app.name()
                    ));
                }
                link_over(target, link)?;
                r.run(&Cmd::new("systemctl", &["--user", "daemon-reload"]))
            }
            Step::Start(app) => match app.unit() {
                Some(unit) => r.run(&systemctl(&["enable", "--now"], unit)),
                None => Ok(()),
            },
            Step::Restart(app) => match app.unit() {
                Some(unit) => r.run(&systemctl(&["restart"], unit)),
                None => Ok(()),
            },
            Step::InitVault(dir) => {
                std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
                if dir.join(".git").exists() {
                    return Ok(());
                }
                r.run(&Cmd::new("git", &["init", "--quiet"]).in_dir(dir))
            }
            Step::Fetch { kind, area, bin } => {
                let cmd = match kind {
                    crate::machine::Kind::Charts => {
                        Cmd::new(bin.display().to_string(), &["fetch", "--region"])
                            .arg(area.to_config())
                    }
                    crate::machine::Kind::Tides => Cmd::new(bin.display().to_string(), &["fetch"]),
                };
                r.run(&cmd)?;
                let path = dirs.fetched();
                let old = std::fs::read_to_string(&path)
                    .map(|t| Fetched::parse(&t))
                    .unwrap_or_default();
                write_atomic(&path, &old.with(*kind, *area).render())
            }
        }
    }
}
