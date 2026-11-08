//! `omahoy gui ...`: the commands a graphical front end calls. Boat text
//! comes in on stdin and one JSON document goes out on stdout, so the
//! panel holds the form and this crate keeps every rule (docs/gui.md).
//! Child output and human progress never reach stdout, except `apply`'s
//! own progress lines.

use crate::apps::App;
use crate::ask::short_device;
use crate::boat::{Boat, KeyLines, Partial, Problem};
use crate::cli::{Env, converge, hub_ready};
use crate::depth::{Units, depths};
use crate::hub::{Heard, Source, Verdict};
use crate::json::Json;
use crate::machine::{Machine, uucp};
use crate::place::{Area, Coverage, coverage};
use crate::plan::{findings, plan};
use crate::probe::probe_with_hub_stopped;
use crate::sys::{Cmd, Runner, Terminal};
use std::path::Path;

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Gui {
    Draft,
    Check,
    Probe { uucp: bool, specs: Vec<String> },
    JoinUucp,
    Plan,
    Apply,
}

impl Gui {
    pub fn reads_stdin(&self) -> bool {
        matches!(self, Gui::Check | Gui::Plan | Gui::Apply)
    }

    /// Only `apply` prints progress on stdout.
    pub fn progress_on_stdout(&self) -> bool {
        matches!(self, Gui::Apply)
    }
}

/// `say` goes to stderr (stdout for apply's progress); the panel can't
/// answer a question, so `ask` is end of input.
pub struct GuiTerm {
    pub stdout: bool,
}

impl Terminal for GuiTerm {
    fn say(&mut self, text: &str) {
        if self.stdout {
            println!("{text}");
        } else {
            eprintln!("{text}");
        }
    }

    fn ask(&mut self, _prompt: &str) -> Option<String> {
        None
    }
}

/// Runs commands for the panel: output is captured so it never reaches
/// stdout, and what needs root asks through polkit rather than a terminal
/// password prompt.
pub struct GuiRunner<'a> {
    pub inner: &'a mut dyn Runner,
}

impl GuiRunner<'_> {
    fn map(cmd: &Cmd) -> Cmd {
        let words: Vec<&str> = cmd.args.iter().map(String::as_str).collect();
        let mapped = match (cmd.program.as_str(), words.as_slice()) {
            ("sudo", ["gpasswd", rest @ ..]) => {
                Cmd::new("pkexec", &["gpasswd"]).args(rest.iter().map(|w| (*w).to_string()))
            }
            ("omarchy", ["pkg", "add", pkgs @ ..]) => {
                Cmd::new("pkexec", &["pacman", "-S", "--noconfirm", "--needed", "--"])
                    .args(pkgs.iter().map(|w| (*w).to_string()))
            }
            _ => return cmd.clone(),
        };
        Cmd {
            dir: cmd.dir.clone(),
            ..mapped
        }
    }
}

impl Runner for GuiRunner<'_> {
    fn read(&mut self, cmd: &Cmd) -> Result<String, String> {
        self.inner.read(cmd)
    }

    fn run(&mut self, cmd: &Cmd) -> Result<(), String> {
        self.inner.read(&GuiRunner::map(cmd)).map(|_| ())
    }
}

/// Appends the JSON (when the command has any) to `out`; the exit code.
pub fn run(g: &Gui, e: &mut Env<'_>, input: &str, out: &mut String) -> Result<i32, String> {
    let json = match g {
        Gui::Draft => draft(e)?,
        Gui::Check => check(input),
        Gui::Probe { uucp: via, specs } => {
            let (good, _) = split_specs(specs);
            let omakeel = if specs.is_empty() || !good.is_empty() {
                need_uucp(e, *via)?;
                Some(hub_ready(e)?)
            } else {
                None
            };
            probe(e, omakeel.as_deref(), *via, specs)?
        }
        Gui::JoinUucp => join_uucp(e)?,
        Gui::Plan => plan_json(e, input),
        Gui::Apply => {
            let boat = Boat::parse(input).map_err(|ps| {
                let mut m = String::from("boat has problems:");
                for p in ps {
                    m += &format!("\n  {p}");
                }
                m
            })?;
            boat.save(e.dirs)?;
            return converge(e, &boat, true);
        }
    };
    out.push_str(&json.render());
    out.push('\n');
    Ok(0)
}

