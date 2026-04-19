use crate::completions::CompletionItem;
use crate::completions::fuzzy::fuzzy_score;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, HashSet};

const FIELD_PATH_ARRAY_FNS: &[&str] = &["sort_by", "group_by", "unique_by", "min_by", "max_by"];
const FIELD_PATH_INPUT_FNS: &[&str] = &["del", "path"];
const STRING_PARAM_PREFIX_FNS: &[&str] = &["startswith", "ltrimstr"];
const STRING_PARAM_SUFFIX_FNS: &[&str] = &["endswith", "rtrimstr"];
const STRING_PARAM_INTERNAL_FNS: &[&str] = &["split"];
const STRING_PARAM_FULLSTRING_FNS: &[&str] = &["contains", "index", "rindex", "indices"];
const MAX_STRING_SOURCE: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StringParamStrategy {
    Prefix,
    Suffix,
    Internal,
    FullString,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StringParamCtx<'a> {
    pub fn_name: &'a str,
    pub strategy: StringParamStrategy,
    pub context_path: &'a str,
    pub inner_prefix: &'a str,
}

pub struct ParamFieldCtx<'a> {
    pub fn_name: &'a str,
    pub context_path: &'a str,
    pub inner_prefix: &'a str,
    inner_start: usize,
}

pub fn get_completions(query: &str, input: &Value) -> Vec<CompletionItem> {
    let mut completions = Vec::new();
    dot_path_completions(query, input, &mut completions);
    obj_constructor_completions(query, input, &mut completions);
    array_index_completions(query, input, &mut completions);
    param_field_completions(query, input, &mut completions);
    string_param_completions(query, input, &mut completions);
    completions
}

pub fn next_structural_hint(query_prefix: &str, input: &Value) -> Option<Vec<CompletionItem>> {
    if query_prefix.ends_with('[') {
        return None;
    }

    if query_prefix.is_empty() {
        return match input {
            Value::Object(_) | Value::Array(_) => Some(vec![CompletionItem {
                label: ".".to_string(),
                detail: None,
                insert_text: ".".to_string(),
            }]),
            _ => None,
        };
    }

    if let Some(Value::Array(_)) = find_value_at_path(input, query_prefix) {
        return Some(vec![CompletionItem {
            label: "[]".to_string(),
            detail: None,
            insert_text: format!("{}[]", query_prefix),
        }]);
    }

    None
}

pub fn param_field_context(query: &str) -> Option<ParamFieldCtx<'_>> {
    if query.is_empty() {
        return None;
    }

    let mut depth: i32 = 0;
    let mut open_paren: Option<usize> = None;
    for (idx, ch) in query.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth < 0 {
                    open_paren = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open_paren?;

    let before_open = query[..open].trim_end();
    if before_open.is_empty() {
        return None;
    }

    let fn_end = before_open.len();
    let mut fn_start = fn_end;
    for (idx, ch) in before_open.char_indices().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            fn_start = idx;
        } else {
            break;
        }
    }
    if fn_start == fn_end {
        return None;
    }

    let fn_name = &before_open[fn_start..fn_end];
    if !FIELD_PATH_ARRAY_FNS.contains(&fn_name) && !FIELD_PATH_INPUT_FNS.contains(&fn_name) {
        return None;
    }

    let context_path = pipe_context_before(before_open[..fn_start].trim_end());

    let inner_full = &query[open + 1..];
    let list_rel = if inner_full.trim_start().starts_with('[') {
        let ws = inner_full.len() - inner_full.trim_start().len();
        Some(ws + 1)
    } else {
        None
    };
    let comma_rel = inner_full.rfind(',').map(|i| i + 1);
    let arg_rel = comma_rel.or(list_rel).unwrap_or(0);
    let after_comma = &inner_full[arg_rel..];
    let leading_ws = after_comma.len() - after_comma.trim_start().len();
    let inner_prefix = after_comma.trim_start();
    let inner_start = open + 1 + arg_rel + leading_ws;

    Some(ParamFieldCtx {
        fn_name,
        context_path,
        inner_prefix,
        inner_start,
    })
}

