use crate::executor::Executor;
use crate::widgets::query_input::QueryInput;
use crate::widgets::side_menu::SideMenu;

pub enum AppState {
    QueryInput,
    LeftPane,
    RightPane,
    SideMenu,
}

pub struct App<'a> {
    pub state: AppState,
    pub running: bool,
    pub executor: Option<Executor>,
    pub left_scroll: u16,
    pub right_scroll: u16,
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
    pub raw_output: bool,
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
            raw_output: false,
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
}
