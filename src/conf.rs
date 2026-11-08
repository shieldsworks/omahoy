//! Line-preserving reads and edits of the flat `key = value` files every
//! app uses. omahoy changes only the keys it owns; every other byte stays.

/// One line's key, value and trailing comment, read the way the apps
/// read it: `#` starts a comment, `[section]` lines are skipped, quotes
/// around a value are dropped.
fn split(raw: &str) -> Option<(&str, &str, Option<&str>)> {
    let (code, comment) = match raw.split_once('#') {
        Some((c, rest)) => (c, Some(rest)),
        None => (raw, None),
    };
    let code = code.trim();
    if code.is_empty() || code.starts_with('[') {
        return None;
    }
    let (k, v) = code.split_once('=')?;
    let k = k.trim();
    if k.is_empty() {
        return None;
    }
    Some((k, v.trim().trim_matches('"'), comment))
}

/// Every value of `key`, in file order.
pub fn values(text: &str, key: &str) -> Vec<String> {
    text.lines()
        .filter_map(split)
        .filter(|(k, _, _)| *k == key)
        .map(|(_, v, _)| v.to_string())
        .collect()
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum KeyState {
    Matches,
    Missing,
    Differs { found: String },
}

/// One managed key: what the boat wants and what the file says.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Diff {
    pub key: &'static str,
    /// Repeated keys are joined with ", ".
    pub want: String,
    pub state: KeyState,
}

/// The same value, as the app would read it: quotes dropped, and lists of
/// numbers compared as numbers, so `8.0` is `8` and `37.8,-122.3` is
/// `37.8, -122.3`.
fn same(a: &str, b: &str) -> bool {
    let a = a.trim().trim_matches('"');
    let b = b.trim().trim_matches('"');
    if a == b {
        return true;
    }
    let nums = |s: &str| {
        s.split(',')
            .map(|p| p.trim().parse::<f64>().ok())
            .collect::<Option<Vec<_>>>()
    };
    match (nums(a), nums(b)) {
        (Some(x), Some(y)) => {
            x.len() == y.len() && x.iter().zip(&y).all(|(p, q)| (p - q).abs() < 1e-9)
        }
        _ => false,
    }
}