fn parse_string_param_context(query: &str) -> Option<(StringParamCtx<'_>, usize)> {
    if query.is_empty() {
        return None;
    }

    let mut depth: i32 = 0;
    let mut open_paren: Option<usize> = None;
    for (idx, ch) in query.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth < 0 {
                    open_paren = Some(idx);
                    break;
                }
            }
            _ => {}
        }
    }
    let open = open_paren?;

    let before_open = query[..open].trim_end();
    if before_open.is_empty() {
        return None;
    }

    let fn_end = before_open.len();
    let mut fn_start = fn_end;
    for (idx, ch) in before_open.char_indices().rev() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            fn_start = idx;
        } else {
            break;
        }
    }
    if fn_start == fn_end {
        return None;
    }

    let fn_name = &before_open[fn_start..fn_end];
    let strategy = if STRING_PARAM_PREFIX_FNS.contains(&fn_name) {
        StringParamStrategy::Prefix
    } else if STRING_PARAM_SUFFIX_FNS.contains(&fn_name) {
        StringParamStrategy::Suffix
    } else if STRING_PARAM_INTERNAL_FNS.contains(&fn_name) {
        StringParamStrategy::Internal
    } else if STRING_PARAM_FULLSTRING_FNS.contains(&fn_name) {
        StringParamStrategy::FullString
    } else {
        return None;
    };

    let raw_context_path = pipe_context_before(before_open[..fn_start].trim_end());
    let context_path = if is_path_like(raw_context_path) {
        raw_context_path
    } else {
        "."
    };

    let mut inner_prefix = query[open + 1..].trim_start();
    if let Some(stripped) = inner_prefix.strip_prefix('"') {
        inner_prefix = stripped;
    }

    Some((
        StringParamCtx {
            fn_name,
            strategy,
            context_path,
            inner_prefix,
        },
        open,
    ))
}

pub fn string_param_context(query: &str) -> Option<StringParamCtx<'_>> {
    parse_string_param_context(query).map(|(ctx, _)| ctx)
}

fn param_field_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>) {
    let Some(ctx) = param_field_context(query) else {
        return;
    };

    let context_value = find_value_at_path(input, ctx.context_path).or_else(|| {
        if ctx
            .context_path
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']'))
        {
            None
        } else {
            Some(input)
        }
    });

    let source_obj = if FIELD_PATH_ARRAY_FNS.contains(&ctx.fn_name) {
        match context_value {
            Some(Value::Array(arr)) => match arr.first() {
                Some(Value::Object(_)) => arr.first(),
                _ => None,
            },
            _ => None,
        }
    } else {
        match context_value {
            Some(Value::Object(_)) => context_value,
            _ => None,
        }
    };

    let Some(source) = source_obj else {
        return;
    };

    let mut param_items = Vec::new();
    dot_path_completions(ctx.inner_prefix, source, &mut param_items);
    for mut item in param_items {
        item.insert_text = format!("{}{}", &query[..ctx.inner_start], item.insert_text);
        if !out
            .iter()
            .any(|c| c.label == item.label && c.insert_text == item.insert_text)
        {
            out.push(item);
        }
    }
}

fn collect_string_values(val: &Value) -> Vec<&str> {
    match val {
        Value::String(s) => vec![s.as_str()],
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str())
            .take(MAX_STRING_SOURCE)
            .collect(),
        Value::Object(map) => map
            .values()
            .filter_map(|v| v.as_str())
            .take(MAX_STRING_SOURCE)
            .collect(),
        _ => Vec::new(),
    }
}

fn is_token_delim(ch: char) -> bool {
    matches!(
        ch,
        '\\' | '-' | '_' | '/' | '.' | ' ' | '\t' | ',' | '|' | '@'
    )
}

fn split_tokens(s: &str) -> Vec<&str> {
    s.split(is_token_delim).filter(|t| !t.is_empty()).collect()
}

fn extract_prefix_candidates(strings: &[&str]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for s in strings {
        if s.is_empty() {
            continue;
        }
        out.insert((*s).to_string());
        // Emit a prefix at the end of each token (s[..i] just before each delimiter).
        // e.g. "2026-04-19" → "2026", "2026-04", "2026-04-19"
        let mut in_token = false;
        for (i, ch) in s.char_indices() {
            if is_token_delim(ch) {
                if in_token {
                    out.insert(s[..i].to_string());
                }
                in_token = false;
            } else {
                in_token = true;
            }
        }
    }
    out.into_iter().collect()
}

fn extract_suffix_candidates(strings: &[&str]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for s in strings {
        if s.is_empty() {
            continue;
        }
        out.insert((*s).to_string());

        let tokens = split_tokens(s);
        if tokens.is_empty() {
            continue;
        }

        let mut token_starts: Vec<usize> = Vec::new();
        let mut pos = 0usize;
        for token in &tokens {
            if let Some(rel) = s[pos..].find(token) {
                let start = pos + rel;
                token_starts.push(start);
                pos = start + token.len();
            }
        }

        for (i, start) in token_starts.iter().enumerate() {
            out.insert(s[*start..].to_string());

            if i > 0 && i + 1 < token_starts.len() {
                let prev_end = token_starts[i - 1] + tokens[i - 1].len();
                if prev_end < *start {
                    out.insert(s[prev_end..].to_string());
                }
            }
        }
    }
    out.into_iter().collect()
}

