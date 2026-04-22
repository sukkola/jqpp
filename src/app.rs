use crate::executor::Executor;
use crate::widgets::query_input::{QueryInput, Suggestion};
use crate::widgets::side_menu::SideMenu;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardKeyword {
    Foreach,
    Reduce,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardStep {
    Keyword,
    Stream,
    StreamSubArg { idx: usize },
    BindKeyword,
    VarName,
    Init,
    UpdateAccum,
    UpdateOp,
    Extract,
}

#[derive(Debug, Clone)]
pub struct WizardFrame {
    pub step: WizardStep,
    pub saved_query: String,
    pub saved_cursor: usize,
    pub saved_suggestions: Vec<Suggestion>,
}

#[derive(Debug, Clone)]
pub struct WizardState {
    pub keyword: WizardKeyword,
    pub stack: Vec<WizardFrame>,
    pub var_name: String,
}

pub enum AppState {
    QueryInput,
    LeftPane,
    RightPane,
    SideMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragTarget {
    LeftScrollbar,
    RightScrollbar,
}

pub struct App<'a> {
    pub state: AppState,
    pub running: bool,
    pub executor: Option<Executor>,
    pub left_scroll: u16,
    pub right_scroll: u16,
    pub left_pane_height: u16,
    pub right_pane_height: u16,
    pub left_content_lines: usize,
    pub right_content_lines: usize,
    pub left_scrollbar_col: u16,
    pub right_scrollbar_col: u16,
    pub left_pane_top: u16,
    pub right_pane_top: u16,
    pub drag_target: Option<DragTarget>,
    pub results: Vec<serde_json::Value>,
    pub error: Option<String>,
    pub query_bar_visible: bool,
    pub query_input: QueryInput<'a>,
    pub side_menu: SideMenu,
    pub footer_message: Option<String>,
    pub footer_message_at: Option<std::time::Instant>,
    pub lsp_status: Option<String>,
    pub lsp_diagnostic: Option<String>,
    pub lsp_enabled: bool,
    pub structural_hint_active: bool,
    pub dismissed_hint_query: Option<String>,
    pub raw_output: bool,
    pub wizard_state: Option<WizardState>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            state: AppState::QueryInput,
            running: true,
            executor: None,
            left_scroll: 0,
            right_scroll: 0,
            left_pane_height: 20,
            right_pane_height: 20,
            left_content_lines: 0,
            right_content_lines: 0,
            left_scrollbar_col: 0,
            right_scrollbar_col: 0,
            left_pane_top: 0,
            right_pane_top: 0,
            drag_target: None,
            results: Vec::new(),
            error: None,
            query_bar_visible: true,
            query_input: QueryInput::new(),
            side_menu: SideMenu::new(),
            footer_message: None,
            footer_message_at: None,
            lsp_status: None,
            lsp_diagnostic: None,
            lsp_enabled: false,
            structural_hint_active: false,
            dismissed_hint_query: None,
            raw_output: false,
            wizard_state: None,
        }
    }

    pub fn next_pane(&mut self) {
        self.state = match self.state {
            AppState::QueryInput => AppState::LeftPane,
            AppState::LeftPane => AppState::RightPane,
            AppState::RightPane => {
                if self.query_bar_visible {
                    AppState::QueryInput
                } else {
                    AppState::LeftPane
                }
            }
            AppState::SideMenu => AppState::QueryInput,
        };
    }

    pub fn prev_pane(&mut self) {
        self.state = match self.state {
            AppState::QueryInput => AppState::RightPane,
            AppState::LeftPane => {
                if self.query_bar_visible {
                    AppState::QueryInput
                } else {
                    AppState::RightPane
                }
            }
            AppState::RightPane => AppState::LeftPane,
            AppState::SideMenu => AppState::QueryInput,
        };
    }

    pub fn max_scroll_offset(content_lines: usize, pane_height: u16) -> u16 {
        content_lines.saturating_sub(usize::from(pane_height)) as u16
    }

    pub fn max_left_scroll(&self) -> u16 {
        Self::max_scroll_offset(self.left_content_lines, self.left_pane_height)
    }

    pub fn max_right_scroll(&self) -> u16 {
        Self::max_scroll_offset(self.right_content_lines, self.right_pane_height)
    }

    pub fn clamp_left_scroll(&mut self) {
        self.left_scroll = self.left_scroll.min(self.max_left_scroll());
    }

    pub fn clamp_right_scroll(&mut self) {
        self.right_scroll = self.right_scroll.min(self.max_right_scroll());
    }

    pub fn scroll_offset_from_row(
        row: u16,
        pane_top: u16,
        pane_height: u16,
        content_lines: usize,
    ) -> u16 {
        if pane_height == 0 || content_lines == 0 {
            return 0;
        }

        let max = Self::max_scroll_offset(content_lines, pane_height);
        if pane_height <= 1 || max == 0 {
            return 0;
        }

        let relative_row = row
            .saturating_sub(pane_top)
            .min(pane_height.saturating_sub(1));
        let proportional =
            (u32::from(relative_row) * u32::from(max)) / u32::from(pane_height.saturating_sub(1));
        (proportional as u16).min(max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_scroll_starts_at_zero() {
        let app = App::new();
        assert_eq!(app.right_scroll, 0);
        assert_eq!(app.left_scroll, 0);
    }

    #[test]
    fn right_scroll_increments_in_right_pane() {
        let mut app = App::new();
        app.state = AppState::RightPane;
        // Simulate the scroll-down logic
        app.right_scroll += 1;
        app.right_scroll += 1;
        assert_eq!(app.right_scroll, 2);
        // Simulate scroll-up
        app.right_scroll = app.right_scroll.saturating_sub(1);
        assert_eq!(app.right_scroll, 1);
    }

    #[test]
    fn left_scroll_increments_in_left_pane() {
        let mut app = App::new();
        app.state = AppState::LeftPane;
        app.left_scroll += 1;
        assert_eq!(app.left_scroll, 1);
        app.left_scroll = app.left_scroll.saturating_sub(1);
        assert_eq!(app.left_scroll, 0);
        // saturating_sub never underflows
        app.left_scroll = app.left_scroll.saturating_sub(1);
        assert_eq!(app.left_scroll, 0);
    }

    #[test]
    fn right_scroll_resets_to_zero_on_new_results() {
        let mut app = App::new();
        app.right_scroll = 5;
        // Simulate what the main loop does when new results arrive
        app.results = vec![serde_json::json!("line1"), serde_json::json!("line2")];
        app.right_scroll = 0;
        assert_eq!(
            app.right_scroll, 0,
            "right_scroll must reset to 0 when new results are set"
        );
    }

    #[test]
    fn right_scroll_not_reset_when_error_occurs() {
        // When jaq returns an error the scroll position is intentionally
        // preserved (user might want to read the partial output that was
        // already on screen). right_scroll is only cleared on Ok results.
        let mut app = App::new();
        app.right_scroll = 3;
        app.error = Some("parse error".to_string());
        // No right_scroll reset — it stays at 3
        assert_eq!(app.right_scroll, 3);
    }

    #[test]
    fn scroll_page_down_clamps_to_max() {
        let mut app = App::new();
        app.right_pane_height = 10;
        app.right_content_lines = 23;
        let max = app.max_right_scroll();

        app.right_scroll = app
            .right_scroll
            .saturating_add(app.right_pane_height)
            .min(max);
        app.right_scroll = app
            .right_scroll
            .saturating_add(app.right_pane_height)
            .min(max);
        app.right_scroll = app
            .right_scroll
            .saturating_add(app.right_pane_height)
            .min(max);

        assert_eq!(app.right_scroll, 13);
        assert_eq!(app.right_scroll, max);
    }

    #[test]
    fn scroll_to_top_sets_zero() {
        let mut app = App::new();
        app.left_scroll = 8;

        app.left_scroll = 0;

        assert_eq!(app.left_scroll, 0);
    }

    #[test]
    fn scroll_to_bottom_uses_max_offset() {
        let mut app = App::new();
        app.right_pane_height = 7;
        app.right_content_lines = 30;

        app.right_scroll = app.max_right_scroll();

        assert_eq!(app.right_scroll, 23);
    }

    #[test]
    fn max_scroll_uses_content_lines_and_pane_height() {
        let mut app = App::new();
        app.right_content_lines = 17;
        app.right_pane_height = 5;

        assert_eq!(app.max_right_scroll(), 12);
    }

    #[test]
    fn scroll_offset_from_row_reaches_max_at_bottom_row() {
        let pane_height = 10;
        let content_lines = 40;
        let pane_top = 4;
        let bottom_row = pane_top + pane_height - 1;

        let offset = App::scroll_offset_from_row(bottom_row, pane_top, pane_height, content_lines);

        assert_eq!(offset, App::max_scroll_offset(content_lines, pane_height));
    }
}
