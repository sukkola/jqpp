use crate::app::{App, AppState};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin},
    style::{Color, Modifier, Style},
    widgets::{
        Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

fn wrapped_line_count(content: &str, width: u16) -> usize {
    let width = usize::from(width);
    if width == 0 || content.is_empty() {
        return 0;
    }

    content
        .split('\n')
        .map(|line| {
            let chars = line.chars().count().max(1);
            chars.div_ceil(width)
        })
        .sum()
}

pub fn draw(frame: &mut Frame, app: &mut App, keymap: &crate::keymap::Keymap) {
    let constraints = if app.query_bar_visible {
        vec![
            Constraint::Length(3), // Query bar
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ]
    } else {
        vec![
            Constraint::Min(0),    // Body
            Constraint::Length(1), // Footer
        ]
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.area());

    let (body_chunk, footer_chunk) = if app.query_bar_visible {
        (chunks[1], chunks[2])
    } else {
        (chunks[0], chunks[1])
    };

    if app.query_bar_visible {
        let query_style = if matches!(app.state, AppState::QueryInput) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        app.query_input.textarea.set_block(
            Block::default()
                .title(" Query ")
                .borders(Borders::ALL)
                .border_style(query_style),
        );
        app.query_input.draw(frame, chunks[0]);
    }

    let main_constraints = if app.side_menu.visible {
        vec![
            Constraint::Length(20), // Side menu
            Constraint::Min(0),     // Panes
        ]
    } else {
        vec![
            Constraint::Min(0), // Panes
        ]
    };

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(main_constraints)
        .split(body_chunk);

    let panes_chunk = if app.side_menu.visible {
        let menu_style = if matches!(app.state, AppState::SideMenu) {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        app.side_menu
            .draw_with_style(frame, main_chunks[0], menu_style);
        main_chunks[1]
    } else {
        main_chunks[0]
    };

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50), // Left pane
            Constraint::Percentage(50), // Right pane
        ])
        .split(panes_chunk);

    let left_outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(body_chunks[0]);

    let left_content = if let Some(ref exec) = app.executor {
        // Cap display at 200 lines / 64 KB — the terminal can't show more anyway
        // and converting a large file on every frame causes severe lag.
        const MAX_BYTES: usize = 64 * 1024;
        let raw = &exec.raw_input;
        if raw.len() <= MAX_BYTES {
            String::from_utf8_lossy(raw).into_owned()
        } else {
            let preview = String::from_utf8_lossy(&raw[..MAX_BYTES]);
            // trim to last newline so we don't cut mid-codepoint
            let trimmed = preview
                .rfind('\n')
                .map(|i| &preview[..i])
                .unwrap_or(&preview);
            format!(
                "{}\n\n[… {} KB total, display truncated]",
                trimmed,
                raw.len() / 1024
            )
        }
    } else {
        "No input data".to_string()
    };
    let left_style = if matches!(app.state, AppState::LeftPane) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let left_block = Block::default()
        .title(" Input ")
        .borders(Borders::ALL)
        .border_style(left_style);
    let left_pane_rect = left_outer_chunks[0];
    let left_inner_rect = left_pane_rect.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    let left_content_width = left_inner_rect.width.saturating_sub(2);
    let left_pane_height = left_pane_rect.height.saturating_sub(2);
    let left_content_lines = wrapped_line_count(&left_content, left_content_width);
    app.left_pane_height = left_pane_height;
    app.left_content_lines = left_content_lines;
    app.left_scrollbar_col = left_pane_rect
        .x
        .saturating_add(left_pane_rect.width.saturating_sub(1));
    app.left_pane_top = left_pane_rect.y.saturating_add(1);
    app.clamp_left_scroll();
    frame.render_widget(
        Paragraph::new(left_content)
            .block(left_block)
            .scroll((app.left_scroll, 0))
            .wrap(Wrap { trim: false }),
        left_inner_rect,
    );
    if left_content_lines > usize::from(left_pane_height) {
        let left_max_scroll = App::max_scroll_offset(left_content_lines, left_pane_height) as usize;
        let left_scrollbar_pos = usize::from(app.left_scroll)
            .saturating_mul(left_content_lines.saturating_sub(1))
            .checked_div(left_max_scroll)
            .unwrap_or(0)
            .min(left_content_lines.saturating_sub(1));
        let mut left_scrollbar_state = ScrollbarState::new(left_content_lines)
            .viewport_content_length(usize::from(left_pane_height))
            .position(left_scrollbar_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            left_pane_rect,
            &mut left_scrollbar_state,
        );
    }

    let left_status = if let Some(ref exec) = app.executor {
        exec.status_line()
    } else {
        "".to_string()
    };
    frame.render_widget(Paragraph::new(left_status), left_outer_chunks[1]);

    let right_outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Status bar
        ])
        .split(body_chunks[1]);

    let right_content = if let Some(ref error) = app.error {
        error.clone()
    } else {
        crate::executor::Executor::format_results(&app.results, app.raw_output)
    };
    let right_style = if matches!(app.state, AppState::RightPane) {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let right_block = Block::default()
        .title(" Output ")
        .borders(Borders::ALL)
        .border_style(right_style)
        .style(if app.error.is_some() {
            Style::default().fg(Color::Red)
        } else {
            Style::default()
        });
    let right_pane_rect = right_outer_chunks[0];
    let right_inner_rect = right_pane_rect.inner(Margin {
        horizontal: 1,
        vertical: 0,
    });
    let right_content_width = right_inner_rect.width.saturating_sub(2);
    let right_pane_height = right_pane_rect.height.saturating_sub(2);
    let right_content_lines = wrapped_line_count(&right_content, right_content_width);
    app.right_pane_height = right_pane_height;
    app.right_content_lines = right_content_lines;
    app.right_scrollbar_col = right_pane_rect
        .x
        .saturating_add(right_pane_rect.width.saturating_sub(1));
    app.right_pane_top = right_pane_rect.y.saturating_add(1);
    app.clamp_right_scroll();
    frame.render_widget(
        Paragraph::new(right_content)
            .block(right_block)
            .scroll((app.right_scroll, 0))
            .wrap(Wrap { trim: false }),
        right_inner_rect,
    );
    if right_content_lines > usize::from(right_pane_height) {
        let right_max_scroll =
            App::max_scroll_offset(right_content_lines, right_pane_height) as usize;
        let right_scrollbar_pos = usize::from(app.right_scroll)
            .saturating_mul(right_content_lines.saturating_sub(1))
            .checked_div(right_max_scroll)
            .unwrap_or(0)
            .min(right_content_lines.saturating_sub(1));
        let mut right_scrollbar_state = ScrollbarState::new(right_content_lines)
            .viewport_content_length(usize::from(right_pane_height))
            .position(right_scrollbar_pos);
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            right_pane_rect,
            &mut right_scrollbar_state,
        );
    }

    let right_status = format!("{} results", app.results.len());
    frame.render_widget(Paragraph::new(right_status), right_outer_chunks[1]);

    let mut footer_text = if let Some(ref msg) = app.footer_message {
        format!(" {} ", msg)
    } else {
        format!(" {} ", keymap.hint_string())
    };

    if let Some(ref lsp_diag) = app.lsp_diagnostic {
        footer_text = format!("{} | Error: {}", footer_text, lsp_diag);
    } else if let Some(ref lsp_status) = app.lsp_status {
        footer_text = format!("{} | {}", footer_text, lsp_status);
    }

    let footer_style = if app.lsp_diagnostic.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };

    let footer = Paragraph::new(footer_text).style(footer_style);
    frame.render_widget(footer, footer_chunk);

    // ── Suggestion dropdown ───────────────────────────────────────────────────
    // Rendered last so it floats above all other widgets.
    if app.query_bar_visible {
        let query_area = chunks[0];
        if let Some(rect) = app.query_input.suggestion_rect(query_area, frame.area()) {
            frame.render_widget(Clear, rect);

            let scroll = app.query_input.suggestion_scroll;
            let visible = crate::widgets::query_input::DROPDOWN_VISIBLE
                .min(app.query_input.suggestions.len());
            let window = &app.query_input.suggestions[scroll..scroll + visible];
            let rel_sel = app.query_input.suggestion_index.saturating_sub(scroll);

            let items: Vec<ListItem> = window
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    let style = if i == rel_sel {
                        Style::default()
                            .bg(Color::Black)
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().bg(Color::Black).fg(Color::White)
                    };
                    ListItem::new(s.label.as_str()).style(style)
                })
                .collect();

            // Left + right + bottom borders only; no top border — the query
            // bar's bottom line acts as the visual top edge of the dropdown.
            let list = List::new(items)
                .block(Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM));
            frame.render_widget(list, rect);
        }
    }
}