/// `sg uucp` asks for a group password on a terminal that isn't there, when
/// the user is not in the group.
fn need_uucp(e: &mut Env<'_>, via: bool) -> Result<(), String> {
    let user = e.dirs.user.clone();
    if via && !uucp(&mut *e.r, &user).0 {
        return Err(format!(
            "{user} is not in the uucp group yet. Add the group first (omahoy gui join-uucp)."
        ));
    }
    Ok(())
}

/// The probe, with `omakeel` built already. None when no SPEC was usable.
fn probe(
    e: &mut Env<'_>,
    omakeel: Option<&Path>,
    via: bool,
    specs: &[String],
) -> Result<Json, String> {
    let (good, bad) = split_specs(specs);
    let mut heard = Vec::new();
    if let Some(bin) = omakeel {
        heard = probe_with_hub_stopped(&mut *e.r, &mut *e.t, bin, &good, via)?;
    }
    let user = e.dirs.user.clone();
    Ok(probe_json(&mut *e.r, &user, heard, bad))
}

fn problems_json(ps: &[Problem]) -> Json {
    Json::Arr(
        ps.iter()
            .map(|p| {
                Json::obj(vec![
                    ("key", p.key.into()),
                    ("line", p.line.into()),
                    ("message", p.message.clone().into()),
                ])
            })
            .collect(),
    )
}

/// The area setup would use for this home: the current one when it holds
/// home, else a degree or so around home.
fn suggested_area(p: &Partial) -> Option<Area> {
    let home = p.home?;
    Some(match p.area {
        Some(a) if a.contains(home) => a,
        _ => Area::around(home),
    })
}

fn area_json(a: Area) -> Json {
    let [south, west, north, east] = a.bounds();
    Json::obj(vec![
        ("south", south.into()),
        ("west", west.into()),
        ("north", north.into()),
        ("east", east.into()),
    ])
}

fn per_unit(f: impl Fn(Units) -> f64) -> Json {
    Json::obj(vec![
        ("feet", f(Units::Feet).into()),
        ("meters", f(Units::Meters).into()),
        ("fathoms", f(Units::Fathoms).into()),
    ])
}

/// A value of the boat's units, in each unit; null until both are known.
fn converted(units: Option<Units>, v: Option<f64>) -> Json {
    match (units, v) {
        (Some(u), Some(v)) => per_unit(|to| u.convert(v, to)),
        _ => Json::Null,
    }
}

/// What the form needs to know about these answers, from the crate's rules.
fn report(p: &Partial, mut problems: Vec<Problem>, keys: &KeyLines) -> Json {
    let line_of = |key: &str| keys.iter().find(|(k, _)| *k == key).map_or(0, |(_, n)| *n);
    for mut c in p.cross_problems() {
        c.line = c.key.map_or(0, line_of);
        problems.push(c);
    }
    for m in p.missing() {
        if !problems.iter().any(|x| x.key == m.key) {
            problems.push(m);
        }
    }
    let suggested = suggested_area(p);
    let cov: Option<Coverage> = suggested.map(coverage);
    let refused = |a: App| cov.and_then(|c| a.refusal(c));
    let sources: Vec<String> = p.hub.as_ref().map(|h| h.specs()).unwrap_or_default();
    let shorts = |apps: Vec<App>| Json::from(apps.iter().map(|a| a.short()).collect::<Vec<_>>());
    let depths_json = match (p.units, p.draft, p.clearance) {
        (Some(u), Some(d), Some(c)) => {
            let d = depths(u, d, c);
            Json::obj(vec![
                ("safety", d.safety_depth.into()),
                ("shallow", d.shallow_contour.into()),
                ("safetyContour", d.safety_contour.into()),
                ("deep", d.deep_contour.into()),
            ])
        }
        _ => Json::Null,
    };
    Json::obj(vec![
        (
            "answers",
            Json::obj(vec![
                ("name", p.name.clone().into()),
                (
                    "home",
                    p.home.map_or(Json::Null, |h| {
                        Json::obj(vec![
                            ("lat", h.lat().into()),
                            ("lon", h.lon().into()),
                            ("show", h.show().into()),
                        ])
                    }),
                ),
                ("area", p.area.map_or(Json::Null, area_json)),
                ("units", p.units.map(Units::name).into()),
                ("draft", p.draft.into()),
                ("clearance", p.clearance.into()),
                ("sources", sources.into()),
                ("apps", p.apps.clone().map_or(Json::Null, shorts)),
                ("vault", p.vault.clone().into()),
                ("git", p.git.into()),
            ]),
        ),
        ("problems", problems_json(&problems)),
        ("complete", problems.is_empty().into()),
        ("suggestedArea", suggested.map_or(Json::Null, area_json)),
        (
            "coverage",
            cov.map_or(Json::Null, |c| {
                Json::obj(vec![("noaa", c.noaa.into()), ("hrrr", c.hrrr.into())])
            }),
        ),
        ("depths", depths_json),
        ("draftIn", converted(p.units, p.draft)),
        ("clearanceIn", converted(p.units, p.clearance)),
        ("defaultClearance", per_unit(Units::default_clearance)),
        (
            "apps",
            Json::Arr(
                App::CHOSEN
                    .iter()
                    .map(|a| {
                        Json::obj(vec![
                            ("short", a.short().into()),
                            ("name", a.name().into()),
                            ("what", a.what().into()),
                            ("refusal", refused(*a).into()),
                            ("caveat", cov.and_then(|c| a.caveat(c)).into()),
                        ])
                    })
                    .collect(),
            ),
        ),
        (
            "appsAllowed",
            shorts(
                p.apps
                    .iter()
                    .flatten()
                    .copied()
                    .filter(|a| refused(*a).is_none())
                    .collect(),
            ),
        ),
    ])
}

