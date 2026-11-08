//! omahoy: one boat file, and a machine that matches it.

mod apps;
mod ask;
mod boat;
mod cli;
mod conf;
mod depth;
mod gui;
mod hub;
mod json;
mod machine;
mod place;
mod plan;
mod probe;
mod run;
mod sys;

use std::path::PathBuf;

/// The `omahoy` bootstrap beside this binary's checkout, when this is
/// `<checkout>/target/release/omahoy`.
fn own_bootstrap() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?.canonicalize().ok()?;
    let root = exe.ancestors().nth(3)?;
    let script = root.join("omahoy");
    (script.is_file() && root.join("Cargo.toml").is_file()).then_some(script)
}

/// A gui command: stdin in, JSON out on stdout, everything else captured
/// or on stderr. A command that failed is exit 2 with nothing on stdout.
fn gui_main(g: &gui::Gui, dirs: &sys::Dirs) -> Result<i32, String> {
    use std::io::{Read, Write};
    let mut input = String::new();
    if g.reads_stdin() {
        std::io::stdin()
            .read_to_string(&mut input)
            .map_err(|e| format!("reading stdin: {e}"))?;
    }
    let mut system = sys::System;
    let mut runner = gui::GuiRunner { inner: &mut system };
    let mut term = gui::GuiTerm {
        stdout: g.progress_on_stdout(),
    };
    let mut out = String::new();
    let code = gui::run(
        g,
        &mut cli::Env {
            t: &mut term,
            r: &mut runner,
            dirs,
            own: own_bootstrap(),
        },
        &input,
        &mut out,
    )?;
    let mut stdout = std::io::stdout();
    stdout
        .write_all(out.as_bytes())
        .and_then(|()| stdout.flush())
        .map_err(|e| format!("writing stdout: {e}"))?;
    Ok(code)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cli::parse_args(&args)
        .and_then(|cmd| {
            let dirs = sys::Dirs::from_env()?;
            if let cli::Command::Gui(g) = &cmd {
                return gui_main(g, &dirs);
            }
            cli::run(
                cmd,
                &mut cli::Env {
                    t: &mut sys::Console,
                    r: &mut sys::System,
                    dirs: &dirs,
                    own: own_bootstrap(),
                },
            )
        })
        .unwrap_or_else(|e| {
            eprintln!("omahoy: {e}");
            2
        });
    std::process::exit(code);
}
