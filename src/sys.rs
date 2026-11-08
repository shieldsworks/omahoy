//! The edges: commands, the terminal, and where things live on disk.
//! Everything that touches the system goes through `Runner` and
//! `Terminal`, so the plan and the questions are tested against fakes.

use std::ffi::OsString;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// One command line, with the directory it runs in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cmd {
    pub program: String,
    pub args: Vec<String>,
    pub dir: Option<PathBuf>,
}

impl Cmd {
    pub fn new(program: impl Into<String>, args: &[&str]) -> Cmd {
        Cmd {
            program: program.into(),
            args: args.iter().map(|a| (*a).to_string()).collect(),
            dir: None,
        }
    }

    pub fn arg(mut self, a: impl Into<String>) -> Cmd {
        self.args.push(a.into());
        self
    }

    pub fn args(mut self, a: impl IntoIterator<Item = String>) -> Cmd {
        self.args.extend(a);
        self
    }

    pub fn path_arg(self, p: &Path) -> Cmd {
        self.arg(p.display().to_string())
    }

    pub fn in_dir(mut self, dir: &Path) -> Cmd {
        self.dir = Some(dir.to_path_buf());
        self
    }

    /// The command as a shell would show it, for plans and errors.
    pub fn line(&self) -> String {
        let mut out = self.program.clone();
        for a in &self.args {
            out.push(' ');
            if a.is_empty() || a.contains([' ', '\'', '"', '$']) {
                out.push('\'');
                out.push_str(&a.replace('\'', r"'\''"));
                out.push('\'');
            } else {
                out.push_str(a);
            }
        }
        out
    }
}

pub trait Runner {
    /// Runs quietly and returns stdout. A non-zero exit is an error that
    /// carries the last line of stderr.
    fn read(&mut self, cmd: &Cmd) -> Result<String, String>;
    /// Runs with the terminal attached, so the sailor sees its progress
    /// and can answer sudo.
    fn run(&mut self, cmd: &Cmd) -> Result<(), String>;
}

pub struct System;

impl System {
    fn command(cmd: &Cmd) -> Command {
        let mut c = Command::new(&cmd.program);
        c.args(&cmd.args);
        if let Some(d) = &cmd.dir {
            c.current_dir(d);
        }
        c
    }
}

impl Runner for System {
    fn read(&mut self, cmd: &Cmd) -> Result<String, String> {
        let out = System::command(cmd)
            .stdin(Stdio::null())
            .output()
            .map_err(|e| format!("{}: {e}", cmd.program))?;
        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let last = err
                .lines()
                .rev()
                .find(|l| !l.trim().is_empty())
                .unwrap_or("");
            let why = match out.status.code() {
                Some(code) => format!("exit {code}"),
                None => "killed".into(),
            };
            match last {
                "" => Err(format!("{} failed ({why})", cmd.line())),
                _ => Err(format!("{} failed ({why}): {last}", cmd.line())),
            }
        }
    }

    fn run(&mut self, cmd: &Cmd) -> Result<(), String> {
        let status = System::command(cmd)
            .status()
            .map_err(|e| format!("{}: {e}", cmd.program))?;
        if status.success() {
            Ok(())
        } else {
            Err(format!("{} exited {status}", cmd.line()))
        }
    }
}

pub trait Terminal {
    fn say(&mut self, text: &str);
    /// One line from the sailor, trimmed. None at end of input.
    fn ask(&mut self, prompt: &str) -> Option<String>;
}

pub struct Console;

impl Terminal for Console {
    fn say(&mut self, text: &str) {
        println!("{text}");
    }

    fn ask(&mut self, prompt: &str) -> Option<String> {
        if !prompt.is_empty() {
            print!("{prompt} ");
            io::stdout().flush().ok()?;
        }
        let mut line = String::new();
        match io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim().to_string()),
        }
    }
}

/// Yes or no, with the default taken on a bare Enter. End of input is no.
pub fn confirm(t: &mut dyn Terminal, prompt: &str, default: bool) -> bool {
    let hint = if default { "[Y/n]" } else { "[y/N]" };
    loop {
        let Some(a) = t.ask(&format!("{prompt} {hint}")) else {
            return false;
        };
        match a.to_ascii_lowercase().as_str() {
            "" => return default,
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => t.say("  Please answer y or n."),
        }
    }
}

