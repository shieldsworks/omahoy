//! `omahoy panel`: the graphical setup in this checkout's `panel/`, linked
//! into the Omarchy shell as the plugin `org.omahoy.setup`, enabled, and
//! opened with the path of this binary, which the panel runs for every
//! `omahoy gui` call (docs/gui.md).

use crate::cli::Env;
use crate::json::Json;
use crate::machine::plugin_states;
use crate::sys::{Cmd, Runner, symlink_atomic};
use std::collections::BTreeMap;
use std::path::Path;

pub const ID: &str = "org.omahoy.setup";

/// Links, enables and opens the panel. `exe` is this binary; the panel
/// gets it in its open payload, because `omahoy` need not be on PATH yet
/// on a first run.
pub fn open(e: &mut Env<'_>, exe: &Path) -> Result<i32, String> {
    let Some(root) = e.own.as_deref().and_then(Path::parent) else {
        return Err(
            "omahoy panel opens the panel of an omahoy checkout. Run it from one: ./omahoy panel"
                .into(),
        );
    };
    let panel = root.join("panel");
    if !panel.join("manifest.json").is_file() {
        return Err(format!("{} has no manifest.json", panel.display()));
    }
    let at = e.dirs.plugins.join(ID);
    let made = link(&panel, &at)?;
    if made {
        e.t.say(&format!(
            "Linked {} -> {}",
            e.dirs.show(&at),
            e.dirs.show(&panel)
        ));
    }

    // The shell finds a new plugin folder only when it rescans; a rescan
    // also reloads every panel, so it runs only when the shell doesn't
    // list this one.
    let mut states = listed(e.r);
    if made || !states.as_ref().is_some_and(|s| s.contains_key(ID)) {
        e.r.read(&Cmd::new("omarchy-shell", &["shell", "rescanPlugins"]))
            .map_err(|err| format!("{err}\n{}", shell_down()))?;
        for _ in 0..40 {
            states = listed(e.r);
            if states.as_ref().is_some_and(|s| s.contains_key(ID)) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }
    match states.as_ref().and_then(|s| s.get(ID).copied()) {
        None => return Err(shell_down()),
        Some(false) => {
            e.r.read(&Cmd::new("omarchy", &["plugin", "enable", ID]))?;
            e.t.say(&format!("Enabled {ID}"));
        }
        Some(true) => {}
    }

    let payload = Json::obj(vec![("bin", Json::Str(exe.display().to_string()))]).render();
    let said = e.r.read(&Cmd::new(
        "omarchy-shell",
        &["shell", "summon", ID, &payload],
    ))?;
    if said.trim() != "ok" {
        return Err(format!(
            "The Omarchy shell would not open {ID}: {}",
            said.trim()
        ));
    }
    e.t.say("The setup panel is open. Escape closes it; omahoy panel opens it again.");
    Ok(0)
}

fn shell_down() -> String {
    format!(
        "The Omarchy shell doesn't list {ID}. Start the shell (omarchy restart shell), then run omahoy panel again."
    )
}

fn listed(r: &mut dyn Runner) -> Option<BTreeMap<String, bool>> {
    r.read(&Cmd::new("omarchy", &["plugin", "list", "--json"]))
        .ok()
        .and_then(|json| plugin_states(&json))
}

/// Points `at` at `panel`: true when it made the link, false when the link
/// was already there. Anything else at `at` is someone else's, and stays.
fn link(panel: &Path, at: &Path) -> Result<bool, String> {
    match std::fs::symlink_metadata(at) {
        Ok(m) if m.file_type().is_symlink() => {
            let real = std::fs::canonicalize(at).ok();
            if real.is_some() && real == std::fs::canonicalize(panel).ok() {
                return Ok(false);
            }
            let to = std::fs::read_link(at)
                .map(|p| p.display().to_string())
                .unwrap_or_default();
            Err(format!(
                "{} links to {to}, not to {}. Remove that link, then run omahoy panel again.",
                at.display(),
                panel.display()
            ))
        }
        Ok(_) => Err(format!(
            "{} is already there and is not a link. Move it aside, then run omahoy panel again.",
            at.display()
        )),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            symlink_atomic(panel, at)?;
            Ok(true)
        }
        Err(err) => Err(format!("{}: {err}", at.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sys::Dirs;
    use crate::sys::fake::{Commands, Script};
    use std::path::PathBuf;

    /// A temp home and a checkout with a panel, as `own_bootstrap` finds it.
    fn checkout(name: &str) -> (Dirs, PathBuf) {
        let tmp = std::env::temp_dir().join(format!("omahoy-panel-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let root = tmp.join("src/omahoy");
        std::fs::create_dir_all(root.join("panel")).unwrap();
        std::fs::write(root.join("panel/manifest.json"), "{}").unwrap();
        std::fs::write(root.join("omahoy"), "").unwrap();
        let dirs = Dirs::resolve(
            Some(tmp.join("home").into()),
            None,
            None,
            None,
            Some("sailor".into()),
        )
        .unwrap();
        (dirs, root)
    }

    fn go(dirs: &Dirs, root: &Path, r: &mut Commands) -> (Result<i32, String>, String) {
        let mut t = Script::new(&[]);
        let code = open(
            &mut Env {
                t: &mut t,
                r,
                dirs,
                own: Some(root.join("omahoy")),
            },
            Path::new("/c/omahoy/target/release/omahoy"),
        );
        (code, t.shown)
    }

    const SUMMON: &str = r#"omarchy-shell shell summon org.omahoy.setup '{"bin":"/c/omahoy/target/release/omahoy"}'"#;

    #[test]
    fn a_first_run_links_rescans_enables_and_opens_with_this_binary() {
        let (dirs, root) = checkout("first");
        let mut r = Commands::default()
            .on(
                "omarchy plugin list --json",
                Ok(r#"[{"id":"org.omahoy.setup","enabled":false}]"#),
            )
            .on("omarchy-shell shell rescanPlugins", Ok(""))
            .on("omarchy plugin enable org.omahoy.setup", Ok("Enabled\n"))
            .on(SUMMON, Ok("ok\n"));
        let (code, shown) = go(&dirs, &root, &mut r);
        let at = dirs.plugins.join(ID);
        let linked = std::fs::canonicalize(&at).ok();
        let _ = std::fs::remove_dir_all(dirs.home.parent().unwrap());
        assert_eq!(code, Ok(0), "{shown}");
        assert_eq!(linked, Some(root.join("panel")));
        assert_eq!(
            r.log,
            vec![
                "omarchy plugin list --json",
                "omarchy-shell shell rescanPlugins",
                "omarchy plugin list --json",
                "omarchy plugin enable org.omahoy.setup",
                SUMMON,
            ]
        );
        assert!(shown.contains("Linked ~/.config/omarchy/plugins/org.omahoy.setup -> "));
    }

    #[test]
    fn an_existing_link_to_this_panel_only_opens_it() {
        let (dirs, root) = checkout("again");
        std::fs::create_dir_all(&dirs.plugins).unwrap();
        std::os::unix::fs::symlink(root.join("panel"), dirs.plugins.join(ID)).unwrap();
        let mut r = Commands::default()
            .on(
                "omarchy plugin list --json",
                Ok(r#"[{"id":"org.omahoy.setup","enabled":true}]"#),
            )
            .on(SUMMON, Ok("ok\n"));
        let (code, shown) = go(&dirs, &root, &mut r);
        let _ = std::fs::remove_dir_all(dirs.home.parent().unwrap());
        assert_eq!(code, Ok(0));
        assert_eq!(r.log, vec!["omarchy plugin list --json", SUMMON]);
        assert!(!shown.contains("Linked"));
    }

    #[test]
    fn a_folder_or_a_link_elsewhere_is_left_alone() {
        let (dirs, root) = checkout("taken");
        let at = dirs.plugins.join(ID);
        std::fs::create_dir_all(&at).unwrap();
        let mut r = Commands::default();
        let (folder, _) = go(&dirs, &root, &mut r);
        std::fs::remove_dir(&at).unwrap();
        std::os::unix::fs::symlink("/elsewhere", &at).unwrap();
        let (elsewhere, _) = go(&dirs, &root, &mut r);
        let still = std::fs::read_link(&at).ok();
        let _ = std::fs::remove_dir_all(dirs.home.parent().unwrap());
        assert!(folder.unwrap_err().contains("is not a link"));
        assert!(elsewhere.unwrap_err().contains("links to /elsewhere"));
        assert_eq!(still, Some(PathBuf::from("/elsewhere")));
        assert!(r.log.is_empty(), "{:?}", r.log);
    }

    #[test]
    fn no_checkout_and_no_shell_are_errors_that_say_what_to_do() {
        let (dirs, root) = checkout("noshell");
        let mut t = Script::new(&[]);
        let mut r = Commands::default();
        let outside = open(
            &mut Env {
                t: &mut t,
                r: &mut r,
                dirs: &dirs,
                own: None,
            },
            Path::new("/x/omahoy"),
        );
        let mut r = Commands::default().on("omarchy-shell shell rescanPlugins", Err("not running"));
        let (down, _) = go(&dirs, &root, &mut r);
        let _ = std::fs::remove_dir_all(dirs.home.parent().unwrap());
        assert!(outside.unwrap_err().contains("./omahoy panel"));
        assert!(down.unwrap_err().contains("Start the shell"));
    }
}
