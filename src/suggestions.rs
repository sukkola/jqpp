use jqpp::app::App;
use jqpp::completions;
use jqpp::executor::Executor;
use jqpp::widgets;
use ratatui::crossterm::event::KeyCode;

use crate::loop_state::LoopState;
use jqpp::completions::lsp::{LspMessage, LspProvider};

pub async fn handle_finished_computes(app: &mut App<'_>, state: &mut LoopState) {
    if let Some(ref handle) = state.compute_handle
        && handle.is_finished()
    {
        match state.compute_handle.take().unwrap().await {
            Ok((Ok((results, raw)), pipe_type)) => {
                app.results = results;
                app.right_scroll = 0;
                app.error = None;
                app.raw_output = raw;
                state.cached_pipe_type = pipe_type;
            }
            Ok((Err(_), pipe_type)) => {
                app.raw_output = false;
                state.cached_pipe_type = pipe_type;
            }
            Err(_) => {}
        }
        if state.suggestion_active {
            app.query_input.suggestions = compute_suggestions(
                &state.pending_qp,
                app.executor.as_ref().map(|e| &e.json_input),
                &state.lsp_completions,
                state.cached_pipe_type.as_deref(),
            );
            app.query_input.suggestion_index = 0;
            app.query_input.suggestion_scroll = 0;
            let all_exact = !app.query_input.suggestions.is_empty()
                && app
                    .query_input
                    .suggestions
                    .iter()
                    .all(|s| s.insert_text == state.pending_qp)
                && !app
                    .query_input
                    .suggestions
                    .iter()
                    .any(|s| crate::accept::is_builder_suggestion(s.detail.as_deref()));
            if all_exact {
                app.query_input.show_suggestions = false;
                state.suggestion_active = false;
                state.lsp_completions.clear();
                state.cached_pipe_type = None;
            } else {
                app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
            }
            app.structural_hint_active = false;
        } else {
            let query_prefix = current_query_prefix(app);
            if !crate::hints::maybe_activate_structural_hint(app, &query_prefix) {
                app.structural_hint_active = false;
                app.query_input.show_suggestions = false;
                app.query_input.suggestions.clear();
            }
        }
    }
}

pub async fn run_debounced_compute(
    app: &mut App<'_>,
    state: &mut LoopState,
    lsp_provider: &mut Option<LspProvider>,
) {
    if state.debounce_pending && state.last_edit_at.elapsed() >= state.debounce_duration {
        state.debounce_pending = false;
        let query = app.query_input.textarea.lines()[0].clone();
        let cursor_col = app.query_input.textarea.cursor().1;
        let query_prefix: String = query.chars().take(cursor_col).collect();
        let has_non_exact_suggestion = if state.suggestion_active {
            app.structural_hint_active = false;
            app.query_input.suggestions = compute_suggestions(
                &query_prefix,
                app.executor.as_ref().map(|e| &e.json_input),
                &state.lsp_completions,
                state.cached_pipe_type.as_deref(),
            );
            app.query_input.suggestion_index = 0;
            app.query_input.suggestion_scroll = 0;
            let all_exact = !app.query_input.suggestions.is_empty()
                && app
                    .query_input
                    .suggestions
                    .iter()
                    .all(|s| s.insert_text == query_prefix)
                && !app
                    .query_input
                    .suggestions
                    .iter()
                    .any(|s| crate::accept::is_builder_suggestion(s.detail.as_deref()));
            if all_exact {
                app.query_input.show_suggestions = false;
                state.suggestion_active = false;
                state.lsp_completions.clear();
                state.cached_pipe_type = None;
                false
            } else {
                app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                has_non_exact_suggestion_for_prefix(&query_prefix, &app.query_input.suggestions)
            }
        } else {
            false
        };
        let hold_output_during_suggestions = state.suggestion_active
            && has_non_exact_suggestion
            && should_hold_output_during_suggestions(&query_prefix);
        let effective_query = if query.trim().is_empty() {
            ".".to_string()
        } else {
            query.clone()
        };

        if hold_output_during_suggestions {
            state.compute_handle = None;
        }

        if let Some(ref exec) = app.executor {
            if query_prefix.rfind('|').is_none() {
                state.cached_pipe_type = None;
            }
            if !hold_output_during_suggestions {
                let eq = effective_query.clone();
                let q = query_prefix.clone();
                let input = exec.json_input.clone();
                state.compute_handle = Some(tokio::task::spawn_blocking(move || {
                    let main_result = Executor::execute_query(&eq, &input);
                    let type_query = Executor::strip_format_op(&q)
                        .map(|(base, _)| base)
                        .unwrap_or_else(|| q.clone());
                    let pipe_type = if let Some(p) = type_query.rfind('|') {
                        let prefix = type_query[..p].trim();
                        if prefix.is_empty() {
                            None
                        } else {
                            Executor::execute(prefix, &input)
                                .ok()
                                .and_then(|mut r| {
                                    if r.is_empty() {
                                        None
                                    } else {
                                        Some(r.swap_remove(0))
                                    }
                                })
                                .map(|v| completions::jq_builtins::jq_type_of(&v).to_string())
                        }
                    } else {
                        // No pipe — try to infer type from the query result; fall back to
                        // the raw input type so partial/invalid queries still filter builtins.
                        main_result
                            .as_ref()
                            .ok()
                            .and_then(|(results, _)| results.first())
                            .map(|v| completions::jq_builtins::jq_type_of(v).to_string())
                            .or_else(|| {
                                Some(completions::jq_builtins::jq_type_of(&input).to_string())
                            })
                    };
                    (main_result, pipe_type)
                }));
                state.pending_qp = query_prefix.clone();
            }
        } else if query_prefix.rfind('|').is_none() {
            state.cached_pipe_type = None;
        }

        if let Some(lsp) = lsp_provider {
            let _ = lsp.did_change(&query).await;
            if state.suggestion_active {
                let _ = lsp.completion(&query).await;
            }
        }
    }
}

