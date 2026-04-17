use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use tui_textarea::TextArea;

#[derive(Clone)]
pub struct Suggestion {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
}

/// Number of items the dropdown can show at once (box height 12 - 1 bottom border).
pub const DROPDOWN_VISIBLE: usize = 11;

pub struct QueryInput<'a> {
    pub textarea: TextArea<'a>,
    pub history: Vec<String>,
    pub history_index: usize,
    pub suggestions: Vec<Suggestion>,
    pub suggestion_index: usize,
    pub suggestion_scroll: usize,
    pub show_suggestions: bool,
}

impl<'a> Default for QueryInput<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> QueryInput<'a> {
    pub fn new() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().title(" Query ").borders(Borders::ALL));
        textarea.set_cursor_line_style(Style::default());
        Self {
            textarea,
            history: Vec::new(),
            history_index: 0,
            suggestions: Vec::new(),
            suggestion_index: 0,
            suggestion_scroll: 0,
            show_suggestions: false,
        }
    }

    /// Keep `suggestion_scroll` such that `suggestion_index` is always visible.
    /// Call this after changing `suggestion_index` or rebuilding `suggestions`.
    pub fn clamp_scroll(&mut self) {
        let visible = DROPDOWN_VISIBLE.min(self.suggestions.len());
        if visible == 0 {
            self.suggestion_scroll = 0;
            return;
        }
        if self.suggestion_index < self.suggestion_scroll {
            self.suggestion_scroll = self.suggestion_index;
        } else if self.suggestion_index >= self.suggestion_scroll + visible {
            self.suggestion_scroll = self.suggestion_index + 1 - visible;
        }
    }

    pub fn push_history(&mut self, query: String) {
        if !query.is_empty() && self.history.last() != Some(&query) {
            self.history.push(query);
        }
        self.history_index = self.history.len();
    }

    pub fn history_up(&mut self) {
        if self.history_index > 0 {
            self.history_index -= 1;
            let query = self.history[self.history_index].clone();
            self.textarea = TextArea::from(vec![query]);
            self.textarea
                .set_block(Block::default().title(" Query ").borders(Borders::ALL));
            self.textarea.set_cursor_line_style(Style::default());
            self.textarea.move_cursor(tui_textarea::CursorMove::End);
        }
    }

    #[allow(dead_code)]
    pub fn history_down(&mut self) {
        if self.history_index + 1 < self.history.len() {
            self.history_index += 1;
            let query = self.history[self.history_index].clone();
            self.textarea = TextArea::from(vec![query]);
            self.textarea
                .set_block(Block::default().title(" Query ").borders(Borders::ALL));
            self.textarea.set_cursor_line_style(Style::default());
            self.textarea.move_cursor(tui_textarea::CursorMove::End);
        } else {
            self.history_index = self.history.len();
            let mut textarea = TextArea::default();
            textarea.set_block(Block::default().title(" Query ").borders(Borders::ALL));
            textarea.set_cursor_line_style(Style::default());
            self.textarea = textarea;
        }
    }

    fn ghost_text(&self) -> Option<String> {
        if self.show_suggestions && !self.suggestions.is_empty() {
            let current = &self.textarea.lines()[0];
            let insert = &self.suggestions[self.suggestion_index].insert_text;
            if insert.starts_with(current.as_str()) && insert.len() > current.len() {
                return Some(insert[current.len()..].to_string());
            }
        }
        None
    }

    pub fn draw(&mut self, frame: &mut Frame, area: Rect) {
        let ghost = self.ghost_text();
        if let Some(suffix) = ghost {
            let current = &self.textarea.lines()[0];
            let block = self.textarea.block().cloned().unwrap_or_default();
            let spans = vec![
                Span::raw(current.as_str()),
                Span::styled(suffix, Style::default().fg(Color::DarkGray)),
            ];
            let paragraph = Paragraph::new(Line::from(spans)).block(block);
            frame.render_widget(paragraph, area);
        } else {
            frame.render_widget(&self.textarea, area);
        }
    }

    /// Returns the Rect where the suggestion dropdown should be drawn, or None
    /// if suggestions are not active.  The caller is responsible for rendering
    /// the overlay (so it appears on top of all other widgets).
    /// Returns where to draw the suggestion list.
    ///
    /// The box is anchored to the cursor column and overlaps the query bar's
    /// bottom border row so the list appears to grow downward from the bar.
    /// Borders: left + right + bottom (no top — the query bar's bottom border
    /// serves as the visual top edge).
    pub fn suggestion_rect(&self, query_area: Rect, screen: Rect) -> Option<Rect> {
        if !self.show_suggestions || self.suggestions.is_empty() {
            return None;
        }
        // Cursor column inside the textarea content (0-based).
        let cursor_col = self.textarea.cursor().1 as u16;
        // +1 for the query bar's left border cell.
        let x = query_area.x + 1 + cursor_col;
        // Overlap the query bar's bottom border line by starting one row earlier.
        let y = query_area.y + query_area.height - 1;
        if y >= screen.height || x >= screen.width {
            return None;
        }
        let max_label = self
            .suggestions
            .iter()
            .map(|s| s.label.len() as u16)
            .max()
            .unwrap_or(8);
        let available_width = screen.width.saturating_sub(x);
        // +2 for left and right border columns.
        let width = (max_label + 2).min(available_width);
        let available_height = screen.height.saturating_sub(y);
        // +1 for the bottom border row; items fill the rest.
        let height = (self.suggestions.len() as u16 + 1)
            .min(12)
            .min(available_height);
        if width <= 2 || height <= 1 {
            return None;
        }
        Some(Rect {
            x,
            y,
            width,
            height,
        })
    }
}

// ── unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn make_qi_with_suggestions(query: &str, labels: &[&str]) -> QueryInput<'static> {
        let mut qi = QueryInput::new();
        qi.textarea = tui_textarea::TextArea::from(vec![query.to_string()]);
        qi.textarea.move_cursor(tui_textarea::CursorMove::End);
        qi.suggestions = labels
            .iter()
            .map(|l| Suggestion {
                label: l.to_string(),
                detail: None,
                insert_text: format!(".{}", l),
            })
            .collect();
        qi.show_suggestions = true;
        qi
    }

    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 24,
    };
    // Query bar occupies rows 0-2 (height=3, border top/bottom + 1 content row).
    const QUERY_AREA: Rect = Rect {
        x: 0,
        y: 0,
        width: 80,
        height: 3,
    };

    // ── suggestion_rect positioning ───────────────────────────────────────────

    #[test]
    fn dropdown_overlaps_query_bar_bottom_border() {
        // QUERY_AREA height=3 → bottom border is at row 2.
        // Dropdown must start at row 2 to cover it.
        let qi = make_qi_with_suggestions(".", &["foo"]);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        assert_eq!(
            r.y, 2,
            "dropdown must overlap the query bar's bottom border row"
        );
    }

    #[test]
    fn dropdown_x_anchored_to_cursor() {
        // cursor after "." → col 1; x = 0 + 1(border) + 1 = 2
        let qi = make_qi_with_suggestions(".", &["foo"]);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        assert_eq!(r.x, 2);
    }

    #[test]
    fn dropdown_x_tracks_longer_prefix() {
        // ".config." → cursor at col 8; x = 0 + 1 + 8 = 9
        let qi = make_qi_with_suggestions(".config.", &["label_rules"]);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        assert_eq!(r.x, 9);
    }

    #[test]
    fn dropdown_height_is_items_plus_bottom_border() {
        let qi = make_qi_with_suggestions(".", &["foo", "bar", "baz"]);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        // 3 items + 1 bottom border row
        assert_eq!(r.height, 4);
    }

    #[test]
    fn dropdown_width_includes_left_right_borders() {
        let qi = make_qi_with_suggestions(".", &["foo"]);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        // "foo".len()=3 + 2 (left+right borders)
        assert_eq!(r.width, 5);
    }

    #[test]
    fn dropdown_height_capped_at_12() {
        let labels: Vec<&str> = (0..20).map(|_| "x").collect();
        let qi = make_qi_with_suggestions(".", &labels);
        let r = qi.suggestion_rect(QUERY_AREA, SCREEN).unwrap();
        assert_eq!(r.height, 12);
    }

    #[test]
    fn dropdown_hidden_when_show_false() {
        let mut qi = make_qi_with_suggestions(".", &["foo"]);
        qi.show_suggestions = false;
        assert!(qi.suggestion_rect(QUERY_AREA, SCREEN).is_none());
    }

    #[test]
    fn dropdown_hidden_when_no_suggestions() {
        let mut qi = make_qi_with_suggestions(".", &[]);
        qi.show_suggestions = true;
        assert!(qi.suggestion_rect(QUERY_AREA, SCREEN).is_none());
    }

    #[test]
    fn dropdown_hidden_when_cursor_at_screen_edge() {
        let narrow = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 3,
        };
        let narrow_screen = Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 24,
        };
        // cursor at col 9 → x = 10 == width → no room
        let mut qi = QueryInput::new();
        qi.textarea = tui_textarea::TextArea::from(vec!["123456789".to_string()]);
        qi.textarea.move_cursor(tui_textarea::CursorMove::End);
        qi.suggestions = vec![Suggestion {
            label: "x".into(),
            detail: None,
            insert_text: ".x".into(),
        }];
        qi.show_suggestions = true;
        assert!(qi.suggestion_rect(narrow, narrow_screen).is_none());
    }

    // ── rendered output — no "Suggestions" heading ────────────────────────────

    fn render_suggestions(qi: &mut QueryInput, width: u16, height: u16) -> ratatui::buffer::Buffer {
        use ratatui::style::{Color, Modifier, Style};
        use ratatui::widgets::{Block, Borders, List, ListItem};

        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        let query_area = Rect {
            x: 0,
            y: 0,
            width,
            height: 3,
        };
        let qi_ptr = qi as *mut QueryInput;

        terminal
            .draw(move |frame| {
                let qi = unsafe { &mut *qi_ptr };
                qi.draw(frame, query_area);
                let screen = frame.area();
                if let Some(rect) = qi.suggestion_rect(query_area, screen) {
                    frame.render_widget(ratatui::widgets::Clear, rect);
                    let items: Vec<ListItem> = qi
                        .suggestions
                        .iter()
                        .enumerate()
                        .map(|(i, s)| {
                            let style = if i == 0 {
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
                    let mut state = ratatui::widgets::ListState::default();
                    state.select(Some(0));
                    let list = List::new(items).block(
                        Block::default().borders(Borders::LEFT | Borders::RIGHT | Borders::BOTTOM),
                    );
                    frame.render_stateful_widget(list, rect, &mut state);
                }
            })
            .unwrap();

        terminal.backend().buffer().clone()
    }

    /// The entire rendered buffer must not contain "Suggestions" anywhere.
    #[test]
    fn rendered_dropdown_has_no_suggestions_heading() {
        let mut qi = make_qi_with_suggestions(".", &["foo", "bar"]);
        let buf = render_suggestions(&mut qi, 80, 24);
        let mut all_text = String::new();
        for y in 0..24u16 {
            for x in 0..80u16 {
                all_text.push_str(buf.cell((x, y)).map(|c| c.symbol()).unwrap_or(" "));
            }
        }
        assert!(
            !all_text.contains("Suggestions"),
            "buffer must not contain a 'Suggestions' heading"
        );
    }

    // ── clamp_scroll / rolling window ─────────────────────────────────────────

    fn make_qi_scrollable(n: usize) -> QueryInput<'static> {
        let labels: Vec<String> = (0..n).map(|i| format!("item{i:02}")).collect();
        let label_refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        make_qi_with_suggestions(".", &label_refs)
    }

    #[test]
    fn scroll_starts_at_zero() {
        let qi = make_qi_scrollable(20);
        assert_eq!(qi.suggestion_scroll, 0);
    }

    #[test]
    fn scroll_stays_zero_while_index_in_view() {
        let mut qi = make_qi_scrollable(20);
        qi.suggestion_index = DROPDOWN_VISIBLE - 1; // last item still visible at offset 0
        qi.clamp_scroll();
        assert_eq!(qi.suggestion_scroll, 0);
    }

    #[test]
    fn scroll_advances_when_index_leaves_bottom() {
        let mut qi = make_qi_scrollable(20);
        qi.suggestion_index = DROPDOWN_VISIBLE; // one past the initial window
        qi.clamp_scroll();
        assert_eq!(qi.suggestion_scroll, 1);
    }

    #[test]
    fn scroll_tracks_index_deep_into_list() {
        let mut qi = make_qi_scrollable(20);
        qi.suggestion_index = 18;
        qi.clamp_scroll();
        // scroll must place item 18 at the last visible position
        assert_eq!(qi.suggestion_scroll, 18 + 1 - DROPDOWN_VISIBLE);
    }

    #[test]
    fn scroll_retreats_when_index_moves_above_window() {
        let mut qi = make_qi_scrollable(20);
        qi.suggestion_scroll = 10;
        qi.suggestion_index = 5; // above scroll offset
        qi.clamp_scroll();
        assert_eq!(qi.suggestion_scroll, 5);
    }

    #[test]
    fn scroll_zero_for_small_list() {
        let mut qi = make_qi_scrollable(3);
        qi.suggestion_index = 2;
        qi.clamp_scroll();
        assert_eq!(
            qi.suggestion_scroll, 0,
            "no scrolling needed for small lists"
        );
    }

    /// Items must start at x = border(1) + cursor_col + 1(left border of box).
    /// For ".config." cursor is at col 8 → box x=9 → item content at col 10.
    /// The overlap row (y=2) holds the first item; y=3 holds the second.
    #[test]
    fn rendered_dropdown_items_start_at_cursor_column() {
        let mut qi = make_qi_with_suggestions(".config.", &["label_rules", "other"]);
        let buf = render_suggestions(&mut qi, 80, 24);

        // y=2: overlap row (query bar bottom border covered by dropdown).
        // x=9 is the left border '│'; item content begins at x=10.
        let left_border = buf.cell((9, 2)).map(|c| c.symbol()).unwrap_or(" ");
        assert_eq!(left_border, "│", "left border must be at col 9, row 2");

        let first_char = buf.cell((10, 2)).map(|c| c.symbol()).unwrap_or(" ");
        assert_eq!(
            first_char, "l",
            "first char of 'label_rules' must be at col 10, row 2"
        );

        // Columns left of the dropdown on the overlap row keep query bar content
        // (not blank), but column 9+ must be the suggestion box — already checked above.
    }
}