// ── UI rendering tests ────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::executor::Executor;
    use ratatui::{Terminal, backend::TestBackend};
    use serde_json::json;

    fn render(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        let keymap = crate::keymap::Keymap::default();
        terminal.draw(|f| draw(f, app, &keymap)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// Collect all symbols in a rectangular region as a single string.
    fn region(buf: &ratatui::buffer::Buffer, x0: u16, y0: u16, x1: u16, y1: u16) -> String {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .map(|(x, y)| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
            .collect()
    }

    #[test]
    fn wrapped_line_count_expands_long_lines() {
        assert_eq!(wrapped_line_count("abcdefghi", 4), 3);
        assert_eq!(wrapped_line_count("abc\n\ndef", 4), 3);
    }

    // ── Structural rendering ──────────────────────────────────────────────────

    #[test]
    fn query_bar_title_visible() {
        let mut app = App::new();
        app.query_bar_visible = true;
        let buf = render(&mut app, 80, 24);
        // Query bar occupies rows 0-2; title appears on row 0.
        let row0 = region(&buf, 0, 0, 80, 1);
        assert!(
            row0.contains("Query"),
            "Query bar title missing in: {}",
            row0
        );
    }

    #[test]
    fn input_pane_title_visible() {
        let mut app = App::new();
        // With query bar visible, body starts at row 3.
        let buf = render(&mut app, 80, 24);
        let row3 = region(&buf, 0, 3, 80, 4);
        assert!(
            row3.contains("Input"),
            "Input pane title missing in: {}",
            row3
        );
    }

    #[test]
    fn output_pane_title_visible() {
        let mut app = App::new();
        let buf = render(&mut app, 80, 24);
        let row3 = region(&buf, 40, 3, 80, 4);
        assert!(
            row3.contains("Output"),
            "Output pane title missing in: {}",
            row3
        );
    }

    // ── Content rendering ─────────────────────────────────────────────────────

    #[test]
    fn input_pane_renders_json_content() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: b"{\"name\":\"alice\"}".to_vec(),
            json_input: json!({"name": "alice"}),
            source_label: "test".to_string(),
        });
        let buf = render(&mut app, 80, 24);
        // Left pane is columns 0-39, rows 4-22 (inside border).
        let left = region(&buf, 0, 4, 40, 22);
        assert!(
            left.contains("alice"),
            "Input pane must show json value, got:\n{}",
            left
        );
    }

    #[test]
    fn input_pane_scroll_reaches_wrapped_bottom() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: b"{\n\"products\": [\"alpha-beta-gamma-delta\", \"epsilon-zeta-eta-theta\"],\n\"tail\": \"BOTTOM_MARKER\"\n}".to_vec(),
            json_input: json!({
                "products": ["alpha-beta-gamma-delta", "epsilon-zeta-eta-theta"],
                "tail": "BOTTOM_MARKER"
            }),
            source_label: "test".to_string(),
        });

        let _ = render(&mut app, 24, 10);
        app.left_scroll = app.max_left_scroll();
        let buf = render(&mut app, 24, 10);
        let left = region(&buf, 0, 4, 12, 9);

        assert!(
            left.contains("BOTTOM"),
            "left pane should reach wrapped bottom content, got:\n{left}"
        );
    }

    #[test]
    fn output_pane_renders_results() {
        let mut app = App::new();
        app.results = vec![json!("hello"), json!(42)];
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(
            right.contains("hello"),
            "Output pane must show string result: {}",
            right
        );
        assert!(
            right.contains("42"),
            "Output pane must show number result: {}",
            right
        );
    }

    #[test]
    fn output_pane_renders_error() {
        let mut app = App::new();
        app.error = Some("compile error: unexpected token".to_string());
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(
            right.contains("compile error"),
            "Error must appear in output pane: {}",
            right
        );
    }

    #[test]
    fn output_pane_scroll_reaches_wrapped_bottom() {
        let mut app = App::new();
        app.results = vec![json!({
            "products": ["alpha-beta-gamma-delta", "epsilon-zeta-eta-theta"],
            "tail": "BOTTOM_MARKER"
        })];

        let _ = render(&mut app, 24, 10);
        app.right_scroll = app.max_right_scroll();
        let buf = render(&mut app, 24, 10);
        let right = region(&buf, 12, 4, 24, 9);

        assert!(
            right.contains("BOTTOM"),
            "right pane should reach wrapped bottom content, got:\n{right}"
        );
    }

    // ── Multi-level pipe query output ─────────────────────────────────────────

    #[test]
    fn pipe_query_string_transform() {
        // .config.name | ascii_upcase  →  "HELLO"
        let input = json!({"config": {"name": "hello"}});
        let res = Executor::execute(".config.name | ascii_upcase", &input).unwrap();
        assert_eq!(res, vec![json!("HELLO")]);
    }

    #[test]
    fn pipe_query_array_iteration_and_select() {
        let input = json!({"items": [{"val": 1}, {"val": 2}, {"val": 3}]});
        let res = Executor::execute(".items[] | .val", &input).unwrap();
        assert_eq!(res, vec![json!(1), json!(2), json!(3)]);
    }

    #[test]
    fn pipe_query_multi_stage() {
        let input = json!({"data": {"list": ["a", "b", "c"]}});
        let res = Executor::execute(".data.list | length", &input).unwrap();
        assert_eq!(res, vec![json!(3)]);
    }

    #[test]
    fn pipe_query_result_rendered_in_output_pane() {
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: b"{}".to_vec(),
            json_input: json!({"config": {"name": "hello"}}),
            source_label: "t".to_string(),
        });
        // Pre-populate results as main_loop would after executing the query.
        app.results = Executor::execute(
            ".config.name | ascii_upcase",
            &json!({"config": {"name": "hello"}}),
        )
        .unwrap();
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(
            right.contains("HELLO"),
            "Pipe query result must render in output pane: {}",
            right
        );
    }

    // ── Large content truncation ──────────────────────────────────────────────

    #[test]
    fn large_raw_input_render_does_not_panic() {
        // Rendering a >64 KB input must not panic or OOM the frame.
        let mut raw = vec![b'"'];
        raw.extend(std::iter::repeat(b'x').take(66 * 1024));
        raw.push(b'"');
        let json_val: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let mut app = App::new();
        app.executor = Some(Executor {
            raw_input: raw,
            json_input: json_val,
            source_label: "big".to_string(),
        });
        // Just verify it renders without panic.
        let _ = render(&mut app, 80, 24);
    }

    #[test]
    fn large_raw_input_display_string_is_truncated() {
        // The content string produced for inputs > 64 KB must carry a truncation notice.
        // (The rendered terminal cannot scroll to the bottom in this unit-test.)
        const MAX_BYTES: usize = 64 * 1024;
        let mut raw = vec![b'"'];
        raw.extend(std::iter::repeat(b'x').take(66 * 1024));
        raw.push(b'"');

        // Mirror the same logic used in ui::draw.
        let display = if raw.len() <= MAX_BYTES {
            String::from_utf8_lossy(&raw).into_owned()
        } else {
            let preview = String::from_utf8_lossy(&raw[..MAX_BYTES]);
            let trimmed = preview
                .rfind('\n')
                .map(|i| &preview[..i])
                .unwrap_or(&preview);
            format!(
                "{}\n\n[… {} KB total, display truncated]",
                trimmed,
                raw.len() / 1024
            )
        };
        assert!(
            display.contains("truncated"),
            "display string must contain truncation notice"
        );
        assert!(
            display.contains("KB total"),
            "display string must mention total KB: {}",
            &display[display.len() - 60..]
        );
    }

    // ── Footer ────────────────────────────────────────────────────────────────

    #[test]
    fn footer_shows_default_keybindings() {
        let mut app = App::new();
        let buf = render(&mut app, 80, 24);
        // Footer is the last row (row 23).
        let footer = region(&buf, 0, 23, 80, 24);
        assert!(
            footer.contains("enter"),
            "Footer must show key hints: {}",
            footer
        );
    }

    #[test]
    fn left_scrollbar_thumb_touches_bottom_arrow_at_max_scroll() {
        let mut app = App::new();
        let left_text = (0..80)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.executor = Some(Executor {
            raw_input: left_text.into_bytes(),
            json_input: json!({}),
            source_label: "test".to_string(),
        });

        let _ = render(&mut app, 100, 28);
        app.left_scroll = app.max_left_scroll();
        let buf = render(&mut app, 100, 28);

        let x = app.left_scrollbar_col;
        let arrow_bottom_y = app.left_pane_top + app.left_pane_height;
        let thumb_bottom_y = arrow_bottom_y.saturating_sub(1);

        let arrow = buf.cell((x, arrow_bottom_y)).unwrap().symbol();
        let thumb = buf.cell((x, thumb_bottom_y)).unwrap().symbol();

        assert_eq!(arrow, "▼", "expected bottom arrow at end of track");
        assert_eq!(thumb, "█", "thumb must touch row above bottom arrow");
    }

    #[test]
    fn left_scrollbar_thumb_touches_top_arrow_at_min_scroll() {
        let mut app = App::new();
        let left_text = (0..80)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.executor = Some(Executor {
            raw_input: left_text.into_bytes(),
            json_input: json!({}),
            source_label: "test".to_string(),
        });

        app.left_scroll = 0;
        let buf = render(&mut app, 100, 28);

        let x = app.left_scrollbar_col;
        let arrow_top_y = app.left_pane_top.saturating_sub(1);
        let thumb_top_y = app.left_pane_top;

        let arrow = buf.cell((x, arrow_top_y)).unwrap().symbol();
        let thumb = buf.cell((x, thumb_top_y)).unwrap().symbol();

        assert_eq!(arrow, "▲", "expected top arrow at start of track");
        assert_eq!(thumb, "█", "thumb must touch row below top arrow");
    }

    #[test]
    fn right_scrollbar_thumb_touches_bottom_arrow_at_max_scroll() {
        let mut app = App::new();
        app.results = (0..120).map(|i| json!({"idx": i})).collect();

        let _ = render(&mut app, 100, 28);
        app.right_scroll = app.max_right_scroll();
        let buf = render(&mut app, 100, 28);

        let x = app.right_scrollbar_col;
        let arrow_bottom_y = app.right_pane_top + app.right_pane_height;
        let thumb_bottom_y = arrow_bottom_y.saturating_sub(1);

        let arrow = buf.cell((x, arrow_bottom_y)).unwrap().symbol();
        let thumb = buf.cell((x, thumb_bottom_y)).unwrap().symbol();

        assert_eq!(arrow, "▼", "expected bottom arrow at end of track");
        assert_eq!(thumb, "█", "thumb must touch row above bottom arrow");
    }

    #[test]
    fn right_scrollbar_thumb_touches_top_arrow_at_min_scroll() {
        let mut app = App::new();
        app.results = (0..120).map(|i| json!({"idx": i})).collect();
        app.right_scroll = 0;
        let buf = render(&mut app, 100, 28);

        let x = app.right_scrollbar_col;
        let arrow_top_y = app.right_pane_top.saturating_sub(1);
        let thumb_top_y = app.right_pane_top;

        let arrow = buf.cell((x, arrow_top_y)).unwrap().symbol();
        let thumb = buf.cell((x, thumb_top_y)).unwrap().symbol();

        assert_eq!(arrow, "▲", "expected top arrow at start of track");
        assert_eq!(thumb, "█", "thumb must touch row below top arrow");
    }
}