fn extract_internal_candidates(strings: &[&str]) -> Vec<String> {
    let mut char_counts: HashMap<char, usize> = HashMap::new();
    let mut run_counts: HashMap<String, usize> = HashMap::new();

    for s in strings {
        let mut seen_chars = HashSet::new();
        for ch in s.chars() {
            if !ch.is_ascii_alphanumeric() {
                seen_chars.insert(ch);
            }
        }
        for ch in seen_chars {
            *char_counts.entry(ch).or_insert(0) += 1;
        }

        let mut seen_runs = HashSet::new();
        let mut run_start: Option<usize> = None;
        for (idx, ch) in s.char_indices() {
            if !ch.is_ascii_alphanumeric() {
                if run_start.is_none() {
                    run_start = Some(idx);
                }
                continue;
            }

            if let Some(start) = run_start.take() {
                let run = &s[start..idx];
                let prev = s[..start].chars().next_back();
                let next = Some(ch);
                if prev.is_some_and(|c| c.is_ascii_alphanumeric())
                    && next.is_some_and(|c| c.is_ascii_alphanumeric())
                {
                    seen_runs.insert(run.to_string());
                }
            }
        }

        if let Some(start) = run_start {
            let prev = s[..start].chars().next_back();
            if prev.is_some_and(|c| c.is_ascii_alphanumeric()) {
                let run = &s[start..];
                seen_runs.insert(run.to_string());
            }
        }

        for run in seen_runs {
            *run_counts.entry(run).or_insert(0) += 1;
        }
    }

    let mut out = BTreeSet::new();
    for (ch, count) in char_counts {
        if count >= 2 {
            out.insert(ch.to_string());
        }
    }
    for (run, count) in run_counts {
        if count >= 2 {
            out.insert(run);
        }
    }
    out.into_iter().collect()
}

fn extract_fullstring_candidates(strings: &[&str]) -> Vec<String> {
    strings
        .iter()
        .map(|s| (*s).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn string_param_completions(query: &str, input: &Value, out: &mut Vec<CompletionItem>) {
    let Some((ctx, open)) = parse_string_param_context(query) else {
        return;
    };

    let strings = if ctx.context_path.contains("[]") {
        collect_string_values_at_path(input, ctx.context_path)
    } else {
        find_value_at_path(input, ctx.context_path)
            .map(collect_string_values)
            .unwrap_or_default()
    };
    if strings.is_empty() {
        return;
    }

    let all_candidates = match ctx.strategy {
        StringParamStrategy::Prefix => extract_prefix_candidates(&strings),
        StringParamStrategy::Suffix => extract_suffix_candidates(&strings),
        StringParamStrategy::Internal => extract_internal_candidates(&strings),
        StringParamStrategy::FullString => extract_fullstring_candidates(&strings),
    };
    if all_candidates.is_empty() {
        return;
    }

    let is_exact_match = |candidate: &str| -> bool {
        if ctx.inner_prefix.is_empty() {
            return true;
        }
        match ctx.strategy {
            StringParamStrategy::Suffix => candidate.ends_with(ctx.inner_prefix),
            _ => candidate.starts_with(ctx.inner_prefix),
        }
    };

    let mut exact: Vec<String> = all_candidates
        .iter()
        .filter(|candidate| is_exact_match(candidate))
        .cloned()
        .collect();
    exact.sort_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)));

    let mut fuzzy: Vec<String> = Vec::new();
    if !ctx.inner_prefix.is_empty() {
        let mut scored: Vec<(i32, String)> = all_candidates
            .iter()
            .filter(|candidate| !is_exact_match(candidate))
            .filter_map(|candidate| {
                fuzzy_score(ctx.inner_prefix, candidate).map(|score| (score, candidate.clone()))
            })
            .collect();
        scored.sort_by(|(score_a, a), (score_b, b)| {
            score_b
                .cmp(score_a)
                .then_with(|| a.len().cmp(&b.len()))
                .then_with(|| a.cmp(b))
        });
        fuzzy = scored.into_iter().map(|(_, candidate)| candidate).collect();
    }

    let exact_count = exact.len();
    let candidates: Vec<String> = exact.into_iter().chain(fuzzy).collect();

    let query_up_to_open_paren = &query[..open + 1];
    for (idx, candidate) in candidates.into_iter().take(20).enumerate() {
        let escaped_candidate = candidate.replace('"', "\\\"");
        let item = CompletionItem {
            label: candidate.clone(),
            detail: Some(if idx < exact_count {
                "string value".to_string()
            } else {
                "~string value".to_string()
            }),
            insert_text: format!("{}\"{}\")", query_up_to_open_paren, escaped_candidate),
        };
        if !out
            .iter()
            .any(|c| c.label == item.label && c.insert_text == item.insert_text)
        {
            out.push(item);
        }
    }
}

fn collect_string_values_at_path<'a>(input: &'a Value, path: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    for value in find_values_at_path(input, path) {
        for s in collect_string_values(value) {
            out.push(s);
            if out.len() >= MAX_STRING_SOURCE {
                return out;
            }
        }
    }
    out
}

