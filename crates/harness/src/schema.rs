//! The probe dump's JSON Schema (`harness/dump-schema.json`) and a validator for it.
//!
//! The schema file is the contract with the probe mods, so it is embedded here at build time
//! and validated against directly. The validator implements the JSON Schema 2020-12 keywords
//! the schema uses, and only those: compiling a schema with any other keyword is an error,
//! so a constraint can never be silently ignored.
//!
//! Supported: `$schema`, `$id`, `$comment`, `title`, `description`, `default`, `examples`,
//! `$defs` (at the root), `$ref` (`#` or `#/$defs/<name>`), `type`, `const`, `enum`,
//! `required`, `properties`, `additionalProperties`, `propertyNames`, `items`, `prefixItems`,
//! `minItems`, `maxItems`, `uniqueItems`, `minLength`, `pattern` and `minimum`.
//!
//! A general-purpose validator crate was considered; the one that covers 2020-12 pulls in a
//! dependency under MIT-0, which `deny.toml` does not allow.

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value;

use crate::canon::to_canonical_string;

/// The text of `harness/dump-schema.json`, embedded at build time.
pub const DUMP_SCHEMA_JSON: &str = include_str!("../../../harness/dump-schema.json");

/// The dump format version this build reads and validates (`"schema"` in the dump).
pub const DUMP_SCHEMA_VERSION: u64 = 1;

/// At most this many violations are collected per validation.
const MAX_VIOLATIONS: usize = 50;

/// Nesting limit for `$ref` chains that do not descend into the instance.
const MAX_REF_DEPTH: usize = 64;

/// A schema that could not be compiled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaError {
    /// JSON Pointer into the schema.
    pub pointer: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for SchemaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "schema {}: {}",
            pointer_or_root(&self.pointer),
            self.message
        )
    }
}

impl std::error::Error for SchemaError {}

/// One place where an instance breaks the schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// JSON Pointer into the instance.
    pub pointer: String,
    /// What is wrong.
    pub message: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", pointer_or_root(&self.pointer), self.message)
    }
}

fn pointer_or_root(pointer: &str) -> &str {
    if pointer.is_empty() { "/" } else { pointer }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JsonType {
    Null,
    Boolean,
    Object,
    Array,
    Number,
    Integer,
    String,
}

impl JsonType {
    fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "null" => JsonType::Null,
            "boolean" => JsonType::Boolean,
            "object" => JsonType::Object,
            "array" => JsonType::Array,
            "number" => JsonType::Number,
            "integer" => JsonType::Integer,
            "string" => JsonType::String,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            JsonType::Null => "null",
            JsonType::Boolean => "boolean",
            JsonType::Object => "object",
            JsonType::Array => "array",
            JsonType::Number => "number",
            JsonType::Integer => "integer",
            JsonType::String => "string",
        }
    }

    fn matches(self, value: &Value) -> bool {
        match self {
            JsonType::Null => value.is_null(),
            JsonType::Boolean => value.is_boolean(),
            JsonType::Object => value.is_object(),
            JsonType::Array => value.is_array(),
            JsonType::Number => value.is_number(),
            JsonType::Integer => is_integer(value),
            JsonType::String => value.is_string(),
        }
    }
}

fn is_integer(value: &Value) -> bool {
    match value {
        Value::Number(n) => {
            n.is_i64()
                || n.is_u64()
                || n.as_f64()
                    .is_some_and(|f| f.is_finite() && f.fract() == 0.0)
        }
        _ => false,
    }
}

#[derive(Debug)]
enum Node {
    Always(bool),
    Rules(Box<Rules>),
}

#[derive(Debug, Default)]
struct Rules {
    reference: Option<String>,
    types: Vec<JsonType>,
    constant: Option<Value>,
    enumeration: Option<Vec<Value>>,
    required: Vec<String>,
    properties: Vec<(String, Node)>,
    additional: Option<Node>,
    property_names: Option<Node>,
    items: Option<Node>,
    prefix_items: Vec<Node>,
    min_items: Option<u64>,
    max_items: Option<u64>,
    unique_items: bool,
    min_length: Option<u64>,
    pattern: Option<Regex>,
    minimum: Option<f64>,
}

/// A compiled schema.
#[derive(Debug)]
pub struct Schema {
    root: Node,
    defs: BTreeMap<String, Node>,
}

