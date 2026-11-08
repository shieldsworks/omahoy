//! What omakeel reads, and what `omakeel probe` heard.

use crate::place::Position;

/// One of omakeel's six rates. The enum is the list.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Baud {
    B4800,
    B9600,
    B19200,
    B38400,
    B57600,
    B115200,
}

impl Baud {
    pub fn parse(s: &str) -> Option<Baud> {
        Some(match s {
            "4800" => Baud::B4800,
            "9600" => Baud::B9600,
            "19200" => Baud::B19200,
            "38400" => Baud::B38400,
            "57600" => Baud::B57600,
            "115200" => Baud::B115200,
            _ => return None,
        })
    }

    pub fn rate(self) -> u32 {
        match self {
            Baud::B4800 => 4800,
            Baud::B9600 => 9600,
            Baud::B19200 => 19200,
            Baud::B38400 => 38400,
            Baud::B57600 => 57600,
            Baud::B115200 => 115_200,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Source {
    /// A /dev/serial/by-id path where there is one, so a replug onto a
    /// different ttyACMn is the same source.
    Serial {
        path: String,
        baud: Baud,
    },
    Tcp {
        host: String,
        port: u16,
    },
    Replay(String),
}

impl Source {
    /// omakeel's `--source` SPEC. In boat.toml a serial source names its
    /// baud: the probe found it, and omakeel's 4800 default would be a
    /// silent guess.
    pub fn parse(spec: &str) -> Result<Source, String> {
        let spec = spec.trim();
        if let Some(rest) = spec.strip_prefix("serial:") {
            let Some((path, baud)) = rest.rsplit_once(':') else {
                return Err(format!("{spec}: name the baud, like {spec}:4800"));
            };
            let baud = Baud::parse(baud).ok_or_else(|| {
                format!("{spec}: baud {baud} is not 4800, 9600, 19200, 38400, 57600 or 115200")
            })?;
            if !path.starts_with('/') {
                return Err(format!("{spec}: the device path must be absolute"));
            }
            return Ok(Source::Serial {
                path: path.to_string(),
                baud,
            });
        }
        if let Some(rest) = spec.strip_prefix("tcp:") {
            let bad = || format!("{spec}: expected tcp:HOST:PORT");
            let (host, port) = rest.rsplit_once(':').ok_or_else(bad)?;
            let port = port.parse::<u16>().map_err(|_| bad())?;
            if host.is_empty() || port == 0 {
                return Err(bad());
            }
            return Ok(Source::Tcp {
                host: host.to_string(),
                port,
            });
        }
        if let Some(file) = spec.strip_prefix("replay:")
            && !file.is_empty()
        {
            return Ok(Source::Replay(file.to_string()));
        }
        Err(format!(
            "{spec}: expected serial:PATH:BAUD, tcp:HOST:PORT, replay:FILE or demo"
        ))
    }

    pub fn spec(&self) -> String {
        match self {
            Source::Serial { path, baud } => format!("serial:{path}:{}", baud.rate()),
            Source::Tcp { host, port } => format!("tcp:{host}:{port}"),
            Source::Replay(f) => format!("replay:{f}"),
        }
    }

    pub fn is_serial(&self) -> bool {
        matches!(self, Source::Serial { .. })
    }
}

/// What the hub reads. The demo sail never mixes with real sources.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Hub {
    Demo,
    Live(Sources),
}

/// At least one source, in order.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Sources(Vec<Source>);

impl Hub {
    /// Live sources, or the demo when there are none.
    pub fn from_sources(sources: Vec<Source>) -> Hub {
        if sources.is_empty() {
            Hub::Demo
        } else {
            Hub::Live(Sources(sources))
        }
    }

    pub fn sources(&self) -> &[Source] {
        match self {
            Hub::Demo => &[],
            Hub::Live(Sources(s)) => s,
        }
    }

    /// The `source` values in omakeel's config.
    pub fn specs(&self) -> Vec<String> {
        match self {
            Hub::Demo => vec!["demo".into()],
            Hub::Live(Sources(s)) => s.iter().map(Source::spec).collect(),
        }
    }
}

/// One line of `omakeel probe`.
#[derive(Clone, PartialEq, Debug)]
pub struct Heard {
    /// The first field: a full SPEC on `ok`, else `serial:PATH` or `tcp:HOST:PORT`.
    pub spec: String,
    pub verdict: Verdict,
}

#[derive(Clone, PartialEq, Debug)]
pub enum Verdict {
    Ok {
        source: Source,
        fix: u32,
        ais: u32,
        at: Option<Position>,
    },
    Denied,
    Busy,
    Silent,
    /// Named, but not there: a by-id link for a receiver that was unplugged.
    Missing,
    Unreachable(String),
}

impl Heard {
    /// The device path of a serial line, with or without its baud.
    pub fn path(&self) -> Option<&str> {
        let rest = self.spec.strip_prefix("serial:")?;
        match rest.rsplit_once(':') {
            Some((path, baud)) if Baud::parse(baud).is_some() => Some(path),
            _ => Some(rest),
        }
    }

    /// What the sailor reads: `GPS, 4800 baud, 41 good sentences, fix ...`.
    pub fn describe(&self) -> String {
        match &self.verdict {
            Verdict::Ok {
                source,
                fix,
                ais,
                at,
            } => {
                let what = match (fix > &0, ais > &0) {
                    (true, true) => "GPS and AIS",
                    (true, false) => "GPS",
                    (false, true) => "AIS",
                    (false, false) => "talking",
                };
                let mut out = match source {
                    Source::Serial { baud, .. } => format!("{what}, {} baud", baud.rate()),
                    _ => what.to_string(),
                };
                if *fix > 0 {
                    out += &format!(", {fix} position sentences");
                }
                if *ais > 0 {
                    out += &format!(", {ais} AIS messages");
                }
                if let Some(p) = at {
                    out += &format!(", fix {}", p.show());
                }
                out
            }
            Verdict::Denied => "permission denied".into(),
            Verdict::Busy => "held by another program".into(),
            Verdict::Silent => "silent at every baud".into(),
            Verdict::Missing => "not plugged in".into(),
            Verdict::Unreachable(why) => format!("unreachable: {why}"),
        }
    }
}

/// Reads `omakeel probe` output. A line it doesn't understand comes back
/// as an error carrying the line, so a newer omakeel is reported, not
/// guessed at.
pub fn parse_probe(text: &str) -> Vec<Result<Heard, String>> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(parse_line)
        .collect()
}

fn parse_line(line: &str) -> Result<Heard, String> {
    let bad = || format!("unexpected probe line: {line}");
    let mut words = line.split_whitespace();
    let spec = words.next().ok_or_else(bad)?.to_string();
    let verdict = match words.next().ok_or_else(bad)? {
        "ok" => {
            let source = Source::parse(&spec).map_err(|_| bad())?;
            let (mut fix, mut ais, mut at) = (0, 0, None);
            for w in words {
                let Some((k, v)) = w.split_once('=') else {
                    continue;
                };
                match k {
                    "fix" => fix = v.parse().map_err(|_| bad())?,
                    "ais" => ais = v.parse().map_err(|_| bad())?,
                    "at" => at = Some(Position::parse(v).map_err(|_| bad())?),
                    _ => {}
                }
            }
            Verdict::Ok {
                source,
                fix,
                ais,
                at,
            }
        }
        "denied" => Verdict::Denied,
        "busy" => Verdict::Busy,
        "silent" => Verdict::Silent,
        "missing" => Verdict::Missing,
        "unreachable" => Verdict::Unreachable(words.collect::<Vec<_>>().join(" ")),
        _ => return Err(bad()),
    };
    Ok(Heard { spec, verdict })
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROBE: &str = "\
serial:/dev/serial/by-id/usb-Prolific-port0:4800  ok  fix=41 ais=0 rejected=2 at=47.6870000,-122.4130000
serial:/dev/serial/by-id/usb-dAISy-if00:38400  ok  fix=0 ais=6 rejected=0
serial:/dev/serial/by-id/usb-u-blox-if00  denied
serial:/dev/serial/by-id/usb-FTDI-if00  busy
serial:/dev/serial/by-id/usb-Other-if00  silent
tcp:192.168.1.20:10110  unreachable  connection refused
serial:/dev/serial/by-id/nope  missing
";

    #[test]
    fn every_probe_verdict_parses() {
        let heard: Vec<Heard> = parse_probe(PROBE).into_iter().map(Result::unwrap).collect();
        assert_eq!(
            heard[0].verdict,
            Verdict::Ok {
                source: Source::Serial {
                    path: "/dev/serial/by-id/usb-Prolific-port0".into(),
                    baud: Baud::B4800
                },
                fix: 41,
                ais: 0,
                at: Some(Position::new(47.687, -122.413).unwrap()),
            }
        );
        assert_eq!(
            heard[0].describe(),
            "GPS, 4800 baud, 41 position sentences, fix 47°41.2′N 122°24.8′W"
        );
        assert_eq!(heard[1].describe(), "AIS, 38400 baud, 6 AIS messages");
        assert_eq!(heard[2].verdict, Verdict::Denied);
        assert_eq!(heard[2].path(), Some("/dev/serial/by-id/usb-u-blox-if00"));
        assert_eq!(heard[1].path(), Some("/dev/serial/by-id/usb-dAISy-if00"));
        assert_eq!(heard[3].verdict, Verdict::Busy);
        assert_eq!(heard[4].verdict, Verdict::Silent);
        assert_eq!(
            heard[5].verdict,
            Verdict::Unreachable("connection refused".into())
        );
        assert_eq!(heard[5].path(), None);
        assert_eq!(heard[6].verdict, Verdict::Missing);
        assert_eq!(heard[6].path(), Some("/dev/serial/by-id/nope"));
    }

    #[test]
    fn an_unknown_line_is_reported_not_guessed() {
        let got = parse_probe("serial:/dev/x  maybe\n");
        assert_eq!(
            got,
            vec![Err("unexpected probe line: serial:/dev/x  maybe".into())]
        );
    }

    #[test]
    fn specs_round_trip_and_serial_names_its_baud() {
        for spec in [
            "serial:/dev/serial/by-id/usb-dAISy-if00:38400",
            "tcp:192.168.1.20:10110",
            "replay:/home/me/sail.nmea",
        ] {
            assert_eq!(Source::parse(spec).unwrap().spec(), spec);
        }
        assert!(Source::parse("serial:/dev/ttyUSB0").is_err());
        assert!(Source::parse("serial:/dev/ttyUSB0:1200").is_err());
        assert!(Source::parse("tcp:host").is_err());
        assert!(Source::parse("usb").is_err());
        assert_eq!(Hub::from_sources(vec![]), Hub::Demo);
        assert_eq!(Hub::Demo.specs(), vec!["demo".to_string()]);
    }
}
