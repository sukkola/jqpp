## Why

The input and output panes currently only support `j`/`k` and arrow-key scrolling with no visible scroll indicator or rich keyboard navigation. Users inspecting large JSON blobs or long query outputs have no efficient way to jump to the beginning/end of content and no visual feedback on where they are in the document.

## What Changes

- Add Page Up / Page Down keyboard scrolling in the input and output panes (scroll by viewport height)
- Add Home / End keyboard shortcuts to jump to the first and last line of a pane's content
- Render a scrollbar on the right edge of the input and output panes (visible only when content overflows)
- Support click-to-position and drag scrolling on the scrollbar track/thumb
- Support mouse wheel and trackpad scroll events when the cursor is inside a pane
- Document all new keybindings in the README keybindings table

## Capabilities

### New Capabilities

- `pane-scrollbar`: Visual scrollbar widget rendered on each content pane, supporting click, drag, and position feedback
- `pane-keyboard-nav`: Page Up/Down and Home/End keybindings for fast vertical navigation inside panes

### Modified Capabilities

- `keybindings-config`: `PageUp`, `PageDown`, `Home`, `End` added as remappable actions to the keymap system

## Impact

- `src/app.rs`: no structural changes; `left_scroll`/`right_scroll` fields already exist
- `src/keymap.rs`: four new `Action` variants (`PageUp`, `PageDown`, `ScrollToTop`, `ScrollToBottom`) with default bindings
- `src/ui.rs`: scrollbar rendering alongside input/output `Paragraph` widgets; pane height needed to compute thumb position
- `src/main.rs`: handle new key actions and drag mouse events; pass pane height from layout into event handling
- `README.md`: keybindings table updated with new entries
- No new dependencies — ratatui's `Scrollbar` widget covers the rendering
