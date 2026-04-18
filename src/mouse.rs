use jqpp::app::{App, AppState};
use ratatui::crossterm::event::{Event, MouseEventKind};
use std::time::Instant;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScrollPane {
    Left,
    Right,
}

pub fn row_in_pane(row: u16, pane_top: u16, pane_height: u16) -> bool {
    row >= pane_top && row < pane_top.saturating_add(pane_height)
}

pub fn mouse_in_left_pane(app: &App<'_>, column: u16, row: u16) -> bool {
    row_in_pane(row, app.left_pane_top, app.left_pane_height) && column <= app.left_scrollbar_col
}

pub fn mouse_in_right_pane(app: &App<'_>, column: u16, row: u16) -> bool {
    row_in_pane(row, app.right_pane_top, app.right_pane_height)
        && column > app.left_scrollbar_col
        && column <= app.right_scrollbar_col
}

pub fn focus_state_from_click(app: &mut App<'_>, column: u16, row: u16) {
    if app.query_bar_visible && row < 3 {
        app.state = AppState::QueryInput;
        return;
    }

    if app.side_menu.visible
        && column < 20
        && row_in_pane(row, app.left_pane_top, app.left_pane_height)
    {
        app.state = AppState::SideMenu;
        return;
    }

    if mouse_in_left_pane(app, column, row) {
        app.state = AppState::LeftPane;
    } else if mouse_in_right_pane(app, column, row) {
        app.state = AppState::RightPane;
    }
}

pub fn mouse_scroll_pane(app: &App<'_>, column: u16, row: u16) -> Option<ScrollPane> {
    if mouse_in_left_pane(app, column, row) {
        Some(ScrollPane::Left)
    } else if mouse_in_right_pane(app, column, row) {
        Some(ScrollPane::Right)
    } else {
        None
    }
}

pub fn mouse_scroll_direction(
    app: &App<'_>,
    mouse: &ratatui::crossterm::event::MouseEvent,
) -> Option<(ScrollPane, i8)> {
    let pane = mouse_scroll_pane(app, mouse.column, mouse.row)?;
    match mouse.kind {
        MouseEventKind::ScrollDown => Some((pane, 1)),
        MouseEventKind::ScrollUp => Some((pane, -1)),
        _ => None,
    }
}

pub fn can_scroll_in_direction(app: &App<'_>, pane: ScrollPane, dir: i8) -> bool {
    match (pane, dir.signum()) {
        (ScrollPane::Left, 1) => app.left_scroll < app.max_left_scroll(),
        (ScrollPane::Left, -1) => app.left_scroll > 0,
        (ScrollPane::Right, 1) => app.right_scroll < app.max_right_scroll(),
        (ScrollPane::Right, -1) => app.right_scroll > 0,
        _ => false,
    }
}

pub fn is_scroll_event(event: &Event) -> bool {
    matches!(
        event,
        Event::Mouse(mouse)
            if matches!(mouse.kind, MouseEventKind::ScrollDown | MouseEventKind::ScrollUp)
    )
}

pub fn scroll_input_suppressed(
    suppress_scroll_until: Option<Instant>,
    drop_scroll_backlog_until: Option<Instant>,
) -> bool {
    suppress_scroll_until
        .map(|until| Instant::now() <= until)
        .unwrap_or(false)
        || drop_scroll_backlog_until
            .map(|until| Instant::now() <= until)
            .unwrap_or(false)
}

pub fn should_drop_boundary_scroll_event(app: &App<'_>, event: &Event) -> bool {
    let Event::Mouse(mouse) = event else {
        return false;
    };

    match mouse.kind {
        MouseEventKind::ScrollDown => mouse_scroll_pane(app, mouse.column, mouse.row)
            .map(|pane| !can_scroll_in_direction(app, pane, 1))
            .unwrap_or(false),
        MouseEventKind::ScrollUp => mouse_scroll_pane(app, mouse.column, mouse.row)
            .map(|pane| !can_scroll_in_direction(app, pane, -1))
            .unwrap_or(false),
        _ => false,
    }
}

pub fn apply_mouse_scroll_delta(app: &mut App<'_>, pane: ScrollPane, delta: i16) -> bool {
    if delta == 0 {
        return false;
    }

    match pane {
        ScrollPane::Left => {
            let prev = app.left_scroll;
            if delta > 0 {
                app.left_scroll = app
                    .left_scroll
                    .saturating_add(delta as u16)
                    .min(app.max_left_scroll());
            } else {
                app.left_scroll = app.left_scroll.saturating_sub((-delta) as u16);
            }
            app.left_scroll != prev
        }
        ScrollPane::Right => {
            let prev = app.right_scroll;
            if delta > 0 {
                app.right_scroll = app
                    .right_scroll
                    .saturating_add(delta as u16)
                    .min(app.max_right_scroll());
            } else {
                app.right_scroll = app.right_scroll.saturating_sub((-delta) as u16);
            }
            app.right_scroll != prev
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jqpp::app::App;
    use ratatui::crossterm::event::{Event, KeyModifiers, MouseEventKind};

    #[test]
    fn boundary_scroll_events_are_dropped() {
        let mut app = App::new();
        app.left_content_lines = 50;
        app.left_pane_height = 10;
        app.left_pane_top = 4;
        app.left_scrollbar_col = 40;
        app.left_scroll = app.max_left_scroll();

        let evt = Event::Mouse(ratatui::crossterm::event::MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 5,
            row: 8,
            modifiers: KeyModifiers::empty(),
        });

        assert!(should_drop_boundary_scroll_event(&app, &evt));
    }
}