/// The wanted keys in first-seen order, each with all its values.
fn groups<'a>(want: &'a [(&'static str, String)]) -> Vec<(&'static str, Vec<&'a str>)> {
    let mut out: Vec<(&'static str, Vec<&str>)> = Vec::new();
    for (k, v) in want {
        match out.iter_mut().find(|(g, _)| g == k) {
            Some((_, vs)) => vs.push(v),
            None => out.push((k, vec![v])),
        }
    }
    out
}

pub fn compare(text: &str, want: &[(&'static str, String)]) -> Vec<Diff> {
    groups(want)
        .into_iter()
        .map(|(key, wanted)| {
            let found = values(text, key);
            let state = if found.is_empty() {
                KeyState::Missing
            } else if found.len() == wanted.len()
                && found.iter().zip(&wanted).all(|(f, w)| same(f, w))
            {
                KeyState::Matches
            } else {
                KeyState::Differs {
                    found: found.join(", "),
                }
            };
            Diff {
                key,
                want: wanted.join(", "),
                state,
            }
        })
        .collect()
}

const MARK: &str = "# Set by omahoy from ~/.config/omahoy/boat.toml.";

/// Puts each managed key that doesn't match in place: its lines are
/// replaced where the first one stood (keeping a trailing comment), later
/// copies are dropped, and keys the file lacks are appended under a mark.
/// Keys that already match, comments, sections and every other key are
/// left byte for byte. Pure: text in, text out.
pub fn edit(text: &str, want: &[(&'static str, String)]) -> String {
    let stale: Vec<(&'static str, Vec<&str>)> = groups(want)
        .into_iter()
        .filter(|(k, vs)| {
            let found = values(text, k);
            !(found.len() == vs.len() && found.iter().zip(vs).all(|(f, w)| same(f, w)))
        })
        .collect();
    if stale.is_empty() {
        return text.to_string();
    }
    let mut placed: Vec<&str> = Vec::new();
    let mut out = String::new();
    for raw in text.lines() {
        let Some((k, _, comment)) = split(raw) else {
            out.push_str(raw);
            out.push('\n');
            continue;
        };
        let Some((key, vs)) = stale.iter().find(|(s, _)| *s == k) else {
            out.push_str(raw);
            out.push('\n');
            continue;
        };
        if placed.contains(key) {
            continue;
        }
        placed.push(key);
        for (i, v) in vs.iter().enumerate() {
            out.push_str(&format!("{key} = {v}"));
            if let (0, Some(c)) = (i, comment) {
                out.push_str(&format!(" #{c}"));
            }
            out.push('\n');
        }
    }
    let missing: Vec<_> = stale.iter().filter(|(k, _)| !placed.contains(k)).collect();
    if !missing.is_empty() {
        if !text.contains(MARK) {
            if !out.is_empty() && !out.ends_with("\n\n") {
                out.push('\n');
            }
            out.push_str(MARK);
            out.push('\n');
        }
        for (key, vs) in missing {
            for v in vs {
                out.push_str(&format!("{key} = {v}\n"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn want(pairs: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
        pairs.iter().map(|(k, v)| (*k, (*v).to_string())).collect()
    }

    const HELM: &str = "\
# My chart settings.
palette = paper
units = \"metres\"
safety_depth = 2 # boat
[later]
logbook = ~/notes/Logbook
safety_depth = 3
";

    #[test]
    fn edit_changes_only_the_keys_it_owns() {
        let w = want(&[
            ("units", "meters"),
            ("safety_depth", "3"),
            ("deep_contour", "10"),
            ("home", "37.8663, -122.3148"),
        ]);
        let got = edit(HELM, &w);
        assert_eq!(
            got,
            "\
# My chart settings.
palette = paper
units = meters
safety_depth = 3 # boat
[later]
logbook = ~/notes/Logbook

# Set by omahoy from ~/.config/omahoy/boat.toml.
deep_contour = 10
home = 37.8663, -122.3148
"
        );
        assert_eq!(edit(&got, &w), got, "a second edit changes nothing");
        assert!(
            compare(&got, &w)
                .iter()
                .all(|d| d.state == KeyState::Matches)
        );
    }

    #[test]
    fn compare_reports_missing_and_differing_values() {
        let w = want(&[("units", "meters"), ("safety_depth", "2"), ("home", "1, 2")]);
        assert_eq!(
            compare(HELM, &w),
            vec![
                Diff {
                    key: "units",
                    want: "meters".into(),
                    state: KeyState::Differs {
                        found: "metres".into()
                    }
                },
                Diff {
                    key: "safety_depth",
                    want: "2".into(),
                    state: KeyState::Differs {
                        found: "2, 3".into()
                    }
                },
                Diff {
                    key: "home",
                    want: "1, 2".into(),
                    state: KeyState::Missing
                },
            ]
        );
    }

    #[test]
    fn numbers_compare_as_the_app_reads_them() {
        let text = "safety_depth = 8.0\nregion = 36.8,-123.8, 38.8,-121.6\n";
        let w = want(&[("safety_depth", "8"), ("region", "36.8,-123.8,38.8,-121.6")]);
        assert!(
            compare(text, &w)
                .iter()
                .all(|d| d.state == KeyState::Matches)
        );
        assert_eq!(edit(text, &w), text);
    }

    #[test]
    fn repeated_source_lines_are_one_group() {
        let keel = "\
# The hub.
source = serial:/dev/ttyACM0:38400
socket_note = kept
source = tcp:10.0.0.5:10110
";
        let same_two = want(&[
            ("source", "serial:/dev/ttyACM0:38400"),
            ("source", "tcp:10.0.0.5:10110"),
        ]);
        assert_eq!(edit(keel, &same_two), keel);
        let swapped = want(&[
            ("source", "serial:/dev/serial/by-id/usb-GPS:4800"),
            ("source", "serial:/dev/serial/by-id/usb-dAISy:38400"),
            ("source", "tcp:10.0.0.5:10110"),
        ]);
        assert_eq!(
            edit(keel, &swapped),
            "\
# The hub.
source = serial:/dev/serial/by-id/usb-GPS:4800
source = serial:/dev/serial/by-id/usb-dAISy:38400
source = tcp:10.0.0.5:10110
socket_note = kept
"
        );
        assert_eq!(
            edit(keel, &want(&[("source", "demo")])),
            "# The hub.\nsource = demo\nsocket_note = kept\n"
        );
    }

    #[test]
    fn an_empty_file_gets_the_mark_and_the_keys() {
        let w = want(&[("boat", "\"Wavelength\""), ("git", "true")]);
        assert_eq!(
            edit("", &w),
            "# Set by omahoy from ~/.config/omahoy/boat.toml.\nboat = \"Wavelength\"\ngit = true\n"
        );
        assert_eq!(
            values("vault = \"/home/casey/notes/Logbook\"\n", "vault"),
            vec!["/home/casey/notes/Logbook".to_string()]
        );
    }
}
