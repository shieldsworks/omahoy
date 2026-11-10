//! A JSON value and its writer, for `omahoy gui`. Output only: the gui
//! reads boat.toml text, never JSON.

#[derive(Clone, PartialEq, Debug)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    /// Keys keep the order they were given.
    Obj(Vec<(String, Json)>),
}

impl Json {
    pub fn obj(fields: Vec<(&str, Json)>) -> Json {
        Json::Obj(
            fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        )
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut String) {
        match self {
            Json::Null => out.push_str("null"),
            Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Json::Num(n) => out.push_str(&number(*n)),
            Json::Str(s) => string(s, out),
            Json::Arr(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write(out);
                }
                out.push(']');
            }
            Json::Obj(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    string(k, out);
                    out.push(':');
                    v.write(out);
                }
                out.push('}');
            }
        }
    }
}

/// Rust prints an f64 as its shortest text that reads back the same, and
/// never in exponent form, so `2.0` is `2` and `37.8667` is `37.8667`.
/// JSON has no NaN or infinity.
fn number(n: f64) -> String {
    if !n.is_finite() {
        return "null".into();
    }
    if n == 0.0 {
        return "0".into();
    }
    n.to_string()
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

impl From<bool> for Json {
    fn from(b: bool) -> Json {
        Json::Bool(b)
    }
}

impl From<f64> for Json {
    fn from(n: f64) -> Json {
        Json::Num(n)
    }
}

impl From<usize> for Json {
    fn from(n: usize) -> Json {
        Json::Num(n as f64)
    }
}

impl From<u32> for Json {
    fn from(n: u32) -> Json {
        Json::Num(f64::from(n))
    }
}

impl From<&str> for Json {
    fn from(s: &str) -> Json {
        Json::Str(s.to_string())
    }
}

impl From<String> for Json {
    fn from(s: String) -> Json {
        Json::Str(s)
    }
}

/// `None` is `null`.
impl<T: Into<Json>> From<Option<T>> for Json {
    fn from(o: Option<T>) -> Json {
        o.map_or(Json::Null, Into::into)
    }
}

impl<T: Into<Json>> From<Vec<T>> for Json {
    fn from(v: Vec<T>) -> Json {
        Json::Arr(v.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        let s = Json::from("a\"b\\c\nd\te\u{1}f\u{1f}é′");
        assert_eq!(s.render(), r#""a\"b\\c\nd\te\u0001f\u001fé′""#);
        assert_eq!(Json::from("\r\u{8}\u{c}").render(), r#""\r\b\f""#);
    }

    #[test]
    fn integers_have_no_point_and_decimals_print_as_written() {
        assert_eq!(Json::from(2.0).render(), "2");
        assert_eq!(Json::from(-123.413).render(), "-123.413");
        assert_eq!(Json::from(37.8667).render(), "37.8667");
        assert_eq!(Json::from(0.58).render(), "0.58");
        assert_eq!(Json::from(-0.0).render(), "0");
        assert_eq!(Json::from(1e21).render(), "1000000000000000000000");
        assert_eq!(Json::from(f64::NAN).render(), "null");
        assert_eq!(Json::from(f64::INFINITY).render(), "null");
        assert_eq!(Json::from(7_u32).render(), "7");
    }

    #[test]
    fn objects_keep_their_order_and_nest() {
        let j = Json::obj(vec![
            ("z", Json::from(true)),
            ("a", Json::from(vec![1_u32, 2])),
            ("m", Json::from(None::<f64>)),
            ("o", Json::obj(vec![("k\"", Json::from("v"))])),
            ("e", Json::Arr(vec![])),
        ]);
        assert_eq!(
            j.render(),
            r#"{"z":true,"a":[1,2],"m":null,"o":{"k\"":"v"},"e":[]}"#
        );
    }
}