impl Schema {
    /// Compiles a schema document.
    pub fn compile(document: &Value) -> Result<Self, SchemaError> {
        let mut defs = BTreeMap::new();
        if let Some(raw) = document.get("$defs") {
            let map = raw
                .as_object()
                .ok_or_else(|| err("/$defs", "must be an object"))?;
            for (name, def) in map {
                let pointer = format!("/$defs/{name}");
                defs.insert(name.clone(), compile_node(def, &pointer, false)?);
            }
        }
        let schema = Self {
            root: compile_node(document, "", true)?,
            defs,
        };
        schema.check_refs(&schema.root, "")?;
        for (name, def) in &schema.defs {
            schema.check_refs(def, &format!("/$defs/{name}"))?;
        }
        Ok(schema)
    }

    /// The embedded probe dump schema, compiled once.
    pub fn dump() -> Result<&'static Schema, SchemaError> {
        static DUMP: OnceLock<Result<Schema, SchemaError>> = OnceLock::new();
        DUMP.get_or_init(|| {
            let document: Value = serde_json::from_str(DUMP_SCHEMA_JSON)
                .map_err(|e| err("", &format!("harness/dump-schema.json is not JSON: {e}")))?;
            Schema::compile(&document)
        })
        .as_ref()
        .map_err(Clone::clone)
    }

    /// Validates `instance`, returning every violation found (up to a limit).
    pub fn validate(&self, instance: &Value) -> Result<(), Vec<Violation>> {
        let mut violations = Vec::new();
        let mut pointer = String::new();
        self.check(&self.root, instance, &mut pointer, 0, &mut violations);
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }

    fn check_refs(&self, node: &Node, pointer: &str) -> Result<(), SchemaError> {
        let Node::Rules(rules) = node else {
            return Ok(());
        };
        if let Some(reference) = &rules.reference
            && reference != "#"
            && !self.defs.contains_key(reference)
        {
            return Err(err(
                pointer,
                &format!("$ref to an unknown definition {reference:?}"),
            ));
        }
        let children = rules
            .properties
            .iter()
            .map(|(_, n)| n)
            .chain(rules.additional.iter())
            .chain(rules.property_names.iter())
            .chain(rules.items.iter())
            .chain(rules.prefix_items.iter());
        for child in children {
            self.check_refs(child, pointer)?;
        }
        Ok(())
    }

    fn check(
        &self,
        node: &Node,
        value: &Value,
        pointer: &mut String,
        ref_depth: usize,
        out: &mut Vec<Violation>,
    ) {
        if out.len() >= MAX_VIOLATIONS {
            return;
        }
        let rules = match node {
            Node::Always(true) => return,
            Node::Always(false) => {
                push(out, pointer, "no value is allowed here".to_owned());
                return;
            }
            Node::Rules(rules) => rules,
        };
        if let Some(reference) = &rules.reference {
            if ref_depth >= MAX_REF_DEPTH {
                push(out, pointer, "schema $ref chain is too deep".to_owned());
                return;
            }
            let target = if reference == "#" {
                Some(&self.root)
            } else {
                self.defs.get(reference)
            };
            if let Some(target) = target {
                self.check(target, value, pointer, ref_depth + 1, out);
            }
        }
        if !rules.types.is_empty() && !rules.types.iter().any(|t| t.matches(value)) {
            let expected: Vec<&str> = rules.types.iter().map(|t| t.name()).collect();
            push(
                out,
                pointer,
                format!(
                    "expected {}, found {}",
                    expected.join(" or "),
                    type_of(value)
                ),
            );
            return;
        }
        if let Some(constant) = &rules.constant
            && to_canonical_string(constant) != to_canonical_string(value)
        {
            push(
                out,
                pointer,
                format!("must be {}", to_canonical_string(constant)),
            );
        }
        if let Some(options) = &rules.enumeration {
            let found = to_canonical_string(value);
            if !options.iter().any(|o| to_canonical_string(o) == found) {
                let names: Vec<String> = options.iter().map(to_canonical_string).collect();
                push(out, pointer, format!("must be one of {}", names.join(", ")));
            }
        }
        match value {
            Value::Object(map) => self.check_object(rules, map, pointer, out),
            Value::Array(items) => self.check_array(rules, items, pointer, out),
            Value::String(s) => check_string(rules, s, pointer, out),
            Value::Number(_) => {
                if let (Some(minimum), Some(found)) = (rules.minimum, value.as_f64())
                    && found < minimum
                {
                    push(out, pointer, format!("must be at least {minimum}"));
                }
            }
            _ => {}
        }
    }

    fn check_object(
        &self,
        rules: &Rules,
        map: &serde_json::Map<String, Value>,
        pointer: &mut String,
        out: &mut Vec<Violation>,
    ) {
        for key in &rules.required {
            if !map.contains_key(key) {
                push(out, pointer, format!("missing required property {key:?}"));
            }
        }
        for (key, inner) in map {
            let len = pointer.len();
            pointer.push('/');
            pointer.push_str(&key.replace('~', "~0").replace('/', "~1"));
            if let Some(names) = &rules.property_names {
                self.check(names, &Value::String(key.clone()), pointer, 0, out);
            }
            match rules.properties.iter().find(|(name, _)| name == key) {
                Some((_, node)) => self.check(node, inner, pointer, 0, out),
                None => {
                    if let Some(additional) = &rules.additional {
                        if matches!(additional, Node::Always(false)) {
                            push(out, pointer, "property is not allowed here".to_owned());
                        } else {
                            self.check(additional, inner, pointer, 0, out);
                        }
                    }
                }
            }
            pointer.truncate(len);
            if out.len() >= MAX_VIOLATIONS {
                return;
            }
        }
    }

    fn check_array(
        &self,
        rules: &Rules,
        items: &[Value],
        pointer: &mut String,
        out: &mut Vec<Violation>,
    ) {
        let count = items.len() as u64;
        if let Some(min) = rules.min_items
            && count < min
        {
            push(
                out,
                pointer,
                format!("needs at least {min} items, has {count}"),
            );
        }
        if let Some(max) = rules.max_items
            && count > max
        {
            push(
                out,
                pointer,
                format!("allows at most {max} items, has {count}"),
            );
        }
        for (i, item) in items.iter().enumerate() {
            let node = rules.prefix_items.get(i).or(rules.items.as_ref());
            if let Some(node) = node {
                let len = pointer.len();
                pointer.push('/');
                pointer.push_str(&i.to_string());
                if matches!(node, Node::Always(false)) {
                    push(
                        out,
                        pointer,
                        "no item is allowed at this position".to_owned(),
                    );
                } else {
                    self.check(node, item, pointer, 0, out);
                }
                pointer.truncate(len);
            }
            if out.len() >= MAX_VIOLATIONS {
                return;
            }
        }
        if rules.unique_items {
            let mut seen = HashSet::with_capacity(items.len());
            for (i, item) in items.iter().enumerate() {
                if !seen.insert(to_canonical_string(item)) {
                    push(
                        out,
                        pointer,
                        format!("items must be unique; item {i} repeats an earlier one"),
                    );
                    break;
                }
            }
        }
    }
}

