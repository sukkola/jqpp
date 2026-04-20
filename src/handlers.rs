use crate::accept::{
    apply_contains_builder_suggestion, apply_numeric_builder_suggestion, apply_selected_suggestion,
    commit_current_string_param_input, cursor_col_after_accept,
    expand_string_param_prefix_with_tab, finalize_contains_builder_on_escape,
    finalize_numeric_builder_on_escape, is_contains_builder_suggestion,
    is_numeric_builder_suggestion, is_string_param_value_suggestion,
    starts_context_aware_function_call,
};
use crate::hints::{
    clear_dismissed_hint_if_query_changed, dismiss_structural_hint, maybe_activate_structural_hint,
    open_suggestions_from_structural_hint,
};
use crate::loop_state::LoopState;
use crate::suggestions::{
    compute_suggestions, current_query_prefix, should_ignore_query_input_key,
    suggestion_mode_for_query_edit,
};
use jqpp::app::{App, AppState};
use jqpp::executor::Executor;
use jqpp::keymap::{self, Keymap};
use ratatui::crossterm::event::{KeyCode, KeyEvent};
use std::time::Instant;

pub fn handle_query_input_key(
    app: &mut App<'_>,
    state: &mut LoopState,
    key: KeyEvent,
    keymap: &Keymap,
) {
    let is_action = |a: keymap::Action| keymap.is_action(a, &key);

    if is_action(keymap::Action::Submit) {
        state.string_param_expansion_stack.clear();
        let can_apply_hidden_string_param_selection = if !app.query_input.show_suggestions
            && state.suggestion_active
            && !app.query_input.suggestions.is_empty()
        {
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();
            let query_prefix: String = full.chars().take(cur).collect();
            let selected = &app.query_input.suggestions[app.query_input.suggestion_index];
            is_string_param_value_suggestion(selected.detail.as_deref())
                && jqpp::completions::json_context::string_param_context(&query_prefix, None)
                    .is_some()
        } else {
            false
        };

        if (app.query_input.show_suggestions || can_apply_hidden_string_param_selection)
            && !app.query_input.suggestions.is_empty()
        {
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();

            let selected = app.query_input.suggestions[app.query_input.suggestion_index].clone();
            let suggestion = selected.insert_text;
            let (new_text, col, keep_active) =
                if is_contains_builder_suggestion(selected.detail.as_deref()) {
                    // Enter on contains-builder value finalizes current selection set.
                    apply_contains_builder_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                        true,
                    )
                } else if is_numeric_builder_suggestion(selected.detail.as_deref()) {
                    apply_numeric_builder_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                        true,
                    )
                } else {
                    let (t, c) = apply_selected_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                    );
                    (t, c, starts_context_aware_function_call(&suggestion))
                };
            app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
            app.query_input.textarea.set_block(
                ratatui::widgets::Block::default()
                    .title(" Query ")
                    .borders(ratatui::widgets::Borders::ALL),
            );
            app.query_input
                .textarea
                .set_cursor_line_style(ratatui::style::Style::default());
            app.query_input
                .textarea
                .move_cursor(tui_textarea::CursorMove::Jump(0, col));
            app.query_input.show_suggestions = keep_active;
            state.suggestion_active = keep_active;
            state.lsp_completions.clear();
            state.cached_pipe_type = None;
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
        } else {
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();
            if let Some((new_text, col)) = commit_current_string_param_input(&full, cur) {
                app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
                app.query_input.textarea.set_block(
                    ratatui::widgets::Block::default()
                        .title(" Query ")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                app.query_input
                    .textarea
                    .set_cursor_line_style(ratatui::style::Style::default());
                app.query_input
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                app.query_input.show_suggestions = false;
                state.suggestion_active = false;
                state.lsp_completions.clear();
                state.cached_pipe_type = None;
                state.last_edit_at = Instant::now() - state.debounce_duration;
                state.debounce_pending = true;
                return;
            }

            app.query_input.show_suggestions = false;
            state.suggestion_active = false;
            let query = app.query_input.textarea.lines()[0].clone();
            app.query_input.push_history(query.clone());
            if let Some(ref exec) = app.executor {
                match Executor::execute_query(&query, &exec.json_input) {
                    Ok((results, raw)) => {
                        app.results = results;
                        app.error = None;
                        app.raw_output = raw;
                    }
                    Err(e) => {
                        app.error = Some(e.to_string());
                        app.results = Vec::new();
                        app.raw_output = false;
                    }
                }
            }
        }
    } else if is_action(keymap::Action::SaveOutput) {
        let output = Executor::format_results(&app.results, app.raw_output);
        if std::fs::write("jqpp-output.json", output).is_ok() {
            state.footer_message = Some(("saved".to_string(), Instant::now()));
        }
    } else if is_action(keymap::Action::AcceptSuggestion) || is_action(keymap::Action::NextPane) {
        if is_action(keymap::Action::AcceptSuggestion)
            && app.structural_hint_active
            && !app.query_input.suggestions.is_empty()
        {
            let suggestion = app.query_input.suggestions[0].clone();
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();
            let suffix: String = full.chars().skip(cur).collect();
            let new_text = format!("{}{}", suggestion.insert_text, suffix);
            let col = cursor_col_after_accept(&suggestion.insert_text);
            app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
            app.query_input.textarea.set_block(
                ratatui::widgets::Block::default()
                    .title(" Query ")
                    .borders(ratatui::widgets::Borders::ALL),
            );
            app.query_input
                .textarea
                .set_cursor_line_style(ratatui::style::Style::default());
            app.query_input
                .textarea
                .move_cursor(tui_textarea::CursorMove::Jump(0, col));

            app.structural_hint_active = false;
            app.query_input.show_suggestions = false;

            if suggestion.label == "." {
                let query_prefix = current_query_prefix(app);
                app.query_input.suggestions = compute_suggestions(
                    &query_prefix,
                    app.executor.as_ref().map(|e| &e.json_input),
                    &state.lsp_completions,
                    state.cached_pipe_type.as_deref(),
                );
                app.query_input.suggestion_index = 0;
                app.query_input.suggestion_scroll = 0;
                app.query_input.show_suggestions = !app.query_input.suggestions.is_empty();
                state.suggestion_active = app.query_input.show_suggestions;
            } else {
                state.suggestion_active = false;
            }

            let new_query = app.query_input.textarea.lines()[0].clone();
            clear_dismissed_hint_if_query_changed(app, &new_query);
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
        } else if app.query_input.show_suggestions
            && !app.query_input.suggestions.is_empty()
            && is_action(keymap::Action::AcceptSuggestion)
        {
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();
            if let Some((new_text, col)) = expand_string_param_prefix_with_tab(
                &full,
                cur,
                &app.query_input.suggestions,
                app.query_input.suggestion_index,
            ) {
                state.string_param_expansion_stack.push((full, cur));
                app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
                app.query_input.textarea.set_block(
                    ratatui::widgets::Block::default()
                        .title(" Query ")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                app.query_input
                    .textarea
                    .set_cursor_line_style(ratatui::style::Style::default());
                app.query_input
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                app.query_input.show_suggestions = true;
                state.suggestion_active = true;
                state.last_edit_at = Instant::now() - state.debounce_duration;
                state.debounce_pending = true;
                return;
            }

            let selected = app.query_input.suggestions[app.query_input.suggestion_index].clone();
            let suggestion = selected.insert_text;
            let (new_text, col, keep_active) =
                if is_contains_builder_suggestion(selected.detail.as_deref()) {
                    // Tab on contains-builder keeps the builder open for additional items.
                    apply_contains_builder_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                        false,
                    )
                } else if is_numeric_builder_suggestion(selected.detail.as_deref()) {
                    apply_numeric_builder_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                        false,
                    )
                } else {
                    let (t, c) = apply_selected_suggestion(
                        &suggestion,
                        selected.detail.as_deref(),
                        &full,
                        cur,
                    );
                    (t, c, starts_context_aware_function_call(&suggestion))
                };
            app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
            app.query_input.textarea.set_block(
                ratatui::widgets::Block::default()
                    .title(" Query ")
                    .borders(ratatui::widgets::Borders::ALL),
            );
            app.query_input
                .textarea
                .set_cursor_line_style(ratatui::style::Style::default());
            app.query_input
                .textarea
                .move_cursor(tui_textarea::CursorMove::Jump(0, col));
            app.query_input.show_suggestions = keep_active;
            state.suggestion_active = keep_active;
            state.lsp_completions.clear();
            state.cached_pipe_type = None;
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
        } else if is_action(keymap::Action::NextPane) {
            app.next_pane();
        }
    } else if is_action(keymap::Action::PrevPane) {
        app.query_input.show_suggestions = false;
        state.suggestion_active = false;
        app.prev_pane();
    } else if is_action(keymap::Action::SuggestionUp) || is_action(keymap::Action::HistoryUp) {
        if app.structural_hint_active && is_action(keymap::Action::SuggestionUp) {
            let query_prefix = current_query_prefix(app);
            open_suggestions_from_structural_hint(
                app,
                &query_prefix,
                &state.lsp_completions,
                state.cached_pipe_type.as_deref(),
                &mut state.suggestion_active,
                true,
            );
            return;
        }
        if app.query_input.show_suggestions && is_action(keymap::Action::SuggestionUp) {
            if app.query_input.suggestion_index > 0 {
                app.query_input.suggestion_index -= 1;
                app.query_input.clamp_scroll();
            } else {
                app.query_input.show_suggestions = false;
                state.suggestion_active = false;
                state.lsp_completions.clear();
                state.cached_pipe_type = None;
            }
        } else if is_action(keymap::Action::HistoryUp) {
            if state.suggestion_active && !app.query_input.suggestions.is_empty() {
                app.query_input.show_suggestions = true;
                app.query_input.suggestion_index =
                    app.query_input.suggestions.len().saturating_sub(1);
                app.query_input.clamp_scroll();
            } else {
                app.query_input.history_up();
            }
        }
    } else if is_action(keymap::Action::SuggestionDown) || is_action(keymap::Action::HistoryDown) {
        if app.structural_hint_active && is_action(keymap::Action::SuggestionDown) {
            let query_prefix = current_query_prefix(app);
            open_suggestions_from_structural_hint(
                app,
                &query_prefix,
                &state.lsp_completions,
                state.cached_pipe_type.as_deref(),
                &mut state.suggestion_active,
                false,
            );
            return;
        }
        if app.query_input.show_suggestions && is_action(keymap::Action::SuggestionDown) {
            if app.query_input.suggestion_index + 1 < app.query_input.suggestions.len() {
                app.query_input.suggestion_index += 1;
                app.query_input.clamp_scroll();
            }
        } else if is_action(keymap::Action::HistoryDown)
            || is_action(keymap::Action::SuggestionDown)
        {
            state.suggestion_active = true;
            app.structural_hint_active = false;
            if !app.query_input.suggestions.is_empty() {
                app.query_input.show_suggestions = true;
                app.query_input.suggestion_index = 0;
                app.query_input.clamp_scroll();
            } else {
                state.last_edit_at = Instant::now() - state.debounce_duration;
                state.debounce_pending = true;
            }
        }
    } else if key.code == KeyCode::Esc {
        if !state.string_param_expansion_stack.is_empty()
            && let Some((prev_query, prev_col)) = state.string_param_expansion_stack.pop()
        {
            app.query_input.textarea = tui_textarea::TextArea::from(vec![prev_query]);
            app.query_input.textarea.set_block(
                ratatui::widgets::Block::default()
                    .title(" Query ")
                    .borders(ratatui::widgets::Borders::ALL),
            );
            app.query_input
                .textarea
                .set_cursor_line_style(ratatui::style::Style::default());
            app.query_input
                .textarea
                .move_cursor(tui_textarea::CursorMove::Jump(0, prev_col as u16));
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
            return;
        }

        let query_prefix = current_query_prefix(app);
        if app.structural_hint_active {
            dismiss_structural_hint(app, &query_prefix);
            state.suggestion_active = false;
            state.last_esc_at = Some(Instant::now());
        } else if app.query_input.show_suggestions {
            let cur = app.query_input.textarea.cursor().1;
            let full = app.query_input.textarea.lines()[0].clone();
            if let Some((new_query, col)) = finalize_numeric_builder_on_escape(&full, cur) {
                app.query_input.textarea = tui_textarea::TextArea::from(vec![new_query]);
                app.query_input.textarea.set_block(
                    ratatui::widgets::Block::default()
                        .title(" Query ")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                app.query_input
                    .textarea
                    .set_cursor_line_style(ratatui::style::Style::default());
                app.query_input
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                app.query_input.show_suggestions = false;
                state.suggestion_active = false;
                return;
            }
            if let Some((new_text, col)) = finalize_contains_builder_on_escape(&full, cur) {
                app.query_input.textarea = tui_textarea::TextArea::from(vec![new_text]);
                app.query_input.textarea.set_block(
                    ratatui::widgets::Block::default()
                        .title(" Query ")
                        .borders(ratatui::widgets::Borders::ALL),
                );
                app.query_input
                    .textarea
                    .set_cursor_line_style(ratatui::style::Style::default());
                app.query_input
                    .textarea
                    .move_cursor(tui_textarea::CursorMove::Jump(0, col));
                state.last_edit_at = Instant::now() - state.debounce_duration;
                state.debounce_pending = true;
            }
            app.query_input.show_suggestions = false;
            state.suggestion_active = false;
            app.structural_hint_active = false;
            state.lsp_completions.clear();
            state.cached_pipe_type = None;
            state.last_esc_at = Some(Instant::now());
        } else if state
            .last_esc_at
            .map(|t| t.elapsed() < std::time::Duration::from_millis(500))
            .unwrap_or(false)
        {
            let mut new_ta = tui_textarea::TextArea::default();
            new_ta.set_block(
                ratatui::widgets::Block::default()
                    .title(" Query ")
                    .borders(ratatui::widgets::Borders::ALL),
            );
            new_ta.set_cursor_line_style(ratatui::style::Style::default());
            app.query_input.textarea = new_ta;
            app.query_input.show_suggestions = false;
            state.suggestion_active = false;
            app.structural_hint_active = false;
            state.lsp_completions.clear();
            state.cached_pipe_type = None;
            state.last_esc_at = None;
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
        } else {
            state.last_esc_at = Some(Instant::now());
        }
    } else if is_action(keymap::Action::ToggleQueryBar) {
        app.query_bar_visible = !app.query_bar_visible;
        if !app.query_bar_visible {
            app.state = AppState::LeftPane;
        }
    } else if is_action(keymap::Action::ToggleMenu) {
        app.side_menu.visible = !app.side_menu.visible;
        if app.side_menu.visible {
            app.state = AppState::SideMenu;
        } else if matches!(app.state, AppState::SideMenu) {
            app.state = AppState::QueryInput;
        }
    } else if matches!(
        key.code,
        KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End
    ) {
        state.string_param_expansion_stack.clear();
        app.structural_hint_active = false;
        app.query_input.show_suggestions = false;
        app.query_input.suggestions.clear();
        app.query_input.textarea.input(key);
        if !state.suggestion_active {
            let new_line = app.query_input.textarea.lines()[0].clone();
            let new_col = app.query_input.textarea.cursor().1;
            let new_prefix: String = new_line.chars().take(new_col).collect();
            maybe_activate_structural_hint(app, &new_prefix);
        }
    } else if !should_ignore_query_input_key(&key) && app.query_input.textarea.input(key) {
        state.string_param_expansion_stack.clear();
        state.last_edit_at = Instant::now();
        state.debounce_pending = true;
        let query_prefix = current_query_prefix(app);
        let next_suggestion_active =
            suggestion_mode_for_query_edit(key.code, &query_prefix, state.suggestion_active);
        state.suggestion_active = next_suggestion_active;
        app.structural_hint_active = false;
        if !state.suggestion_active {
            app.query_input.show_suggestions = false;
            app.query_input.suggestions.clear();
        }
        let new_query = app.query_input.textarea.lines()[0].clone();
        clear_dismissed_hint_if_query_changed(app, &new_query);
    }
}

