use crate::accept::strip_sgr_mouse_sequences;
use crate::handlers::{handle_pane_key, handle_query_input_key, handle_side_menu_key};
use crate::hints::clear_dismissed_hint_if_query_changed;
use crate::mouse::{
    ScrollPane, apply_mouse_scroll_delta, focus_state_from_click, is_scroll_event,
    mouse_scroll_direction, mouse_scroll_pane, row_in_pane, scroll_input_suppressed,
    should_drop_boundary_scroll_event,
};
use crate::output::{copy_text_to_clipboard, right_pane_copy_text};
use jqpp::app::{App, AppState, DragTarget};
use jqpp::completions::CompletionItem;
use jqpp::keymap::Keymap;
use ratatui::Terminal;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use std::fs::File;
use std::io::Write;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

pub type ComputeResult = (
    anyhow::Result<(Vec<serde_json::Value>, bool)>,
    Option<String>,
);

pub struct LoopState {
    pub suggestion_active: bool,
    pub cached_pipe_type: Option<String>,
    pub lsp_completions: Vec<CompletionItem>,
    pub debounce_pending: bool,
    pub last_edit_at: Instant,
    pub last_esc_at: Option<Instant>,
    pub discard_sgr_mouse_until: Option<Instant>,
    pub suppress_scroll_until: Option<Instant>,
    pub drop_scroll_backlog_until: Option<Instant>,
    pub footer_message: Option<(String, Instant)>,
    pub compute_handle: Option<JoinHandle<ComputeResult>>,
    pub pending_qp: String,
    pub debounce_duration: Duration,
}

impl LoopState {
    pub fn new() -> Self {
        Self {
            suggestion_active: false,
            cached_pipe_type: None,
            lsp_completions: Vec::new(),
            debounce_pending: false,
            last_edit_at: Instant::now(),
            last_esc_at: None,
            discard_sgr_mouse_until: None,
            suppress_scroll_until: None,
            drop_scroll_backlog_until: None,
            footer_message: None,
            compute_handle: None,
            pending_qp: String::new(),
            debounce_duration: Duration::from_millis(50),
        }
    }

