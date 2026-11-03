//! Canonical JSON.
//!
//! Two loaders can encode the same recipe with keys in a different order, or write `1.0` where
//! the other writes `1`. Canonicalisation removes those differences and nothing else:
//!
//! - object keys are sorted by Unicode code point (the order of Rust's `str`);
//! - a float with no fractional part that fits in an `i64` becomes that integer, and `-0.0`
//!   becomes `0`;
//! - array order is kept, because it carries meaning (a shaped recipe's pattern, a loot pool's
//!   entries);
//! - strings are compared exactly.
//!
//! [`to_canonical_string`] writes the compact form, which is what the diff compares and what
//! report hashes are computed over. It sorts keys itself, so the result does not depend on
//! whether `serde_json` was built with `preserve_order`.

use serde_json::{Map, Number, Value};

/// The largest magnitude at which every integer is exactly representable as an `f64` (2^53).
const MAX_EXACT_INT: f64 = 9_007_199_254_740_992.0;

/// Returns the canonical form of `value`.
pub fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for key in keys {
                if let Some(inner) = map.get(key) {
                    out.insert(key.clone(), canonicalize(inner));
                }
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize).collect()),
        Value::Number(number) => Value::Number(canonical_number(number)),
        other => other.clone(),
    }
}

fn canonical_number(number: &Number) -> Number {
    if number.is_i64() || number.is_u64() {
        return number.clone();
    }
    match number.as_f64() {
        // The cast is exact: `f` is integral and |f| <= 2^53 (and -0.0 becomes 0).
        Some(f) if f.fract() == 0.0 && f.abs() <= MAX_EXACT_INT => Number::from(f as i64),
        _ => number.clone(),
    }
}

/// Writes `value` as compact canonical JSON (see the module docs).
pub fn to_canonical_string(value: &Value) -> String {
    let mut out = String::new();
    write_canonical(&canonicalize(value), &mut out);
    out
}

fn write_canonical(value: &Value, out: &mut String) {
    match value {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&n.to_string()),
        Value::String(s) => write_string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            out.push('{');
            for (i, (key, inner)) in entries.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(key, out);
                out.push(':');
                write_canonical(inner, out);
            }
            out.push('}');
        }
    }
}

fn write_string(s: &str, out: &mut String) {
    // serde_json's string escaping is deterministic and minimal; serialising a &str cannot fail.
    match serde_json::to_string(s) {
        Ok(escaped) => out.push_str(&escaped),
        Err(_) => out.push_str("\"\""),
    }
}

/// Whether two JSON values are equal after canonicalisation.
pub fn canonical_eq(a: &Value, b: &Value) -> bool {
    canonicalize(a) == canonicalize(b)
}

/// The first place two canonical values differ, as a JSON Pointer with a short description,
/// or `None` when they are equal. Used to explain a changed recipe or loot table in a report.
pub fn first_difference(a: &Value, b: &Value) -> Option<String> {
    first_difference_at(&canonicalize(a), &canonicalize(b), &mut String::new())
}

fn first_difference_at(a: &Value, b: &Value, path: &mut String) -> Option<String> {
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let len = path.len();
                path.push('/');
                path.push_str(&escape_pointer(key));
                let found = match (ma.get(key), mb.get(key)) {
                    (Some(x), Some(y)) => first_difference_at(x, y, path),
                    (Some(_), None) => Some(format!("{path}: only in the reference")),
                    (None, Some(_)) => Some(format!("{path}: only in the candidate")),
                    (None, None) => None,
                };
                path.truncate(len);
                if found.is_some() {
                    return found;
                }
            }
            None
        }
        (Value::Array(xa), Value::Array(xb)) => {
            for (i, (x, y)) in xa.iter().zip(xb.iter()).enumerate() {
                let len = path.len();
                path.push('/');
                path.push_str(&i.to_string());
                let found = first_difference_at(x, y, path);
                path.truncate(len);
                if found.is_some() {
                    return found;
                }
            }
            (xa.len() != xb.len()).then(|| {
                format!(
                    "{}: {} items in the reference, {} in the candidate",
                    pointer_or_root(path),
                    xa.len(),
                    xb.len()
                )
            })
        }
        _ if a == b => None,
        _ => Some(format!(
            "{}: {} in the reference, {} in the candidate",
            pointer_or_root(path),
            short(a),
            short(b)
        )),
    }
}