pub fn handle_lsp_message(app: &mut App<'_>, state: &mut LoopState, msg: LspMessage) {
    match msg {
        LspMessage::Status(s) => {
            app.lsp_status = if s == "ready" { None } else { Some(s) };
        }
        LspMessage::Diagnostic(d) => {
            app.lsp_diagnostic = d;
        }
        LspMessage::Completions(c) => {
            if !c.is_empty() {
                state.lsp_completions = c;
            }
            if state.suggestion_active {
                let query_line = app.query_input.textarea.lines()[0].clone();
                let cur = app.query_input.textarea.cursor().1;
                let query_prefix: String = query_line.chars().take(cur).collect();
                app.query_input.suggestions = compute_suggestions(
                    &query_prefix,
                    app.executor.as_ref().map(|e| &e.json_input),
                    &state.lsp_completions,
                    state.cached_pipe_type.as_deref(),
                );
                app.query_input.suggestion_index = 0;
                app.query_input.suggestion_scroll = 0;
                let all_exact = !app.query_input.suggestions.is_empty()
                    && app
                        .query_input
                        .suggestions
                        .iter()
                        .all(|s| s.insert_text == query_prefix)
                    && !app
                        .query_input
                        .suggestions
                        .iter()
                        .any(|s| crate::accept::is_builder_suggestion(s.detail.as_deref()));
                if all_exact {
                    app.query_input.show_suggestions = false;
                    state.suggestion_active = false;
                    state.lsp_completions.clear();
                    state.cached_pipe_type = None;
                } else {
                    app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                }
            }
        }
    }
}

pub fn compute_suggestions(
    query_prefix: &str,
    json_input: Option<&serde_json::Value>,
    lsp_completions: &[completions::CompletionItem],
    pipe_context_type: Option<&str>,
) -> Vec<widgets::query_input::Suggestion> {
    let in_contains_builder_context = completions::json_context::param_field_context(query_prefix)
        .map(|ctx| ctx.fn_name == "contains")
        .unwrap_or(false);
    let in_string_param_context =
        completions::json_context::string_param_context(query_prefix, pipe_context_type).is_some();
    if is_inside_string_literal(query_prefix)
        && !in_string_param_context
        && !in_contains_builder_context
    {
        return Vec::new();
    }

    if in_string_param_context {
        let json_only = if let Some(input) = json_input {
            let evaluated =
                evaluated_string_param_input(query_prefix, input).unwrap_or_else(|| input.clone());
            if let Some((head, tail)) = split_string_param_query_prefix(query_prefix) {
                completions::json_context::get_completions(&tail, &evaluated)
                    .into_iter()
                    .map(|i| completions::CompletionItem {
                        insert_text: format!("{}{}", head, i.insert_text),
                        ..i
                    })
                    .collect()
            } else {
                completions::json_context::get_completions(query_prefix, &evaluated)
            }
        } else {
            Vec::new()
        };

        let mut deduped: Vec<completions::CompletionItem> = Vec::new();
        for item in json_only {
            if !deduped
                .iter()
                .any(|d| d.label == item.label && d.insert_text == item.insert_text)
            {
                deduped.push(item);
            }
        }

        return deduped
            .into_iter()
            .map(|i| widgets::query_input::Suggestion {
                label: i.label,
                detail: i.detail,
                insert_text: i.insert_text,
            })
            .collect();
    }

    let token = current_token(query_prefix);
    let fuzzy_token = fuzzy_token_fragment(token);
    let prefix = crate::suggestions::lsp_pipe_prefix(query_prefix);

    let (eval_input, eval_tail) = if let Some(input) = json_input {
        if let Some((head, tail)) = split_at_last_pipe(query_prefix) {
            let tail_is_contains_builder = tail.trim_start().starts_with("contains(");
            let eval_query = Executor::strip_format_op(&head)
                .map(|(base, _)| base)
                .unwrap_or(head);
            let evaluated = Executor::execute(&eval_query, input)
                .ok()
                .and_then(|mut r| {
                    if r.is_empty() {
                        None
                    } else if tail_is_contains_builder && r.len() > 1 {
                        Some(serde_json::Value::Array(r))
                    } else {
                        Some(r.swap_remove(0))
                    }
                })
                .unwrap_or_else(|| input.clone());
            (Some(evaluated), tail)
        } else {
            (Some(input.clone()), query_prefix.to_string())
        }
    } else {
        (None, query_prefix.to_string())
    };

    let json_completions = if let Some(ref input) = eval_input {
        completions::json_context::get_completions(&eval_tail, input)
            .into_iter()
            .map(|i| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, i.insert_text),
                ..i
            })
            .collect()
    } else {
        Vec::new()
    };

    // When no pipe context type was supplied (e.g. no pipe in query, or async
    // task hasn't resolved yet), derive it from eval_input so that type-gated
    // builtins like strftime (Number) are suppressed at the root level.
    let derived_pipe_type: Option<String> = if pipe_context_type.is_none() {
        eval_input
            .as_ref()
            .map(|v| completions::jq_builtins::jq_type_of(v).to_string())
    } else {
        None
    };
    let effective_pipe_type = pipe_context_type.or(derived_pipe_type.as_deref());

    let with_pipe_prefix = |items: Vec<completions::CompletionItem>| {
        items
            .into_iter()
            .map(|c| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, c.insert_text),
                ..c
            })
            .collect::<Vec<_>>()
    };

    let builtin_completions: Vec<completions::CompletionItem> = with_pipe_prefix(
        completions::jq_builtins::get_completions(token, effective_pipe_type),
    );

    let all_builtin_completions: Vec<completions::CompletionItem> = with_pipe_prefix(
        completions::jq_builtins::get_completions("", effective_pipe_type),
    );

    let fuzzy_builtin_completions: Vec<completions::CompletionItem> =
        if fuzzy_token.is_empty() || !should_offer_builtin_fuzzy(token) {
            Vec::new()
        } else {
            completions::fuzzy::fuzzy_completions(fuzzy_token, &all_builtin_completions)
        };

    let fuzzy_json_completions: Vec<completions::CompletionItem> = if fuzzy_token.is_empty() {
        Vec::new()
    } else if let Some(ref input) = eval_input {
        let fuzzy_tail_prefix = eval_tail.strip_suffix(fuzzy_token).unwrap_or(&eval_tail);
        let all_json_fields = completions::json_context::get_completions(fuzzy_tail_prefix, input);
        completions::fuzzy::fuzzy_completions(fuzzy_token, &all_json_fields)
            .into_iter()
            .map(|i| completions::CompletionItem {
                insert_text: format!("{}{}", prefix, i.insert_text),
                ..i
            })
            .collect()
    } else {
        Vec::new()
    };

    let lsp_patched: Vec<completions::CompletionItem> =
        build_lsp_suggestions(lsp_completions, token, prefix);

    let variable_completions = if let Some(partial) = token.strip_prefix('$') {
        extract_bound_variables(query_prefix)
            .into_iter()
            .filter(|name| name.starts_with(partial))
            .map(|name| completions::CompletionItem {
                label: format!("${}", name),
                detail: Some("bound variable".to_string()),
                insert_text: format!("{}${}", prefix, name),
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let mut merged = variable_completions;
    for item in json_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in builtin_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in fuzzy_json_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in fuzzy_builtin_completions {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }
    for item in lsp_patched {
        if !merged
            .iter()
            .any(|i: &completions::CompletionItem| i.label == item.label)
        {
            merged.push(item);
        }
    }

    merged
        .into_iter()
        .map(|i| widgets::query_input::Suggestion {
            label: i.label,
            detail: i.detail,
            insert_text: i.insert_text,
        })
        .collect()
}

pub fn current_query_prefix(app: &App<'_>) -> String {
    let query = app.query_input.textarea.lines()[0].clone();
    let cursor_col = app.query_input.textarea.cursor().1;
    query.chars().take(cursor_col).collect()
}

pub fn active_string_param_prefix_query(query: &str) -> Option<String> {
    completions::json_context::string_param_context(query, None)?;

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

    let prefix = before_open[..fn_start]
        .trim_end()
        .trim_end_matches('|')
        .trim_end();
    if prefix.is_empty() {
        None
    } else {
        Some(prefix.to_string())
    }
}

pub fn evaluated_string_param_input(
    query_prefix: &str,
    input: &serde_json::Value,
) -> Option<serde_json::Value> {
    let prefix = active_string_param_prefix_query(query_prefix)?;
    let eval_query = Executor::strip_format_op(&prefix)
        .map(|(base, _)| base)
        .unwrap_or(prefix);
    let mut out = Executor::execute(&eval_query, input).ok()?;
    if out.is_empty() {
        Some(serde_json::Value::Null)
    } else if out.len() == 1 {
        Some(out.swap_remove(0))
    } else {
        Some(serde_json::Value::Array(out))
    }
}

pub fn split_at_last_pipe(query: &str) -> Option<(String, String)> {
    if let Some(p) = query.rfind('|') {
        let head = query[..p].to_string();
        let tail = query[p + 1..].to_string();
        Some((head, tail))
    } else {
        None
    }
}

pub fn split_string_param_query_prefix(query: &str) -> Option<(String, String)> {
    completions::json_context::string_param_context(query, None)?;

    let open = crate::accept::find_unmatched_open_paren(query)?;
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

    Some((query[..fn_start].to_string(), query[fn_start..].to_string()))
}

pub fn is_inside_string_literal(query: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in query.chars() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        match quote {
            Some(open) if ch == open => quote = None,
            None if matches!(ch, '"' | '\'') => quote = Some(ch),
            _ => {}
        }
    }

    quote.is_some()
}

pub fn is_inside_double_quoted_string(query_prefix: &str) -> bool {
    let mut in_string = false;
    let mut escaped = false;

    for ch in query_prefix.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
        }
    }

    in_string
}

