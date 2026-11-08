//! omahoy: one boat file, and a machine that matches it.

mod apps;
mod ask;
mod boat;
mod cli;
mod conf;
mod depth;
mod hub;
mod machine;
mod place;
mod plan;
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

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = cli::parse_args(&args)
        .and_then(|cmd| {
            let dirs = sys::Dirs::from_env()?;
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