/// At most this many pointers are listed by [`differing_pointers`]; past it the list ends with
/// [`TRUNCATED_POINTER`].
pub const MAX_POINTERS: usize = 256;

/// The last entry of a [`differing_pointers`] list that was cut short.
pub const TRUNCATED_POINTER: &str = "/...";

/// Every place two canonical values differ, as JSON Pointers with every array index written
/// `*`, sorted and without repeats. A key on one side only is its own pointer; arrays of
/// different lengths add `<array pointer>#length`.
///
/// Wildcarded indices make the list a shape of the difference: "NeoForge names every loot
/// pool" is `/pools/*/name` however many pools a table has, so a mod that adds a pool on both
/// loaders still matches the loader noise, while a pool missing on one side adds
/// `/pools#length`, which the noise does not have.
pub fn differing_pointers(a: &Value, b: &Value) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    pointers_at(
        &canonicalize(a),
        &canonicalize(b),
        &mut String::new(),
        &mut out,
    );
    let mut list: Vec<String> = out.into_iter().collect();
    if list.len() > MAX_POINTERS {
        list.truncate(MAX_POINTERS);
        list.push(TRUNCATED_POINTER.to_owned());
    }
    list
}

fn pointers_at(
    a: &Value,
    b: &Value,
    path: &mut String,
    out: &mut std::collections::BTreeSet<String>,
) {
    if out.len() > MAX_POINTERS {
        return;
    }
    match (a, b) {
        (Value::Object(ma), Value::Object(mb)) => {
            let mut keys: Vec<&String> = ma.keys().chain(mb.keys()).collect();
            keys.sort();
            keys.dedup();
            for key in keys {
                let len = path.len();
                path.push('/');
                path.push_str(&escape_pointer(key));
                match (ma.get(key), mb.get(key)) {
                    (Some(x), Some(y)) => pointers_at(x, y, path, out),
                    (None, None) => {}
                    _ => {
                        out.insert(path.clone());
                    }
                }
                path.truncate(len);
            }
        }
        (Value::Array(xa), Value::Array(xb)) => {
            let len = path.len();
            path.push_str("/*");
            for (x, y) in xa.iter().zip(xb.iter()) {
                pointers_at(x, y, path, out);
            }
            path.truncate(len);
            if xa.len() != xb.len() {
                out.insert(format!("{path}#length"));
            }
        }
        _ if a == b => {}
        _ => {
            out.insert(pointer_or_root(path).to_owned());
        }
    }
}

fn pointer_or_root(path: &str) -> &str {
    if path.is_empty() { "/" } else { path }
}