fn find_values_at_path<'a>(input: &'a Value, path: &str) -> Vec<&'a Value> {
    if path.is_empty() || path == "." {
        return vec![input];
    }

    let mut current: Vec<&Value> = vec![input];
    for part in path.split('.').filter(|s| !s.is_empty()) {
        let (key, is_array_access) = if let Some(bracket) = part.find('[') {
            (&part[..bracket], true)
        } else {
            (part, false)
        };

        let mut next_values: Vec<&Value> = Vec::new();
        for value in current {
            match value {
                Value::Object(map) => {
                    let next = if key.is_empty() {
                        Some(value)
                    } else {
                        map.get(key)
                    };
                    if let Some(next) = next {
                        if is_array_access {
                            if let Value::Array(arr) = next {
                                next_values.extend(arr.iter());
                            }
                        } else {
                            next_values.push(next);
                        }
                    }
                }
                Value::Array(arr) if is_array_access || key.is_empty() => {
                    next_values.extend(arr.iter());
                }
                _ => {}
            }
        }
        if next_values.is_empty() {
            return Vec::new();
        }
        current = next_values;
    }
    current
}

fn is_path_like(path: &str) -> bool {
    path == "."
        || (path.starts_with('.')
            && path
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '.' | '[' | ']')))
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

    if let Some(value) = find_value_at_path(input, path_str) {
        if let Value::Object(map) = value {
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
        } else if path_str.is_empty()
            && matches!(value, Value::Array(_))
            && "[]".starts_with(prefix)
        {
            out.push(CompletionItem {
                label: "[]".to_string(),
                detail: Some("iterate array".to_string()),
                insert_text: ".[]".to_string(),
            });
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
    fn test_root_array_dot_completion_offers_iterate() {
        let input = json!([{"name": "Alice"}, {"name": "Bob"}]);
        let c = get_completions(".", &input);
        assert!(
            c.iter().any(|c| c.label == "[]" && c.insert_text == ".[]"),
            "expected .[] completion at root array, got: {:?}",
            c
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

    #[test]
    fn structural_hint_array_path_returns_brackets() {
        let input = json!({"items": [1, 2, 3]});
        let hints = next_structural_hint(".items", &input).unwrap();
        assert_eq!(hints[0].label, "[]");
        assert_eq!(hints[0].insert_text, ".items[]");
    }

    #[test]
    fn structural_hint_empty_query_object_or_array_returns_dot() {
        let object_input = json!({"name": "alice"});
        let object_hints = next_structural_hint("", &object_input).unwrap();
        assert_eq!(object_hints[0].label, ".");
        assert_eq!(object_hints[0].insert_text, ".");

        let array_input = json!([{"name": "alice"}]);
        let array_hints = next_structural_hint("", &array_input).unwrap();
        assert_eq!(array_hints[0].label, ".");
        assert_eq!(array_hints[0].insert_text, ".");
    }

    #[test]
    fn structural_hint_array_of_objects_returns_none() {
        let input = json!({"items": [{"name": "a"}]});
        assert!(next_structural_hint(".items[]", &input).is_none());
    }

    #[test]
    fn structural_hint_scalar_returns_none() {
        let input = json!({"name": "alice"});
        assert!(next_structural_hint(".name", &input).is_none());
    }

    #[test]
    fn structural_hint_query_ending_bracket_returns_none() {
        let input = json!({"items": [1, 2, 3]});
        assert!(next_structural_hint(".items[", &input).is_none());
    }

    fn labels(v: &[CompletionItem]) -> Vec<String> {
        v.iter().map(|c| c.label.clone()).collect()
    }

    fn has_insert(v: &[CompletionItem], insert: &str) -> bool {
        v.iter().any(|c| c.insert_text == insert)
    }

    #[test]
    fn param_ctx_none_after_closing_paren() {
        assert!(param_field_context("sort_by(.name)").is_none());
        assert!(param_field_context("sort_by(.name) | .").is_none());
        assert!(param_field_context("sort_by(.name).foo").is_none());
    }

    #[test]
    fn param_ctx_inside_parens_variants() {
        let c = param_field_context("sort_by(").unwrap();
        assert_eq!(c.fn_name, "sort_by");
        assert_eq!(c.inner_prefix, "");

        assert_eq!(param_field_context("sort_by(.").unwrap().inner_prefix, ".");
        assert_eq!(
            param_field_context("sort_by(.na").unwrap().inner_prefix,
            ".na"
        );
        assert_eq!(
            param_field_context("sort_by(.customer.na")
                .unwrap()
                .inner_prefix,
            ".customer.na"
        );
    }

    #[test]
    fn param_ctx_pipe_and_function_filtering() {
        let c = param_field_context(".orders[] | sort_by(.na").unwrap();
        assert_eq!(c.context_path, ".orders[]");
        assert_eq!(c.inner_prefix, ".na");

        assert!(param_field_context("map(.").is_none());
        assert!(param_field_context("select(.").is_none());
        assert!(param_field_context("with_entries(.").is_none());
        assert!(param_field_context("").is_none());
        assert!(param_field_context(".").is_none());
        assert_eq!(param_field_context("del(.").unwrap().fn_name, "del");
        assert_eq!(param_field_context("path(.").unwrap().fn_name, "path");
        assert_eq!(
            param_field_context("sort_by(.a) | group_by(.")
                .unwrap()
                .fn_name,
            "group_by"
        );
        assert_eq!(
            param_field_context("sort_by(.name, .")
                .unwrap()
                .inner_prefix,
            "."
        );
    }

    #[test]
    fn param_sort_by_basic_and_prefix() {
        let c = get_completions("sort_by(.", &json!([{"name":"a","age":1}]));
        assert!(labels(&c).contains(&"name".to_string()));
        assert!(labels(&c).contains(&"age".to_string()));
        assert!(has_insert(&c, "sort_by(.name"));
        assert!(has_insert(&c, "sort_by(.age"));

        let c = get_completions(
            "sort_by(.na",
            &json!([{"name":"a","namespace":"b","age":1}]),
        );
        let ls = labels(&c);
        assert!(ls.contains(&"name".to_string()));
        assert!(ls.contains(&"namespace".to_string()));
        assert!(!ls.contains(&"age".to_string()));
    }

    #[test]
    fn param_group_unique_min_max_and_pipe_context() {
        let c = get_completions("group_by(.", &json!([{"status":"x"}]));
        assert!(has_insert(&c, "group_by(.status"));

        let c = get_completions("unique_by(.", &json!([{"id":1}]));
        assert!(has_insert(&c, "unique_by(.id"));

        let c = get_completions("min_by(.", &json!([{"price":1.0,"qty":5}]));
        assert!(labels(&c).contains(&"price".to_string()));
        assert!(labels(&c).contains(&"qty".to_string()));

        let c = get_completions("max_by(.", &json!([{"score":99}]));
        assert!(labels(&c).contains(&"score".to_string()));

        let c = get_completions(
            ".orders[] | sort_by(.",
            &json!({"orders":[{"id":1,"total":9.9}]}),
        );
        assert!(has_insert(&c, ".orders[] | sort_by(.id"));
        assert!(has_insert(&c, ".orders[] | sort_by(.total"));
    }

    #[test]
    fn param_del_and_path_completions() {
        let c = get_completions("del(.", &json!({"name":"alice","age":30}));
        assert!(has_insert(&c, "del(.name"));
        assert!(has_insert(&c, "del(.age"));

        let c = get_completions("del(.ag", &json!({"name":"alice","age":30}));
        let ls = labels(&c);
        assert_eq!(ls, vec!["age".to_string()]);

        let c = get_completions("path(.", &json!({"a":1,"b":2}));
        assert!(has_insert(&c, "path(.a"));
        assert!(has_insert(&c, "path(.b"));

        let c = get_completions(".user | del(.", &json!({"user":{"id":1,"token":"x"}}));
        assert!(has_insert(&c, ".user | del(.id"));
        assert!(has_insert(&c, ".user | del(.token"));
    }

    #[test]
    fn param_nested_field_paths() {
        let c = get_completions(
            "sort_by(.customer.",
            &json!([{"customer":{"name":"a","id":1}}]),
        );
        assert!(has_insert(&c, "sort_by(.customer.name"));
        assert!(has_insert(&c, "sort_by(.customer.id"));
    }

    #[test]
    fn param_list_syntax_completions_supported() {
        let c = get_completions("sort_by([.", &json!([{"name":"Alice","age":30}]));
        assert!(has_insert(&c, "sort_by([.name"));
        assert!(has_insert(&c, "sort_by([.age"));

        let c = get_completions(
            "sort_by([.name, .",
            &json!([{"name":"Alice","age":30,"order_date":"2024-01-01"}]),
        );
        assert!(has_insert(&c, "sort_by([.name, .age"));
        assert!(has_insert(&c, "sort_by([.name, .order_date"));
    }

    #[test]
    fn param_list_syntax_exit_context_after_closing_paren() {
        let input = json!([{"name":"Alice","age":30}]);
        assert!(
            get_completions("sort_by([.name, .age])", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("sort_by(["))
        );
        assert!(
            get_completions("sort_by([.name, .age]) | .", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("sort_by(["))
        );
    }

    #[test]
    fn param_exit_and_excluded_functions() {
        let input = json!([{"x":1,"name":"a"}]);
        assert!(
            get_completions("sort_by(.name)", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("sort_by("))
        );
        assert!(
            get_completions("sort_by(.name) | .", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("sort_by("))
        );
        assert!(
            get_completions("sort_by(.name).f", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("sort_by("))
        );
        assert!(
            get_completions("map(.", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("map("))
        );
        assert!(
            get_completions("select(.", &json!({"a":1}))
                .iter()
                .all(|c| !c.insert_text.starts_with("select("))
        );
        assert!(
            get_completions("with_entries(.", &json!({"a":1}))
                .iter()
                .all(|c| !c.insert_text.starts_with("with_entries("))
        );
        assert!(
            get_completions("any(.", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("any("))
        );
        assert!(
            get_completions("all(.", &input)
                .iter()
                .all(|c| !c.insert_text.starts_with("all("))
        );
    }

    #[test]
    fn param_edge_cases_and_guard_conditions() {
        assert!(get_completions("sort_by(.", &json!([])).is_empty());
        assert!(get_completions("sort_by(.", &json!([1, 2, 3])).is_empty());
        assert!(get_completions("sort_by(.", &json!("hello")).is_empty());
        assert!(get_completions("sort_by(.", &json!(null)).is_empty());
        assert!(get_completions(".missing | sort_by(.", &json!({"other":[]})).is_empty());
        assert!(get_completions("del(.", &json!([1, 2])).is_empty());
        assert!(get_completions("del(.", &json!("hello")).is_empty());

        let c = get_completions("sort_by(.na", &json!([{"name":"a"}]));
        assert!(
            c.iter()
                .filter(|i| i.insert_text.starts_with("sort_by("))
                .all(|i| !i.insert_text.ends_with(')'))
        );

        let c = get_completions("sort_by(.a) | group_by(.", &json!([{"x":1}]));
        assert!(has_insert(&c, "sort_by(.a) | group_by(.x"));

        let c = get_completions(".a | del(.", &json!({"a":{"x":1}}));
        let cnt = c
            .iter()
            .filter(|i| i.label == "x" && i.insert_text == ".a | del(.x")
            .count();
        assert_eq!(cnt, 1);
    }

    fn has_label(v: &[CompletionItem], label: &str) -> bool {
        v.iter().any(|c| c.label == label)
    }

    fn has_insert_text(v: &[CompletionItem], insert_text: &str) -> bool {
        v.iter().any(|c| c.insert_text == insert_text)
    }

    #[test]
    fn string_param_context_enters_expected_variants() {
        let split = string_param_context("split(").unwrap();
        assert_eq!(split.fn_name, "split");
        assert_eq!(split.strategy, StringParamStrategy::Internal);
        assert_eq!(split.context_path, ".");
        assert_eq!(split.inner_prefix, "");

        assert_eq!(string_param_context("split(\"").unwrap().inner_prefix, "");
        assert_eq!(string_param_context("split(\"-").unwrap().inner_prefix, "-");

        let starts = string_param_context("startswith(\"shi").unwrap();
        assert_eq!(starts.strategy, StringParamStrategy::Prefix);
        assert_eq!(starts.inner_prefix, "shi");

        let ends = string_param_context("endswith(\".com").unwrap();
        assert_eq!(ends.strategy, StringParamStrategy::Suffix);
        assert_eq!(ends.inner_prefix, ".com");

        assert_eq!(
            string_param_context("ltrimstr(").unwrap().strategy,
            StringParamStrategy::Prefix
        );
        assert_eq!(
            string_param_context("rtrimstr(").unwrap().strategy,
            StringParamStrategy::Suffix
        );
        assert_eq!(
            string_param_context("contains(").unwrap().strategy,
            StringParamStrategy::FullString
        );
        assert_eq!(
            string_param_context("index(").unwrap().strategy,
            StringParamStrategy::FullString
        );
        assert_eq!(
            string_param_context("rindex(").unwrap().strategy,
            StringParamStrategy::FullString
        );
        assert_eq!(
            string_param_context("indices(").unwrap().strategy,
            StringParamStrategy::FullString
        );
    }

    #[test]
    fn string_param_context_pipe_and_nested_function_cases() {
        let pipe = string_param_context(".orders[].order_status | split(\"_").unwrap();
        assert_eq!(pipe.context_path, ".orders[].order_status");
        assert_eq!(pipe.inner_prefix, "_");

        let nested = string_param_context("map(split(\"").unwrap();
        assert_eq!(nested.fn_name, "split");
        assert_eq!(nested.context_path, ".");

        let piped_fn = string_param_context("ascii_upcase|endswith(\"").unwrap();
        assert_eq!(piped_fn.fn_name, "endswith");
        assert_eq!(piped_fn.context_path, ".");
    }

    #[test]
    fn string_param_context_exits_and_excludes_functions() {
        assert!(string_param_context("split(\"-\")").is_none());
        assert!(string_param_context("split(\"-\") | .").is_none());
        assert!(string_param_context("split(\"-\").foo").is_none());

        assert!(string_param_context("test(\"").is_none());
        assert!(string_param_context("match(\"").is_none());
        assert!(string_param_context("scan(\"").is_none());
        assert!(string_param_context("sub(\"").is_none());
        assert!(string_param_context("gsub(\"").is_none());
        assert!(string_param_context("capture(\"").is_none());
        assert!(string_param_context("strptime(\"").is_none());
        assert!(string_param_context("strftime(\"").is_none());

        assert!(string_param_context("").is_none());
        assert!(string_param_context(".").is_none());
        assert!(string_param_context("sort_by(.").is_none());
    }

    #[test]
    fn collect_string_values_handles_scalar_array_object_and_caps() {
        assert_eq!(collect_string_values(&json!("x")), vec!["x"]);
        assert_eq!(
            collect_string_values(&json!(["x", 1, null, "y"])),
            vec!["x", "y"]
        );

        let obj = json!({"a":"foo", "b":1, "c":"bar"});
        let mut got = collect_string_values(&obj);
        got.sort();
        assert_eq!(got, vec!["bar", "foo"]);

        assert!(collect_string_values(&json!(42)).is_empty());
        assert!(collect_string_values(&json!(null)).is_empty());

        let many = json!((0..1000).map(|i| format!("s{}", i)).collect::<Vec<_>>());
        assert_eq!(collect_string_values(&many).len(), 500);
    }

    #[test]
    fn extract_prefix_candidates_behaviour() {
        let c = extract_prefix_candidates(&["CUST-42", "CUST-17"]);
        assert!(c.contains(&"CUST".to_string()));
        assert!(!c.contains(&"CUST-".to_string()));

        // Multi-segment strings emit an intermediate prefix per token boundary
        let c = extract_prefix_candidates(&["2026-04-19"]);
        assert!(c.contains(&"2026".to_string()));
        assert!(c.contains(&"2026-04".to_string()));
        assert!(c.contains(&"2026-04-19".to_string()));

        let c = extract_prefix_candidates(&["shipped", "processing", "delivered"]);
        assert!(c.contains(&"shipped".to_string()));
        assert!(c.contains(&"processing".to_string()));
        assert!(c.contains(&"delivered".to_string()));

        assert!(extract_prefix_candidates(&[]).is_empty());

        let c = extract_prefix_candidates(&["aa-bb", "aa-cc", "aa-bb"]);
        let mut sorted = c.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(c, sorted);
    }

    #[test]
    fn extract_suffix_candidates_behaviour() {
        let c = extract_suffix_candidates(&["alice@example.com", "mikko@example.com"]);
        assert!(c.contains(&"com".to_string()));
        assert!(!c.contains(&".com".to_string()));
        assert!(c.contains(&"example.com".to_string()));
        assert!(c.contains(&"@example.com".to_string()));

        let c = extract_suffix_candidates(&["shipped", "delivered"]);
        assert!(c.contains(&"shipped".to_string()));
        assert!(c.contains(&"delivered".to_string()));

        assert!(extract_suffix_candidates(&[]).is_empty());

        let c = extract_suffix_candidates(&["x-a", "y-a", "x-a"]);
        let mut sorted = c.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(c, sorted);
    }

    #[test]
    fn extract_internal_candidates_behaviour() {
        let c = extract_internal_candidates(&["CUST-42", "ORD-001", "STORE-001"]);
        assert!(c.contains(&"-".to_string()));

        let c = extract_internal_candidates(&["foo.bar", "alpha", "beta"]);
        assert!(!c.contains(&".".to_string()));

        let c = extract_internal_candidates(&["foo__bar", "x__y", "plain"]);
        assert!(c.contains(&"__".to_string()));

        assert!(extract_internal_candidates(&[]).is_empty());
        assert!(extract_internal_candidates(&["alpha", "beta"]).is_empty());
    }

    #[test]
    fn extract_fullstring_candidates_behaviour() {
        assert_eq!(
            extract_fullstring_candidates(&["shipped", "processing", "shipped"]),
            vec!["processing".to_string(), "shipped".to_string()]
        );
        assert!(extract_fullstring_candidates(&[]).is_empty());
    }

    #[test]
    fn string_param_completion_resolves_for_entering_context() {
        let c = get_completions("split(", &json!(["a-b", "c-d"]));
        assert!(has_label(&c, "-"), "got: {:?}", c);
        assert!(has_insert_text(&c, "split(\"-\")"));

        let c = get_completions("split(\"-", &json!(["a-b", "c-d"]));
        assert!(has_label(&c, "-"));

        let c = get_completions("startswith(", &json!(["shipped", "processing"]));
        assert!(has_label(&c, "shipped"));
        assert!(has_label(&c, "processing"));
        assert!(has_insert_text(&c, "startswith(\"shipped\")"));

        let c = get_completions("startswith(\"sh", &json!(["shipped", "processing"]));
        assert!(has_label(&c, "shipped"));
        assert!(!has_label(&c, "processing"));

        let c = get_completions("endswith(", &json!(["alice@example.com"]));
        assert!(has_label(&c, "com"));
        assert!(has_label(&c, "example.com"));
        assert!(has_label(&c, "@example.com"));

        let c = get_completions("ltrimstr(", &json!(["CUST-42", "CUST-17"]));
        assert!(has_label(&c, "CUST"));
        assert!(!has_label(&c, "CUST-"));

        let c = get_completions("rtrimstr(", &json!(["alice@example.com"]));
        assert!(has_label(&c, "com"));

        let c = get_completions("contains(", &json!(["shipped", "processing"]));
        assert!(has_label(&c, "shipped"));
        assert!(has_label(&c, "processing"));

        let c = get_completions("index(", &json!(["shipped"]));
        assert!(has_label(&c, "shipped"));

        let c = get_completions(
            ".orders[].order_status | split(",
            &json!({"orders": [{"order_status": "ship-fast"}, {"order_status": "plan-ahead"}]}),
        );
        assert!(has_label(&c, "-"));

        let c = get_completions("ascii_upcase|endswith(\"", &json!("kakaka"));
        assert!(has_label(&c, "kakaka"));
    }

    #[test]
    fn string_tokenization_breaks_on_requested_special_characters() {
        let values = [
            "Alice Smith",
            "Alice\tJones",
            "Alice,Brown",
            "Alice.Green",
            "Alice|White",
            "Alice-Black",
        ];

        let starts = extract_prefix_candidates(&values);
        let alice_count = starts.iter().filter(|i| *i == "Alice").count();
        assert_eq!(
            alice_count, 1,
            "duplicate token suggestions are not allowed"
        );

        let ends = extract_suffix_candidates(&values);
        assert!(ends.contains(&"Smith".to_string()));
        assert!(ends.contains(&"Jones".to_string()));
        assert!(ends.contains(&"Brown".to_string()));
        assert!(ends.contains(&"Green".to_string()));
        assert!(ends.contains(&"White".to_string()));
        assert!(ends.contains(&"Black".to_string()));
    }

    #[test]
    fn string_param_completion_not_offered_when_outside_or_excluded() {
        let c = get_completions("split(\"-\")", &json!(["a-b"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("split(\"")));

        let c = get_completions("split(\"-\") | .", &json!(["a-b"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("split(\"")));

        let c = get_completions("test(", &json!(["foo"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("test(\"")));

        let c = get_completions("match(", &json!(["foo"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("match(\"")));

        let c = get_completions("gsub(", &json!(["foo"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("gsub(\"")));

        let c = get_completions("strptime(", &json!(["2024-01-01"]));
        assert!(c.iter().all(|i| !i.insert_text.starts_with("strptime(\"")));
    }

    #[test]
    fn suffix_filtering_and_shortest_first_ordering() {
        let c = get_completions(
            "endswith(\"com",
            &json!(["alice@corp.com", "mikko@example.com"]),
        );
        let labels: Vec<_> = c.iter().map(|i| i.label.as_str()).collect();
        assert_eq!(labels.first().copied(), Some("com"));
        assert!(labels.contains(&"corp.com"));
        assert!(labels.contains(&"@corp.com"));
    }

    #[test]
    fn exact_candidates_precede_fuzzy_candidates() {
        let c = get_completions("startswith(\"alp", &json!(["alpha", "alpine", "beta"]));
        let first_fuzzy = c
            .iter()
            .position(|i| i.detail.as_deref().is_some_and(|d| d.starts_with('~')))
            .unwrap_or(c.len());
        let last_exact = c
            .iter()
            .rposition(|i| !i.detail.as_deref().is_some_and(|d| d.starts_with('~')))
            .unwrap_or(0);
        assert!(last_exact <= first_fuzzy);
    }

    #[test]
    fn string_param_completion_edge_cases() {
        assert!(get_completions("split(", &json!([])).is_empty());
        assert!(get_completions("split(", &json!([1, 2, 3])).is_empty());
        assert!(get_completions("split(", &json!(42)).is_empty());
        assert!(get_completions("split(", &json!(null)).is_empty());
        assert!(get_completions(".missing | split(", &json!({"other": ["x"]})).is_empty());

        let c = get_completions("contains(", &json!(["say \"hi\""]));
        assert!(
            c.iter()
                .any(|i| i.insert_text == "contains(\"say \\\"hi\\\"\")")
        );

        let c = get_completions("contains(", &json!(["abc", "abc"]));
        let count = c.iter().filter(|i| i.label == "abc").count();
        assert_eq!(count, 1);

        let c = get_completions("startswith(\"shiped", &json!(["shipped"]));
        assert!(has_label(&c, "shipped"));
    }
}
