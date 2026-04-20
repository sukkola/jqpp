use jqpp::completions;
use jqpp::widgets;

pub fn strip_sgr_mouse_sequences(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let bytes = input.as_bytes();

    while i < bytes.len() {
        if let Some(skip) = sgr_mouse_sequence_len(&bytes[i..]) {
            i += skip;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

pub fn sgr_mouse_sequence_len(input: &[u8]) -> Option<usize> {
    let mut idx = 0;
    if input.first().copied() == Some(0x1b) {
        idx += 1;
    }

    if input.get(idx).copied() != Some(b'[') || input.get(idx + 1).copied() != Some(b'<') {
        return None;
    }
    idx += 2;

    let take_digits = |input: &[u8], idx: &mut usize| -> bool {
        let start = *idx;
        while input.get(*idx).copied().is_some_and(|b| b.is_ascii_digit()) {
            *idx += 1;
        }
        *idx > start
    };

    if !take_digits(input, &mut idx) || input.get(idx).copied() != Some(b';') {
        return None;
    }
    idx += 1;

    if !take_digits(input, &mut idx) || input.get(idx).copied() != Some(b';') {
        return None;
    }
    idx += 1;

    if !take_digits(input, &mut idx) {
        return None;
    }

    match input.get(idx).copied() {
        Some(b'm') | Some(b'M') => Some(idx + 1),
        _ => None,
    }
}

pub fn is_field_path_function_call_start(suggestion: &str) -> bool {
    let trimmed = suggestion.trim_end();
    if !(trimmed.ends_with('(') || trimmed.ends_with(')')) {
        return false;
    }
    let Some(open_idx) = trimmed.rfind('(') else {
        return false;
    };
    let before = trimmed[..open_idx].trim_end();
    let fn_name = before
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .rfind(|s| !s.is_empty())
        .unwrap_or("");
    matches!(
        fn_name,
        "sort_by"
            | "group_by"
            | "unique_by"
            | "min_by"
            | "max_by"
            | "del"
            | "path"
            | "has"
            | "flatten" // "range" intentionally omitted: json_context returns no live completions for
                        // range arguments, so keeping keep_active=true only causes stale suggestions to
                        // stay visible and lets them overwrite the accepted form.
    )
}

pub fn starts_context_aware_function_call(suggestion: &str) -> bool {
    is_field_path_function_call_start(suggestion)
        || completions::json_context::string_param_context(suggestion, None).is_some()
        || suggestion
            .strip_suffix(')')
            .map(|s| completions::json_context::string_param_context(s, None).is_some())
            .unwrap_or(false)
}

pub fn apply_suggestion_with_suffix(suggestion: &str, suffix: &str) -> String {
    let mut s_idx = 0;
    if (suggestion.ends_with(')') && suffix.starts_with(')'))
        || (suggestion.ends_with(']') && suffix.starts_with(']'))
        || (suggestion.ends_with('}') && suffix.starts_with('}'))
    {
        s_idx = 1;
    }
    format!("{}{}", suggestion, &suffix[s_idx..])
}

pub fn is_string_param_value_suggestion(detail: Option<&str>) -> bool {
    detail
        .map(|d| d == "string value" || d == "~string value")
        .unwrap_or(false)
}

pub fn is_contains_builder_suggestion(detail: Option<&str>) -> bool {
    matches!(
        detail,
        Some("contains array value") | Some("contains object key") | Some("contains object value")
    )
}

pub fn is_numeric_builder_suggestion(detail: Option<&str>) -> bool {
    matches!(
        detail,
        Some("flatten nested arrays") | Some("integer generator") | Some("depth")
    )
}

pub fn is_builder_suggestion(detail: Option<&str>) -> bool {
    is_contains_builder_suggestion(detail) || is_numeric_builder_suggestion(detail)
}

fn trim_trailing_array_or_object_separators(s: &mut String) {
    while s.ends_with(' ') || s.ends_with(',') {
        s.pop();
    }
}

pub fn apply_contains_builder_suggestion(
    insert_text: &str,
    detail: Option<&str>,
    full_query: &str,
    cursor_col: usize,
    finalize: bool,
) -> (String, u16, bool) {
    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let tail_after_call = suffix
        .find(')')
        .map(|i| suffix[i + 1..].to_string())
        .unwrap_or_default();
    let mut merged = format!("{}{}", insert_text, tail_after_call);

    match detail {
        Some("contains object key") => {
            let col = merged.chars().count() as u16;
            (merged, col, true)
        }
        Some("contains object value") => {
            if finalize {
                trim_trailing_array_or_object_separators(&mut merged);
                if merged.ends_with(')') {
                    merged.pop();
                }
                if !merged.ends_with('}') {
                    merged.push('}');
                }
                merged.push(')');
                let col = merged.chars().count() as u16;
                (merged, col, false)
            } else {
                if !merged.ends_with(", ") {
                    merged.push_str(", ");
                }
                let col = merged.chars().count() as u16;
                (merged, col, true)
            }
        }
        Some("contains array value") => {
            if finalize {
                trim_trailing_array_or_object_separators(&mut merged);
                if merged.ends_with(')') {
                    merged.pop();
                }
                if !merged.ends_with(']') {
                    merged.push(']');
                }
                merged.push(')');
                let col = merged.chars().count() as u16;
                (merged, col, false)
            } else {
                if !merged.ends_with(", ") {
                    merged.push_str(", ");
                }
                let col = merged.chars().count() as u16;
                (merged, col, true)
            }
        }
        _ => {
            let col = merged.chars().count() as u16;
            (merged, col, false)
        }
    }
}

/// Given the function name of the suggestion being inserted (e.g. `"flatten"`),
/// the token the user was typing (e.g. `"fla"`), and the text after the cursor,
/// strip any overlapping function call from the suffix.
///
/// This handles the case where the user typed a partial name directly before an
/// existing call for the same function.  For example:
/// - insert_fn = "flatten", token = "fla", suffix = "flatten(3)" → returns ""
/// - insert_fn = "flatten", token = "fla", suffix = "flatten(1)|..."  → returns "|..."
/// - insert_fn = "flatten", token = "",    suffix = " | other"        → unchanged
fn strip_overlapping_call<'a>(insert_fn: &str, token: &str, suffix: &'a str) -> &'a str {
    if insert_fn.is_empty() || !insert_fn.starts_with(token) {
        return suffix;
    }
    if !suffix.starts_with(insert_fn) {
        return suffix;
    }
    let after_name = &suffix[insert_fn.len()..];
    if after_name.starts_with('(') {
        let mut depth = 0i32;
        for (i, ch) in after_name.char_indices() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return &after_name[i + 1..];
                    }
                }
                _ => {}
            }
        }
    }
    after_name
}

