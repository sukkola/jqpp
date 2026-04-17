use super::CompletionItem;

/// Which JSON input types a built-in function accepts.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum InputType {
    /// Works on every JSON type (string, number, array, object, boolean, null).
    Any,
    /// Works on everything *except* boolean.
    /// Use this for functions like `length` that error on `true`/`false`.
    NonBoolean,
    String,
    Number,
    Array,
    Object,
    StringOrArray, // length, indices, contains, …
    ArrayOrObject, // keys, values, has, map_values, …
}

impl InputType {
    /// Returns true when `jq_type` (the runtime type string "string", "number",
    /// "array", "object", "boolean", "null") is compatible with this filter.
    pub fn compatible_with(self, jq_type: &str) -> bool {
        match self {
            InputType::Any => true,
            InputType::NonBoolean => jq_type != "boolean",
            InputType::String => jq_type == "string",
            InputType::Number => jq_type == "number",
            InputType::Array => jq_type == "array",
            InputType::Object => jq_type == "object",
            InputType::StringOrArray => jq_type == "string" || jq_type == "array",
            InputType::ArrayOrObject => jq_type == "array" || jq_type == "object",
        }
    }
}

/// (name, insert_text, detail, input_type)
/// `insert_text` includes parentheses / arguments where jq requires them.
const BUILTINS: &[(&str, &str, &str, InputType)] = &[
    // ── strings ──────────────────────────────────────────────────────────────
    (
        "ascii_downcase",
        "ascii_downcase",
        "string → lowercase",
        InputType::String,
    ),
    (
        "ascii_upcase",
        "ascii_upcase",
        "string → uppercase",
        InputType::String,
    ),
    (
        "ltrimstr",
        "ltrimstr(\"\")",
        "remove prefix",
        InputType::String,
    ),
    (
        "rtrimstr",
        "rtrimstr(\"\")",
        "remove suffix",
        InputType::String,
    ),
    (
        "startswith",
        "startswith(\"\")",
        "string → bool",
        InputType::String,
    ),
    (
        "endswith",
        "endswith(\"\")",
        "string → bool",
        InputType::String,
    ),
    (
        "split",
        "split(\",\")",
        "split on separator",
        InputType::String,
    ),
    (
        "test",
        "test(\"\")",
        "regex match → bool",
        InputType::String,
    ),
    (
        "match",
        "match(\"\")",
        "regex match → object",
        InputType::String,
    ),
    (
        "capture",
        "capture(\"(?P<x>)\")",
        "regex capture → object",
        InputType::String,
    ),
    ("scan", "scan(\"\")", "all regex matches", InputType::String),
    (
        "sub",
        "sub(\"pat\"; \"rep\")",
        "replace first match",
        InputType::String,
    ),
    (
        "gsub",
        "gsub(\"pat\"; \"rep\")",
        "replace all matches",
        InputType::String,
    ),
    (
        "explode",
        "explode",
        "string → [codepoints]",
        InputType::String,
    ),
    ("tonumber", "tonumber", "string → number", InputType::String),
    (
        "fromjson",
        "fromjson",
        "JSON string → value",
        InputType::String,
    ),
    (
        "strptime",
        "strptime(\"%Y-%m-%d\")",
        "parse date string",
        InputType::String,
    ),
    ("@base64d", "@base64d", "decode base64", InputType::String),
    ("@uri", "@uri", "percent-encode for URI", InputType::String),
    ("@html", "@html", "escape HTML entities", InputType::String),
    ("@sh", "@sh", "shell-quote string", InputType::String),
    ("@csv", "@csv", "encode row as CSV", InputType::Array),
    ("@tsv", "@tsv", "encode row as TSV", InputType::Array),
    // ── numbers ───────────────────────────────────────────────────────────────
    ("floor", "floor", "round down to integer", InputType::Number),
    ("ceil", "ceil", "round up to integer", InputType::Number),
    ("round", "round", "round to nearest", InputType::Number),
    ("sqrt", "sqrt", "square root", InputType::Number),
    ("fabs", "fabs", "absolute value", InputType::Number),
    ("log", "log", "natural logarithm", InputType::Number),
    ("log2", "log2", "log base-2", InputType::Number),
    ("log10", "log10", "log base-10", InputType::Number),
    ("exp", "exp", "e^x", InputType::Number),
    ("exp2", "exp2", "2^x", InputType::Number),
    ("exp10", "exp10", "10^x", InputType::Number),
    ("pow", "pow(.; 2)", "x^y", InputType::Number),
    ("isnan", "isnan", "test for NaN", InputType::Number),
    (
        "isinfinite",
        "isinfinite",
        "test for infinity",
        InputType::Number,
    ),
    (
        "isfinite",
        "isfinite",
        "test for finite float",
        InputType::Number,
    ),
    (
        "isnormal",
        "isnormal",
        "test for normal float",
        InputType::Number,
    ),
    ("nan", "nan", "produce NaN", InputType::Number),
    (
        "infinite",
        "infinite",
        "produce infinity",
        InputType::Number,
    ),
    ("tostring", "tostring", "number → string", InputType::Number),
    (
        "strftime",
        "strftime(\"%Y-%m-%d\")",
        "format UNIX time",
        InputType::Number,
    ),
    (
        "gmtime",
        "gmtime",
        "UNIX ts → broken-down",
        InputType::Number,
    ),
    (
        "ascii",
        "ascii",
        "codepoint → char string",
        InputType::Number,
    ),
    // ── arrays ────────────────────────────────────────────────────────────────
    ("sort", "sort", "sort elements", InputType::Array),
    ("sort_by", "sort_by()", "sort by key expr", InputType::Array),
    (
        "group_by",
        "group_by()",
        "group into sub-arrays",
        InputType::Array,
    ),
    ("unique", "unique", "deduplicate", InputType::Array),
    (
        "unique_by",
        "unique_by()",
        "deduplicate by key",
        InputType::Array,
    ),
    (
        "flatten",
        "flatten",
        "flatten nested arrays",
        InputType::Array,
    ),
    ("reverse", "reverse", "reverse order", InputType::Array),
    ("add", "add", "sum / concatenate", InputType::Array),
    ("min", "min", "minimum element", InputType::Array),
    ("max", "max", "maximum element", InputType::Array),
    ("min_by", "min_by()", "min by key expr", InputType::Array),
    ("max_by", "max_by()", "max by key expr", InputType::Array),
    ("map", "map(.)", "transform each element", InputType::Array),
    (
        "any",
        "any(. > 0)",
        "test any element matches pred",
        InputType::Array,
    ),
    (
        "all",
        "all(. > 0)",
        "test all elements match pred",
        InputType::Array,
    ),
    ("first", "first", "first element", InputType::Array),
    ("last", "last", "last element", InputType::Array),
    ("nth", "nth(0)", "nth element", InputType::Array),
    (
        "transpose",
        "transpose",
        "flip rows and columns",
        InputType::Array,
    ),
    (
        "implode",
        "implode",
        "[codepoints] → string",
        InputType::Array,
    ),
    (
        "from_entries",
        "from_entries",
        "[{key,value}] → object",
        InputType::Array,
    ),
    (
        "mktime",
        "mktime",
        "broken-down time → UNIX",
        InputType::Array,
    ),
    (
        "inside",
        "inside([])",
        "test if inside value",
        InputType::Array,
    ),
    // ── objects ───────────────────────────────────────────────────────────────
    (
        "to_entries",
        "to_entries",
        "→ [{key,value}]",
        InputType::Object,
    ),
    (
        "with_entries",
        "with_entries(.value)",
        "map over entries",
        InputType::Object,
    ),
    (
        "keys_unsorted",
        "keys_unsorted",
        "keys without sort",
        InputType::Object,
    ),
    ("del", "del()", "delete key/path", InputType::Object),
    // ── arrays or objects ─────────────────────────────────────────────────────
    (
        "keys",
        "keys",
        "sorted keys or indices",
        InputType::ArrayOrObject,
    ),
    (
        "values",
        "values",
        "values as array",
        InputType::ArrayOrObject,
    ),
    (
        "has",
        "has(\"key\")",
        "test for key / index",
        InputType::ArrayOrObject,
    ),
    (
        "map_values",
        "map_values(.)",
        "transform each value",
        InputType::ArrayOrObject,
    ),
    (
        "to_entries",
        "to_entries",
        "→ [{key,value}]",
        InputType::ArrayOrObject,
    ),
    (
        "contains",
        "contains({})",
        "test containment",
        InputType::ArrayOrObject,
    ),
    // ── strings or arrays ─────────────────────────────────────────────────────
    // `length` works on string, number, array, object, null — but NOT boolean.
    // Use NonBoolean so it never appears after endswith/test/not/… output.
    (
        "length",
        "length",
        "count chars / elements",
        InputType::NonBoolean,
    ),
    (
        "indices",
        "indices(\"\")",
        "all indices of value",
        InputType::StringOrArray,
    ),
    (
        "index",
        "index(\"\")",
        "first index of value",
        InputType::StringOrArray,
    ),
    (
        "rindex",
        "rindex(\"\")",
        "last index of value",
        InputType::StringOrArray,
    ),
    // ── universal ─────────────────────────────────────────────────────────────
    ("type", "type", "get JSON type name", InputType::Any),
    ("not", "not", "boolean negation", InputType::Any),
    ("tojson", "tojson", "encode to JSON string", InputType::Any),
    ("tostring", "tostring", "convert to string", InputType::Any),
    ("tonumber", "tonumber", "convert to number", InputType::Any),
    ("path", "path()", "path to value", InputType::Any),
    ("paths", "paths", "all paths in value", InputType::Any),
    (
        "leaf_paths",
        "leaf_paths",
        "paths to leaf values",
        InputType::Any,
    ),
    (
        "getpath",
        "getpath([\"key\"])",
        "value at path",
        InputType::Any,
    ),
    (
        "setpath",
        "setpath([\"key\"]; .)",
        "set value at path",
        InputType::Any,
    ),
    (
        "delpaths",
        "delpaths([[\"k\"]])",
        "delete paths",
        InputType::Any,
    ),
    (
        "select",
        "select(. != null)",
        "filter — pass or empty",
        InputType::Any,
    ),
    ("recurse", "recurse", "recursive descent", InputType::Any),
    (
        "walk",
        "walk(if type == \"array\" then sort else . end)",
        "depth-first walk",
        InputType::Any,
    ),
    ("env", "env", "environment variables", InputType::Any),
    ("debug", "debug", "print to stderr", InputType::Any),
    ("error", "error(\"msg\")", "raise error", InputType::Any),
    ("empty", "empty", "produce no output", InputType::Any),
    ("null", "null", "null literal", InputType::Any),
    ("true", "true", "boolean true", InputType::Any),
    ("false", "false", "boolean false", InputType::Any),
    ("now", "now", "current UNIX timestamp", InputType::Any),
    ("builtins", "builtins", "list all built-ins", InputType::Any),
    ("limit", "limit(10; .[])", "take N outputs", InputType::Any),
    (
        "first",
        "first(.[])  ",
        "first output of expr",
        InputType::Any,
    ),
    ("last", "last(.[])", "last output of expr", InputType::Any),
    (
        "range",
        "range(10)",
        "0..N integer generator",
        InputType::Any,
    ),
    (
        "reduce",
        "reduce .[] as $x (0; . + $x)",
        "fold / accumulate",
        InputType::Any,
    ),
    (
        "foreach",
        "foreach .[] as $x (0; . + $x)",
        "stateful iteration",
        InputType::Any,
    ),
    (
        "until",
        "until(. > 10; . + 1)",
        "repeat until condition",
        InputType::Any,
    ),
    (
        "while",
        "while(. < 10; . + 1)",
        "repeat while condition",
        InputType::Any,
    ),
    (
        "recurse_down",
        "recurse_down",
        "recursive descent (alias)",
        InputType::Any,
    ),
    ("@base64", "@base64", "encode as base64", InputType::String),
    ("@json", "@json", "format as JSON string", InputType::Any),
    ("@text", "@text", "same as tostring", InputType::Any),
    ("input", "input", "next input value", InputType::Any),
    ("inputs", "inputs", "remaining input values", InputType::Any),
    (
        "contains",
        "contains(null)",
        "test containment",
        InputType::Any,
    ),
    (
        "inside",
        "inside(null)",
        "test if inside value",
        InputType::Any,
    ),
    (
        "format",
        "format(\"text\")",
        "string interpolation",
        InputType::Any,
    ),
    ("infinite", "infinite", "IEEE infinity", InputType::Any),
    ("nan", "nan", "IEEE NaN", InputType::Any),
];

