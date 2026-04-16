use jqt::config;
use jqt::keymap::{parse_key_binding, Action, KeyBinding, Keymap};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use std::path::Path;

#[test]
fn test_config_remap_and_action_for() {
    let dir = std::env::temp_dir();
    let path = dir.join("remap.toml");
    // Remap Quit to F10
    fs::write(&path, "[keys]\nquit = \"F10\"").unwrap();

    let (keymap, err) = config::load_keymap(Some(&path));
    assert!(err.is_none());

    // F10 should now be Quit
    let event = KeyEvent::new(KeyCode::F(10), KeyModifiers::empty());
    assert_eq!(keymap.action_for(&event), Some(Action::Quit));

    // Ctrl+C should NO LONGER be Quit (it was overridden)
    let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(keymap.action_for(&event), None);

    let _ = fs::remove_file(path);
}

#[test]
fn test_config_conflict_returns_defaults() {
    let dir = std::env::temp_dir();
    let path = dir.join("conflict_integration.toml");
    // Remap Quit and SaveOutput to the same key
    fs::write(&path, "[keys]\nquit = \"Ctrl+s\"\nsave-output = \"Ctrl+s\"").unwrap();

    let (keymap, err) = config::load_keymap(Some(&path));
    assert!(err.is_some());
    assert!(err.unwrap().contains("Config conflict"));

    // Should be defaults: Ctrl+C is Quit
    let event = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
    assert_eq!(keymap.action_for(&event), Some(Action::Quit));

    let _ = fs::remove_file(path);
}
