use crate::completions::CompletionItem;
use serde_json::Value;

pub fn get_completions(query: &str, input: &Value) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    dot_path_completions(query, input, &mut completions);
    obj_constructor_completions(query, input, &mut completions);
    array_index_completions(query, input, &mut completions);
    completions
}

// ──────────────────────────────────────────────────────────────────────────────
// Dot-path completions  (.foo.bar, .filesets[].)
// ──────────────────────────────────────────────────────────────────────────────

fn dot_path_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>) {
    let (path_str, prefix) = if let Some(last_dot) = query.rfind('.') {
        (&query[..last_dot], &query[last_dot + 1..])
    } else if query.is_empty() {
        // Empty query → complete top-level fields
        ("", "")
    } else {
        return;
    };

    if let Some(Value::Object(map)) = find_value_at_path(input, path_str) {
        for key in map.keys() {
            if key.starts_with(prefix) {
                let insert_text = if path_str.is_empty() {
                    format!(".{}", key)
                } else {
                    format!("{}.{}", path_str, key)
                };
                out.push(CompletionItem {
                    label: key.clone(),
                    detail: Some("field".to_string()),
                    insert_text,
                });
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Object-constructor completions  ({run_id, inp → {run_id, input_root)
// ──────────────────────────────────────────────────────────────────────────────

fn obj_constructor_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>) {
    // Find the last unclosed `{`.
    let mut depth = 0i32;
    let mut last_open = None;
    for (i, ch) in query.char_indices() {
        match ch {
            '{' => {
                depth += 1;
                last_open = Some(i);
            }
            '}' => depth -= 1,
            _ => {}
        }
    }
    if depth <= 0 {
        return;
    }
    let open_pos = match last_open {
        Some(p) => p,
        None => return,
    };

    let before_brace = &query[..open_pos];
    let inside_brace = &query[open_pos + 1..];

    // Locate the partial field name: everything after the last `,`
    // (preserving any leading whitespace so insert_text looks natural).
    let field_offset = inside_brace
        .rfind(',')
        .map(|comma| {
            let after = &inside_brace[comma + 1..];
            // skip leading whitespace after the comma
            comma + 1 + (after.len() - after.trim_start().len())
        })
        .unwrap_or_else(|| {
            // no comma → skip any leading whitespace at the very start
            inside_brace.len() - inside_brace.trim_start().len()
        });

    let typed_before_field = &inside_brace[..field_offset]; // e.g. "run_id, "
    let partial_field = &inside_brace[field_offset..]; // e.g. "inp"

    // Determine what object to source field names from.
    // For  `.foo | {bar`  the context is `.foo`.
    let context_path = pipe_context_before(before_brace);

    if let Some(Value::Object(map)) = find_value_at_path(input, context_path) {
        for key in map.keys() {
            if key.starts_with(partial_field) {
                // `.{field}` is invalid jq — strip a bare leading dot so we produce
                // `{field}` (or `.foo | {field}`) instead.
                let insert_prefix = if before_brace.trim() == "." {
                    ""
                } else {
                    before_brace
                };
                // insert_text replaces the whole query so Tab gives the right result.
                let insert_text = format!("{}{{{}{}", insert_prefix, typed_before_field, key);
                out.push(CompletionItem {
                    label: key.clone(),
                    detail: Some("field".to_string()),
                    insert_text,
                });
            }
        }
    }
}

/// Return the jq-path context that feeds into a `| {…}` expression.
/// ".config | "  →  ".config"
/// ""            →  "."
fn pipe_context_before(s: &str) -> &str {
    let t = s.trim();
    if t.is_empty() {
        return ".";
    }
    if let Some(pos) = t.rfind('|') {
        t[..pos].trim()
    } else {
        t
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Array-index completions  (.items[  →  [], [0], [1], …)
// ──────────────────────────────────────────────────────────────────────────────

fn array_index_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>) {
    // Only fires when query ends with `[` or `[<digits>`
    let bracket_pos = match query.rfind('[') {
        Some(p) => p,
        None => return,
    };
    // Bail if there is already a closing `]` after the last `[`
    if query[bracket_pos..].contains(']') {
        return;
    }
    let path_before = &query[..bracket_pos];
    let index_prefix = &query[bracket_pos + 1..];

    // index_prefix must be empty or consist only of digits (user is typing an index)
    if !index_prefix.is_empty() && !index_prefix.chars().all(|c| c.is_ascii_digit()) {
        return;
    }

    if let Some(Value::Array(arr)) = find_value_at_path(input, path_before) {
        let len = arr.len();

        // `[]` — iterate all items (only offered when no digit typed yet)
        if index_prefix.is_empty() {
            out.push(CompletionItem {
                label: "[]".to_string(),
                detail: Some(format!("iterate ({} items)", len)),
                insert_text: format!("{}[]", path_before),
            });
        }

        // Individual numeric indices up to min(len, 10)
        for i in 0..len.min(10) {
            let idx_str = i.to_string();
            if idx_str.starts_with(index_prefix) {
                out.push(CompletionItem {
                    label: format!("[{}]", i),
                    detail: Some("index".to_string()),
                    insert_text: format!("{}[{}]", path_before, i),
                });
            }
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Path traversal
// ──────────────────────────────────────────────────────────────────────────────

fn find_value_at_path<'a>(input: &'a Value, path: &str) -> Option<&'a Value> {
    if path.is_empty() || path == "." {
        return Some(input);
    }

    let mut current = input;
    for part in path.split('.').filter(|s| !s.is_empty()) {
        // Strip array accessor suffix: "key[]", "key[0]", "key[1]", etc.
        let (key, is_array_access) = if let Some(bracket) = part.find('[') {
            (&part[..bracket], true)
        } else {
            (part, false)
        };

        match current {
            Value::Object(map) => {
                let next = if key.is_empty() {
                    // bare `[]` on an object — skip (jq iterates all values,
                    // but for type-inference we just stay at the object)
                    current
                } else {
                    map.get(key)?
                };
                if is_array_access {
                    match next {
                        Value::Array(arr) => current = arr.first()?,
                        _ => return None,
                    }
                } else {
                    current = next;
                }
            }
            Value::Array(arr)
                // Already inside an array; `[]` or `[n]` descends into first element.
                if (is_array_access || key.is_empty()) =>
            {
                current = arr.first()?;
            }
            Value::Array(_) => return None,
            _ => return None,
        }
    }
    Some(current)
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- dot-path completions ---

    #[test]
    fn test_top_level_completions() {
        let input = json!({"foo": 1, "bar": 2});
        let c = get_completions("", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "foo" && c.insert_text == ".foo")
        );
        assert!(
            c.iter()
                .any(|c| c.label == "bar" && c.insert_text == ".bar")
        );
    }

    #[test]
    fn test_dot_completions() {
        let input = json!({"foo": 1, "bar": 2});
        let c = get_completions(".", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "foo" && c.insert_text == ".foo")
        );
        assert!(
            c.iter()
                .any(|c| c.label == "bar" && c.insert_text == ".bar")
        );
    }

    #[test]
    fn test_nested_completions() {
        let input = json!({"a": {"b": 1, "c": 2}});
        let c = get_completions(".a.", &input);
        assert!(c.iter().any(|c| c.label == "b" && c.insert_text == ".a.b"));
        assert!(c.iter().any(|c| c.label == "c" && c.insert_text == ".a.c"));
    }

    #[test]
    fn test_prefix_filtering() {
        let input = json!({"foo": 1, "food": 2, "bar": 3});
        let c = get_completions(".", &input);
        // only dot-path completions here (no object constructor)
        let dot_completions: Vec<_> = c
            .iter()
            .filter(|c| c.insert_text.starts_with('.'))
            .collect();
        assert!(dot_completions.iter().any(|c| c.label == "foo"));
        assert!(dot_completions.iter().any(|c| c.label == "food"));
        assert!(
            !dot_completions
                .iter()
                .any(|c| c.label == "bar" && c.insert_text.starts_with(".f"))
        );

        let c2 = get_completions(".f", &input);
        assert_eq!(c2.len(), 2);
        assert!(c2.iter().any(|c| c.label == "foo"));
        assert!(c2.iter().any(|c| c.label == "food"));
    }

    // --- array traversal ---

    #[test]
    fn test_array_iteration_completions() {
        let input = json!({
            "items": [{"name": "a", "value": 1}, {"name": "b", "value": 2}]
        });
        let c = get_completions(".items[].", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "name" && c.insert_text == ".items[].name"),
            "expected .items[].name, got: {:?}",
            c
        );
        assert!(
            c.iter()
                .any(|c| c.label == "value" && c.insert_text == ".items[].value")
        );
    }

    #[test]
    fn test_array_indexed_completions() {
        let input = json!({
            "items": [{"x": 1, "y": 2}]
        });
        let c = get_completions(".items[0].", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "x" && c.insert_text == ".items[0].x")
        );
        assert!(
            c.iter()
                .any(|c| c.label == "y" && c.insert_text == ".items[0].y")
        );
    }

    // --- object constructor ---

    #[test]
    fn test_obj_constructor_empty_prefix() {
        let input = json!({"run_id": "x", "schema_version": "1"});
        let c = get_completions("{", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "run_id" && c.insert_text == "{run_id")
        );
        assert!(c.iter().any(|c| c.label == "schema_version"));
    }

    #[test]
    fn test_obj_constructor_with_partial_field() {
        let input = json!({"run_id": "x", "input_root": "/path", "schema_version": "1"});
        let c = get_completions("{run_id, inp", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "input_root" && c.insert_text == "{run_id, input_root"),
            "got: {:?}",
            c
        );
    }

    #[test]
    fn test_obj_constructor_with_pipe_context() {
        let input = json!({"config": {"schema_version": "1", "threshold": 0.5}});
        let c = get_completions(".config | {schema", &input);
        assert!(
            c.iter().any(|c| c.label == "schema_version"),
            "got: {:?}",
            c
        );
    }

    // --- array index completions ---

    #[test]
    fn test_array_index_open_bracket_offers_iterate_and_indices() {
        let input = json!({"items": ["a", "b", "c"]});
        let c = get_completions(".items[", &input);
        assert!(
            c.iter()
                .any(|c| c.label == "[]" && c.insert_text == ".items[]"),
            "expected [] iterate: {:?}",
            c
        );
        assert!(
            c.iter()
                .any(|c| c.label == "[0]" && c.insert_text == ".items[0]"),
            "expected [0]: {:?}",
            c
        );
        assert!(
            c.iter()
                .any(|c| c.label == "[2]" && c.insert_text == ".items[2]"),
            "expected [2]: {:?}",
            c
        );
    }

    #[test]
    fn test_array_index_digit_prefix_filters() {
        let input = json!({"x": [0,1,2,3,4,5,6,7,8,9,10,11]});
        // typing `.x[1` should only offer [1] and [10], [11]
        let c = get_completions(".x[1", &input);
        assert!(
            c.iter().all(|c| c.label.starts_with("[1")),
            "all labels must start with [1: {:?}",
            c
        );
        assert!(
            !c.iter().any(|c| c.label == "[]"),
            "[] should not appear when digit prefix typed: {:?}",
            c
        );
    }

    #[test]
    fn test_array_index_no_suggestion_on_closed_bracket() {
        let input = json!({"x": [1, 2]});
        let c = get_completions(".x[0]", &input);
        // completed path — no index suggestions
        assert!(
            !c.iter().any(|c| c.label.starts_with('[')),
            "no index suggestions after closing bracket: {:?}",
            c
        );
    }

    #[test]
    fn test_array_index_nested_path() {
        let input = json!({"a": {"b": [10, 20, 30]}});
        let c = get_completions(".a.b[", &input);
        assert!(c.iter().any(|c| c.insert_text == ".a.b[]"), "got: {:?}", c);
        assert!(c.iter().any(|c| c.insert_text == ".a.b[0]"), "got: {:?}", c);
    }

    #[test]
    fn test_array_index_capped_at_10() {
        let arr: Vec<i32> = (0..20).collect();
        let input = json!({"big": arr});
        let c = get_completions(".big[", &input);
        // [] + indices 0..9 = 11 items max, not 21
        let index_items: Vec<_> = c.iter().filter(|c| c.label != "[]").collect();
        assert_eq!(index_items.len(), 10, "at most 10 numeric indices: {:?}", c);
    }
}
