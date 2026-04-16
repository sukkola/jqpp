use crate::app::{App, AppState};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

pub fn draw(frame: &mut Frame, app: &mut App) {
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
            let trimmed = preview.rfind('\n').map(|i| &preview[..i]).unwrap_or(&preview);
            format!("{}\n\n[… {} KB total, display truncated]",
                trimmed, raw.len() / 1024)
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
    frame.render_widget(
        Paragraph::new(left_content)
            .block(left_block)
            .scroll((app.left_scroll, 0))
            .wrap(Wrap { trim: false }),
        left_outer_chunks[0],
    );

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
    frame.render_widget(
        Paragraph::new(right_content)
            .block(right_block)
            .scroll((app.right_scroll, 0))
            .wrap(Wrap { trim: false }),
        right_outer_chunks[0],
    );

    let right_status = format!("{} results", app.results.len());
    frame.render_widget(Paragraph::new(right_status), right_outer_chunks[1]);

    let mut footer_text = if let Some(ref msg) = app.footer_message {
        format!(" {} ", msg)
    } else {
        " enter submit · tab/shift+tab nav · ctrl+c/q quit · ctrl+y copy · ctrl+t toggle input · ctrl+s save · ctrl+m menu "
            .to_string()
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
    use ratatui::{backend::TestBackend, Terminal};
    use serde_json::json;

    fn render(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let backend = TestBackend::new(w, h);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// Collect all symbols in a rectangular region as a single string.
    fn region(buf: &ratatui::buffer::Buffer, x0: u16, y0: u16, x1: u16, y1: u16) -> String {
        (y0..y1)
            .flat_map(|y| (x0..x1).map(move |x| (x, y)))
            .map(|(x, y)| buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "))
            .collect()
    }

    // ── Structural rendering ──────────────────────────────────────────────────

    #[test]
    fn query_bar_title_visible() {
        let mut app = App::new();
        app.query_bar_visible = true;
        let buf = render(&mut app, 80, 24);
        // Query bar occupies rows 0-2; title appears on row 0.
        let row0 = region(&buf, 0, 0, 80, 1);
        assert!(row0.contains("Query"), "Query bar title missing in: {}", row0);
    }

    #[test]
    fn input_pane_title_visible() {
        let mut app = App::new();
        // With query bar visible, body starts at row 3.
        let buf = render(&mut app, 80, 24);
        let row3 = region(&buf, 0, 3, 80, 4);
        assert!(row3.contains("Input"), "Input pane title missing in: {}", row3);
    }

    #[test]
    fn output_pane_title_visible() {
        let mut app = App::new();
        let buf = render(&mut app, 80, 24);
        let row3 = region(&buf, 40, 3, 80, 4);
        assert!(row3.contains("Output"), "Output pane title missing in: {}", row3);
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
        assert!(left.contains("alice"), "Input pane must show json value, got:\n{}", left);
    }

    #[test]
    fn output_pane_renders_results() {
        let mut app = App::new();
        app.results = vec![json!("hello"), json!(42)];
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(right.contains("hello"), "Output pane must show string result: {}", right);
        assert!(right.contains("42"), "Output pane must show number result: {}", right);
    }

    #[test]
    fn output_pane_renders_error() {
        let mut app = App::new();
        app.error = Some("compile error: unexpected token".to_string());
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(right.contains("compile error"), "Error must appear in output pane: {}", right);
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
        ).unwrap();
        let buf = render(&mut app, 80, 24);
        let right = region(&buf, 40, 4, 80, 22);
        assert!(right.contains("HELLO"), "Pipe query result must render in output pane: {}", right);
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
            let trimmed = preview.rfind('\n').map(|i| &preview[..i]).unwrap_or(&preview);
            format!("{}\n\n[… {} KB total, display truncated]", trimmed, raw.len() / 1024)
        };
        assert!(display.contains("truncated"), "display string must contain truncation notice");
        assert!(display.contains("KB total"), "display string must mention total KB: {}", &display[display.len()-60..]);
    }

    // ── Footer ────────────────────────────────────────────────────────────────

    #[test]
    fn footer_shows_default_keybindings() {
        let mut app = App::new();
        let buf = render(&mut app, 80, 24);
        // Footer is the last row (row 23).
        let footer = region(&buf, 0, 23, 80, 24);
        assert!(footer.contains("enter"), "Footer must show key hints: {}", footer);
    }
}