pub fn current_token(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        query[p + 1..].trim_start()
    } else {
        query
    }
}

pub fn fuzzy_token_fragment(token: &str) -> &str {
    token
        .rsplit(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .next()
        .unwrap_or("")
}

pub fn should_offer_builtin_fuzzy(token: &str) -> bool {
    let t = token.trim_start();
    !t.starts_with('.') && !t.contains('.') && !t.contains('[') && !t.contains('{')
}

pub fn lsp_pipe_prefix(query: &str) -> &str {
    if let Some(p) = query.rfind('|') {
        &query[..p + 1]
    } else {
        ""
    }
}

pub fn normalize_lsp_insert_text(insert_text: &str, label: &str) -> String {
    let mut out = String::new();
    let chars: Vec<char> = insert_text.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let ch = chars[i];
        if ch == '$' {
            if i + 1 < chars.len() && chars[i + 1].is_ascii_digit() {
                i += 2;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                continue;
            }

            if i + 1 < chars.len() && chars[i + 1] == '{' {
                let mut j = i + 2;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j < chars.len() && chars[j] == '}' {
                    i = j + 1;
                    continue;
                }
                if j < chars.len() && chars[j] == ':' {
                    j += 1;
                    while j < chars.len() && chars[j] != '}' {
                        out.push(chars[j]);
                        j += 1;
                    }
                    if j < chars.len() && chars[j] == '}' {
                        i = j + 1;
                        continue;
                    }
                }
            }
        }

        out.push(ch);
        i += 1;
    }

    if out.is_empty() {
        label.to_string()
    } else {
        out
    }
}

pub fn build_lsp_suggestions(
    cache: &[completions::CompletionItem],
    token: &str,
    prefix: &str,
) -> Vec<completions::CompletionItem> {
    cache
        .iter()
        .filter(|c| c.label.starts_with(token))
        .map(|c| completions::CompletionItem {
            insert_text: format!(
                "{}{}",
                prefix,
                normalize_lsp_insert_text(&c.insert_text, &c.label)
            ),
            ..c.clone()
        })
        .collect()
}

