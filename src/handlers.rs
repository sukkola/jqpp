use crate::accept::{
    apply_selected_suggestion, commit_current_string_param_input, cursor_col_after_accept,
    expand_string_param_prefix_with_tab, starts_context_aware_function_call,
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
        if app.query_input.show_suggestions && !app.query_input.suggestions.is_empty() {
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

            let selected = app.query_input.suggestions[app.query_input.suggestion_index].clone();
            let suggestion = selected.insert_text;
            let (new_text, col) =
                apply_selected_suggestion(&suggestion, selected.detail.as_deref(), &full, cur);
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
            state.suggestion_active = starts_context_aware_function_call(&suggestion);
            state.lsp_completions.clear();
            state.cached_pipe_type = None;
            state.last_edit_at = Instant::now() - state.debounce_duration;
            state.debounce_pending = true;
        } else {
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
            let (new_text, col) =
                apply_selected_suggestion(&suggestion, selected.detail.as_deref(), &full, cur);
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
            state.suggestion_active = starts_context_aware_function_call(&suggestion);
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
        let query_prefix = current_query_prefix(app);
        if app.structural_hint_active {
            dismiss_structural_hint(app, &query_prefix);
            state.suggestion_active = false;
            state.last_esc_at = Some(Instant::now());
        } else if app.query_input.show_suggestions {
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