/// Where everything lives. Built once from the environment; the rest of
/// the program never reads an environment variable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dirs {
    pub home: PathBuf,
    /// `$XDG_CONFIG_HOME`, where every app keeps its config.toml.
    pub config: PathBuf,
    /// `$XDG_DATA_HOME`: omahelm's charts, omatide's stations, omakeel's clone.
    pub data: PathBuf,
    /// `$XDG_STATE_HOME`: the fetch record.
    pub state: PathBuf,
    pub bin: PathBuf,
    /// The shell scans ~/.config/omarchy/plugins whatever XDG_CONFIG_HOME says.
    pub plugins: PathBuf,
    pub units: PathBuf,
    pub user: String,
}

impl Dirs {
    pub fn from_env() -> Result<Dirs, String> {
        let var = std::env::var_os;
        Dirs::resolve(
            var("HOME"),
            var("XDG_CONFIG_HOME"),
            var("XDG_DATA_HOME"),
            var("XDG_STATE_HOME"),
            var("USER"),
        )
    }

    /// An XDG value counts only when it is absolute, as in every app.
    pub fn resolve(
        home: Option<OsString>,
        config: Option<OsString>,
        data: Option<OsString>,
        state: Option<OsString>,
        user: Option<OsString>,
    ) -> Result<Dirs, String> {
        let home = home
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .ok_or("HOME must be set to an absolute path")?;
        let xdg = |v: Option<OsString>, rel: &str| {
            v.map(PathBuf::from)
                .filter(|p| p.is_absolute())
                .unwrap_or_else(|| home.join(rel))
        };
        let config = xdg(config, ".config");
        Ok(Dirs {
            data: xdg(data, ".local/share"),
            state: xdg(state, ".local/state"),
            bin: home.join(".local/bin"),
            plugins: home.join(".config/omarchy/plugins"),
            units: config.join("systemd/user"),
            user: user
                .and_then(|u| u.into_string().ok())
                .filter(|u| !u.is_empty())
                .unwrap_or_else(|| "you".into()),
            config,
            home,
        })
    }

    pub fn boat(&self) -> PathBuf {
        self.config.join("omahoy/boat.toml")
    }

    pub fn fetched(&self) -> PathBuf {
        self.state.join("omahoy/fetched")
    }

    /// `~/x` for a path under home, for the sailor to read.
    pub fn show(&self, p: &Path) -> String {
        match p.strip_prefix(&self.home) {
            Ok(rest) => format!("~/{}", rest.display()),
            Err(_) => p.display().to_string(),
        }
    }

    /// `~/x` expanded against home; any other path as given.
    pub fn expand(&self, p: &str) -> PathBuf {
        match p.strip_prefix("~/") {
            Some(rest) => self.home.join(rest),
            None if p == "~" => self.home.clone(),
            None => PathBuf::from(p),
        }
    }
}