/// Return the token (trailing identifier) that ends at `cursor_col`.
fn token_at_cursor(full_query: &str, cursor_col: usize) -> &str {
    let prefix: &str = &full_query[..full_query
        .char_indices()
        .nth(cursor_col)
        .map(|(b, _)| b)
        .unwrap_or(full_query.len())];
    let start = prefix
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_')
        .last()
        .map(|(b, _)| b)
        .unwrap_or(prefix.len());
    &prefix[start..]
}

pub fn apply_numeric_builder_suggestion(
    insert_text: &str,
    _detail: Option<&str>,
    full_query: &str,
    cursor_col: usize,
    finalize: bool,
) -> (String, u16, bool) {
    // Treat as initial acceptance when:
    //   (a) no `(` in the query at all, OR
    //   (b) cursor is outside all open parens AND a non-empty token is being
    //       typed.  The token check excludes the case where the cursor sits
    //       right after an existing closing `)` (e.g. in `range(0)` at the
    //       end) which must fall through to the step-through / finalize logic.
    let in_open_paren = find_unmatched_open_paren(&full_query[..cursor_col]).is_some();
    let active_token = token_at_cursor(full_query, cursor_col);
    let is_initial_acceptance =
        !full_query.contains('(') || (!in_open_paren && !active_token.is_empty());

    if is_initial_acceptance {
        let raw_suffix = &full_query[cursor_col..];
        let token = token_at_cursor(full_query, cursor_col);
        let insert_fn = insert_text.split('(').next().unwrap_or("");
        let suffix = strip_overlapping_call(insert_fn, token, raw_suffix);
        let merged = apply_suggestion_with_suffix(insert_text, suffix);
        let col = cursor_col_after_accept(&merged);
        return (merged, col, true);
    }

    // Depth candidates generated by json_context (e.g. "flatten(1)" for the
    // `flatten` builder) are complete single-arg calls.  The general finalize /
    // step logic below never touches `insert_text`, so it would silently discard
    // the value.  Apply the insert_text directly instead.
    // Exclude empty-paren forms like "range()" which are builder placeholders,
    // not complete calls.
    if insert_text.ends_with(')') && !insert_text.contains(';') && !insert_text.ends_with("()") {
        let suffix: String = full_query.chars().skip(cursor_col).collect();
        let tail_after = suffix
            .find(')')
            .map(|i| suffix[i + 1..].to_string())
            .unwrap_or_default();
        let merged = format!("{}{}", insert_text, tail_after);
        let col = merged.chars().count() as u16;
        return (merged, col, false);
    }

    if finalize {
        let open = find_unmatched_open_paren(&full_query[..cursor_col])
            .or_else(|| full_query.rfind('('))
            .unwrap_or(0);
        let close = full_query[open..]
            .find(')')
            .unwrap_or(full_query.len() - open);
        (full_query.to_string(), (open + close + 1) as u16, false)
    } else {
        let mut merged = full_query.to_string();
        let open = find_unmatched_open_paren(&full_query[..cursor_col])
            .or_else(|| full_query.rfind('('))
            .unwrap_or(0);
        let close = full_query[open..]
            .find(')')
            .unwrap_or(full_query.len() - open);
        let inner = &full_query[open + 1..open + close];
        let fn_name = full_query[..open]
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .rfind(|s| !s.is_empty())
            .unwrap_or("");

        let semi_count = inner.chars().filter(|c| *c == ';').count();

        let tail_end = open + close;
        let tail_start = cursor_col.min(tail_end);
        let tail = &full_query[tail_start..tail_end];
        if let Some(semi_rel) = tail.find(';') {
            let mut next_pos = tail_start + semi_rel + 1;
            while merged.chars().nth(next_pos) == Some(' ') {
                next_pos += 1;
            }
            (merged, next_pos as u16, true)
        } else {
            let max_semis = match fn_name {
                "range" => 2,
                _ => 0,
            };

            if semi_count < max_semis {
                let insert_pos = cursor_col.min(tail_end);
                merged.insert_str(insert_pos, "; ");
                (merged, (insert_pos + 2) as u16, true)
            } else {
                (merged, (open + close + 1) as u16, false)
            }
        }
    }
}