/// Return completions from the built-in catalog, filtered by:
/// - `token` — label must start with this prefix (empty = all)
/// - `input_type` — runtime JSON type string ("string", "number", …), or None for all types
///
/// When `input_type` is Some, type-specific functions are listed first, then
/// universal ones, so the most relevant suggestions appear at the top.
pub fn get_completions(token: &str, input_type: Option<&str>) -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();

    // Two passes: specific first, then Any — so typed functions bubble to top.
    let passes: &[bool] = &[false, true]; // false = specific first, true = Any pass
    let mut out = Vec::new();

    for is_any_pass in passes {
        for &(name, insert_text, detail, type_filter) in BUILTINS {
            // `Any` is the universal pass; everything else (including NonBoolean)
            // goes in the specific pass so type-relevant items bubble to the top.
            let is_any_filter = type_filter == InputType::Any;
            if is_any_pass != &is_any_filter {
                continue;
            }
            if !token.is_empty() && !name.starts_with(token) {
                continue;
            }
            if matches!(input_type, Some(jq_type) if !type_filter.compatible_with(jq_type)) {
                continue;
            }
            // Deduplicate by name (the same name appears in multiple InputType rows).
            if seen.insert(name) {
                out.push(CompletionItem {
                    label: name.to_string(),
                    detail: Some(detail.to_string()),
                    insert_text: insert_text.to_string(),
                });
            }
        }
    }
    out
}