/// What the terminal questions would offer for answers not given yet.
fn with_defaults(mut p: Partial) -> Partial {
    let units = *p.units.get_or_insert(Units::Feet);
    p.clearance.get_or_insert(units.default_clearance());
    p.vault.get_or_insert_with(|| "~/Logbook".into());
    p.git.get_or_insert(true);
    if p.apps.is_none() {
        let cov = suggested_area(&p).map(coverage);
        p.apps = Some(
            App::CHOSEN
                .into_iter()
                .filter(|a| cov.is_none_or(|c| a.refusal(c).is_none()))
                .collect(),
        );
    }
    p
}

fn draft(e: &mut Env<'_>) -> Result<Json, String> {
    let boat = Boat::load(e.dirs)?;
    let exists = boat.is_some();
    let p = match boat {
        Some(b) => Partial::from_boat(&b),
        None => Partial::from_existing(e.dirs),
    };
    let report = report(&with_defaults(p), Vec::new(), &Vec::new());
    Ok(Json::obj(vec![
        ("exists", exists.into()),
        ("report", report),
    ]))
}

fn check(input: &str) -> Json {
    let (p, problems, keys) = Partial::parse(input);
    report(&p, problems, &keys)
}

fn plan_json(e: &mut Env<'_>, input: &str) -> Json {
    match Boat::parse(input) {
        Err(ps) => Json::obj(vec![
            ("problems", problems_json(&ps)),
            ("steps", Json::Arr(vec![])),
            ("findings", Json::Arr(vec![])),
        ]),
        Ok(boat) => {
            let m = Machine::survey(e.r, e.dirs, &boat, e.own.as_deref());
            let steps: Vec<String> = plan(&boat, &m).iter().map(|s| s.describe(e.dirs)).collect();
            Json::obj(vec![
                ("problems", Json::Arr(vec![])),
                ("steps", steps.into()),
                ("findings", findings(&boat, &m).into()),
            ])
        }
    }
}

fn join_uucp(e: &mut Env<'_>) -> Result<Json, String> {
    let user = e.dirs.user.clone();
    e.r.run(&Cmd::new("pkexec", &["gpasswd", "-a", &user, "uucp"]))
        .map_err(|err| format!("couldn't add {user} to the uucp group: {err}"))?;
    let (_, session) = uucp(&mut *e.r, &user);
    Ok(Json::obj(vec![
        ("member", true.into()),
        ("session", session.into()),
    ]))
}

/// A SPEC the probe takes: `serial:PATH[:BAUD]` or `tcp:HOST:PORT`, with
/// `HOST:PORT` read as tcp.
fn normalize_spec(spec: &str) -> Result<String, String> {
    let spec = spec.trim();
    if let Some(rest) = spec.strip_prefix("serial:") {
        let path = match rest.rsplit_once(':') {
            Some((path, baud)) if crate::hub::Baud::parse(baud).is_some() => path,
            _ => rest,
        };
        return if path.starts_with('/') {
            Ok(spec.to_string())
        } else {
            Err(format!("{spec}: the device path must be absolute"))
        };
    }
    let tcp = if spec.starts_with("tcp:") {
        spec.to_string()
    } else if !spec.starts_with('/') && spec.contains(':') && !spec.contains("://") {
        format!("tcp:{spec}")
    } else {
        return Err(format!("{spec}: expected serial:PATH or tcp:HOST:PORT"));
    };
    Source::parse(&tcp).map(|s| s.spec())
}