    pub async fn poll_and_process_events<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
        app: &mut App<'_>,
        keymap: &Keymap,
        key_log: &mut Option<File>,
    ) -> anyhow::Result<()> {
        const SCROLL_CIRCUIT_THRESHOLD: usize = 64;
        const SCROLL_CIRCUIT_MS: u64 = 120;
        const MAX_PENDING_EVENTS: usize = 256;
        const MAX_READ_EVENTS_NORMAL: usize = 512;
        const MAX_READ_EVENTS_WHILE_SUPPRESSED: usize = 4096;
        const BACKLOG_DROP_MS: u64 = 220;

        if self
            .suppress_scroll_until
            .map(|until| Instant::now() > until)
            .unwrap_or(false)
        {
            self.suppress_scroll_until = None;
        }
        if self
            .drop_scroll_backlog_until
            .map(|until| Instant::now() > until)
            .unwrap_or(false)
        {
            self.drop_scroll_backlog_until = None;
        }

        let now = Instant::now();
        let mut pending_events = Vec::with_capacity(48);
        let mut queued_left_scroll = app.left_scroll;
        let mut queued_right_scroll = app.right_scroll;
        let mut latest_scroll: Option<(ScrollPane, i16)> = None;
        let mut queued_scroll_events = 0usize;
        let mut drained_reads = 0usize;
        let read_budget = if scroll_input_suppressed(
            self.suppress_scroll_until,
            self.drop_scroll_backlog_until,
        ) {
            MAX_READ_EVENTS_WHILE_SUPPRESSED
        } else {
            MAX_READ_EVENTS_NORMAL
        };

        let first_event = match event::read() {
            Ok(evt) => evt,
            Err(_) => return Ok(()),
        };

        let mut queue_event = |evt: Event, pending_events: &mut Vec<Event>| {
            if is_scroll_event(&evt) {
                if scroll_input_suppressed(
                    self.suppress_scroll_until,
                    self.drop_scroll_backlog_until,
                ) {
                    return;
                }
                queued_scroll_events += 1;
                if queued_scroll_events > SCROLL_CIRCUIT_THRESHOLD {
                    self.suppress_scroll_until =
                        Some(now + Duration::from_millis(SCROLL_CIRCUIT_MS));
                    return;
                }
            }

            if let Event::Mouse(mouse) = &evt
                && let Some((pane, dir)) = mouse_scroll_direction(app, mouse)
            {
                let delta = if dir > 0 { 1 } else { -1 };
                latest_scroll = match latest_scroll {
                    Some((current_pane, current_delta)) if current_pane == pane => {
                        Some((pane, (current_delta + delta).clamp(-24, 24)))
                    }
                    _ => Some((pane, delta)),
                };

                let (virt_scroll, max_scroll) = match pane {
                    ScrollPane::Left => (&mut queued_left_scroll, app.max_left_scroll()),
                    ScrollPane::Right => (&mut queued_right_scroll, app.max_right_scroll()),
                };
                let can_scroll = if dir > 0 {
                    *virt_scroll < max_scroll
                } else {
                    *virt_scroll > 0
                };
                if !can_scroll {
                    return;
                }
                if dir > 0 {
                    *virt_scroll = virt_scroll.saturating_add(1).min(max_scroll);
                } else {
                    *virt_scroll = virt_scroll.saturating_sub(1);
                }

                return;
            }

            if should_drop_boundary_scroll_event(app, &evt) {
                return;
            }

            if pending_events.len() < MAX_PENDING_EVENTS {
                pending_events.push(evt);
            }
        };

        queue_event(first_event, &mut pending_events);
        drained_reads += 1;

        while event::poll(Duration::from_millis(0))? {
            if drained_reads >= read_budget {
                self.drop_scroll_backlog_until = Some(now + Duration::from_millis(BACKLOG_DROP_MS));
                break;
            }
            let evt = match event::read() {
                Ok(evt) => evt,
                Err(_) => break,
            };
            queue_event(evt, &mut pending_events);
            drained_reads += 1;
        }

        let scroll_boost: i16 = if pending_events.len() >= 96 {
            8
        } else if pending_events.len() >= 48 {
            4
        } else if pending_events.len() >= 16 {
            2
        } else {
            1
        };

        for event in pending_events {
            match event {
                Event::FocusGained => {
                    terminal.clear().ok();
                }
                Event::Key(key) => {
                    if matches!(app.state, AppState::QueryInput) {
                        if key.code == KeyCode::Esc {
                            self.discard_sgr_mouse_until =
                                Some(Instant::now() + Duration::from_millis(60));
                        } else if let Some(deadline) = self.discard_sgr_mouse_until {
                            if Instant::now() <= deadline
                                && let KeyCode::Char(c) = key.code
                                && matches!(c, '[' | '<' | ';' | 'M' | 'm' | '0'..='9')
                            {
                                if matches!(c, 'M' | 'm') {
                                    self.discard_sgr_mouse_until = None;
                                }
                                continue;
                            }
                            self.discard_sgr_mouse_until = None;
                        }
                    }

                    if let Some(log) = key_log {
                        let _ = writeln!(
                            log,
                            "key: {:?} mods: {:?} kind: {:?}",
                            key.code, key.modifiers, key.kind
                        );
                    }

                    let is_action = |a: jqpp::keymap::Action| keymap.is_action(a, &key);

                    let is_ctrl_quit = is_action(jqpp::keymap::Action::Quit)
                        || (key.modifiers.contains(KeyModifiers::CONTROL)
                            && matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q')));
                    let is_pane_quit = !matches!(app.state, AppState::QueryInput)
                        && (matches!(key.code, KeyCode::Char('q') | KeyCode::Char('Q'))
                            || key.code == KeyCode::Esc);

                    if is_ctrl_quit || is_pane_quit {
                        app.running = false;
                        continue;
                    }

                    let is_copy = is_action(jqpp::keymap::Action::CopyClipboard)
                        || (key.modifiers.contains(KeyModifiers::SUPER)
                            && key.code == KeyCode::Char('c'));
                    if is_copy {
                        let text = match app.state {
                            AppState::QueryInput => {
                                Some(app.query_input.textarea.lines()[0].clone())
                            }
                            AppState::LeftPane => app
                                .executor
                                .as_ref()
                                .map(|e| String::from_utf8_lossy(&e.raw_input).into_owned()),
                            AppState::RightPane => Some(right_pane_copy_text(app)),
                            AppState::SideMenu => None,
                        };
                        if let Some(t) = text {
                            copy_text_to_clipboard(t);
                            self.footer_message = Some(("copied".to_string(), Instant::now()));
                        }
                        continue;
                    }

                    match app.state {
                        AppState::QueryInput => {
                            handle_query_input_key(app, self, key, keymap);
                        }
                        AppState::SideMenu => {
                            handle_side_menu_key(app, self, key, keymap);
                        }
                        _ => {
                            handle_pane_key(app, self, key, keymap);
                        }
                    }
                }
                Event::Paste(text) => {
                    if matches!(app.state, AppState::QueryInput) {
                        let cleaned = strip_sgr_mouse_sequences(&text);
                        for ch in cleaned
                            .chars()
                            .filter(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t'))
                            .filter(|c| *c != '\n' && *c != '\r')
                        {
                            app.query_input.textarea.insert_char(ch);
                        }
                        app.query_input.show_suggestions = false;
                        self.suggestion_active = false;
                        app.structural_hint_active = false;
                        self.lsp_completions.clear();
                        self.cached_pipe_type = None;
                        let new_query = app.query_input.textarea.lines()[0].clone();
                        clear_dismissed_hint_if_query_changed(app, &new_query);
                        self.last_edit_at = Instant::now();
                        self.debounce_pending = true;
                    }
                }
                Event::Mouse(mouse) => match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        if scroll_input_suppressed(
                            self.suppress_scroll_until,
                            self.drop_scroll_backlog_until,
                        ) {
                            continue;
                        }
                        if let Some(pane) = mouse_scroll_pane(app, mouse.column, mouse.row)
                            && crate::mouse::can_scroll_in_direction(app, pane, 1)
                        {
                            let _ = apply_mouse_scroll_delta(app, pane, scroll_boost);
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if scroll_input_suppressed(
                            self.suppress_scroll_until,
                            self.drop_scroll_backlog_until,
                        ) {
                            continue;
                        }
                        if let Some(pane) = mouse_scroll_pane(app, mouse.column, mouse.row)
                            && crate::mouse::can_scroll_in_direction(app, pane, -1)
                        {
                            let _ = apply_mouse_scroll_delta(app, pane, -scroll_boost);
                        }
                    }
                    MouseEventKind::Down(ratatui::crossterm::event::MouseButton::Left) => {
                        focus_state_from_click(app, mouse.column, mouse.row);

                        if mouse.column == app.left_scrollbar_col
                            && row_in_pane(mouse.row, app.left_pane_top, app.left_pane_height)
                        {
                            app.left_scroll = App::scroll_offset_from_row(
                                mouse.row,
                                app.left_pane_top,
                                app.left_pane_height,
                                app.left_content_lines,
                            );
                            app.drag_target = Some(DragTarget::LeftScrollbar);
                        } else if mouse.column == app.right_scrollbar_col
                            && row_in_pane(mouse.row, app.right_pane_top, app.right_pane_height)
                        {
                            app.right_scroll = App::scroll_offset_from_row(
                                mouse.row,
                                app.right_pane_top,
                                app.right_pane_height,
                                app.right_content_lines,
                            );
                            app.drag_target = Some(DragTarget::RightScrollbar);
                        }
                    }
                    MouseEventKind::Drag(ratatui::crossterm::event::MouseButton::Left) => {
                        match app.drag_target {
                            Some(DragTarget::LeftScrollbar) => {
                                app.left_scroll = App::scroll_offset_from_row(
                                    mouse.row,
                                    app.left_pane_top,
                                    app.left_pane_height,
                                    app.left_content_lines,
                                );
                            }
                            Some(DragTarget::RightScrollbar) => {
                                app.right_scroll = App::scroll_offset_from_row(
                                    mouse.row,
                                    app.right_pane_top,
                                    app.right_pane_height,
                                    app.right_content_lines,
                                );
                            }
                            None => {}
                        }
                    }
                    MouseEventKind::Up(ratatui::crossterm::event::MouseButton::Left) => {
                        app.drag_target = None;
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        if !scroll_input_suppressed(self.suppress_scroll_until, self.drop_scroll_backlog_until)
            && let Some((pane, delta)) = latest_scroll
            && delta != 0
        {
            let _ = apply_mouse_scroll_delta(app, pane, delta);
        }

        Ok(())
    }
}