/// Classify a `serde_json::Value` as its jq type string.
pub fn jq_type_of(val: &serde_json::Value) -> &'static str {
    match val {
        serde_json::Value::String(_) => "string",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Null => "null",
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_type_shows_ascii_funcs() {
        let c = get_completions("", Some("string"));
        assert!(
            c.iter().any(|i| i.label == "ascii_upcase"),
            "ascii_upcase must appear for string input"
        );
        assert!(c.iter().any(|i| i.label == "ascii_downcase"));
        assert!(c.iter().any(|i| i.label == "split"));
    }

    #[test]
    fn string_type_excludes_number_only_funcs() {
        let c = get_completions("", Some("string"));
        let number_only = ["floor", "ceil", "round", "sqrt", "log", "exp"];
        for name in &number_only {
            assert!(
                !c.iter().any(|i| &i.label == name),
                "{} must NOT appear for string input",
                name
            );
        }
    }

    #[test]
    fn number_type_shows_math_funcs() {
        let c = get_completions("", Some("number"));
        for name in &["floor", "ceil", "round", "sqrt", "log"] {
            assert!(
                c.iter().any(|i| &i.label == name),
                "{} must appear for number input",
                name
            );
        }
    }

    #[test]
    fn number_type_excludes_string_only_funcs() {
        let c = get_completions("", Some("number"));
        for name in &["ascii_upcase", "split", "test", "ltrimstr"] {
            assert!(
                !c.iter().any(|i| &i.label == name),
                "{} must NOT appear for number input",
                name
            );
        }
    }

    #[test]
    fn array_type_shows_sort_and_map() {
        let c = get_completions("", Some("array"));
        for name in &[
            "sort", "map", "reverse", "flatten", "unique", "first", "last",
        ] {
            assert!(
                c.iter().any(|i| &i.label == name),
                "{} must appear for array input",
                name
            );
        }
    }

    #[test]
    fn object_type_shows_to_entries() {
        let c = get_completions("", Some("object"));
        assert!(c.iter().any(|i| i.label == "to_entries"));
        assert!(c.iter().any(|i| i.label == "with_entries"));
        assert!(c.iter().any(|i| i.label == "keys"));
    }

    #[test]
    fn token_prefix_filters_results() {
        let c = get_completions("asc", Some("string"));
        assert!(
            c.iter().all(|i| i.label.starts_with("asc")),
            "all results must start with 'asc': {:?}",
            c.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn token_prefix_no_match_returns_empty() {
        let c = get_completions("zzznomatch", None);
        assert!(c.is_empty());
    }

    #[test]
    fn no_duplicates_in_output() {
        let c = get_completions("", None);
        let mut seen = std::collections::HashSet::new();
        for item in &c {
            assert!(
                seen.insert(item.label.clone()),
                "duplicate label: {}",
                item.label
            );
        }
    }

    #[test]
    fn typed_funcs_appear_before_any_funcs_for_string() {
        let c = get_completions("", Some("string"));
        // ascii_upcase is string-specific; "type" is universal.
        // string-specific should come first.
        let pos_ascii = c.iter().position(|i| i.label == "ascii_upcase");
        let pos_type = c.iter().position(|i| i.label == "type");
        if let (Some(a), Some(t)) = (pos_ascii, pos_type) {
            assert!(
                a < t,
                "type-specific funcs must precede universal ones (got ascii_upcase={}, type={})",
                a,
                t
            );
        }
    }

    #[test]
    fn none_type_shows_all_builtins() {
        let all = get_completions("", None);
        let string_only = get_completions("", Some("string"));
        // all must have at least as many entries as string-only
        assert!(all.len() >= string_only.len());
        // and must include string-specific items not in number-only
        assert!(all.iter().any(|i| i.label == "floor")); // number fn
        assert!(all.iter().any(|i| i.label == "ascii_upcase")); // string fn
    }

    #[test]
    fn jq_type_of_covers_all_variants() {
        use serde_json::json;
        assert_eq!(jq_type_of(&json!("hi")), "string");
        assert_eq!(jq_type_of(&json!(42)), "number");
        assert_eq!(jq_type_of(&json!([])), "array");
        assert_eq!(jq_type_of(&json!({})), "object");
        assert_eq!(jq_type_of(&json!(true)), "boolean");
        assert_eq!(jq_type_of(&json!(null)), "null");
    }

    #[test]
    fn insert_text_for_floor_is_bare() {
        let c = get_completions("floor", Some("number"));
        let item = c.iter().find(|i| i.label == "floor").unwrap();
        assert_eq!(item.insert_text, "floor");
    }

    #[test]
    fn insert_text_for_split_has_parens() {
        let c = get_completions("split", Some("string"));
        let item = c.iter().find(|i| i.label == "split").unwrap();
        assert!(
            item.insert_text.contains("("),
            "split insert_text must include parens: {}",
            item.insert_text
        );
    }

    #[test]
    fn field_path_functions_insert_text_starts_param_context() {
        let c = get_completions("", Some("array"));
        let sort_by = c.iter().find(|i| i.label == "sort_by").unwrap();
        assert_eq!(sort_by.insert_text, "sort_by()");

        let group_by = c.iter().find(|i| i.label == "group_by").unwrap();
        assert_eq!(group_by.insert_text, "group_by()");

        let unique_by = c.iter().find(|i| i.label == "unique_by").unwrap();
        assert_eq!(unique_by.insert_text, "unique_by()");
    }

    #[test]
    fn del_and_path_insert_text_starts_param_context() {
        let obj = get_completions("", Some("object"));
        let del = obj.iter().find(|i| i.label == "del").unwrap();
        assert_eq!(del.insert_text, "del()");

        let any = get_completions("", None);
        let path = any.iter().find(|i| i.label == "path").unwrap();
        assert_eq!(path.insert_text, "path()");
    }

    // ── Boolean type exclusions ───────────────────────────────────────────────
    // These are the functions that are NOT valid for boolean input in jq.
    // The exact scenario reported: `endswith("") | length` must not suggest
    // `length` because `true/false | length` is a runtime error.

    #[test]
    fn boolean_type_excludes_length() {
        let c = get_completions("", Some("boolean"));
        assert!(
            !c.iter().any(|i| i.label == "length"),
            "length must NOT appear for boolean input — `true | length` is a jq error"
        );
    }

    #[test]
    fn boolean_type_excludes_string_and_number_funcs() {
        let c = get_completions("", Some("boolean"));
        let excluded = [
            "ascii_upcase",
            "ascii_downcase",
            "split",
            "floor",
            "ceil",
            "sqrt",
            "sort",
            "map",
            "to_entries",
            "@base64",
        ];
        for name in &excluded {
            assert!(
                !c.iter().any(|i| &i.label == name),
                "{} must NOT appear for boolean input",
                name
            );
        }
    }

    #[test]
    fn boolean_type_shows_universal_funcs() {
        // type, not, tostring, debug, select, etc. must still appear for boolean.
        let c = get_completions("", Some("boolean"));
        for name in &["type", "not", "tostring", "tojson", "debug", "select"] {
            assert!(
                c.iter().any(|i| &i.label == name),
                "{} must appear for boolean input",
                name
            );
        }
    }

    #[test]
    fn length_appears_for_string() {
        let c = get_completions("", Some("string"));
        assert!(
            c.iter().any(|i| i.label == "length"),
            "length must appear for string input"
        );
    }

    #[test]
    fn length_appears_for_number() {
        let c = get_completions("", Some("number"));
        assert!(
            c.iter().any(|i| i.label == "length"),
            "length must appear for number input (returns absolute value)"
        );
    }

    #[test]
    fn length_appears_for_array() {
        let c = get_completions("", Some("array"));
        assert!(
            c.iter().any(|i| i.label == "length"),
            "length must appear for array input"
        );
    }

    #[test]
    fn length_appears_for_object() {
        let c = get_completions("", Some("object"));
        assert!(
            c.iter().any(|i| i.label == "length"),
            "length must appear for object input"
        );
    }

    #[test]
    fn length_appears_for_null() {
        let c = get_completions("", Some("null"));
        assert!(
            c.iter().any(|i| i.label == "length"),
            "length must appear for null input (returns 0)"
        );
    }

    #[test]
    fn base64_excluded_for_non_string() {
        for jq_type in &["number", "array", "object", "boolean", "null"] {
            let c = get_completions("", Some(jq_type));
            assert!(
                !c.iter().any(|i| i.label == "@base64"),
                "@base64 must NOT appear for {} input",
                jq_type
            );
        }
    }

    #[test]
    fn base64_appears_for_string() {
        let c = get_completions("", Some("string"));
        assert!(
            c.iter().any(|i| i.label == "@base64"),
            "@base64 must appear for string input"
        );
    }

    // ── NonBoolean coverage ───────────────────────────────────────────────────

    #[test]
    fn nonboolean_compatible_with_correct_types() {
        let nb = InputType::NonBoolean;
        assert!(nb.compatible_with("string"));
        assert!(nb.compatible_with("number"));
        assert!(nb.compatible_with("array"));
        assert!(nb.compatible_with("object"));
        assert!(nb.compatible_with("null"));
        assert!(
            !nb.compatible_with("boolean"),
            "NonBoolean must reject 'boolean'"
        );
    }
}