pub fn finalize_numeric_builder_on_escape(
    full_query: &str,
    cursor_col: usize,
) -> Option<(String, u16)> {
    let open = find_unmatched_open_paren(&full_query[..cursor_col])?;
    let close = full_query[open..].find(')')?;
    Some((full_query.to_string(), (open + close + 1) as u16))
}

pub fn finalize_contains_builder_on_escape(
    full_query: &str,
    cursor_col: usize,
) -> Option<(String, u16)> {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let ctx = completions::json_context::param_field_context(&query_prefix)?;
    if ctx.fn_name != "contains" {
        return None;
    }

    let open = find_unmatched_open_paren(&query_prefix)?;
    let mut inner = query_prefix[open + 1..].trim_end().to_string();
    if inner.is_empty() {
        return None;
    }

    while inner.ends_with(' ') || inner.ends_with(',') {
        inner.pop();
    }

    if inner.starts_with('{') {
        if inner.ends_with(':') {
            if let Some(comma) = inner.rfind(',') {
                inner.truncate(comma);
                while inner.ends_with(' ') || inner.ends_with(',') {
                    inner.pop();
                }
            } else {
                inner = "{".to_string();
            }
        }
        if !inner.ends_with('}') {
            inner.push('}');
        }
    } else if inner.starts_with('[') {
        if !inner.ends_with(']') {
            inner.push(']');
        }
    } else {
        return None;
    }

    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let tail_after_call = suffix
        .find(')')
        .map(|i| suffix[i + 1..].to_string())
        .unwrap_or_default();

    let committed = format!("{}{})", &query_prefix[..open + 1], inner);
    let new_query = format!("{}{}", committed, tail_after_call);
    Some((new_query, committed.chars().count() as u16))
}

pub fn apply_selected_suggestion(
    insert_text: &str,
    detail: Option<&str>,
    full_query: &str,
    cursor_col: usize,
) -> (String, u16) {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let mut suffix: String = full_query.chars().skip(cursor_col).collect();

    if is_string_param_value_suggestion(detail)
        && completions::json_context::string_param_context(&query_prefix, None).is_some()
    {
        if let Some(close_idx) = suffix.find(')') {
            suffix = suffix[close_idx + 1..].to_string();
        } else {
            suffix.clear();
        }
        let merged = format!("{}{}", insert_text, suffix);
        return (merged, insert_text.chars().count() as u16);
    }

    // Strip any overlapping same-function call from the suffix so that accepting
    // "flatten()" when cursor is in the middle of "flaflatten(3)" replaces the
    // whole existing call instead of concatenating.
    let token = token_at_cursor(full_query, cursor_col);
    let insert_fn = insert_text.split('(').next().unwrap_or("");
    let clean_suffix = strip_overlapping_call(insert_fn, token, &suffix);

    let merged = apply_suggestion_with_suffix(insert_text, clean_suffix);
    let col = if starts_context_aware_function_call(insert_text) {
        cursor_col_after_accept(insert_text)
    } else {
        insert_text.chars().count() as u16
    };
    (merged, col)
}