/// Writes through a temp file in the same directory and a rename, so a
/// reader never sees half a file. A path that is a symlink (a dotfiles
/// manager's) is written through to the file it points at, and the file
/// keeps its permissions.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let path = &through_links(path);
    let dir = path
        .parent()
        .ok_or_else(|| format!("{} has no directory", path.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.omahoy-{}", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
    if let Ok(old) = std::fs::metadata(path) {
        std::fs::set_permissions(&tmp, old.permissions())
            .map_err(|e| format!("{}: {e}", tmp.display()))?;
    }
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

/// The file a symlink chain ends at, even when that file doesn't exist
/// yet. A path that isn't a link comes back as it is.
fn through_links(path: &Path) -> PathBuf {
    let mut at = path.to_path_buf();
    for _ in 0..40 {
        let Ok(meta) = std::fs::symlink_metadata(&at) else {
            return at;
        };
        if !meta.file_type().is_symlink() {
            return at;
        }
        let Ok(to) = std::fs::read_link(&at) else {
            return at;
        };
        at = match at.parent() {
            Some(dir) => dir.join(to),
            None => to,
        };
    }
    at
}

/// Points `link` at `target`, through a temp link and a rename.
pub fn symlink_atomic(target: &Path, link: &Path) -> Result<(), String> {
    let dir = link
        .parent()
        .ok_or_else(|| format!("{} has no directory", link.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let name = link
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.omahoy-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    std::os::unix::fs::symlink(target, &tmp).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, link).map_err(|e| format!("{}: {e}", link.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_symlinked_config_is_written_through_and_keeps_its_mode() {
        let tmp = std::env::temp_dir().join(format!("omahoy-atomic-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(tmp.join("dotfiles")).unwrap();
        std::fs::create_dir_all(tmp.join("config")).unwrap();
        let real = tmp.join("dotfiles/config.toml");
        std::fs::write(&real, "old = 1\n").unwrap();
        std::fs::set_permissions(&real, std::fs::Permissions::from_mode(0o600)).unwrap();
        let link = tmp.join("config/config.toml");
        std::os::unix::fs::symlink("../dotfiles/config.toml", &link).unwrap();

        write_atomic(&link, "new = 2\n").unwrap();

        let still_link = std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink();
        let text = std::fs::read_to_string(&real).unwrap();
        let mode = std::fs::metadata(&real).unwrap().permissions().mode() & 0o777;
        let leftovers = std::fs::read_dir(tmp.join("config")).unwrap().count();
        std::fs::remove_dir_all(&tmp).unwrap();
        assert!(still_link, "the link stays a link");
        assert_eq!(text, "new = 2\n");
        assert_eq!(mode, 0o600);
        assert_eq!(leftovers, 1, "no temp file beside the link");
    }

    #[test]
    fn a_dangling_link_creates_the_file_it_names() {
        let tmp = std::env::temp_dir().join(format!("omahoy-dangle-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::os::unix::fs::symlink("made.toml", tmp.join("c.toml")).unwrap();
        write_atomic(&tmp.join("c.toml"), "x = 1\n").unwrap();
        let text = std::fs::read_to_string(tmp.join("made.toml")).ok();
        let link = std::fs::symlink_metadata(tmp.join("c.toml")).unwrap();
        std::fs::remove_dir_all(&tmp).unwrap();
        assert_eq!(text.as_deref(), Some("x = 1\n"));
        assert!(link.file_type().is_symlink());
    }

    #[test]
    fn a_plain_new_file_is_written_as_before() {
        let tmp = std::env::temp_dir().join(format!("omahoy-plain-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        write_atomic(&tmp.join("a/b.toml"), "k = 1\n").unwrap();
        let text = std::fs::read_to_string(tmp.join("a/b.toml")).ok();
        std::fs::remove_dir_all(&tmp).unwrap();
        assert_eq!(text.as_deref(), Some("k = 1\n"));
    }
}

#[cfg(test)]
pub mod fake {
    //! Fakes for the two edges. Commands answer from a script keyed by
    //! their command line; anything unscripted fails, so a test never
    //! reaches the real system by accident.
    use super::{Cmd, Runner, Terminal};
    use std::collections::VecDeque;

    #[derive(Default)]
    pub struct Commands {
        pub answers: Vec<(String, Result<String, String>)>,
        pub log: Vec<String>,
    }

    impl Commands {
        pub fn on(mut self, line: &str, out: Result<&str, &str>) -> Commands {
            let out = out.map(str::to_string).map_err(str::to_string);
            self.answers.push((line.to_string(), out));
            self
        }

        fn answer(&mut self, cmd: &Cmd) -> Result<String, String> {
            let line = cmd.line();
            self.log.push(line.clone());
            match self.answers.iter().position(|(l, _)| *l == line) {
                Some(i) => self.answers[i].1.clone(),
                None => Err(format!("unscripted: {line}")),
            }
        }
    }

    impl Runner for Commands {
        fn read(&mut self, cmd: &Cmd) -> Result<String, String> {
            self.answer(cmd)
        }

        fn run(&mut self, cmd: &Cmd) -> Result<(), String> {
            self.answer(cmd).map(|_| ())
        }
    }

    /// Answers in order, and a transcript of what was shown.
    pub struct Script {
        pub answers: VecDeque<String>,
        pub shown: String,
    }

    impl Script {
        pub fn new(answers: &[&str]) -> Script {
            Script {
                answers: answers.iter().map(|a| (*a).to_string()).collect(),
                shown: String::new(),
            }
        }
    }

    impl Terminal for Script {
        fn say(&mut self, text: &str) {
            self.shown.push_str(text);
            self.shown.push('\n');
        }

        fn ask(&mut self, prompt: &str) -> Option<String> {
            let a = self.answers.pop_front();
            self.shown.push_str(prompt);
            self.shown
                .push_str(&format!(" <{}>\n", a.as_deref().unwrap_or("EOF")));
            a
        }
    }
}