fn check_string(rules: &Rules, s: &str, pointer: &str, out: &mut Vec<Violation>) {
    if let Some(min) = rules.min_length
        && (s.chars().count() as u64) < min
    {
        push(
            out,
            pointer,
            format!("must be at least {min} characters long"),
        );
    }
    if let Some(pattern) = &rules.pattern
        && !pattern.is_match(s)
    {
        push(
            out,
            pointer,
            format!("{s:?} does not match {}", pattern.as_str()),
        );
    }
}

fn push(out: &mut Vec<Violation>, pointer: &str, message: String) {
    if out.len() < MAX_VIOLATIONS {
        out.push(Violation {
            pointer: pointer.to_owned(),
            message,
        });
    }
}

fn type_of(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) if is_integer(value) => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn err(pointer: &str, message: &str) -> SchemaError {
    SchemaError {
        pointer: pointer.to_owned(),
        message: message.to_owned(),
    }
}

const ANNOTATIONS: [&str; 7] = [
    "$schema",
    "$id",
    "$comment",
    "title",
    "description",
    "default",
    "examples",
];

fn compile_node(value: &Value, pointer: &str, is_root: bool) -> Result<Node, SchemaError> {
    let map = match value {
        Value::Bool(b) => return Ok(Node::Always(*b)),
        Value::Object(map) => map,
        _ => return Err(err(pointer, "a schema must be an object or a boolean")),
    };
    let mut rules = Rules::default();
    for (keyword, arg) in map {
        let here = format!("{pointer}/{keyword}");
        match keyword.as_str() {
            k if ANNOTATIONS.contains(&k) => {}
            "$defs" if is_root => {}
            "$ref" => {
                let target = arg.as_str().ok_or_else(|| err(&here, "must be a string"))?;
                let reference = if target == "#" {
                    "#".to_owned()
                } else {
                    target
                        .strip_prefix("#/$defs/")
                        .filter(|name| !name.is_empty() && !name.contains('/'))
                        .ok_or_else(|| {
                            err(&here, "only \"#\" and \"#/$defs/<name>\" are supported")
                        })?
                        .to_owned()
                };
                rules.reference = Some(reference);
            }
            "type" => {
                let names: Vec<&Value> = match arg {
                    Value::Array(items) => items.iter().collect(),
                    other => vec![other],
                };
                for name in names {
                    let t = name
                        .as_str()
                        .and_then(JsonType::parse)
                        .ok_or_else(|| err(&here, "unknown type"))?;
                    rules.types.push(t);
                }
            }
            "const" => rules.constant = Some(arg.clone()),
            "enum" => {
                let options = arg
                    .as_array()
                    .ok_or_else(|| err(&here, "must be an array"))?;
                rules.enumeration = Some(options.clone());
            }
            "required" => {
                let names = arg
                    .as_array()
                    .ok_or_else(|| err(&here, "must be an array"))?;
                for name in names {
                    let name = name
                        .as_str()
                        .ok_or_else(|| err(&here, "must hold strings"))?;
                    rules.required.push(name.to_owned());
                }
            }
            "properties" => {
                let props = arg
                    .as_object()
                    .ok_or_else(|| err(&here, "must be an object"))?;
                for (name, schema) in props {
                    let node = compile_node(schema, &format!("{here}/{name}"), false)?;
                    rules.properties.push((name.clone(), node));
                }
            }
            "additionalProperties" => rules.additional = Some(compile_node(arg, &here, false)?),
            "propertyNames" => rules.property_names = Some(compile_node(arg, &here, false)?),
            "items" => rules.items = Some(compile_node(arg, &here, false)?),
            "prefixItems" => {
                let list = arg
                    .as_array()
                    .ok_or_else(|| err(&here, "must be an array"))?;
                for (i, schema) in list.iter().enumerate() {
                    rules
                        .prefix_items
                        .push(compile_node(schema, &format!("{here}/{i}"), false)?);
                }
            }
            "minItems" => rules.min_items = Some(non_negative(arg, &here)?),
            "maxItems" => rules.max_items = Some(non_negative(arg, &here)?),
            "minLength" => rules.min_length = Some(non_negative(arg, &here)?),
            "uniqueItems" => {
                rules.unique_items = arg
                    .as_bool()
                    .ok_or_else(|| err(&here, "must be a boolean"))?;
            }
            "pattern" => {
                let source = arg.as_str().ok_or_else(|| err(&here, "must be a string"))?;
                let regex = Regex::new(source)
                    .map_err(|e| err(&here, &format!("not a valid pattern: {e}")))?;
                rules.pattern = Some(regex);
            }
            "minimum" => {
                rules.minimum = Some(arg.as_f64().ok_or_else(|| err(&here, "must be a number"))?);
            }
            other => {
                return Err(err(
                    &here,
                    &format!("keyword {other:?} is not supported by this validator"),
                ));
            }
        }
    }
    Ok(Node::Rules(Box::new(rules)))
}

