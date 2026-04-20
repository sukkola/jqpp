    #[test]
    fn test_flatten_builder_tab_moves_to_end() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["flat".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "flatten".to_string(),
            detail: Some("flatten nested arrays".to_string()),
            insert_text: "flatten()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let tab_key = KeyEvent::new(KeyCode::Tab, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "flatten()");
        assert_eq!(app.query_input.textarea.cursor().1, 8); // inside ()
        assert!(app.query_input.show_suggestions);

        // Second tab moves to end and closes
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.cursor().1, 10);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_range_builder_tab_jumps_after_semicolon() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["ran".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let tab_key = KeyEvent::new(KeyCode::Tab, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range()");
        assert_eq!(app.query_input.textarea.cursor().1, 6); // inside ()

        // Second tab adds semicolon
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.lines()[0], "range(; )");
        assert_eq!(app.query_input.textarea.cursor().1, 8); // after "; "
        assert!(app.query_input.show_suggestions);

        // Third tab adds second semicolon
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.lines()[0], "range(; ; )");
        assert_eq!(app.query_input.textarea.cursor().1, 10);
        assert!(app.query_input.show_suggestions);

        // Fourth tab moves to end and closes
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);
        assert_eq!(app.query_input.textarea.cursor().1, 12);
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_range_builder_tab_adds_semicolon_if_missing() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0)".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, 8)); // after 0
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];

        let tab_key = KeyEvent::new(KeyCode::Tab, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, tab_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range(0; )");
        assert_eq!(app.query_input.textarea.cursor().1, 10);
        assert!(app.query_input.show_suggestions);
    }

    #[test]
    fn test_builder_enter_moves_inside_on_initial_acceptance() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["ran".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::End);
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];
        app.query_input.suggestion_index = 0;

        let enter_key = KeyEvent::new(KeyCode::Enter, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(app.query_input.textarea.lines()[0], "range()");
        assert_eq!(app.query_input.textarea.cursor().1, 6); // inside ()
        assert!(app.query_input.show_suggestions); // Keep active for parameters
    }

    #[test]
    fn test_builder_enter_finalizes_if_already_inside() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0; 10)".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, 6));
        app.query_input.show_suggestions = true;
        app.query_input.suggestions = vec![widgets::query_input::Suggestion {
            label: "range".to_string(),
            detail: Some("integer generator".to_string()),
            insert_text: "range()".to_string(),
        }];

        let enter_key = KeyEvent::new(KeyCode::Enter, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, enter_key, &keymap);

        assert_eq!(app.query_input.textarea.cursor().1, 12); // end
        assert!(!app.query_input.show_suggestions);
    }

    #[test]
    fn test_builder_esc_moves_to_end_and_closes() {
        let mut app = App::new();
        let mut state = LoopState::new();
        let keymap = Keymap::default();

        app.query_input.textarea = tui_textarea::TextArea::from(vec!["range(0; 10)".to_string()]);
        app.query_input.textarea.move_cursor(tui_textarea::CursorMove::Jump(0, 6)); // at 0
        app.query_input.show_suggestions = true;
        state.suggestion_active = true;

        let esc_key = KeyEvent::new(KeyCode::Esc, ratatui::crossterm::event::KeyModifiers::empty());
        handle_query_input_key(&mut app, &mut state, esc_key, &keymap);

        assert_eq!(app.query_input.textarea.cursor().1, 12); // end
        assert!(!app.query_input.show_suggestions);
    }
}