pub fn handle_side_menu_key(
    app: &mut App<'_>,
    _state: &mut LoopState,
    key: KeyEvent,
    keymap: &Keymap,
) {
    let is_action = |a: keymap::Action| keymap.is_action(a, &key);

    if is_action(keymap::Action::NextPane) {
        app.next_pane();
    } else if is_action(keymap::Action::PrevPane) {
        app.prev_pane();
    } else if is_action(keymap::Action::SuggestionUp) {
        if app.side_menu.selected > 0 {
            app.side_menu.selected -= 1;
        } else {
            app.side_menu.selected = app.side_menu.items.len() - 1;
        }
    } else if is_action(keymap::Action::SuggestionDown) {
        if app.side_menu.selected + 1 < app.side_menu.items.len() {
            app.side_menu.selected += 1;
        } else {
            app.side_menu.selected = 0;
        }
    } else if is_action(keymap::Action::ToggleMenu) {
        app.side_menu.visible = false;
        app.state = AppState::QueryInput;
    }
}

pub fn handle_pane_key(app: &mut App<'_>, _state: &mut LoopState, key: KeyEvent, keymap: &Keymap) {
    let is_action = |a: keymap::Action| keymap.is_action(a, &key);

    if is_action(keymap::Action::NextPane) {
        app.next_pane();
    } else if is_action(keymap::Action::PrevPane) {
        app.prev_pane();
    } else if is_action(keymap::Action::ScrollDown) || matches!(key.code, KeyCode::Down) {
        let (scroll, pane_height, content_lines) = if matches!(app.state, AppState::LeftPane) {
            (
                &mut app.left_scroll,
                app.left_pane_height,
                app.left_content_lines,
            )
        } else {
            (
                &mut app.right_scroll,
                app.right_pane_height,
                app.right_content_lines,
            )
        };
        let max_scroll = App::max_scroll_offset(content_lines, pane_height);
        *scroll = scroll.saturating_add(1).min(max_scroll);
    } else if is_action(keymap::Action::ScrollUp) || matches!(key.code, KeyCode::Up) {
        if matches!(app.state, AppState::LeftPane) {
            app.left_scroll = app.left_scroll.saturating_sub(1);
        } else {
            app.right_scroll = app.right_scroll.saturating_sub(1);
        }
    } else if is_action(keymap::Action::ScrollPageDown) {
        let (scroll, pane_height, content_lines) = if matches!(app.state, AppState::LeftPane) {
            (
                &mut app.left_scroll,
                app.left_pane_height,
                app.left_content_lines,
            )
        } else {
            (
                &mut app.right_scroll,
                app.right_pane_height,
                app.right_content_lines,
            )
        };
        let max_scroll = App::max_scroll_offset(content_lines, pane_height);
        *scroll = scroll.saturating_add(pane_height).min(max_scroll);
    } else if is_action(keymap::Action::ScrollPageUp) {
        if matches!(app.state, AppState::LeftPane) {
            app.left_scroll = app.left_scroll.saturating_sub(app.left_pane_height);
        } else {
            app.right_scroll = app.right_scroll.saturating_sub(app.right_pane_height);
        }
    } else if is_action(keymap::Action::ScrollToTop) {
        if matches!(app.state, AppState::LeftPane) {
            app.left_scroll = 0;
        } else {
            app.right_scroll = 0;
        }
    } else if is_action(keymap::Action::ScrollToBottom) {
        if matches!(app.state, AppState::LeftPane) {
            app.left_scroll = app.max_left_scroll();
        } else {
            app.right_scroll = app.max_right_scroll();
        }
    } else if is_action(keymap::Action::ToggleQueryBar) {
        app.query_bar_visible = !app.query_bar_visible;
        if !app.query_bar_visible && matches!(app.state, AppState::QueryInput) {
            app.state = AppState::LeftPane;
        }
    } else if is_action(keymap::Action::ToggleMenu) {
        app.side_menu.visible = !app.side_menu.visible;
        if app.side_menu.visible {
            app.state = AppState::SideMenu;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jqpp::app::App;
    use jqpp::executor::Executor;
    use jqpp::keymap::Keymap;
    use jqpp::widgets;
    use ratatui::crossterm::event::{KeyCode, KeyEvent};
    use serde_json::json;

    #[test]
    fn test_esc_rolls_back_tab_expansion() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        let original_query = "startswith(\"A\"".to_string();
        let original_col = original_query.len();
        app.query_input.textarea = tui_textarea::TextArea::from(vec![original_query.clone()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, original_col as u16));

        let expanded_query = "startswith(\"Alice\"".to_string();
        let expanded_col = expanded_query.len();
        state
            .string_param_expansion_stack
            .push((original_query.clone(), original_col));
        app.query_input.textarea = tui_textarea::TextArea::from(vec![expanded_query.clone()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, expanded_col as u16));

        let esc_key = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], original_query);
        assert_eq!(app.query_input.textarea.cursor().1, original_col);
        assert!(state.string_param_expansion_stack.is_empty());
        assert!(state.debounce_pending);
    }

    #[test]
    fn test_text_edit_clears_expansion_stack() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        state
            .string_param_expansion_stack
            .push(("prev".to_string(), 0));

        let char_key = KeyEvent::new(
            KeyCode::Char('x'),
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, char_key, &keymap);

        assert!(state.string_param_expansion_stack.is_empty());
    }

    #[test]
    fn test_enter_applies_selected_suggestion_instead_of_committing_partial_input() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        // 1. Setup a query with an existing value and cursor inside
        // startswith("Alice")
        let full_query = "startswith(\"Alice\")".to_string();
        let cursor_col = 16; // Cursor between 'e' and '"' -> startswith("Alic|e")
        app.query_input.textarea = tui_textarea::TextArea::from(vec![full_query.clone()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, cursor_col as u16));

        // 2. Mock suggestions showing, with "Bob" selected at index 1
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Bob".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Bob\")".to_string(),
            },
        ];
        app.query_input.suggestion_index = 1; // "Bob"

        // 3. Trigger Enter (Submit)
        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        // 4. Verify result: should be "Bob", not "Alic"
        assert_eq!(app.query_input.textarea.lines()[0], "startswith(\"Bob\")");
    }

    #[test]
    fn test_enter_commits_partial_input_if_suggestions_hidden() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        // startswith("Alic|e")
        let full_query = "startswith(\"Alice\")".to_string();
        let cursor_col = 16;
        app.query_input.textarea = tui_textarea::TextArea::from(vec![full_query.clone()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, cursor_col as u16));

        // Suggestions are NOT showing
        app.query_input.show_suggestions = false;

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        // Should commit current input "Alic" -> startswith("Alic")
        assert_eq!(app.query_input.textarea.lines()[0], "startswith(\"Alic\")");
    }

    #[test]
    fn test_enter_applies_string_value_selection_when_dropdown_auto_closes() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        // Cursor is still inside the existing value.
        let full_query = "startswith(\"Alice\")".to_string();
        let cursor_col = 16; // startswith("Alic|e")
        app.query_input.textarea = tui_textarea::TextArea::from(vec![full_query]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, cursor_col as u16));

        // Suggestions remain available but dropdown is hidden.
        app.query_input.show_suggestions = false;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![
            widgets::query_input::Suggestion {
                label: "Alice".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Alice\")".to_string(),
            },
            widgets::query_input::Suggestion {
                label: "Bob".to_string(),
                detail: Some("string value".to_string()),
                insert_text: "startswith(\"Bob\")".to_string(),
            },
        ];
        app.query_input.suggestion_index = 1;

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "startswith(\"Bob\")");
    }

    #[test]
    fn test_hidden_non_string_suggestions_still_submit_current_query() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec![".items[".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, 7));

        app.query_input.show_suggestions = false;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "[]".to_string(),
            detail: None,
            insert_text: ".items[]".to_string(),
        }];

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        // Without a visible dropdown (and outside string-param context), Enter should submit as-is.
        assert_eq!(app.query_input.textarea.lines()[0], ".items[");
    }

    #[test]
    fn tab_on_contains_array_value_keeps_builder_open_with_comma() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea =
            tui_textarea::TextArea::from(vec!["contains([\"123\", ".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "foo".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"123\", \"foo\"".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let tab_key = KeyEvent::new(
            KeyCode::Tab,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            "contains([\"123\", \"foo\", "
        );
        assert!(app.query_input.show_suggestions);
        assert!(state.suggestion_active);
    }

    #[test]
    fn enter_on_contains_array_value_finalizes_and_closes_builder() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea =
            tui_textarea::TextArea::from(vec!["contains([\"123\", ".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "foo".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"123\", \"foo\"".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            "contains([\"123\", \"foo\"])"
        );
        assert!(!app.query_input.show_suggestions);
        assert!(!state.suggestion_active);
    }

    #[test]
    fn enter_on_contains_object_key_keeps_builder_open_for_value_selection() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea =
            tui_textarea::TextArea::from(vec![".orders[] | contains({".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "order_id".to_string(),
            detail: Some("contains object key".to_string()),
            insert_text: ".orders[] | contains({order_id: ".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            ".orders[] | contains({order_id: "
        );
        assert!(app.query_input.show_suggestions);
        assert!(state.suggestion_active);
    }

    #[test]
    fn esc_on_contains_array_builder_removes_trailing_comma_and_closes_array() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea =
            tui_textarea::TextArea::from(vec!["contains([\"123\", \"foo\", ".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "bar".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"123\", \"foo\", \"bar\"".to_string(),
        }];

        let esc_key = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            "contains([\"123\", \"foo\"])"
        );
        assert!(!app.query_input.show_suggestions);
        assert!(!state.suggestion_active);
    }

    #[test]
    fn esc_on_contains_object_builder_removes_trailing_comma_and_closes_object() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec![
            ".orders[] | contains({order_id: \"ORD-001\", ".to_string(),
        ]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "status".to_string(),
            detail: Some("contains object key".to_string()),
            insert_text: ".orders[] | contains({order_id: \"ORD-001\", status: ".to_string(),
        }];

        let esc_key = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            ".orders[] | contains({order_id: \"ORD-001\"})"
        );
        assert!(!app.query_input.show_suggestions);
        assert!(!state.suggestion_active);
        let q = &app.query_input.textarea.lines()[0];
        assert!(Executor::execute_query(q, &json!({"orders":[{"order_id":"ORD-001"}]})).is_ok());
    }

    #[test]
    fn enter_on_existing_contains_array_edit_finalizes_without_trailing_comma() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        let query = "contains([\"foo\", \"bar\", )".to_string();
        let cursor = "contains([\"foo\", \"bar\", ".chars().count();
        app.query_input.textarea = tui_textarea::TextArea::from(vec![query]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, cursor as u16));
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "bar baz".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"foo\", \"bar\", \"bar baz\"".to_string(),
        }];

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            "contains([\"foo\", \"bar\", \"bar baz\"])"
        );
        assert!(!app.query_input.textarea.lines()[0].ends_with(','));
    }

    #[test]
    fn tab_on_existing_contains_array_edit_replaces_value_and_starts_next() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        let query = "contains([\"foo\", \"bar\"])".to_string();
        let cursor = "contains([\"foo\", \"b".chars().count();
        app.query_input.textarea = tui_textarea::TextArea::from(vec![query]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, cursor as u16));
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "bar baz".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"foo\", \"bar baz\"".to_string(),
        }];

        let tab_key = KeyEvent::new(
            KeyCode::Tab,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(
            app.query_input.textarea.lines()[0],
            "contains([\"foo\", \"bar baz\", "
        );
        assert!(app.query_input.show_suggestions);
        assert!(state.suggestion_active);
    }

    #[test]
    fn esc_after_tab_on_existing_contains_array_edit_closes_valid_query() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea =
            tui_textarea::TextArea::from(vec!["contains([\"foo\", \"bar baz\", ".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "next".to_string(),
            detail: Some("contains array value".to_string()),
            insert_text: "contains([\"foo\", \"bar baz\", \"next\"".to_string(),
        }];

        let esc_key = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        let q = &app.query_input.textarea.lines()[0];
        assert_eq!(q, "contains([\"foo\", \"bar baz\"])");
        assert!(Executor::execute_query(q, &json!(["foo", "bar baz", "zzz"])).is_ok());
    }

    #[test]
    fn test_flatten_builder_tab_moves_to_end() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["flat".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "flatten".to_string(),
            detail: Some("flatten nested arrays".to_string()),
            insert_text: "flatten()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let tab_key = KeyEvent::new(
            KeyCode::Tab,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "flatten()");
        assert_eq!(app.query_input.textarea.cursor().1, 8); // inside ()
        assert!(app.query_input.show_suggestions);

        // Second tab moves to end and closes
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.cursor().1, 9);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_range_builder_tab_jumps_after_semicolon() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["ran".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let tab_key = KeyEvent::new(
            KeyCode::Tab,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range()");
        assert_eq!(app.query_input.textarea.cursor().1, 6); // inside ()

        // Second tab adds semicolon
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.lines()[0], "range(; )");
        assert_eq!(app.query_input.textarea.cursor().1, 8); // after "; "
        assert!(app.query_input.show_suggestions);

        // Third tab adds second semicolon
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.lines()[0], "range(; ; )");
        assert_eq!(app.query_input.textarea.cursor().1, 10);
        assert!(app.query_input.show_suggestions);

        // Fourth tab moves to end and closes
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.cursor().1, 11);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_range_builder_tab_adds_semicolon_if_missing() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0)".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, 8)); // after 0
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];

        let tab_key = KeyEvent::new(
            KeyCode::Tab,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range(0; )");
        assert_eq!(app.query_input.textarea.cursor().1, 9); // inside ()
        assert!(app.query_input.show_suggestions);
    }

    #[test]
    fn test_builder_enter_moves_inside_on_initial_acceptance() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["ran".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range()");
        assert_eq!(app.query_input.textarea.cursor().1, 6); // inside ()
        assert!(app.query_input.show_suggestions); // Keep active for parameters
    }

    #[test]
    fn test_builder_enter_finalizes_if_already_inside() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0; 10)".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, 6));
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];

        let enter_key = KeyEvent::new(
            KeyCode::Enter,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(app.query_input.textarea.cursor().1, 12); // end
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_builder_esc_moves_to_end_and_closes() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0; 10)".to_string()]);
        app.query_input
            .textarea
            .move_cursor(tui_textarea::CursorMove::Jump(0, 6)); // at 0
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;

        let esc_key = KeyEvent::new(
            KeyCode::Esc,
            ratatui::crossterm::event::KeyModifiers::empty(),
        );
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        assert_eq!(app.query_input.textarea.cursor().1, 12); // end
        assert!(!app.query_input.show_suggestions);
    }
}
