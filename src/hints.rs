use crate::suggestions::compute_suggestions;
use jqpp::app::App;
use jqpp::completions;
use jqpp::widgets;

pub fn maybe_activate_structural_hint(app: &mut App<'_>, query_prefix: &str) -> bool {
    if app.dismissed_hint_query.as_deref() == Some(query_prefix) {
        return false;
    }

    let Some(exec) = app.executor.as_ref() else {
        return false;
    };

    if let Some(hints) =
        completions::json_context::next_structural_hint(query_prefix, &exec.json_input)
        && let Some(suggestion) = hints.first()
    {
        app.structural_hint_active = true;
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: suggestion.label.clone(),
            detail: suggestion.detail.clone(),
            insert_text: suggestion.insert_text.clone(),
        }];
        app.query_input.suggestion_index = 0;
        app.query_input.suggestion_scroll = 0;
        app.query_input.suggestion_anchor_col = Some(query_prefix.chars().count() as u16);
        return true;
    }
    false
}

pub fn dismiss_structural_hint(app: &mut App<'_>, query_prefix: &str) {
    app.structural_hint_active = false;
    app.query_input.show_suggestions = false;
    app.query_input.suggestion_anchor_col = None;
    app.query_input.suggestions.clear();
    app.dismissed_hint_query = Some(query_prefix.to_string());
}

pub fn clear_dismissed_hint_if_query_changed(app: &mut App<'_>, current_query: &str) {
    if let Some(ref dismissed) = app.dismissed_hint_query
        && !current_query.starts_with(dismissed)
    {
        app.dismissed_hint_query = None;
    }
}

pub fn open_suggestions_from_structural_hint(
    app: &mut App<'_>,
    query_prefix: &str,
    lsp_completions: &[completions::CompletionItem],
    cached_pipe_type: Option<&str>,
    suggestion_active: &mut bool,
    select_last: bool,
) {
    let Some(hint) = app.query_input.suggestions.first() else {
        return;
    };

    let trigger = match hint.label.as_str() {
        "[]" => "[",
        "." => ".",
        _ => "",
    };
    if trigger.is_empty() {
        return;
    }

    let prefix = format!("{}{}", query_prefix, trigger);
    app.query_input.suggestions = compute_suggestions(
        &prefix,
        app.executor.as_ref().map(|e| &e.json_input),
        lsp_completions,
        cached_pipe_type,
    );
    app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
    app.query_input.suggestion_anchor_col = if app.query_input.show_suggestions {
        Some(prefix.chars().count() as u16)
    } else {
        None
    };
    app.structural_hint_active = false;
    *suggestion_active = true;
    if app.query_input.show_suggestions {
        app.query_input.suggestion_index = if select_last {
            app.query_input.suggestions.len().saturating_sub(1)
        } else {
            0
        };
        app.query_input.clamp_scroll();
    }
}