pub fn should_hold_output_during_suggestions(query_prefix: &str) -> bool {
    let token = current_token(query_prefix).trim_end();
    // A bare "." is the identity expression — already complete, don't hold.
    if token == "." {
        return false;
    }
    let Some(last) = token.chars().last() else {
        return false;
    };
    matches!(last, '.' | '|' | '[' | '{' | '(' | ',' | ':')
        || last.is_ascii_alphanumeric()
        || matches!(last, '_' | '-' | '@' | '"' | '\'')
}

pub fn has_non_exact_suggestion_for_prefix(
    query_prefix: &str,
    suggestions: &[widgets::query_input::Suggestion],
) -> bool {
    suggestions.iter().any(|s| s.insert_text != query_prefix)
}

pub fn should_ignore_query_input_key(key: &ratatui::crossterm::event::KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(c) if c.is_control())
        || (matches!(key.code, KeyCode::Char(_))
            && key.modifiers.intersects(
                ratatui::crossterm::event::KeyModifiers::CONTROL
                    | ratatui::crossterm::event::KeyModifiers::ALT
                    | ratatui::crossterm::event::KeyModifiers::SUPER,
            ))
}

pub fn suggestion_mode_for_query_edit(
    key_code: KeyCode,
    query_prefix: &str,
    _current_active: bool,
) -> bool {
    if is_inside_double_quoted_string(query_prefix)
        && completions::json_context::string_param_context(query_prefix, None).is_none()
    {
        return false;
    }

    match key_code {
        KeyCode::Char('.')
        | KeyCode::Char('|')
        | KeyCode::Char('{')
        | KeyCode::Char('[')
        | KeyCode::Char(',')
        | KeyCode::Char('@')
        | KeyCode::Backspace
        | KeyCode::Delete => true,
        KeyCode::Char(c) if c.is_alphanumeric() || c == '_' || c == '-' || c == ' ' => {
            // Always activate suggestions when typing alphanumeric characters so that
            // typing `c` in an empty box immediately offers field completions like `.created`.
            // Users should not need to press Down first to get the suggestion dropdown.
            true
        }
        _ => false,
    }
}