/// The specs the probe can take, and a device entry for each it can't.
fn split_specs(specs: &[String]) -> (Vec<String>, Vec<Json>) {
    let mut good = Vec::new();
    let mut bad = Vec::new();
    for s in specs {
        match normalize_spec(s) {
            Ok(n) => good.push(n),
            Err(why) => bad.push(device(s, "error", &why, Json::Null, Json::Null)),
        }
    }
    (good, bad)
}

fn kind(spec: &str) -> &'static str {
    if spec.starts_with("tcp:") {
        "tcp"
    } else {
        "serial"
    }
}

fn device(spec: &str, verdict: &str, describe: &str, source: Json, fix: Json) -> Json {
    let name = match spec.strip_prefix("serial:") {
        Some(rest) => short_device(rest.rsplit_once(':').map_or(rest, |(p, b)| {
            if crate::hub::Baud::parse(b).is_some() {
                p
            } else {
                rest
            }
        })),
        None => spec.to_string(),
    };
    Json::obj(vec![
        ("spec", spec.into()),
        ("name", name.into()),
        ("kind", kind(spec).into()),
        ("verdict", verdict.into()),
        ("describe", describe.into()),
        ("source", source),
        ("fix", fix),
    ])
}

fn heard_json(h: Result<Heard, String>) -> Json {
    let h = match h {
        Ok(h) => h,
        Err(e) => {
            let line = e.strip_prefix("unexpected probe line: ").unwrap_or(&e);
            let spec = line.split_whitespace().next().unwrap_or("");
            return device(spec, "error", &e, Json::Null, Json::Null);
        }
    };
    let verdict = match &h.verdict {
        Verdict::Ok { .. } => "ok",
        Verdict::Denied => "denied",
        Verdict::Busy => "busy",
        Verdict::Silent => "silent",
        Verdict::Missing => "missing",
        Verdict::Unreachable(_) => "unreachable",
    };
    let source = match &h.verdict {
        Verdict::Ok { source, .. } => Json::from(source.spec()),
        Verdict::Silent => h
            .path()
            .map_or(Json::Null, |p| Json::from(format!("serial:{p}:38400"))),
        _ if h.spec.starts_with("tcp:") => Source::parse(&h.spec).ok().map(|s| s.spec()).into(),
        _ => Json::Null,
    };
    let fix = match &h.verdict {
        Verdict::Ok { at: Some(p), .. } => {
            Json::obj(vec![("lat", p.lat().into()), ("lon", p.lon().into())])
        }
        _ => Json::Null,
    };
    device(&h.spec, verdict, &h.describe(), source, fix)
}

