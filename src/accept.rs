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
    if !(trimmed.ends_with('(') || trimmed.ends_with("()")) {
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
        "sort_by" | "group_by" | "unique_by" | "min_by" | "max_by" | "del" | "path"
    )
}

pub fn starts_context_aware_function_call(suggestion: &str) -> bool {
    is_field_path_function_call_start(suggestion)
        || completions::json_context::string_param_context(suggestion).is_some()
        || suggestion
            .strip_suffix(')')
            .map(|s| completions::json_context::string_param_context(s).is_some())
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

pub fn apply_selected_suggestion(
    insert_text: &str,
    detail: Option<&str>,
    full_query: &str,
    cursor_col: usize,
) -> (String, u16) {
    let query_prefix: String = full_query.chars().take(cursor_col).collect();
    let mut suffix: String = full_query.chars().skip(cursor_col).collect();

    if is_string_param_value_suggestion(detail)
        && completions::json_context::string_param_context(&query_prefix).is_some()
    {
        if let Some(close_idx) = suffix.find(')') {
            suffix = suffix[close_idx + 1..].to_string();
        } else {
            suffix.clear();
        }
        let merged = format!("{}{}", insert_text, suffix);
        return (merged, insert_text.chars().count() as u16);
    }

    let merged = apply_suggestion_with_suffix(insert_text, &suffix);
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
    let ctx = completions::json_context::string_param_context(&query_prefix)?;
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
    let ctx = completions::json_context::string_param_context(&query_prefix)?;
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

    let extended = if suggestion_index > 0 || candidates.len() == 1 {
        // If the user has explicitly moved the selection beyond the first item,
        // OR if there is only one candidate, complete fully to that item.
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
            let lcp = longest_common_prefix(&candidates);
            if lcp.chars().count() > ctx.inner_prefix.chars().count() {
                Some(lcp)
            } else {
                None
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
    } else if suggestion.ends_with("()") {
        suggestion.chars().count().saturating_sub(1) as u16
    } else {
        suggestion.chars().count() as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!is_field_path_function_call_start("map(.)"));
    }

    #[test]
    fn context_aware_function_start_detection_includes_string_param_functions() {
        assert!(starts_context_aware_function_call("sort_by()"));
        assert!(starts_context_aware_function_call("split()"));
        assert!(starts_context_aware_function_call("startswith()"));
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

        // If we have selected the second item (index 1), Tab should give us "apple pie" fully,
        // even though "apple" is shorter and a prefix.
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
}