fn escape_pointer(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn short(value: &Value) -> String {
    let text = to_canonical_string(value);
    const LIMIT: usize = 80;
    if text.chars().count() <= LIMIT {
        text
    } else {
        let cut: String = text.chars().take(LIMIT).collect();
        format!("{cut}...")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn key_order_does_not_matter() {
        let a: Value =
            serde_json::from_str(r#"{"b":1,"a":{"y":2,"x":[3,{"q":1,"p":2}]}}"#).unwrap();
        let b: Value =
            serde_json::from_str(r#"{"a":{"x":[3,{"p":2,"q":1}],"y":2},"b":1}"#).unwrap();
        assert!(canonical_eq(&a, &b));
        assert_eq!(
            to_canonical_string(&a),
            r#"{"a":{"x":[3,{"p":2,"q":1}],"y":2},"b":1}"#
        );
        assert_eq!(to_canonical_string(&a), to_canonical_string(&b));
        assert_eq!(first_difference(&a, &b), None);
    }

    #[test]
    fn integral_floats_become_integers() {
        let a = json!({"count": 1.0, "chance": 0.5, "neg": -3.0, "zero": -0.0});
        let b = json!({"count": 1, "chance": 0.5, "neg": -3, "zero": 0});
        assert!(canonical_eq(&a, &b));
        assert_eq!(
            to_canonical_string(&a),
            r#"{"chance":0.5,"count":1,"neg":-3,"zero":0}"#
        );
        // Beyond 2^53 a float is not silently turned into an integer.
        let big = json!(1.0e300);
        assert_eq!(canonicalize(&big), big);
    }

    #[test]
    fn array_order_and_string_case_matter() {
        assert!(!canonical_eq(&json!([1, 2]), &json!([2, 1])));
        assert!(!canonical_eq(&json!("Stone"), &json!("stone")));
        assert!(!canonical_eq(&json!(1), &json!("1")));
        assert!(!canonical_eq(&json!(null), &json!(false)));
    }

    #[test]
    fn strings_are_escaped_like_json() {
        let v = json!({"k\"ey": "line\nbreak \u{e9}"});
        let s = to_canonical_string(&v);
        assert_eq!(s, "{\"k\\\"ey\":\"line\\nbreak \u{e9}\"}");
        let back: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(back, v);
    }

    #[test]
    fn first_difference_names_the_pointer() {
        let a = json!({"result": {"count": 2, "id": "minecraft:stick"}, "pattern": ["#", "#"]});
        let b = json!({"result": {"count": 1, "id": "minecraft:stick"}, "pattern": ["#", "#"]});
        assert_eq!(
            first_difference(&a, &b).unwrap(),
            "/result/count: 2 in the reference, 1 in the candidate"
        );
        let c = json!({"pattern": ["#"], "result": {"count": 2, "id": "minecraft:stick"}});
        assert_eq!(
            first_difference(&a, &c).unwrap(),
            "/pattern: 2 items in the reference, 1 in the candidate"
        );
        let d = json!({"a/b": 1});
        let e = json!({});
        assert_eq!(
            first_difference(&d, &e).unwrap(),
            "/a~1b: only in the reference"
        );
        assert_eq!(
            first_difference(&json!(1), &json!(2)).unwrap(),
            "/: 1 in the reference, 2 in the candidate"
        );
    }

    #[test]
    fn differing_pointers_wildcard_array_indices() {
        // The shape NeoForge gives vanilla loot tables: every pool gets a name.
        let fabric = json!({"pools": [{"rolls": 1}, {"rolls": 2}]});
        let neoforge =
            json!({"pools": [{"rolls": 1, "name": "main"}, {"rolls": 2, "name": "pool1"}]});
        assert_eq!(differing_pointers(&fabric, &neoforge), ["/pools/*/name"]);
        let lost_pool = json!({"pools": [{"rolls": 1, "name": "main"}]});
        assert_eq!(
            differing_pointers(&fabric, &lost_pool),
            ["/pools#length", "/pools/*/name"]
        );
        let recount = json!({"pools": [{"rolls": 3}, {"rolls": 2}]});
        assert_eq!(differing_pointers(&fabric, &recount), ["/pools/*/rolls"]);
        assert!(
            differing_pointers(&fabric, &json!({"pools": [{"rolls": 1.0}, {"rolls": 2}]}))
                .is_empty()
        );
        assert_eq!(differing_pointers(&json!(1), &json!("1")), ["/"]);
    }

    #[test]
    fn differing_pointers_are_capped() {
        let a = Value::Object((0..400).map(|i| (format!("k{i}"), json!(i))).collect());
        let b = json!({});
        let pointers = differing_pointers(&a, &b);
        assert_eq!(pointers.len(), MAX_POINTERS + 1);
        assert_eq!(pointers.last().map(String::as_str), Some(TRUNCATED_POINTER));
    }
}
