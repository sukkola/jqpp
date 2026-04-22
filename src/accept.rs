use jqpp::app::{WizardFrame, WizardKeyword, WizardState, WizardStep};
use jqpp::completions;
use jqpp::widgets;
use jqpp::widgets::query_input::Suggestion;

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

pub fn is_foreach_reduce_wizard_suggestion(detail: Option<&str>) -> bool {
    matches!(detail, Some("foreach-wizard") | Some("reduce-wizard"))
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
    is_contains_builder_suggestion(detail)
        || is_numeric_builder_suggestion(detail)
        || is_foreach_reduce_wizard_suggestion(detail)
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

// ── Wizard result type ────────────────────────────────────────────────────────

pub struct WizardStepResult {
    pub new_query: String,
    pub new_cursor: usize,
    pub new_state: Option<WizardState>,
    pub new_suggestions: Vec<Suggestion>,
}

impl WizardStepResult {
    fn exit(query: String, cursor: usize) -> Self {
        Self {
            new_query: query,
            new_cursor: cursor,
            new_state: None,
            new_suggestions: Vec::new(),
        }
    }

    fn advance(
        query: String,
        cursor: usize,
        state: WizardState,
        suggestions: Vec<Suggestion>,
    ) -> Self {
        Self {
            new_query: query,
            new_cursor: cursor,
            new_state: Some(state),
            new_suggestions: suggestions,
        }
    }
}

// ── Helper: push a frame onto the wizard stack ────────────────────────────────

fn push_frame(
    state: &WizardState,
    next_step: WizardStep,
    saved_query: String,
    saved_cursor: usize,
    saved_suggestions: Vec<Suggestion>,
) -> WizardState {
    let mut new_state = state.clone();
    new_state.stack.push(WizardFrame {
        step: next_step,
        saved_query,
        saved_cursor,
        saved_suggestions,
    });
    new_state
}

// ── 5.1 Enter keyword step ────────────────────────────────────────────────────

pub fn wizard_enter_keyword(
    keyword: WizardKeyword,
    query: &str,
    cursor: usize,
    stream_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    // Find the start of the keyword being accepted (strip the token being typed)
    let token_start = prefix
        .char_indices()
        .rev()
        .take_while(|(_, c)| c.is_ascii_alphanumeric() || *c == '_')
        .last()
        .map(|(i, _)| i)
        .unwrap_or(prefix.len());

    let pipe_prefix = &prefix[..token_start];
    let kw = match keyword {
        WizardKeyword::Foreach => "foreach",
        WizardKeyword::Reduce => "reduce",
    };
    let new_query = format!("{}{} {}", pipe_prefix, kw, suffix);
    let new_cursor = pipe_prefix.chars().count() + kw.chars().count() + 1;

    let state = WizardState {
        keyword,
        stack: vec![WizardFrame {
            step: WizardStep::Stream,
            saved_query: new_query.clone(),
            saved_cursor: new_cursor,
            saved_suggestions: stream_suggestions.clone(),
        }],
        var_name: String::new(),
    };

    WizardStepResult::advance(new_query, new_cursor, state, stream_suggestions)
}

// ── 5.2 Accept stream ─────────────────────────────────────────────────────────

pub fn wizard_accept_stream(
    selected_text: &str,
    is_sub_wizard: bool,
    state: &WizardState,
    query: &str,
    cursor: usize,
    bind_suggestions: Vec<Suggestion>,
    sub_wizard_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    // The query currently ends at cursor; append stream and a space
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    if is_sub_wizard {
        // Enter sub-wizard: rewrite the stream as e.g. `foreach range(|0|; 5) `
        // with cursor positioned at the first argument slot.
        // The selected_text is e.g. "range(0; 5)" or "recurse(.children[])"
        let new_query = format!("{}{} {}", prefix, selected_text, suffix);
        // Position cursor at the first argument inside the function call
        let open = selected_text.find('(').unwrap_or(selected_text.len());
        let new_cursor = prefix.chars().count() + open + 1;

        let new_state = push_frame(
            state,
            WizardStep::StreamSubArg { idx: 0 },
            new_query.clone(),
            new_cursor,
            sub_wizard_suggestions.clone(),
        );
        WizardStepResult::advance(new_query, new_cursor, new_state, sub_wizard_suggestions)
    } else {
        // Simple stream: append stream text + space, go to BindKeyword
        let new_query = format!("{}{} {}", prefix, selected_text, suffix);
        let new_cursor = prefix.chars().count() + selected_text.chars().count() + 1;

        let new_state = push_frame(
            state,
            WizardStep::BindKeyword,
            new_query.clone(),
            new_cursor,
            bind_suggestions.clone(),
        );
        WizardStepResult::advance(new_query, new_cursor, new_state, bind_suggestions)
    }
}

// ── 5.3 Accept stream sub-arg ─────────────────────────────────────────────────

pub fn wizard_accept_stream_sub_arg(
    idx: usize,
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    next_suggestions: Vec<Suggestion>,
    bind_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    // Replace text in current slot with selected_text, then advance to next slot
    // or to BindKeyword if last slot.
    //
    // The query has the form `foreach range(|cursor|; end) ` or similar.
    // We need to find the open paren, then locate the idx-th semicolon-separated slot.
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    // Find the unmatched open paren before cursor
    let open = find_unmatched_open_paren(&prefix).unwrap_or(0);

    // Locate the current slot boundaries
    let inner = &prefix[open + 1..];
    let semis: Vec<usize> = inner
        .char_indices()
        .filter(|(_, c)| *c == ';')
        .map(|(i, _)| i)
        .collect();

    let slot_start = if idx == 0 {
        0
    } else {
        semis.get(idx - 1).copied().map(|i| i + 1).unwrap_or(0)
    };
    let slot_start =
        inner[slot_start..].len() - inner[slot_start..].trim_start().len() + slot_start;

    // Build new inner with this slot replaced
    let slot_text = selected_text;

    // Check if this is the last slot (determine by function name)
    // For range: 2 slots (idx 0 and 1); for recurse: 1 slot (idx 0)
    let fn_name = prefix[..open]
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .rfind(|s: &&str| !s.is_empty())
        .unwrap_or("");

    let max_idx = match fn_name {
        "range" => 1,   // slots 0 and 1
        "recurse" => 0, // slot 0 only
        _ => 0,
    };

    // Replace the slot content in prefix
    let slot_abs_start = open + 1 + slot_start;

    // Find end of this slot (next ';' or end of inner before cursor)
    let slot_end = if let Some(&semi) = semis.get(idx) {
        open + 1 + semi
    } else {
        prefix.len()
    };

    let new_prefix = format!(
        "{}{}{}",
        &prefix[..slot_abs_start],
        slot_text,
        &prefix[slot_end..]
    );

    if idx < max_idx {
        // Advance to next slot: cursor after the next semicolon
        let new_inner = &new_prefix[open + 1..];
        let next_semi = new_inner
            .find(';')
            .map(|i| i + 1)
            .unwrap_or(new_inner.len());
        let after_semi =
            new_inner[next_semi..].len() - new_inner[next_semi..].trim_start().len() + next_semi;
        let new_cursor = open + 1 + after_semi;
        let new_query = format!("{}{}", new_prefix, suffix);

        let new_state = push_frame(
            state,
            WizardStep::StreamSubArg { idx: idx + 1 },
            new_query.clone(),
            new_cursor,
            next_suggestions.clone(),
        );
        WizardStepResult::advance(new_query, new_cursor, new_state, next_suggestions)
    } else {
        // Last sub-wizard slot done: close the function call, add space, go to BindKeyword
        // Find the close paren in suffix
        let close_in_suffix = suffix.find(')').unwrap_or(0);
        let new_query = format!("{}{}{}", &new_prefix, &suffix[..close_in_suffix + 1], " ")
            + &suffix[close_in_suffix + 1..];
        let new_cursor = new_prefix.chars().count() + close_in_suffix + 2; // after `) `

        let new_state = push_frame(
            state,
            WizardStep::BindKeyword,
            new_query.clone(),
            new_cursor,
            bind_suggestions.clone(),
        );
        WizardStepResult::advance(new_query, new_cursor, new_state, bind_suggestions)
    }
}

// ── 5.4 Accept bind keyword ───────────────────────────────────────────────────

pub fn wizard_accept_bind_keyword(
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    var_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    if selected_text == "|" {
        // Exit wizard with a pipe
        let new_query = format!("{}| {}", prefix, suffix);
        let new_cursor = prefix.chars().count() + 2;
        return WizardStepResult::exit(new_query, new_cursor);
    }

    // "as" → insert "as $" and move to VarName (ensure a separator before "as")
    let sep = if prefix.ends_with(' ') { "" } else { " " };
    let new_query = format!("{}{}as ${}", prefix, sep, suffix);
    let new_cursor = prefix.chars().count() + sep.len() + 4; // after "as $"

    let new_state = push_frame(
        state,
        WizardStep::VarName,
        new_query.clone(),
        new_cursor,
        var_suggestions.clone(),
    );
    WizardStepResult::advance(new_query, new_cursor, new_state, var_suggestions)
}

// ── 5.5 Accept var name ───────────────────────────────────────────────────────

pub fn wizard_accept_var_name(
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    init_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    // The prefix ends with "as $" or possibly "as $partial"
    // Strip the existing $ prefix text and replace with selected name
    let dollar_pos = prefix.rfind('$').unwrap_or(prefix.len());
    let before_dollar = &prefix[..dollar_pos + 1]; // includes '$'

    // selected_text is like "$x" or "$item" — strip leading $
    let var_bare = selected_text.strip_prefix('$').unwrap_or(selected_text);

    let new_query = format!("{}{} ({}", before_dollar, var_bare, suffix);
    let new_cursor = before_dollar.chars().count() + var_bare.chars().count() + 3; // after " ("

    let mut new_state = push_frame(
        state,
        WizardStep::Init,
        new_query.clone(),
        new_cursor,
        init_suggestions.clone(),
    );
    new_state.var_name = var_bare.to_string();

    WizardStepResult::advance(new_query, new_cursor, new_state, init_suggestions)
}

// ── 5.6 Accept init ───────────────────────────────────────────────────────────

pub fn wizard_accept_init(
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    accum_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    let new_query = format!("{}{}; {}", prefix, selected_text, suffix);
    let new_cursor = prefix.chars().count() + selected_text.chars().count() + 2;

    let new_state = push_frame(
        state,
        WizardStep::UpdateAccum,
        new_query.clone(),
        new_cursor,
        accum_suggestions.clone(),
    );
    WizardStepResult::advance(new_query, new_cursor, new_state, accum_suggestions)
}

// ── 5.7 Accept update accum ───────────────────────────────────────────────────

pub fn wizard_accept_update_accum(
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    op_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    let new_cursor = prefix.chars().count() + selected_text.chars().count();
    let new_query = format!("{}{}{}", prefix, selected_text, suffix);

    let new_state = push_frame(
        state,
        WizardStep::UpdateOp,
        new_query.clone(),
        new_cursor,
        op_suggestions.clone(),
    );
    WizardStepResult::advance(new_query, new_cursor, new_state, op_suggestions)
}

// ── 5.8 Accept update op ──────────────────────────────────────────────────────

pub fn wizard_accept_update_op(
    selected_text: &str,
    state: &WizardState,
    query: &str,
    cursor: usize,
    extract_suggestions: Vec<Suggestion>,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    // op_text is e.g. "+ $x", "- $x", "$x" (relative to current cursor position,
    // which sits right after the accum prefix inserted by wizard_accept_update_accum)
    let op_text = selected_text;

    match state.keyword {
        WizardKeyword::Reduce => {
            // Close the reduce: append ")" then any suffix
            let new_query = format!("{}{}){}", prefix, op_text, suffix);
            let new_cursor = prefix.chars().count() + op_text.chars().count() + 1;
            WizardStepResult::exit(new_query, new_cursor)
        }
        WizardKeyword::Foreach => {
            // Show extract step
            let new_query = format!("{}{}{}", prefix, op_text, suffix);
            let new_cursor = prefix.chars().count() + op_text.chars().count();

            let new_state = push_frame(
                state,
                WizardStep::Extract,
                new_query.clone(),
                new_cursor,
                extract_suggestions.clone(),
            );
            WizardStepResult::advance(new_query, new_cursor, new_state, extract_suggestions)
        }
    }
}

// ── 5.9 Accept extract ────────────────────────────────────────────────────────

pub fn wizard_accept_extract(
    selected_text: &str,
    _state: &WizardState,
    query: &str,
    cursor: usize,
) -> WizardStepResult {
    let prefix: String = query.chars().take(cursor).collect();
    let suffix: String = query.chars().skip(cursor).collect();

    if selected_text == "; ." {
        // Insert "; .)" with cursor at "."
        let new_query = format!("{}; .){}", prefix, suffix);
        let new_cursor = prefix.chars().count() + 3; // at "."
        WizardStepResult::exit(new_query, new_cursor)
    } else {
        // ")" → close clause
        let new_query = format!("{}){}", prefix, suffix);
        let new_cursor = prefix.chars().count() + 1;
        WizardStepResult::exit(new_query, new_cursor)
    }
}

// ── 6.1-6.2 Enter fast-forward ────────────────────────────────────────────────

pub fn wizard_fast_forward(
    keyword: &WizardKeyword,
    current_step: &WizardStep,
    partial_query: &str,
    cursor: usize,
    var_name: &str,
) -> (String, usize) {
    let prefix: String = partial_query.chars().take(cursor).collect();
    let suffix: String = partial_query.chars().skip(cursor).collect();

    let full = assemble_fast_forward(keyword, current_step, &prefix, &suffix, var_name);
    let new_cursor = full.chars().count();
    (full, new_cursor)
}

fn assemble_fast_forward(
    keyword: &WizardKeyword,
    current_step: &WizardStep,
    prefix: &str,
    suffix: &str,
    var_name: &str,
) -> String {
    // Use the actual variable name chosen by the user, falling back to "x".
    let v = if var_name.is_empty() { "x" } else { var_name };

    match current_step {
        WizardStep::Stream | WizardStep::Keyword => {
            format!("{}.[] as ${v} (0; . + ${v}){}", prefix, suffix)
        }
        WizardStep::StreamSubArg { .. } => {
            let open = find_unmatched_open_paren(prefix).unwrap_or(prefix.len() - 1);
            let fn_name = prefix[..open]
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .rfind(|s: &&str| !s.is_empty())
                .unwrap_or("");
            let close_idx = suffix.find(')').unwrap_or(0);
            let after_fn = &suffix[close_idx + 1..];
            match fn_name {
                "range" => format!(
                    "{prefix}0; 5{}  as ${v} (0; . + ${v}){}",
                    &suffix[..close_idx + 1],
                    after_fn
                ),
                "recurse" => format!(
                    "{prefix}.children[]{} as ${v} (0; . + ${v}){}",
                    &suffix[..close_idx + 1],
                    after_fn
                ),
                _ => format!(
                    "{prefix}{} as ${v} (0; . + ${v}){}",
                    suffix[..close_idx + 1].trim_end_matches(')'),
                    after_fn
                ),
            }
        }
        WizardStep::BindKeyword => {
            format!("{}as ${v} (0; . + ${v}){}", prefix, suffix)
        }
        WizardStep::VarName => {
            let dollar = prefix.rfind('$').unwrap_or(prefix.len());
            let before_dollar = &prefix[..dollar + 1];
            format!("{}{v} (0; . + ${v}){}", before_dollar, suffix)
        }
        WizardStep::Init => {
            format!("{}0; . + ${v}){}", prefix, suffix)
        }
        WizardStep::UpdateAccum => {
            // Default: accumulate by applying `. + $var` to whatever is in prefix.
            format!("{}. + ${v}){}", prefix, suffix)
        }
        WizardStep::UpdateOp => {
            // The user has already chosen their body expression (it sits in prefix).
            // Just close the clause — do not append another operator.
            match keyword {
                WizardKeyword::Reduce => format!("{}){}", prefix, suffix),
                WizardKeyword::Foreach => format!("{}){}", prefix, suffix),
            }
        }
        WizardStep::Extract => {
            format!("{}){}", prefix, suffix)
        }
    }
}

// ── 7.1 Esc step back ─────────────────────────────────────────────────────────

pub fn wizard_pop_step(state: &mut WizardState) -> Option<(String, usize, Vec<Suggestion>)> {
    // Pop top frame
    state.stack.pop();
    // Look at the new top frame (if any) to restore
    state.stack.last().map(|frame| {
        (
            frame.saved_query.clone(),
            frame.saved_cursor,
            frame.saved_suggestions.clone(),
        )
    })
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

    // ── Wizard step transition tests (tasks 10.x) ─────────────────────────────

    fn empty_stream_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: ".[]".to_string(),
            detail: None,
            insert_text: ".[]".to_string(),
        }]
    }

    fn empty_bind_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: "as".to_string(),
            detail: None,
            insert_text: "as".to_string(),
        }]
    }

    fn empty_var_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: "$x".to_string(),
            detail: None,
            insert_text: "$x".to_string(),
        }]
    }

    fn empty_init_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: "0".to_string(),
            detail: None,
            insert_text: "0".to_string(),
        }]
    }

    fn empty_accum_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: ".".to_string(),
            detail: None,
            insert_text: ".".to_string(),
        }]
    }

    fn empty_op_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: "+ $x".to_string(),
            detail: None,
            insert_text: "+ $x".to_string(),
        }]
    }

    fn empty_extract_suggestions() -> Vec<Suggestion> {
        vec![Suggestion {
            label: ")".to_string(),
            detail: None,
            insert_text: ")".to_string(),
        }]
    }

    // 10.1 wizard_enter_keyword for foreach and reduce
    #[test]
    fn wizard_enter_keyword_foreach_places_cursor_after_space() {
        let query = "fore";
        let cursor = 4;
        let result = wizard_enter_keyword(
            WizardKeyword::Foreach,
            query,
            cursor,
            empty_stream_suggestions(),
        );
        assert_eq!(result.new_query, "foreach ");
        assert_eq!(result.new_cursor, 8);
        assert!(result.new_state.is_some());
    }

    #[test]
    fn wizard_enter_keyword_reduce_places_cursor_after_space() {
        let query = "red";
        let cursor = 3;
        let result = wizard_enter_keyword(
            WizardKeyword::Reduce,
            query,
            cursor,
            empty_stream_suggestions(),
        );
        assert_eq!(result.new_query, "reduce ");
        assert_eq!(result.new_cursor, 7);
        assert!(result.new_state.is_some());
    }

    #[test]
    fn wizard_enter_keyword_preserves_pipe_prefix() {
        let query = ".items | fore";
        let cursor = query.chars().count();
        let result = wizard_enter_keyword(
            WizardKeyword::Foreach,
            query,
            cursor,
            empty_stream_suggestions(),
        );
        assert!(result.new_query.starts_with(".items | foreach "));
        let expected_cursor = ".items | foreach ".chars().count();
        assert_eq!(result.new_cursor, expected_cursor);
    }

    // 10.2 Test full foreach step chain: Stream → BindKeyword → VarName → Init → UpdateAccum → UpdateOp → Extract → close
    #[test]
    fn wizard_foreach_full_step_chain() {
        // Step 0: enter keyword
        let r0 = wizard_enter_keyword(
            WizardKeyword::Foreach,
            "fore",
            4,
            empty_stream_suggestions(),
        );
        assert_eq!(r0.new_query, "foreach ");
        let state0 = r0.new_state.unwrap();

        // Step 1: accept stream ".[]"
        let r1 = wizard_accept_stream(
            ".[]",
            false,
            &state0,
            &r0.new_query,
            r0.new_cursor,
            empty_bind_suggestions(),
            Vec::new(),
        );
        assert_eq!(r1.new_query, "foreach .[] ");
        let state1 = r1.new_state.unwrap();
        assert_eq!(state1.stack.last().unwrap().step, WizardStep::BindKeyword);

        // Step 2: accept bind keyword "as"
        let r2 = wizard_accept_bind_keyword(
            "as",
            &state1,
            &r1.new_query,
            r1.new_cursor,
            empty_var_suggestions(),
        );
        assert!(
            r2.new_query.contains("as $"),
            "query should contain 'as $': {}",
            r2.new_query
        );
        let state2 = r2.new_state.unwrap();
        assert_eq!(state2.stack.last().unwrap().step, WizardStep::VarName);

        // Step 3: accept var name "$x"
        let r3 = wizard_accept_var_name(
            "$x",
            &state2,
            &r2.new_query,
            r2.new_cursor,
            empty_init_suggestions(),
        );
        assert!(
            r3.new_query.contains("as $x ("),
            "query should contain 'as $x (': {}",
            r3.new_query
        );
        let state3 = r3.new_state.unwrap();
        assert_eq!(state3.stack.last().unwrap().step, WizardStep::Init);

        // Step 4: accept init "0"
        let r4 = wizard_accept_init(
            "0",
            &state3,
            &r3.new_query,
            r3.new_cursor,
            empty_accum_suggestions(),
        );
        assert!(
            r4.new_query.contains("0; "),
            "query should contain '0; ': {}",
            r4.new_query
        );
        let state4 = r4.new_state.unwrap();
        assert_eq!(state4.stack.last().unwrap().step, WizardStep::UpdateAccum);

        // Step 5: accept update accum "."
        let r5 = wizard_accept_update_accum(
            ".",
            &state4,
            &r4.new_query,
            r4.new_cursor,
            empty_op_suggestions(),
        );
        assert!(
            r5.new_query.ends_with('.') || r5.new_query.contains("0; ."),
            "query after accum: {}",
            r5.new_query
        );
        let state5 = r5.new_state.unwrap();
        assert_eq!(state5.stack.last().unwrap().step, WizardStep::UpdateOp);

        // Step 6: accept update op "+ $x"
        let r6 = wizard_accept_update_op(
            "+ $x",
            &state5,
            &r5.new_query,
            r5.new_cursor,
            empty_extract_suggestions(),
        );
        // foreach → Extract step
        assert!(
            r6.new_state.is_some(),
            "foreach should proceed to Extract step"
        );
        let state6 = r6.new_state.unwrap();
        assert_eq!(state6.stack.last().unwrap().step, WizardStep::Extract);

        // Step 7: accept extract ")"
        let r7 = wizard_accept_extract(")", &state6, &r6.new_query, r6.new_cursor);
        assert!(r7.new_state.is_none(), "wizard should exit after extract");
        assert!(r7.new_query.contains(')'), "should close with )");
    }

    // 10.3 Test full reduce step chain (no Extract step)
    #[test]
    fn wizard_reduce_full_step_chain() {
        let r0 = wizard_enter_keyword(WizardKeyword::Reduce, "red", 3, empty_stream_suggestions());
        let state0 = r0.new_state.unwrap();

        let r1 = wizard_accept_stream(
            ".[]",
            false,
            &state0,
            &r0.new_query,
            r0.new_cursor,
            empty_bind_suggestions(),
            Vec::new(),
        );
        let state1 = r1.new_state.unwrap();

        let r2 = wizard_accept_bind_keyword(
            "as",
            &state1,
            &r1.new_query,
            r1.new_cursor,
            empty_var_suggestions(),
        );
        let state2 = r2.new_state.unwrap();

        let r3 = wizard_accept_var_name(
            "$x",
            &state2,
            &r2.new_query,
            r2.new_cursor,
            empty_init_suggestions(),
        );
        let state3 = r3.new_state.unwrap();

        let r4 = wizard_accept_init(
            "0",
            &state3,
            &r3.new_query,
            r3.new_cursor,
            empty_accum_suggestions(),
        );
        let state4 = r4.new_state.unwrap();

        let r5 = wizard_accept_update_accum(
            ".",
            &state4,
            &r4.new_query,
            r4.new_cursor,
            empty_op_suggestions(),
        );
        let state5 = r5.new_state.unwrap();

        // For reduce, accepting UpdateOp should close with ")" and exit
        let r6 = wizard_accept_update_op("+ $x", &state5, &r5.new_query, r5.new_cursor, Vec::new());
        assert!(
            r6.new_state.is_none(),
            "reduce wizard should exit after UpdateOp"
        );
        assert!(
            r6.new_query.ends_with(')'),
            "reduce should close with ): {}",
            r6.new_query
        );
    }

    // 10.6 Test wizard_fast_forward from each step
    #[test]
    fn wizard_fast_forward_from_stream_step_produces_complete_foreach() {
        let (q, _cursor) = wizard_fast_forward(
            &WizardKeyword::Foreach,
            &WizardStep::Stream,
            "foreach ",
            8,
            "x",
        );
        assert!(q.contains(".[]"), "should contain .[]");
        assert!(q.contains("as $x"), "should contain as $x");
        assert!(q.contains("(0; . + $x)"), "should contain (0; . + $x)");
    }

    #[test]
    fn wizard_fast_forward_from_stream_step_produces_complete_reduce() {
        let (q, _cursor) = wizard_fast_forward(
            &WizardKeyword::Reduce,
            &WizardStep::Stream,
            "reduce ",
            7,
            "x",
        );
        assert!(q.contains(".[]"), "should contain .[]");
        assert!(q.contains("as $x"), "should contain as $x");
        assert!(q.contains("(0; . + $x)"), "should contain (0; . + $x)");
    }

    #[test]
    fn wizard_fast_forward_from_init_step_completes_remaining() {
        let (q, _) = wizard_fast_forward(
            &WizardKeyword::Foreach,
            &WizardStep::Init,
            "foreach .[] as $x (",
            "foreach .[] as $x (".chars().count(),
            "x",
        );
        assert!(
            q.contains("0; . + $x)"),
            "should contain defaults from init: {}",
            q
        );
    }

    #[test]
    fn wizard_fast_forward_from_update_accum_step() {
        let (q, _) = wizard_fast_forward(
            &WizardKeyword::Reduce,
            &WizardStep::UpdateAccum,
            "reduce .[] as $x (0; ",
            "reduce .[] as $x (0; ".chars().count(),
            "x",
        );
        assert!(q.contains(". + $x)"), "should contain accum + op: {}", q);
    }

    #[test]
    fn wizard_fast_forward_at_update_op_step_just_closes() {
        // After the user selected their body expression at UpdateOp, Enter should
        // close with ")" — NOT append another operator like "+ $x)".
        let query = "reduce .[].name as $name (null; $name";
        let cursor = query.chars().count();
        let (q, _) = wizard_fast_forward(
            &WizardKeyword::Reduce,
            &WizardStep::UpdateOp,
            query,
            cursor,
            "name",
        );
        assert_eq!(
            q, "reduce .[].name as $name (null; $name)",
            "should just close with )"
        );
    }

    #[test]
    fn wizard_fast_forward_uses_actual_var_name_not_x() {
        let (q, _) = wizard_fast_forward(
            &WizardKeyword::Reduce,
            &WizardStep::Stream,
            "reduce ",
            7,
            "items",
        );
        assert!(q.contains("as $items"), "should use $items, got: {}", q);
        assert!(
            q.contains(". + $items)"),
            "should use $items in op, got: {}",
            q
        );
        assert!(!q.contains("$x"), "should not contain $x, got: {}", q);
    }

    // 10.7 Test wizard_pop_step at every position
    #[test]
    fn wizard_pop_step_returns_previous_frame() {
        let mut state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![
                WizardFrame {
                    step: WizardStep::Stream,
                    saved_query: "foreach ".to_string(),
                    saved_cursor: 8,
                    saved_suggestions: empty_stream_suggestions(),
                },
                WizardFrame {
                    step: WizardStep::BindKeyword,
                    saved_query: "foreach .[] ".to_string(),
                    saved_cursor: 12,
                    saved_suggestions: empty_bind_suggestions(),
                },
            ],
            var_name: String::new(),
        };

        let result = wizard_pop_step(&mut state);
        assert!(result.is_some());
        let (q, col, _suggs) = result.unwrap();
        assert_eq!(q, "foreach ");
        assert_eq!(col, 8);
        assert_eq!(state.stack.len(), 1);
    }

    #[test]
    fn wizard_pop_step_from_first_step_returns_none() {
        let mut state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![WizardFrame {
                step: WizardStep::Stream,
                saved_query: "foreach ".to_string(),
                saved_cursor: 8,
                saved_suggestions: empty_stream_suggestions(),
            }],
            var_name: String::new(),
        };

        // Pop the only frame
        state.stack.pop();
        let result = wizard_pop_step(&mut state);
        assert!(result.is_none(), "empty stack should return None");
    }

    // 10.8 Test "|" bind-keyword selection exits wizard
    #[test]
    fn wizard_bind_keyword_pipe_exits_wizard() {
        let state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![WizardFrame {
                step: WizardStep::BindKeyword,
                saved_query: "foreach .[] ".to_string(),
                saved_cursor: 12,
                saved_suggestions: empty_bind_suggestions(),
            }],
            var_name: String::new(),
        };
        let r = wizard_accept_bind_keyword("|", &state, "foreach .[] ", 12, Vec::new());
        assert!(r.new_state.is_none(), "pipe should exit wizard");
        assert!(
            r.new_query.contains("| "),
            "should insert pipe: {}",
            r.new_query
        );
    }

    // 10.9 Test extract step: ")" closes clause; "; ." inserts extract slot
    #[test]
    fn wizard_extract_close_paren_exits_wizard() {
        let state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![],
            var_name: "x".to_string(),
        };
        let query = "foreach .[] as $x (0; . + $x";
        let cursor = query.chars().count();
        let r = wizard_accept_extract(")", &state, query, cursor);
        assert!(r.new_state.is_none());
        assert!(
            r.new_query.ends_with(')'),
            "should end with ): {}",
            r.new_query
        );
    }

    #[test]
    fn wizard_extract_semicolon_dot_inserts_extract_slot() {
        let state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![],
            var_name: "x".to_string(),
        };
        let query = "foreach .[] as $x (0; . + $x";
        let cursor = query.chars().count();
        let r = wizard_accept_extract("; .", &state, query, cursor);
        assert!(r.new_state.is_none());
        assert!(
            r.new_query.contains("; .)"),
            "should contain '; .)': {}",
            r.new_query
        );
    }

    // 10.4 Test range sub-wizard: slot 0 → slot 1 → BindKeyword
    #[test]
    fn wizard_range_sub_wizard_slot_0_to_slot_1() {
        let state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![WizardFrame {
                step: WizardStep::StreamSubArg { idx: 0 },
                saved_query: "foreach range(0; 5) ".to_string(),
                saved_cursor: 14, // at "0"
                saved_suggestions: Vec::new(),
            }],
            var_name: String::new(),
        };
        let query = "foreach range(0; 5) ";
        // cursor at slot 0 (position 14, inside "0")
        let cursor = "foreach range(".chars().count();
        let next_suggs = vec![Suggestion {
            label: "5".to_string(),
            detail: None,
            insert_text: "5".to_string(),
        }];
        let bind_suggs = empty_bind_suggestions();
        let r = wizard_accept_stream_sub_arg(0, "0", &state, query, cursor, next_suggs, bind_suggs);
        // Should advance to slot 1
        if let Some(ref new_state) = r.new_state {
            let top = new_state.stack.last().unwrap();
            assert!(
                matches!(top.step, WizardStep::StreamSubArg { idx: 1 })
                    || matches!(top.step, WizardStep::BindKeyword),
                "should advance to slot 1 or BindKeyword: {:?}",
                top.step
            );
        }
    }

    // 10.5 Test recurse sub-wizard: slot 0 → BindKeyword
    #[test]
    fn wizard_recurse_sub_wizard_slot_0_to_bind_keyword() {
        let state = WizardState {
            keyword: WizardKeyword::Foreach,
            stack: vec![WizardFrame {
                step: WizardStep::StreamSubArg { idx: 0 },
                saved_query: "foreach recurse(.children[]) ".to_string(),
                saved_cursor: 16, // inside recurse
                saved_suggestions: Vec::new(),
            }],
            var_name: String::new(),
        };
        let query = "foreach recurse(.children[]) ";
        let cursor = "foreach recurse(".chars().count();
        let bind_suggs = empty_bind_suggestions();
        let r = wizard_accept_stream_sub_arg(
            0,
            ".children[]",
            &state,
            query,
            cursor,
            Vec::new(),
            bind_suggs,
        );
        // For recurse (max_idx=0), this should go to BindKeyword
        if let Some(ref new_state) = r.new_state {
            let top = new_state.stack.last().unwrap();
            assert_eq!(
                top.step,
                WizardStep::BindKeyword,
                "recurse slot 0 should advance to BindKeyword"
            );
        }
    }

    // is_foreach_reduce_wizard_suggestion tests
    #[test]
    fn foreach_reduce_wizard_detection() {
        assert!(is_foreach_reduce_wizard_suggestion(Some("foreach-wizard")));
        assert!(is_foreach_reduce_wizard_suggestion(Some("reduce-wizard")));
        assert!(!is_foreach_reduce_wizard_suggestion(Some(
            "integer generator"
        )));
        assert!(!is_foreach_reduce_wizard_suggestion(None));
    }

    #[test]
    fn is_builder_suggestion_includes_wizard_details() {
        assert!(is_builder_suggestion(Some("foreach-wizard")));
        assert!(is_builder_suggestion(Some("reduce-wizard")));
        assert!(is_builder_suggestion(Some("integer generator")));
    }
}