fn probe_json(
    r: &mut dyn Runner,
    user: &str,
    heard: Vec<Result<Heard, String>>,
    bad: Vec<Json>,
) -> Json {
    let (member, session) = uucp(r, user);
    let mut devices = bad;
    devices.extend(heard.into_iter().map(heard_json));
    Json::obj(vec![
        ("devices", Json::Arr(devices)),
        (
            "uucp",
            Json::obj(vec![("member", member.into()), ("session", session.into())]),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boat::tests::WAVELENGTH;
    use crate::sys::Dirs;
    use crate::sys::fake::{Commands, Script};

    fn tmp_dirs(name: &str) -> Dirs {
        let tmp = std::env::temp_dir().join(format!("omahoy-gui-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        Dirs::resolve(Some(tmp.into()), None, None, None, Some("sailor".into())).unwrap()
    }

    fn go(g: &Gui, dirs: &Dirs, r: &mut Commands, input: &str) -> (Result<i32, String>, String) {
        let mut t = Script::new(&[]);
        let mut out = String::new();
        let code = run(
            g,
            &mut Env {
                t: &mut t,
                r,
                dirs,
                own: None,
            },
            input,
            &mut out,
        );
        (code, out)
    }

    #[test]
    fn check_reports_a_whole_boat() {
        let dirs = tmp_dirs("whole");
        let (code, out) = go(&Gui::Check, &dirs, &mut Commands::default(), WAVELENGTH);
        assert_eq!(code, Ok(0));
        let want = concat!(
            r#"{"answers":{"name":"Wavelength","home":{"lat":47.687,"lon":-122.413,"show":"47°41.2′N 122°24.8′W"},"#,
            r#""area":{"south":46.687,"west":-123.513,"north":48.687,"east":-121.313},"units":"feet","draft":5.5,"clearance":2,"#,
            r#""sources":["serial:/dev/serial/by-id/usb-Prolific_Technology_Inc._USB-Serial_Controller-if00-port0:4800","serial:/dev/serial/by-id/usb-dAISy_AIS_Receiver_0001-if00:38400"],"#,
            r#""apps":["helm","lookout","logbook","tide","wind"],"vault":"~/Logbook","git":true},"#,
            r#""problems":[],"complete":true,"#,
            r#""suggestedArea":{"south":46.687,"west":-123.513,"north":48.687,"east":-121.313},"#,
            r#""coverage":{"noaa":true,"hrrr":true},"#,
            r#""depths":{"safety":8,"shallow":4,"safetyContour":8,"deep":30},"#,
            r#""draftIn":{"feet":5.5,"meters":1.68,"fathoms":0.92},"#,
            r#""clearanceIn":{"feet":2,"meters":0.61,"fathoms":0.33},"#,
            r#""defaultClearance":{"feet":2,"meters":0.6,"fathoms":0.5},"#,
            r#""apps":[{"short":"helm","name":"omahelm","what":"chartplotter","refusal":null,"caveat":null},"#,
            r#"{"short":"lookout","name":"omalookout","what":"AIS targets and collision alarms","refusal":null,"caveat":null},"#,
            r#"{"short":"logbook","name":"omalogbook","what":"ship's log","refusal":null,"caveat":null},"#,
            r#"{"short":"tide","name":"omatide","what":"tides and currents","refusal":null,"caveat":null},"#,
            r#"{"short":"wind","name":"omawind","what":"wind forecasts","refusal":null,"caveat":null}],"#,
            r#""appsAllowed":["helm","lookout","logbook","tide","wind"]}"#,
            "\n"
        );
        assert_eq!(out, want);
    }

    #[test]
    fn check_names_the_key_and_line_of_each_problem_and_nulls_what_is_unknown() {
        let dirs = tmp_dirs("bad");
        let text = "name = \"Dash\"\ndraft = 0\nunits = feet\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), text);
        assert!(
            out.contains(
                r#""problems":[{"key":"draft","line":2,"message":"draft 0 is not a depth"},"#
            ),
            "{out}"
        );
        assert!(out.contains(r#"{"key":"vault","line":0,"message":"missing vault"}"#));
        assert!(out.contains(r#""complete":false"#));
        assert!(out.contains(r#""suggestedArea":null,"coverage":null,"depths":null"#));
        assert!(out.contains(r#""draftIn":null,"clearanceIn":null"#));
        assert!(out.contains(r#""appsAllowed":[]"#));
        assert!(out.contains(r#""refusal":null"#));
    }

    #[test]
    fn depths_wait_for_units_draft_and_clearance_and_conversions_follow_units() {
        let dirs = tmp_dirs("depths");
        let text = "units = meters\ndraft = 1.7\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), text);
        assert!(out.contains(r#""depths":null"#));
        assert!(
            out.contains(r#""draftIn":{"feet":5.58,"meters":1.7,"fathoms":0.93}"#),
            "{out}"
        );
        assert!(out.contains(r#""clearanceIn":null"#));
        let (_, out) = go(
            &Gui::Check,
            &dirs,
            &mut Commands::default(),
            "units = meters\ndraft = 1.7\nclearance = 0.6\n",
        );
        assert!(
            out.contains(r#""depths":{"safety":3,"shallow":1,"safetyContour":3,"deep":10}"#),
            "{out}"
        );
    }

    #[test]
    fn the_suggested_area_keeps_the_current_one_around_home_and_follows_a_new_home() {
        let dirs = tmp_dirs("area");
        let wide = "home = 47.5, -122.0\narea = 45,-125,50,-120\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), wide);
        assert!(
            out.contains(r#""suggestedArea":{"south":45,"west":-125,"north":50,"east":-120}"#),
            "{out}"
        );
        let moved = "home = 37.8, -122.3\narea = 45,-125,50,-120\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), moved);
        assert!(
            out.contains(
                r#""suggestedArea":{"south":36.8,"west":-123.4,"north":38.8,"east":-121.2}"#
            ),
            "{out}"
        );
        assert!(out.contains(r#"{"key":"area","line":2,"message":"area 45,-125,50,-120 doesn't contain home 37.8, -122.3"}"#), "{out}");
    }

    #[test]
    fn apps_allowed_drops_the_apps_refused_where_the_area_is() {
        let dirs = tmp_dirs("mallorca");
        let text = "home = 39.47, 2.73\narea = 38.47,1.63,40.47,3.83\napps = helm, tide, wind\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), text);
        assert!(
            out.contains(r#""coverage":{"noaa":false,"hrrr":false}"#),
            "{out}"
        );
        assert!(out.contains(r#""appsAllowed":["helm"]"#), "{out}");
        assert!(out.contains(r#""refusal":"NOAA tide stations cover US waters only""#));
        assert!(out.contains(
            r#""caveat":"no NOAA charts here; import your own ENC: omahelm import ZIP|DIR""#
        ));
        assert!(
            out.contains(r#""key":"apps","line":3,"message":"omatide can't work here"#),
            "{out}"
        );
    }

    #[test]
    fn demo_beside_another_source_is_a_problem_on_source() {
        let dirs = tmp_dirs("demo");
        let text = "source = demo\nsource = tcp:h:1\n";
        let (_, out) = go(&Gui::Check, &dirs, &mut Commands::default(), text);
        assert!(out.contains(r#""sources":[]"#));
        assert!(
            out.contains(
                r#"{"key":"source","line":1,"message":"source = demo must be the only source"}"#
            ),
            "{out}"
        );
    }

    #[test]
    fn draft_offers_the_terminal_defaults_for_what_is_unset() {
        let dirs = tmp_dirs("draft-new");
        let (code, out) = go(&Gui::Draft, &dirs, &mut Commands::default(), "");
        assert_eq!(code, Ok(0));
        assert!(out.starts_with(r#"{"exists":false,"report":{"answers":{"name":null,"home":null,"area":null,"units":"feet","draft":null,"clearance":2,"sources":[],"apps":["helm","lookout","logbook","tide","wind"],"vault":"~/Logbook","git":true}"#), "{out}");
        assert!(out.contains(r#""complete":false"#));
        assert!(out.contains(r#"{"key":"name","line":0,"message":"missing name"}"#));
    }

    #[test]
    fn draft_of_a_saved_boat_is_that_boat_and_a_broken_file_is_exit_2() {
        let dirs = tmp_dirs("draft-saved");
        std::fs::create_dir_all(dirs.boat().parent().unwrap()).unwrap();
        std::fs::write(dirs.boat(), WAVELENGTH).unwrap();
        let (code, out) = go(&Gui::Draft, &dirs, &mut Commands::default(), "");
        assert_eq!(code, Ok(0));
        assert!(
            out.starts_with(r#"{"exists":true,"report":{"answers":{"name":"Wavelength""#),
            "{out}"
        );
        assert!(out.contains(r#""problems":[],"complete":true"#));
        std::fs::write(dirs.boat(), "junk\n").unwrap();
        let (code, out) = go(&Gui::Draft, &dirs, &mut Commands::default(), "");
        let _ = std::fs::remove_dir_all(&dirs.home);
        assert!(code.unwrap_err().contains("line 1: expected key = value"));
        assert_eq!(out, "");
    }

    #[test]
    fn plan_of_a_bad_boat_has_problems_and_no_steps() {
        let dirs = tmp_dirs("plan-bad");
        let (code, out) = go(
            &Gui::Plan,
            &dirs,
            &mut Commands::default(),
            "name = \"X\"\n",
        );
        assert_eq!(code, Ok(0));
        assert!(
            out.starts_with(r#"{"problems":[{"key":null,"line":0,"message":"missing source"#),
            "{out}"
        );
        assert!(out.ends_with("\"steps\":[],\"findings\":[]}\n"), "{out}");
    }

    #[test]
    fn plan_of_a_good_boat_lists_steps_and_saves_nothing() {
        let dirs = tmp_dirs("plan-good");
        let (code, out) = go(&Gui::Plan, &dirs, &mut Commands::default(), WAVELENGTH);
        let saved = dirs.boat().exists();
        let _ = std::fs::remove_dir_all(&dirs.home);
        assert_eq!(code, Ok(0));
        assert!(
            out.starts_with(r#"{"problems":[],"steps":["install packages:"#),
            "{out}"
        );
        assert!(!saved);
    }

    #[test]
    fn apply_of_a_bad_boat_is_exit_2_and_saves_nothing() {
        let dirs = tmp_dirs("apply-bad");
        let (code, out) = go(
            &Gui::Apply,
            &dirs,
            &mut Commands::default(),
            "name = \"X\"\n",
        );
        assert!(code.unwrap_err().contains("missing source"));
        assert_eq!(out, "");
        assert!(!dirs.boat().exists());
    }

    #[test]
    fn join_uucp_asks_polkit_and_reports_the_session() {
        let dirs = tmp_dirs("join");
        let mut r = Commands::default()
            .on("pkexec gpasswd -a sailor uucp", Ok(""))
            .on("getent group uucp", Ok("uucp:x:987:sailor\n"))
            .on("id -nG", Ok("sailor wheel\n"));
        let (code, out) = go(&Gui::JoinUucp, &dirs, &mut r, "");
        assert_eq!(
            (code, out.as_str()),
            (Ok(0), "{\"member\":true,\"session\":false}\n")
        );
        let mut r = Commands::default().on("pkexec gpasswd -a sailor uucp", Err("dismissed"));
        let (code, _) = go(&Gui::JoinUucp, &dirs, &mut r, "");
        assert!(code.unwrap_err().contains("dismissed"));
    }

    fn probe_out(
        heard: &str,
        specs: &[&str],
        via: bool,
        mut r: Commands,
    ) -> (Result<Json, String>, Vec<String>) {
        let dirs = tmp_dirs("probe");
        let mut t = Script::new(&[]);
        let specs: Vec<String> = specs.iter().map(|s| (*s).to_string()).collect();
        let _ = heard;
        let got = probe(
            &mut Env {
                t: &mut t,
                r: &mut r,
                dirs: &dirs,
                own: None,
            },
            Some(Path::new("/k/omakeel")),
            via,
            &specs,
        );
        (got, r.log)
    }

    #[test]
    fn probe_describes_each_device_the_way_the_terminal_does() {
        let heard = "\
serial:/dev/serial/by-id/usb-GPS-if00:4800  ok  fix=41 ais=0 rejected=2 at=37.8667000,-122.3134000
serial:/dev/serial/by-id/usb-dAISy-if00  silent
serial:/dev/serial/by-id/usb-u-blox-if00  denied
tcp:192.168.1.20:10110  unreachable  connection refused
serial:/dev/serial/by-id/usb-X  nonsense
";
        let r = Commands::default()
            .on("/k/omakeel probe", Ok(heard))
            .on("getent group uucp", Ok("uucp:x:987:sailor\n"))
            .on("id -nG", Ok("sailor uucp\n"));
        let (got, _) = probe_out(heard, &[], false, r);
        let out = got.unwrap().render();
        let want = concat!(
            r#"{"devices":["#,
            r#"{"spec":"serial:/dev/serial/by-id/usb-GPS-if00:4800","name":"usb-GPS-if00","kind":"serial","verdict":"ok","describe":"GPS, 4800 baud, 41 position sentences, fix 37°52.0′N 122°18.8′W","source":"serial:/dev/serial/by-id/usb-GPS-if00:4800","fix":{"lat":37.8667,"lon":-122.3134}},"#,
            r#"{"spec":"serial:/dev/serial/by-id/usb-dAISy-if00","name":"usb-dAISy-if00","kind":"serial","verdict":"silent","describe":"silent at every baud","source":"serial:/dev/serial/by-id/usb-dAISy-if00:38400","fix":null},"#,
            r#"{"spec":"serial:/dev/serial/by-id/usb-u-blox-if00","name":"usb-u-blox-if00","kind":"serial","verdict":"denied","describe":"permission denied","source":null,"fix":null},"#,
            r#"{"spec":"tcp:192.168.1.20:10110","name":"tcp:192.168.1.20:10110","kind":"tcp","verdict":"unreachable","describe":"unreachable: connection refused","source":"tcp:192.168.1.20:10110","fix":null},"#,
            r#"{"spec":"serial:/dev/serial/by-id/usb-X","name":"usb-X","kind":"serial","verdict":"error","describe":"unexpected probe line: serial:/dev/serial/by-id/usb-X  nonsense","source":null,"fix":null}],"#,
            r#""uucp":{"member":true,"session":true}}"#
        );
        assert_eq!(out, want);
    }

    #[test]
    fn probe_specs_are_normalized_and_a_bad_one_is_an_error_device() {
        let r = Commands::default()
            .on(
                "/k/omakeel probe tcp:192.168.1.20:10110 serial:/dev/ttyUSB0",
                Ok("tcp:192.168.1.20:10110  ok  fix=3 ais=0 rejected=0\nserial:/dev/ttyUSB0  busy\n"),
            )
            .on("getent group uucp", Ok("uucp:x:987:\n"))
            .on("id -nG", Ok("sailor\n"));
        let (got, _) = probe_out(
            "",
            &[
                "192.168.1.20:10110",
                "serial:/dev/ttyUSB0",
                "ttyUSB1",
                "tcp:h:0",
            ],
            false,
            r,
        );
        let out = got.unwrap().render();
        assert!(out.starts_with(r#"{"devices":[{"spec":"ttyUSB1","name":"ttyUSB1","kind":"serial","verdict":"error","describe":"ttyUSB1: expected serial:PATH or tcp:HOST:PORT","source":null,"fix":null},{"spec":"tcp:h:0","#), "{out}");
        assert!(
            out.contains(r#""verdict":"error","describe":"tcp:h:0: expected tcp:HOST:PORT""#),
            "{out}"
        );
        assert!(out.contains(r#""spec":"tcp:192.168.1.20:10110","name":"tcp:192.168.1.20:10110","kind":"tcp","verdict":"ok","describe":"GPS, 3 position sentences""#), "{out}");
        assert!(
            out.ends_with(r#""uucp":{"member":false,"session":false}}"#),
            "{out}"
        );
    }

    #[test]
    fn all_specs_bad_runs_no_probe() {
        let r = Commands::default()
            .on("getent group uucp", Ok(""))
            .on("id -nG", Ok(""));
        let dirs = tmp_dirs("nobuild");
        let mut t = Script::new(&[]);
        let mut r = r;
        let mut out = String::new();
        let code = run(
            &Gui::Probe {
                uucp: false,
                specs: vec!["nope".into()],
            },
            &mut Env {
                t: &mut t,
                r: &mut r,
                dirs: &dirs,
                own: None,
            },
            "",
            &mut out,
        );
        assert_eq!(code, Ok(0));
        assert!(out.contains(r#""verdict":"error""#));
        assert_eq!(r.log, vec!["getent group uucp", "id -nG"]);
    }

    #[test]
    fn probe_through_sg_needs_the_group_first() {
        let dirs = tmp_dirs("sg");
        let mut r = Commands::default().on("getent group uucp", Ok("uucp:x:987:\n"));
        let (code, out) = go(
            &Gui::Probe {
                uucp: true,
                specs: vec![],
            },
            &dirs,
            &mut r,
            "",
        );
        assert!(code.unwrap_err().contains("not in the uucp group"));
        assert_eq!(out, "");
        assert!(
            !r.log.iter().any(|l| l.starts_with("sg ")),
            "sg never ran: {:?}",
            r.log
        );
    }

    #[test]
    fn a_probe_that_fails_to_run_is_exit_2() {
        let r = Commands::default().on("/k/omakeel probe", Err("exit 2"));
        let (got, _) = probe_out("", &[], false, r);
        assert!(got.unwrap_err().contains("couldn't run omakeel probe"));
    }

    /// Records which Runner method a command came through.
    #[derive(Default)]
    struct Spy {
        read: Vec<String>,
        run: Vec<String>,
    }

    impl Runner for Spy {
        fn read(&mut self, cmd: &Cmd) -> Result<String, String> {
            self.read.push(cmd.line());
            Ok(String::new())
        }

        fn run(&mut self, cmd: &Cmd) -> Result<(), String> {
            self.run.push(cmd.line());
            Ok(())
        }
    }

    #[test]
    fn root_commands_go_through_pkexec_and_run_is_captured() {
        let mut spy = Spy::default();
        let mut r = GuiRunner { inner: &mut spy };
        r.run(&Cmd::new("sudo", &["gpasswd", "-a", "sailor", "uucp"]))
            .unwrap();
        r.run(&Cmd::new("omarchy", &["pkg", "add", "git", "curl"]))
            .unwrap();
        r.run(&Cmd::new("omarchy", &["plugin", "enable", "x"]))
            .unwrap();
        r.run(&Cmd::new("sudo", &["reboot"])).unwrap();
        r.read(&Cmd::new("sudo", &["gpasswd", "-a", "a", "b"]))
            .unwrap();
        assert!(
            spy.run.is_empty(),
            "nothing inherits the terminal: {:?}",
            spy.run
        );
        assert_eq!(
            spy.read,
            vec![
                "pkexec gpasswd -a sailor uucp",
                "pkexec pacman -S --noconfirm --needed -- git curl",
                "omarchy plugin enable x",
                "sudo reboot",
                "sudo gpasswd -a a b",
            ]
        );
    }
}