pub fn find_unmatched_open_paren(query: &str) -> Option<usize> {
    let mut depth: i32 = 0;
    for (idx, ch) in query.char_indices().rev() {
        match ch {
            ')' => depth += 1,
            '(' => {
                depth -= 1;
                if depth < 0 {
                    return Some(idx);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn commit_current_string_param_input(
    full_query: &str,
    cursor_col: usize,
) -> Option<(String, u16)> {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let ctx = completions::json_context::string_param_context(&query_prefix, None)?;
    let open = find_unmatched_open_paren(&query_prefix)?;
    let escaped = ctx.inner_prefix.replace('"', "\\\"");
    let committed = format!("{}\"{}\")", &query_prefix[..open + 1], escaped);
    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let tail = suffix
        .find(')')
        .map(|i| suffix[i + 1..].to_string())
        .unwrap_or_default();
    let new_query = format!("{}{}", committed, tail);
    Some((new_query, committed.chars().count() as u16))
}

pub fn longest_common_prefix(values: &[String]) -> String {
    let Some(first) = values.first() else {
        return String::new();
    };
    let mut prefix = first.clone();
    for value in values.iter().skip(1) {
        let mut bytes = 0usize;
        for (a, b) in prefix.chars().zip(value.chars()) {
            if a != b {
                break;
            }
            bytes += a.len_utf8();
        }
        prefix.truncate(bytes);
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

pub fn is_string_token_delim(ch: char) -> bool {
    matches!(
        ch,
        '\\' | '-' | '_' | '/' | '.' | ' ' | '\t' | ',' | '|' | '@'
    )
}

pub fn extend_to_next_token_boundary(current: &str, candidate: &str) -> Option<String> {
    if !candidate.starts_with(current) || candidate == current {
        return None;
    }

    let mut out = String::from(current);
    let mut seen_non_delim = false;
    for ch in candidate[current.len()..].chars() {
        if seen_non_delim && is_string_token_delim(ch) {
            break;
        }
        if !is_string_token_delim(ch) {
            seen_non_delim = true;
        }
        out.push(ch);
    }

    if out.len() > current.len() {
        Some(out)
    } else {
        None
    }
}

pub fn expand_string_param_prefix_with_tab(
    full_query: &str,
    cursor_col: usize,
    suggestions: &[widgets::query_input::Suggestion],
    suggestion_index: usize,
) -> Option<(String, u16)> {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let ctx = completions::json_context::string_param_context(&query_prefix, None)?;
    let open = find_unmatched_open_paren(&query_prefix)?;

    let candidates: Vec<String> = suggestions
        .iter()
        .filter(|s| is_string_param_value_suggestion(s.detail.as_deref()))
        .map(|s| s.label.clone())
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let preferred = suggestions
        .get(suggestion_index)
        .filter(|s| is_string_param_value_suggestion(s.detail.as_deref()))
        .map(|s| s.label.as_str())
        .or_else(|| {
            candidates
                .iter()
                .find(|c| c.starts_with(ctx.inner_prefix))
                .map(|s| s.as_str())
        })?;

    let extended = if suggestion_index > 0 {
        Some(preferred.to_string())
    } else if matches!(
        ctx.strategy,
        completions::json_context::StringParamStrategy::Suffix
    ) {
        candidates
            .iter()
            .filter(|c| c.len() > ctx.inner_prefix.len() && c.ends_with(ctx.inner_prefix))
            .min_by(|a, b| a.len().cmp(&b.len()).then_with(|| a.cmp(b)))
            .cloned()
    } else {
        extend_to_next_token_boundary(ctx.inner_prefix, preferred).or_else(|| {
            if candidates.len() == 1 {
                Some(preferred.to_string())
            } else {
                let lcp = longest_common_prefix(&candidates);
                if lcp.chars().count() > ctx.inner_prefix.chars().count() {
                    Some(lcp)
                } else {
                    None
                }
            }
        })
    }?;
    if extended.chars().count() <= ctx.inner_prefix.chars().count() {
        return None;
    }

    let escaped = extended.replace('"', "\\\"");
    let expanded = format!("{}\"{}", &query_prefix[..open + 1], escaped);
    let suffix: String = full_query.chars().skip(cursor_col).collect();
    let new_query = format!("{}{}", expanded, suffix);
    Some((new_query, expanded.chars().count() as u16))
}

pub fn cursor_col_after_accept(suggestion: &str) -> u16 {
    if let Some(p) = suggestion.rfind("(\"") {
        (p + 2) as u16
    } else if let Some(p) = suggestion.rfind('(') {
        if suggestion.ends_with(')') {
            (p + 1) as u16
        } else {
            suggestion.chars().count() as u16
        }
    } else {
        suggestion.chars().count() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── New multi-arg catalog entries: accept / cursor behavior ──────────────

    // ── flatten depth-candidate builder fix ───────────────────────────────────

    #[test]
    fn flatten_depth_candidate_enter_inserts_value_when_paren_already_open() {
        // User typed `flatten(` manually, json_context offers depth "1" with
        // insert_text "flatten(1)".  Enter should produce "flatten(1)" not
        // leave the query as "flatten(" with a stray cursor.
        let (q, col, keep) = apply_numeric_builder_suggestion(
            "flatten(1)",
            Some("depth"),
            "flatten(",
            "flatten(".chars().count(),
            true,
        );
        assert_eq!(q, "flatten(1)");
        assert_eq!(col as usize, "flatten(1)".chars().count());
        assert!(!keep, "should close after finalizing depth");
    }

    #[test]
    fn flatten_depth_candidate_tab_also_inserts_value_when_paren_already_open() {
        // Tab (finalize=false) must also insert the value — flatten has only
        // one arg, so there is nothing to step through.
        let (q, col, keep) = apply_numeric_builder_suggestion(
            "flatten(2)",
            Some("depth"),
            "flatten(",
            "flatten(".chars().count(),
            false,
        );
        assert_eq!(q, "flatten(2)");
        assert_eq!(col as usize, "flatten(2)".chars().count());
        assert!(!keep);
    }

    #[test]
    fn flatten_depth_candidate_preserves_tail_after_closing_paren() {
        // If there is already a `)` in the suffix (e.g. user typed `flatten()`)
        // the tail after `)` must not be duplicated.
        let full = "flatten()";
        let cursor = "flatten(".chars().count();
        let (q, col, keep) =
            apply_numeric_builder_suggestion("flatten(1)", Some("depth"), full, cursor, true);
        assert_eq!(q, "flatten(1)");
        assert_eq!(col as usize, "flatten(1)".chars().count());
        assert!(!keep);
    }

    #[test]
    fn flatten_depth_candidate_enter_in_pipe_context_inserts_value() {
        // Same fix should work when flatten is part of a longer expression.
        let full = ".items | flatten(";
        let cursor = full.chars().count();
        let (q, col, keep) = apply_numeric_builder_suggestion(
            ".items | flatten(1)",
            Some("depth"),
            full,
            cursor,
            true,
        );
        assert_eq!(q, ".items | flatten(1)");
        assert_eq!(col as usize, ".items | flatten(1)".chars().count());
        assert!(!keep);
    }

    #[test]
    fn flatten_depth_form_is_context_aware_with_cursor_inside_parens() {
        // flatten(1) is in FIELD_PATH_INPUT_FNS → context-aware, cursor enters parens.
        assert!(is_field_path_function_call_start("flatten(1)"));
        assert!(starts_context_aware_function_call("flatten(1)"));
        // rfind '(' at 7, ends with ')' → cursor at 8 (after '(', before '1')
        assert_eq!(cursor_col_after_accept("flatten(1)"), 8);
    }

    #[test]
    fn flatten_depth_form_is_not_numeric_builder() {
        // "flatten N levels deep" must NOT trigger the numeric builder path —
        // the user already has a concrete depth value in the insert text.
        assert!(!is_numeric_builder_suggestion(Some(
            "flatten N levels deep"
        )));
        // Bare 'flatten' still IS a builder (user enters depth interactively).
        assert!(is_numeric_builder_suggestion(Some("flatten nested arrays")));
        // Depth completions generated by json_context (1/2/3) still are builders.
        assert!(is_numeric_builder_suggestion(Some("depth")));
    }

    #[test]
    fn flatten_depth_form_accept_inserts_and_places_cursor_inside_parens() {
        let (q, col) = apply_selected_suggestion("flatten(1)", None, "flat", 4);
        assert_eq!(q, "flatten(1)");
        assert_eq!(col, 8);
    }

    #[test]
    fn range_multi_arg_forms_are_plain_inserts_not_context_aware() {
        // range was removed from FIELD_PATH_INPUT_FNS because json_context returns no
        // live completions for it — keeping it context-aware only caused stale suggestions
        // to stay open and overwrite the accepted form.
        assert!(!is_field_path_function_call_start("range(0; 10)"));
        assert!(!starts_context_aware_function_call("range(0; 10)"));
        assert!(!starts_context_aware_function_call("range(0; 10; 2)"));
        // cursor_col_after_accept still reports 6 (the raw textual position inside
        // parens), but apply_selected_suggestion uses end position for non-context-aware
        // suggestions, so the actual cursor after acceptance lands at end.
        assert_eq!(cursor_col_after_accept("range(0; 10)"), 6);
    }

    #[test]
    fn range_multi_arg_accept_inserts_cursor_at_end() {
        // Not context-aware → apply_selected_suggestion puts cursor at end so the
        // dropdown closes and the user can navigate the complete form freely.
        let (q, col) = apply_selected_suggestion("range(0; 10)", None, "ran", 3);
        assert_eq!(q, "range(0; 10)");
        assert_eq!(col as usize, "range(0; 10)".chars().count());
    }

    #[test]
    fn paths_predicate_form_is_not_context_aware_cursor_at_end() {
        // 'paths' is not in FIELD_PATH_INPUT_FNS or any string-param list.
        // Cursor goes to end after accept — the full call is already complete.
        assert!(!is_field_path_function_call_start("paths(scalars)"));
        assert!(!starts_context_aware_function_call("paths(scalars)"));
        let (q, col) = apply_selected_suggestion("paths(scalars)", None, "paths", 5);
        assert_eq!(q, "paths(scalars)");
        assert_eq!(col as usize, "paths(scalars)".chars().count());
    }

    #[test]
    fn recurse_safe_form_is_not_context_aware_cursor_at_end() {
        // recurse(.[]?) is a complete expression — cursor goes to end.
        assert!(!starts_context_aware_function_call("recurse(.[]?)"));
        let (q, col) = apply_selected_suggestion("recurse(.[]?)", None, "rec", 3);
        assert_eq!(q, "recurse(.[]?)");
        assert_eq!(col as usize, "recurse(.[]?)".chars().count());
    }

    #[test]
    fn strptime_strftime_format_entries_not_context_aware_cursor_at_end() {
        // These are complete calls — not in any field-path or string-param list.
        // Accepting a strptime/strftime format suggestion puts cursor at end.
        for insert in &[
            "strptime(\"%Y-%m-%d\")",
            "strptime(\"%Y-%m-%dT%H:%M:%S\")",
            "strptime(\"%d/%m/%Y\")",
            "strftime(\"%Y-%m-%d\")",
            "strftime(\"%H:%M:%S\")",
        ] {
            assert!(
                !starts_context_aware_function_call(insert),
                "{} should not be context-aware",
                insert
            );
            let (q, col) = apply_selected_suggestion(insert, None, "str", 3);
            assert_eq!(q.as_str(), *insert);
            assert_eq!(
                col as usize,
                insert.chars().count(),
                "cursor should be at end for {}",
                insert
            );
        }
    }

    // ── has / contains accept behavior ────────────────────────────────────────

    #[test]
    fn has_string_form_places_cursor_after_opening_quote() {
        // has("key") — rfind '("' at 3, cursor at 5 (inside string, before "key")
        assert!(is_field_path_function_call_start("has(\"key\")"));
        assert_eq!(cursor_col_after_accept("has(\"key\")"), 5);
        let (q, col) = apply_selected_suggestion("has(\"key\")", None, "has", 3);
        assert_eq!(q, "has(\"key\")");
        assert_eq!(col, 5);
    }

    #[test]
    fn has_index_form_places_cursor_inside_parens_before_index() {
        // has(0) — rfind '(' at 3, ends with ')', cursor at 4 (before '0')
        assert!(is_field_path_function_call_start("has(0)"));
        assert_eq!(cursor_col_after_accept("has(0)"), 4);
        let (q, col) = apply_selected_suggestion("has(0)", None, "has", 3);
        assert_eq!(q, "has(0)");
        assert_eq!(col, 4);
    }

    #[test]
    fn contains_string_form_places_cursor_inside_empty_string() {
        // contains("") — string-param context, rfind '("' at 8, cursor at 10
        assert!(starts_context_aware_function_call("contains(\"\")"));
        assert_eq!(cursor_col_after_accept("contains(\"\")"), 10);
        let (q, col) = apply_selected_suggestion("contains(\"\")", None, "cont", 4);
        assert_eq!(q, "contains(\"\")");
        assert_eq!(col, 10);
    }

    #[test]
    fn contains_array_form_is_context_aware_cursor_inside_brackets() {
        // contains([]) — context-aware via strip_suffix branch, cursor at 9 (after '(')
        assert!(starts_context_aware_function_call("contains([])"));
        assert_eq!(cursor_col_after_accept("contains([])"), 9);
        let (q, col) = apply_selected_suggestion("contains([])", None, "cont", 4);
        assert_eq!(q, "contains([])");
        assert_eq!(col, 9);
    }

    #[test]
    fn contains_object_form_is_context_aware_cursor_inside_braces() {
        // contains({}) — context-aware via strip_suffix branch, cursor at 9 (after '(')
        assert!(starts_context_aware_function_call("contains({})"));
        assert_eq!(cursor_col_after_accept("contains({})"), 9);
        let (q, col) = apply_selected_suggestion("contains({})", None, "cont", 4);
        assert_eq!(q, "contains({})");
        assert_eq!(col, 9);
    }

    // ── overlapping token / pipe-context fixes ───────────────────────────────

    #[test]
    fn numeric_builder_in_pipe_context_completes_correctly() {
        // flatten(1)|fla + Tab on "flatten()" should produce flatten(1)|flatten()
        // with cursor inside the second flatten's parens, not cursor-to-pipe.
        let (q, col, keep) = apply_numeric_builder_suggestion(
            "flatten(1)|flatten()",
            Some("flatten nested arrays"),
            "flatten(1)|fla",
            "flatten(1)|fla".chars().count(),
            false,
        );
        assert_eq!(q, "flatten(1)|flatten()");
        assert_eq!(col, cursor_col_after_accept("flatten(1)|flatten()"));
        assert!(keep);
    }

    #[test]
    fn numeric_builder_replaces_overlapping_existing_call() {
        // "fla" typed at start of "flatten(3)" → "flaflatten(3)", cursor at 3.
        // Tab on "flatten()" should replace the whole thing with "flatten()".
        let (q, col, keep) = apply_numeric_builder_suggestion(
            "flatten()",
            Some("flatten nested arrays"),
            "flaflatten(3)",
            3,
            false,
        );
        assert_eq!(q, "flatten()");
        assert_eq!(col, 8); // cursor inside parens
        assert!(keep);
    }

    #[test]
    fn selected_suggestion_replaces_overlapping_existing_call() {
        // Same scenario but for a non-builder entry like flatten(1).
        let (q, col) = apply_selected_suggestion("flatten(1)", None, "flaflatten(3)", 3);
        assert_eq!(q, "flatten(1)");
        // flatten(1) is context-aware → cursor inside parens
        assert_eq!(col, cursor_col_after_accept("flatten(1)"));
    }

    #[test]
    fn selected_suggestion_no_overlap_when_suffix_is_unrelated() {
        // If suffix doesn't start with the function name, keep it unchanged.
        let (q, _col) = apply_selected_suggestion("flatten()", None, "flat | .items", 4);
        // The suffix " | .items" doesn't start with "flatten" → kept.
        assert!(q.contains(".items"), "unrelated suffix should be preserved");
    }

    #[test]
    fn numeric_builder_at_end_of_existing_call_is_not_initial() {
        // Cursor at end of "range(0)" — cursor sits right after ")", no token.
        // Must NOT be treated as initial acceptance; should step through args.
        // (regression guard for the !in_open_paren && !token condition)
        let full = "range(0)";
        let cursor = full.chars().count(); // cursor at end = position 8
        let (q, _col, _keep) = apply_numeric_builder_suggestion(
            "range()",
            Some("integer generator"),
            full,
            cursor,
            false,
        );
        // Step-through should insert "; " for the second range argument.
        assert_eq!(q, "range(0; )");
    }

    #[test]
    fn strips_single_sgr_mouse_sequence() {
        let input = "\u{1b}[<65;211;13M";
        assert_eq!(strip_sgr_mouse_sequences(input), "");
    }

    #[test]
    fn strips_repeated_sgr_mouse_sequences_from_paste() {
        let input = "[<65;211;13M[<65;211;13M[<64;211;13M";
        assert_eq!(strip_sgr_mouse_sequences(input), "");
    }

    #[test]
    fn cursor_position_enters_parentheses_for_field_path_functions() {
        assert_eq!(cursor_col_after_accept("sort_by()"), 8);
        assert_eq!(cursor_col_after_accept(".orders | sort_by()"), 18);
    }

    #[test]
    fn cursor_position_enters_parentheses_for_string_param_functions() {
        assert_eq!(cursor_col_after_accept("split()"), 6);
        assert_eq!(cursor_col_after_accept("startswith()"), 11);
    }

    #[test]
    fn field_path_function_start_detection_supports_empty_parens() {
        assert!(is_field_path_function_call_start("sort_by()"));
        assert!(is_field_path_function_call_start(".orders | del()"));
        assert!(is_field_path_function_call_start("has()"));
        assert!(!is_field_path_function_call_start("map(.)"));
    }

    #[test]
    fn context_aware_function_start_detection_includes_string_param_functions() {
        assert!(starts_context_aware_function_call("sort_by()"));
        assert!(starts_context_aware_function_call("has()"));
        assert!(starts_context_aware_function_call("split()"));
        assert!(starts_context_aware_function_call("startswith()"));
        assert!(starts_context_aware_function_call("contains()"));
        assert!(!starts_context_aware_function_call("map(.)"));
    }

    #[test]
    fn suggestion_accept_drops_redundant_closing_paren_from_suffix() {
        assert_eq!(
            apply_suggestion_with_suffix("split(\"-\")", ")"),
            "split(\"-\")"
        );
        assert_eq!(
            apply_suggestion_with_suffix("split(\"-\")", ") | ."),
            "split(\"-\") | ."
        );
    }

    #[test]
    fn suggestion_accept_drops_redundant_closing_bracket_from_suffix() {
        assert_eq!(apply_suggestion_with_suffix("[0]", "]"), "[0]");
        assert_eq!(
            apply_suggestion_with_suffix(".products[0]", "].name"),
            ".products[0].name"
        );
    }

    #[test]
    fn suggestion_accept_drops_redundant_closing_brace_from_suffix() {
        assert_eq!(apply_suggestion_with_suffix("{field: .", "}"), "{field: .}");
    }

    #[test]
    fn apply_selected_suggestion_for_string_param_replaces_existing_arg_and_moves_to_end() {
        let full = ".[].name|startswith(\"Alice\")";
        let cursor = ".[].name|startswith(\"".chars().count();
        let (new_query, col) = apply_selected_suggestion(
            ".[].name|startswith(\"Bob\")",
            Some("string value"),
            full,
            cursor,
        );
        assert_eq!(new_query, ".[].name|startswith(\"Bob\")");
        assert_eq!(col as usize, ".[].name|startswith(\"Bob\")".chars().count());
    }

    #[test]
    fn apply_selected_suggestion_keeps_function_cursor_inside_parens() {
        let full = "startswith";
        let cursor = full.chars().count();
        let (new_query, col) = apply_selected_suggestion("startswith()", None, full, cursor);
        assert_eq!(new_query, "startswith()");
        assert_eq!(col, 11);

        let (new_query, col) = apply_selected_suggestion("has()", None, "has", 3);
        assert_eq!(new_query, "has()");
        assert_eq!(col, 4);

        let (new_query, col) = apply_selected_suggestion("contains()", None, "cont", 4);
        assert_eq!(new_query, "contains()");
        assert_eq!(col, 9);
    }

    #[test]
    fn normalize_lsp_insert_text_removes_tabstop_for_string_functions() {
        assert_eq!(
            crate::suggestions::normalize_lsp_insert_text("startswith($0)", "startswith"),
            "startswith()"
        );
        assert_eq!(
            crate::suggestions::normalize_lsp_insert_text("endswith(${0})", "endswith"),
            "endswith()"
        );
    }

    #[test]
    fn build_lsp_suggestions_normalizes_snippets_and_keeps_pipe_prefix() {
        let cache = vec![completions::CompletionItem {
            label: "startswith".to_string(),
            detail: None,
            insert_text: "startswith($0)".to_string(),
        }];

        let s = crate::suggestions::build_lsp_suggestions(&cache, "st", ".users[].name|");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].insert_text, ".users[].name|startswith()");
        assert!(starts_context_aware_function_call(&s[0].insert_text));
        assert_eq!(cursor_col_after_accept(&s[0].insert_text), 25);
    }

    #[test]
    fn tab_completes_selected_item_fully_if_not_first_item() {
        let full = "startswith(\"";
        let cursor = full.chars().count();
        let suggestions = vec![
            widgets::query_input::Suggestion {
                label: "apple".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"apple\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "apple pie".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"apple pie\")".to_string(),
            },
        ];

        let (new_query, _) =
            expand_string_param_prefix_with_tab(full, cursor, &suggestions, 1).unwrap();
        assert_eq!(new_query, "startswith(\"apple pie");
    }

    #[test]
    fn tab_completes_fully_if_only_one_item() {
        let full = "startswith(\"ap";
        let cursor = full.chars().count();
        let suggestions = vec![widgets::query_input::Suggestion {
            label: "apple".to_string(),
            detail: Some("string value".to_string()),
            insert_text: "startswith(\"apple\")".to_string(),
        }];

        let (new_query, _) =
            expand_string_param_prefix_with_tab(full, cursor, &suggestions, 0).unwrap();
        assert_eq!(new_query, "startswith(\"apple");
    }

    #[test]
    fn contains_builder_key_insert_adds_open_brace_and_keeps_builder_open() {
        let (q, col, keep) = apply_contains_builder_suggestion(
            "contains({order_id: ",
            Some("contains object key"),
            "contains()",
            "contains(".chars().count(),
            false,
        );
        assert_eq!(q, "contains({order_id: ");
        assert_eq!(col as usize, q.chars().count());
        assert!(keep);
    }

    #[test]
    fn contains_builder_tab_drops_suffix_paren_and_appends_comma() {
        let (q, _, keep) = apply_contains_builder_suggestion(
            "contains([\"123\", \"foo\"",
            Some("contains array value"),
            "contains([\"123\", )",
            "contains([\"123\", ".chars().count(),
            false,
        );
        assert_eq!(q, "contains([\"123\", \"foo\", ");
        assert!(keep);
    }

    #[test]
    fn contains_builder_enter_finalizes_array_and_moves_out() {
        let (q, col, keep) = apply_contains_builder_suggestion(
            "contains([\"123\", \"foo\"",
            Some("contains array value"),
            "contains([\"123\", )",
            "contains([\"123\", ".chars().count(),
            true,
        );
        assert_eq!(q, "contains([\"123\", \"foo\"])");
        assert_eq!(col as usize, q.chars().count());
        assert!(!keep);
    }
}
