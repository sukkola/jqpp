use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders, List, ListItem};

pub struct SideMenu {
    pub items: Vec<String>,
    pub selected: usize,
    pub visible: bool,
}

impl Default for SideMenu {
    fn default() -> Self {
        Self {
            items: vec![
                "Query".to_string(),
                "Config".to_string(),
                "Runs".to_string(),
                "Output".to_string(),
                "History".to_string(),
                "Saved".to_string(),
            ],
            selected: 0,
            visible: false,
        }
    }
}

impl SideMenu {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn draw_with_style(&self, frame: &mut Frame, area: Rect, style: Style) {
        if !self.visible {
            return;
        }
        let list_items: Vec<ListItem> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                if i == self.selected {
                    ListItem::new(format!("> {}", item))
                } else {
                    ListItem::new(format!("  {}", item))
                }
            })
            .collect();
        let list = List::new(list_items).block(
            Block::default()
                .title(" Menu ")
                .borders(Borders::ALL)
                .border_style(style),
        );
        frame.render_widget(list, area);
    }
}