fn non_negative(arg: &Value, pointer: &str) -> Result<u64, SchemaError> {
    arg.as_u64()
        .ok_or_else(|| err(pointer, "must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn messages(schema: &Value, instance: &Value) -> Vec<String> {
        match Schema::compile(schema).unwrap().validate(instance) {
            Ok(()) => Vec::new(),
            Err(v) => v.iter().map(ToString::to_string).collect(),
        }
    }

    #[test]
    fn the_embedded_dump_schema_compiles() {
        let schema = Schema::dump().unwrap();
        // The schema is itself an object with the keys the harness relies on.
        let doc: Value = serde_json::from_str(DUMP_SCHEMA_JSON).unwrap();
        assert_eq!(
            doc["properties"]["schema"]["const"],
            json!(DUMP_SCHEMA_VERSION)
        );
        assert!(schema.validate(&json!({})).is_err());
    }

    #[test]
    fn unknown_keywords_are_refused() {
        let error = Schema::compile(&json!({"type": "object", "maxProperties": 3})).unwrap_err();
        assert!(error.message.contains("maxProperties"), "{error}");
        let error = Schema::compile(&json!({"$ref": "https://example.com/s.json"})).unwrap_err();
        assert!(error.message.contains("supported"), "{error}");
        let error = Schema::compile(&json!({"$ref": "#/$defs/missing"})).unwrap_err();
        assert!(error.message.contains("unknown definition"), "{error}");
        assert!(Schema::compile(&json!({"pattern": "("})).is_err());
        assert!(Schema::compile(&json!({"type": "thing"})).is_err());
        assert!(Schema::compile(&json!(3)).is_err());
    }

    #[test]
    fn object_keywords() {
        let schema = json!({
            "type": "object",
            "required": ["a"],
            "additionalProperties": false,
            "properties": {"a": {"type": "integer", "minimum": 0}, "b": {"type": "string"}}
        });
        assert!(messages(&schema, &json!({"a": 1, "b": "x"})).is_empty());
        assert!(
            messages(&schema, &json!({"a": 1.0})).is_empty(),
            "1.0 is an integer"
        );
        assert_eq!(
            messages(&schema, &json!({})),
            ["/: missing required property \"a\""]
        );
        assert_eq!(
            messages(&schema, &json!({"a": -1})),
            ["/a: must be at least 0"]
        );
        assert_eq!(
            messages(&schema, &json!({"a": 1.5})),
            ["/a: expected integer, found number"]
        );
        assert_eq!(
            messages(&schema, &json!({"a": 1, "c": 2})),
            ["/c: property is not allowed here"]
        );
        assert_eq!(
            messages(&schema, &json!([])),
            ["/: expected object, found array"]
        );
    }

    #[test]
    fn property_names_and_patterns() {
        let schema = json!({
            "type": "object",
            "propertyNames": {"type": "string", "pattern": "^[a-z]+:[a-z/]+$"},
            "additionalProperties": {"type": "array", "items": {"type": "string", "minLength": 2}, "uniqueItems": true}
        });
        assert!(messages(&schema, &json!({"minecraft:item": ["ab", "cd"]})).is_empty());
        let found = messages(
            &schema,
            &json!({"Bad Key": ["ab"], "a:b": ["ab", "ab", "x"]}),
        );
        assert!(
            found
                .iter()
                .any(|m| m.starts_with("/Bad Key: \"Bad Key\" does not match")),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|m| m == "/a:b/2: must be at least 2 characters long"),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|m| m.contains("items must be unique; item 1")),
            "{found:?}"
        );
    }

    #[test]
    fn tuples_consts_enums_and_refs() {
        let schema = json!({
            "$defs": {"pair": {"type": "array", "prefixItems": [{"type": "integer"}, {"type": "integer"}], "items": false, "minItems": 2, "maxItems": 2}},
            "type": "object",
            "properties": {
                "v": {"const": 1},
                "loader": {"enum": ["fabric", "neoforge"]},
                "chunks": {"type": "array", "items": {"$ref": "#/$defs/pair"}}
            }
        });
        assert!(
            messages(
                &schema,
                &json!({"v": 1, "loader": "fabric", "chunks": [[0, 0], [-1, 2]]})
            )
            .is_empty()
        );
        let found = messages(
            &schema,
            &json!({"v": 2, "loader": "forge", "chunks": [[0], [1, 2, 3], ["a", 0]]}),
        );
        assert!(found.contains(&"/v: must be 1".to_owned()), "{found:?}");
        assert!(
            found.contains(&"/loader: must be one of \"fabric\", \"neoforge\"".to_owned()),
            "{found:?}"
        );
        assert!(
            found.contains(&"/chunks/0: needs at least 2 items, has 1".to_owned()),
            "{found:?}"
        );
        assert!(
            found.contains(&"/chunks/1: allows at most 2 items, has 3".to_owned()),
            "{found:?}"
        );
        assert!(
            found.contains(&"/chunks/1/2: no item is allowed at this position".to_owned()),
            "{found:?}"
        );
        assert!(
            found.contains(&"/chunks/2/0: expected integer, found string".to_owned()),
            "{found:?}"
        );
    }

    #[test]
    fn violations_are_capped() {
        let schema = json!({"type": "array", "items": {"type": "string"}});
        let instance = Value::Array((0..500).map(|i| json!(i)).collect());
        let found = Schema::compile(&schema)
            .unwrap()
            .validate(&instance)
            .unwrap_err();
        assert_eq!(found.len(), MAX_VIOLATIONS);
    }

    #[test]
    fn self_reference_is_bounded() {
        let schema = json!({"$defs": {"a": {"$ref": "#/$defs/a"}}, "$ref": "#/$defs/a"});
        let found = Schema::compile(&schema)
            .unwrap()
            .validate(&json!(1))
            .unwrap_err();
        assert!(found[0].message.contains("too deep"));
    }
}
