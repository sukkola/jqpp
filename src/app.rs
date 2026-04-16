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
                if self.query_bar_visible { AppState::QueryInput } else { AppState::LeftPane }
            }
            AppState::SideMenu => AppState::QueryInput,
        };
    }

    pub fn prev_pane(&mut self) {
        self.state = match self.state {
            AppState::QueryInput => AppState::RightPane,
            AppState::LeftPane => {
                if self.query_bar_visible { AppState::QueryInput } else { AppState::RightPane }
            }
            AppState::RightPane => AppState::LeftPane,
            AppState::SideMenu => AppState::QueryInput,
        };
    }
}