pub fn extract_bound_variables(query_prefix: &str) -> Vec<String> {
    let bytes = query_prefix.as_bytes();
    let mut i = 0usize;
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    while i + 1 < bytes.len() {
        if bytes[i] != b'a' || bytes[i + 1] != b's' {
            i += 1;
            continue;
        }

        if i > 0 {
            let prev = bytes[i - 1] as char;
            if prev.is_ascii_alphanumeric() || prev == '_' {
                i += 1;
                continue;
            }
        }

        let mut j = i + 2;
        let mut had_space = false;
        while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
            had_space = true;
            j += 1;
        }
        if !had_space || j >= bytes.len() || bytes[j] != b'$' {
            i += 1;
            continue;
        }

        j += 1;
        let name_start = j;
        while j < bytes.len() {
            let ch = bytes[j] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' {
                j += 1;
            } else {
                break;
            }
        }
        if j > name_start {
            let name = &query_prefix[name_start..j];
            if seen.insert(name.to_string()) {
                out.push(name.to_string());
            }
        }
        i = j;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use jqpp::app::App;
    use ratatui::crossterm::event::KeyCode;

    #[test]
    fn holds_output_for_partial_suggestion_token() {
        assert!(should_hold_output_during_suggestions(".items[].na"));
        assert!(should_hold_output_during_suggestions(".items."));
    }

    #[test]
    fn releases_output_for_committed_parent_segment() {
        assert!(!should_hold_output_during_suggestions(".items[]"));
        assert!(!should_hold_output_during_suggestions(".items[] | ."));
    }

    #[test]
    fn ui_validation_holds_output_while_backspacing_partial_token() {
        let suggestions = vec![widgets::query_input::Suggestion {
            label: ".metadata.exported_at".to_string(),
            detail: Some("field".to_string()),
            insert_text: ".metadata.exported_at".to_string(),
        }];

        assert!(has_non_exact_suggestion_for_prefix(
            ".metadata.exported_a",
            &suggestions
        ));
        assert!(should_hold_output_during_suggestions(
            ".metadata.exported_a"
        ));
    }

    #[test]
    fn ui_validation_releases_output_when_query_matches_suggestion() {
        let suggestions = vec![widgets::query_input::Suggestion {
            label: ".metadata.exported_at".to_string(),
            detail: Some("field".to_string()),
            insert_text: ".metadata.exported_at".to_string(),
        }];

        assert!(!has_non_exact_suggestion_for_prefix(
            ".metadata.exported_at",
            &suggestions
        ));
    }

    #[test]
    fn ui_validation_does_not_hold_when_no_suggestions() {
        let suggestions: Vec<widgets::query_input::Suggestion> = Vec::new();
        assert!(!has_non_exact_suggestion_for_prefix(
            ".metadata",
            &suggestions
        ));
    }

    #[test]
    fn fuzzy_results_appear_with_tilde_detail_when_no_exact_prefix() {
        // ascii_upcase is a string function — use string input so it is eligible.
        let input = serde_json::json!("alice");
        let suggestions = compute_suggestions("up", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "ascii_upcase" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
    }

    #[test]
    fn exact_results_appear_before_fuzzy_results() {
        // startswith / tostring are string functions — use string input so both are eligible.
        let input = serde_json::json!("");
        let suggestions = compute_suggestions("st", Some(&input), &[], None);

        let exact_pos = suggestions.iter().position(|s| {
            s.label == "startswith" && !s.detail.as_deref().unwrap_or("").starts_with('~')
        });
        let fuzzy_pos = suggestions.iter().position(|s| {
            s.label == "tostring" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        });

        assert!(exact_pos.is_some(), "expected exact prefix builtin");
        assert!(fuzzy_pos.is_some(), "expected fuzzy builtin");
        assert!(exact_pos.unwrap() < fuzzy_pos.unwrap());
    }

    #[test]
    fn empty_token_produces_no_fuzzy_candidates() {
        let input = serde_json::json!({"customer_name": "alice"});
        let suggestions = compute_suggestions(".customer | ", Some(&input), &[], None);

        assert!(
            suggestions
                .iter()
                .all(|s| !s.detail.as_deref().unwrap_or("").starts_with('~'))
        );
    }

    #[test]
    fn fuzzy_json_field_matches_when_query_starts_with_dot() {
        let input = serde_json::json!({
            "store_region": "EU-NORTH",
            "store_name": "Nordic Widgets"
        });

        let suggestions = compute_suggestions(".egion", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "store_region" && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
    }

    #[test]
    fn fuzzy_json_respects_nested_context_path() {
        let input = serde_json::json!({
            "store_name": "Nordic Widgets",
            "orders": [{
                "customer": {
                    "name": "Alice",
                    "email": "alice@example.com"
                }
            }]
        });

        let suggestions = compute_suggestions(".orders[].customer.ame", Some(&input), &[], None);

        assert!(suggestions.iter().any(|s| {
            s.label == "name"
                && s.insert_text == ".orders[].customer.name"
                && s.detail.as_deref().is_some_and(|d| d.starts_with('~'))
        }));
        assert!(!suggestions.iter().any(|s| s.insert_text == ".store_name"));
    }

    #[test]
    fn fuzzy_does_not_offer_builtin_functions_in_dot_path_context() {
        let input = serde_json::json!({
            "orders": [{
                "customer": { "name": "Alice" }
            }]
        });

        let suggestions = compute_suggestions(".orders[].customer.ame", Some(&input), &[], None);

        assert!(!suggestions.iter().any(|s| s.label == "ascii_upcase"));
        assert!(!suggestions.iter().any(|s| s.insert_text == "ascii_upcase"));
    }

    #[test]
    fn suggestions_are_suppressed_inside_function_string_arguments() {
        let input = serde_json::json!("alice");

        let suggestions = compute_suggestions("startswith(\"b", Some(&input), &[], Some("string"));

        assert!(
            suggestions.is_empty(),
            "no completions should appear while editing inside quoted function arguments"
        );
    }

    #[test]
    fn string_literal_detection_handles_escaped_quotes() {
        assert!(is_inside_string_literal("startswith(\"a\\\"b"));
        assert!(!is_inside_string_literal("startswith(\"a\\\"b\")"));
    }

    #[test]
    fn parse_input_accepts_plain_text_as_json_string() {
        let parsed = crate::output::parse_input_as_json_or_string(b"kakaka\n").unwrap();
        assert_eq!(parsed, serde_json::json!("kakaka"));
    }

    #[test]
    fn parse_input_keeps_valid_json_behavior() {
        let parsed = crate::output::parse_input_as_json_or_string(br#"{"name":"alice"}"#).unwrap();
        assert_eq!(parsed, serde_json::json!({"name": "alice"}));
    }

    #[test]
    fn parse_input_rejects_non_json_whitespace_text() {
        let err = crate::output::parse_input_as_json_or_string(b"this is not json").unwrap_err();
        assert!(err.to_string().contains("Failed to parse input as JSON"));
    }

    #[test]
    fn structural_hint_suppressed_when_dismissed_query_matches() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[1,2,3]}"#.to_vec(),
            json_input: serde_json::json!({"items": [1, 2, 3]}),
            source_label: "test".to_string(),
            source_format: None,
        });
        app.dismissed_hint_query = Some(".items".to_string());

        let activated = crate::hints::maybe_activate_structural_hint(&mut app, ".items");

        assert!(!activated);
        assert!(!app.structural_hint_active);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn structural_hint_activates_for_empty_query_with_root_array() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"[{"name":"Alice"}]"#.to_vec(),
            json_input: serde_json::json!([{"name": "Alice"}]),
            source_label: "test".to_string(),
            source_format: None,
        });

        let activated = crate::hints::maybe_activate_structural_hint(&mut app, "");

        assert!(activated);
        assert!(app.structural_hint_active);
        assert!(app.query_input.show_suggestions);
        assert_eq!(app.query_input.suggestions[0].label, ".");
    }

    #[test]
    fn cursor_movement_dismisses_structural_hint_without_suppressing_reappearance() {
        // Simulate the state after the [] ghost suggestion has appeared.
        let mut app = App::new();
        app.structural_hint_active = true;
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "[]".to_string(),
            detail: None,
            insert_text: ".items[]".to_string(),
        }];

        // Simulate what the cursor-movement handler does: clear without
        // setting dismissed_hint_query (unlike Esc which sets it).
        app.structural_hint_active = false;
        app.query_input.show_suggestions = false;
        app.query_input.suggestions.clear();

        assert!(
            !app.structural_hint_active,
            "hint should be cleared after cursor move"
        );
        assert!(
            !app.query_input.show_suggestions,
            "dropdown should be hidden"
        );
        assert!(
            app.query_input.suggestions.is_empty(),
            "suggestions should be cleared"
        );
        // dismissed_hint_query must NOT be set — hint must be allowed to reappear
        assert!(
            app.dismissed_hint_query.is_none(),
            "cursor movement must not suppress hint reappearance"
        );
    }

    #[test]
    fn esc_dismisses_structural_hint_and_sets_dismissed_query() {
        let mut app = App::new();
        app.structural_hint_active = true;
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "[]".to_string(),
            detail: None,
            insert_text: ".items[]".to_string(),
        }];

        crate::hints::dismiss_structural_hint(&mut app, ".items");

        assert!(!app.structural_hint_active);
        assert!(!app.query_input.show_suggestions);
        assert!(app.query_input.suggestions.is_empty());
        assert_eq!(app.dismissed_hint_query.as_deref(), Some(".items"));
    }

    #[test]
    fn hint_reappears_when_cursor_returns_to_triggering_position() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[{"id":1}]}"#.to_vec(),
            json_input: serde_json::json!({"items": [{"id": 1}]}),
            source_label: "test".to_string(),
            source_format: None,
        });

        // Hint is showing for ".items"
        crate::hints::maybe_activate_structural_hint(&mut app, ".items");
        assert!(
            app.structural_hint_active,
            "hint should be active at .items"
        );

        // Cursor moves left — hint clears without setting dismissed_hint_query
        app.structural_hint_active = false;
        app.query_input.show_suggestions = false;
        app.query_input.suggestions.clear();
        assert!(app.dismissed_hint_query.is_none());

        // Cursor moves back to ".items" — hint must reappear
        let reactivated = crate::hints::maybe_activate_structural_hint(&mut app, ".items");
        assert!(
            reactivated,
            "hint should reappear when cursor returns to .items"
        );
        assert!(app.structural_hint_active);
        assert!(app.query_input.show_suggestions);
        assert!(!app.query_input.suggestions.is_empty());
    }

    #[test]
    fn builtins_filtered_to_string_type_when_pipe_context_is_string() {
        let input = serde_json::json!({"name": "Alice"});
        // Explicit string context: after `.name |` the pipe value is a string.
        let string_suggestions = compute_suggestions(".name | ", Some(&input), &[], Some("string"));
        // Object context: no pipe, root is an object — derived type is "object".
        let object_suggestions = compute_suggestions("", Some(&input), &[], None);

        // ascii_upcase is a string-only builtin — must appear in string context
        assert!(
            string_suggestions.iter().any(|s| s.label == "ascii_upcase"),
            "ascii_upcase should be suggested for string context"
        );
        // ascii_upcase must NOT appear for object context
        assert!(
            !object_suggestions.iter().any(|s| s.label == "ascii_upcase"),
            "ascii_upcase should not be suggested for object context"
        );

        // length applies to any type — must appear in both
        assert!(
            string_suggestions.iter().any(|s| s.label == "length"),
            "length should be suggested for string context"
        );
        assert!(
            object_suggestions.iter().any(|s| s.label == "length"),
            "length should be suggested for object context"
        );

        // keys is object/array only — must NOT appear in string context
        assert!(
            !string_suggestions.iter().any(|s| s.label == "keys"),
            "keys should not be suggested for string context"
        );
        // keys must appear for object context
        assert!(
            object_suggestions.iter().any(|s| s.label == "keys"),
            "keys should be suggested for object context"
        );
    }

    #[test]
    fn strftime_absent_for_object_root_input() {
        // strftime requires a number — must not appear when root input is an object,
        // even with no explicit pipe_context_type (the common case when typing at root).
        let input = serde_json::json!({"ts": 1700000000});
        let suggestions = compute_suggestions("strf", Some(&input), &[], None);
        assert!(
            !suggestions.iter().any(|s| s.label == "strftime"),
            "strftime should not be suggested when root input is an object"
        );
    }

    #[test]
    fn strptime_absent_for_object_root_input() {
        // strptime requires a string — must not appear when root input is an object.
        let input = serde_json::json!({"date": "2024-01-01"});
        let suggestions = compute_suggestions("strp", Some(&input), &[], None);
        assert!(
            !suggestions.iter().any(|s| s.label == "strptime"),
            "strptime should not be suggested when root input is an object"
        );
    }

    #[test]
    fn strftime_appears_for_number_root_input() {
        // strftime should appear when root input IS a number.
        let input = serde_json::json!(1700000000);
        let suggestions = compute_suggestions("strf", Some(&input), &[], None);
        assert!(
            suggestions.iter().any(|s| s.label == "strftime"),
            "strftime should be suggested when root input is a number"
        );
    }

    #[test]
    fn builtins_filtered_to_array_type_when_pipe_context_is_array() {
        let input = serde_json::json!({"items": [1, 2, 3]});
        let array_suggestions = compute_suggestions(".items | ", Some(&input), &[], Some("array"));

        // map is array-only — must appear
        assert!(
            array_suggestions.iter().any(|s| s.label.starts_with("map")),
            "map should be suggested for array context"
        );
        // ascii_upcase is string-only — must NOT appear
        assert!(
            !array_suggestions.iter().any(|s| s.label == "ascii_upcase"),
            "ascii_upcase should not be suggested for array context"
        );
    }

    #[test]
    fn structural_hint_resets_suggestion_index_to_zero() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: br#"{"items":[1,2,3]}"#.to_vec(),
            json_input: serde_json::json!({"items": [1, 2, 3]}),
            source_label: "test".to_string(),
            source_format: None,
        });
        app.query_input.suggestion_index = 3;
        app.query_input.suggestion_scroll = 2;

        let activated = crate::hints::maybe_activate_structural_hint(&mut app, ".items");

        assert!(activated);
        assert_eq!(app.query_input.suggestion_index, 0);
        assert_eq!(app.query_input.suggestion_scroll, 0);
    }

    #[test]
    fn detects_when_cursor_is_inside_double_quoted_string() {
        assert!(is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"a"
        ));
        assert!(is_inside_double_quoted_string(
            ".foo|startswith(\"escaped \\\" quote"
        ));
    }

    #[test]
    fn detects_when_cursor_is_outside_double_quoted_string() {
        assert!(!is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"a\")"
        ));
        assert!(!is_inside_double_quoted_string(
            ".orders[].customer.customer_id|ascii_downcase|startswith(\"\")|."
        ));
    }

    #[test]
    fn string_param_quoted_text_edit_keeps_suggestions_active() {
        let q1 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"a";
        let s1 = suggestion_mode_for_query_edit(KeyCode::Char('a'), q1, true);
        assert!(s1);

        let q2 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"";
        let s2 = suggestion_mode_for_query_edit(KeyCode::Backspace, q2, s1);
        assert!(s2);

        let q3 = ".orders[].customer.customer_id|ascii_downcase|startswith(\"b";
        let s3 = suggestion_mode_for_query_edit(KeyCode::Char('b'), q3, s2);
        assert!(s3);
    }

    // --- suggestion_mode_for_query_edit: trigger-context rules ---

    #[test]
    fn typing_alpha_on_empty_query_activates_suggestions() {
        // User starts typing from scratch — suggestions should appear immediately
        // so that e.g. typing `c` offers field completions like `.created`.
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('a'),
            "a",
            false
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('s'),
            "as",
            false
        ));
    }

    #[test]
    fn typing_alpha_activates_suggestions_regardless_of_prior_state() {
        // Suggestions activate on alphanumeric whether or not they were already on.
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('a'),
            "a",
            false
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('a'),
            "a",
            true
        ));
    }

    #[test]
    fn typing_alpha_after_dot_keeps_suggestions_active() {
        // ".f" → user is filtering a field name; suggestions stay visible.
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('f'),
            ".f",
            true
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('o'),
            ".fo",
            true
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('o'),
            ".foo",
            true
        ));
    }

    #[test]
    fn typing_alpha_after_pipe_keeps_suggestions_active() {
        // ".x | f" → user is typing a function name after a pipe.
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('f'),
            ".x|f",
            true
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('o'),
            ".x|fo",
            true
        ));
    }

    #[test]
    fn typing_alpha_after_array_accessor_keeps_suggestions_active() {
        // ".[].name" → inside an array path.
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('n'),
            "[].n",
            true
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('e'),
            ".items[].name",
            true
        ));
    }

    #[test]
    fn dot_and_pipe_always_activate_regardless_of_current_state() {
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('.'),
            ".",
            false
        ));
        assert!(suggestion_mode_for_query_edit(
            KeyCode::Char('|'),
            "a|",
            false
        ));
    }

    #[test]
    fn suggestions_for_complex_pipe_chain_in_obj_constructor() {
        let input = serde_json::from_str::<serde_json::Value>(
            &std::fs::read_to_string("examples/string-functions-kitchen-sink.json").unwrap(),
        )
        .unwrap();

        let suggestions = compute_suggestions(
            ".users | sort_by([.role, .email])[] | {",
            Some(&input),
            &[],
            None,
        );

        assert!(
            suggestions.iter().any(|s| s.label == "role"),
            "Expected 'role' in suggestions, but got: {:?}",
            suggestions
        );
        assert!(
            suggestions.iter().any(|s| s.label == "email"),
            "Expected 'email' in suggestions"
        );
    }

    #[test]
    fn suggestions_for_nested_array_field_access() {
        let input = serde_json::json!({
            "orders": [{"customer": {"id": 1, "name": "Alice"}}]
        });

        let suggestions = compute_suggestions(".orders[] | .customer | {", Some(&input), &[], None);

        assert!(
            suggestions.iter().any(|s| s.label == "name"),
            "Expected 'name' in suggestions, but got: {:?}",
            suggestions
        );
    }

    #[test]
    fn compute_suggestions_in_string_param_context_prefers_runtime_candidates() {
        let input = serde_json::json!(["a-b", "c-d"]);
        let s = compute_suggestions("split(\"", Some(&input), &[], Some("string"));
        assert!(s.iter().any(|i| i.label == "-"));
        assert!(!s.iter().any(|i| i.label == "split"));
    }

    #[test]
    fn contains_object_value_suggestions_include_all_items_after_pipe_evaluation() {
        let input = serde_json::json!({
            "orders": [
                {"order_id": "ORD-001", "status": "shipped"},
                {"order_id": "ORD-002", "status": "processing"}
            ]
        });

        let s = compute_suggestions(
            ".orders[]|contains({order_id: \"ORD-",
            Some(&input),
            &[],
            Some("object"),
        );

        assert!(s.iter().any(|i| i.label == "ORD-001"));
        assert!(s.iter().any(|i| i.label == "ORD-002"));
    }

    #[test]
    fn contains_object_value_suggestions_work_when_returning_to_existing_field() {
        let input = serde_json::json!({
            "orders": [
                {"order_id": "ORD-001", "status": "shipped"},
                {"order_id": "ORD-002", "status": "processing"}
            ]
        });

        let s = compute_suggestions(
            ".orders[]|contains({order_id: \"ORD-",
            Some(&input),
            &[],
            None,
        );

        assert!(s.iter().any(|i| i.label == "ORD-001"));
        assert!(s.iter().any(|i| i.label == "ORD-002"));
    }

    #[test]
    fn extract_bound_variables_handles_required_patterns() {
        assert_eq!(extract_bound_variables("5 as $x |"), vec!["x".to_string()]);
        assert_eq!(
            extract_bound_variables(".[] as $item | $item.tags[] as $tag |"),
            vec!["item".to_string(), "tag".to_string()]
        );
        assert_eq!(
            extract_bound_variables("reduce .[] as $acc (0;"),
            vec!["acc".to_string()]
        );
        assert_eq!(
            extract_bound_variables("foreach .[] as $x (0;"),
            vec!["x".to_string()]
        );
        assert!(extract_bound_variables(".foo | .bar").is_empty());
        assert!(extract_bound_variables("").is_empty());
    }

    #[test]
    fn dollar_token_offers_bound_variables_with_filtering() {
        let all = compute_suggestions("5 as $x | 10 as $y | $", None, &[], None);
        assert!(all.iter().any(|s| s.label == "$x"));
        assert!(all.iter().any(|s| s.label == "$y"));
        assert!(
            all.iter()
                .filter(|s| s.label == "$x" || s.label == "$y")
                .all(|s| s.detail.as_deref() == Some("bound variable"))
        );

        let filtered = compute_suggestions("5 as $foo | 10 as $bar | $f", None, &[], None);
        assert!(filtered.iter().any(|s| s.label == "$foo"));
        assert!(!filtered.iter().any(|s| s.label == "$bar"));

        let none = compute_suggestions("5 as $foo | $z", None, &[], None);
        assert!(none.iter().all(|s| s.label != "$foo"));
    }

    #[test]
    fn dollar_token_bound_variables_precede_lsp_items() {
        let lsp = vec![completions::CompletionItem {
            label: "$x_ext".to_string(),
            detail: Some("lsp".to_string()),
            insert_text: "$x_ext".to_string(),
        }];
        let suggestions = compute_suggestions("1 as $x | $", None, &lsp, None);

        let var_pos = suggestions.iter().position(|s| s.label == "$x").unwrap();
        let lsp_pos = suggestions
            .iter()
            .position(|s| s.label == "$x_ext")
            .unwrap();
        assert!(var_pos < lsp_pos);
    }

    #[test]
    fn active_string_param_prefix_query_extracts_pipe_prefix() {
        assert_eq!(
            active_string_param_prefix_query("ascii_upcase|endswith(\""),
            Some("ascii_upcase".to_string())
        );
        assert_eq!(
            active_string_param_prefix_query(".name | ascii_upcase | endswith(\"a"),
            Some(".name | ascii_upcase".to_string())
        );
        assert_eq!(active_string_param_prefix_query("startswith(\""), None);
    }

    #[test]
    fn split_string_param_query_prefix_splits_head_and_tail() {
        assert_eq!(
            split_string_param_query_prefix(".users[].name|endswith("),
            Some((".users[].name|".to_string(), "endswith(".to_string()))
        );
        assert_eq!(
            split_string_param_query_prefix("endswith(\"a"),
            Some(("".to_string(), "endswith(\"a".to_string()))
        );
    }

    #[test]
    fn string_param_suggestions_use_evaluated_pipe_output_value() {
        let input = serde_json::json!("kakaka");
        let s = compute_suggestions(
            "ascii_upcase|endswith(\"",
            Some(&input),
            &[],
            Some("string"),
        );
        assert!(s.iter().any(|i| i.label == "KAKAKA"));
    }

    #[test]
    fn string_param_suggestions_follow_type_changes_through_pipe_chain() {
        let input = serde_json::json!({"n": 12, "s": "hello"});

        let tostring = compute_suggestions(
            ".n | tostring | startswith(\"",
            Some(&input),
            &[],
            Some("string"),
        );
        assert!(tostring.iter().any(|i| i.label == "12"));

        let non_string =
            compute_suggestions(".s | length | startswith(\"", Some(&input), &[], None);
        assert!(non_string.is_empty());
    }

    #[test]
    fn endswith_suggestions_work_after_pipe_expression_prefix() {
        let input = serde_json::json!({
            "users": [{"name": "Alice"}, {"name": "Bob"}, {"name": "Alicia"}]
        });
        let suggestions =
            compute_suggestions(".users[].name|endswith(", Some(&input), &[], Some("string"));

        assert!(suggestions.iter().any(|s| s.label == "Alice"));
        assert!(
            suggestions
                .iter()
                .any(|s| s.insert_text.starts_with(".users[].name|endswith(\""))
        );
    }

    #[test]
    fn enter_commits_current_string_param_prefix_and_closes_call() {
        let full = ".[].name|startswith(\"Ali";
        let cursor = full.chars().count();
        let (new_query, col) =
            crate::accept::commit_current_string_param_input(full, cursor).unwrap();
        assert_eq!(new_query, ".[].name|startswith(\"Ali\")");
        assert_eq!(col as usize, ".[].name|startswith(\"Ali\")".chars().count());
    }

    #[test]
    fn enter_commit_replaces_existing_param_and_preserves_tail() {
        let full = ".[].name|startswith(\"Ali\") | .age";
        let cursor = ".[].name|startswith(\"Ali".chars().count();
        let (new_query, col) =
            crate::accept::commit_current_string_param_input(full, cursor).unwrap();
        assert_eq!(new_query, ".[].name|startswith(\"Ali\") | .age");
        assert_eq!(col as usize, ".[].name|startswith(\"Ali\")".chars().count());
    }

    #[test]
    fn tab_expands_string_param_to_longest_common_prefix() {
        let full = "startswith(\"A";
        let cursor = full.chars().count();
        let suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Alina".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alina\")".to_string(),
            },
        ];

        let (new_query, col) =
            crate::accept::expand_string_param_prefix_with_tab(full, cursor, &suggestions, 0)
                .unwrap();
        assert_eq!(new_query, "startswith(\"Alice");
        assert_eq!(col as usize, "startswith(\"Alice".chars().count());
    }

    #[test]
    fn tab_prefix_expand_noop_when_no_further_common_prefix() {
        let full = "startswith(\"Ali";
        let cursor = full.chars().count();
        let suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Alina".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alina\")".to_string(),
            },
        ];

        let (new_query, _) =
            crate::accept::expand_string_param_prefix_with_tab(full, cursor, &suggestions, 0)
                .unwrap();
        assert_eq!(new_query, "startswith(\"Alice");
    }

    #[test]
    fn tab_can_extend_across_multiple_token_boundaries() {
        let s = vec![widgets::query_input::Suggestion {
            label: "Alice Smith".to_string(),
            detail: Some("string value".to_string()),
            insert_text: "startswith(\"Alice Smith\")".to_string(),
        }];

        let q1 = "startswith(\"A";
        let c1 = q1.chars().count();
        let (q2, _) = crate::accept::expand_string_param_prefix_with_tab(q1, c1, &s, 0).unwrap();
        assert_eq!(q2, "startswith(\"Alice");

        let c2 = q2.chars().count();
        let (q3, _) = crate::accept::expand_string_param_prefix_with_tab(&q2, c2, &s, 0).unwrap();
        assert_eq!(q3, "startswith(\"Alice Smith");
    }

    #[test]
    fn tab_extends_suffix_from_short_to_longer_suffix_tokens() {
        let s = vec![
            widgets::query_input::Suggestion {
                label: "com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"com\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "corp.com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"corp.com\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "@corp.com".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "endswith(\"@corp.com\")".to_string(),
            },
        ];

        let q1 = "endswith(\"com";
        let c1 = q1.chars().count();
        let (q2, _) = crate::accept::expand_string_param_prefix_with_tab(q1, c1, &s, 0).unwrap();
        assert_eq!(q2, "endswith(\"corp.com");

        let c2 = q2.chars().count();
        let (q3, _) = crate::accept::expand_string_param_prefix_with_tab(&q2, c2, &s, 0).unwrap();
        assert_eq!(q3, "endswith(\"@corp.com");
    }

    #[test]
    fn string_param_context_strips_trailing_chars_correctly() {
        let full = "startswith(\"Alice\")";
        let cursor = "startswith(\"Alic".chars().count();
        let query_prefix: String = full.chars().take(cursor).collect();

        let ctx = completions::json_context::string_param_context(&query_prefix, None).unwrap();
        assert_eq!(ctx.inner_prefix, "Alic");
    }
}
