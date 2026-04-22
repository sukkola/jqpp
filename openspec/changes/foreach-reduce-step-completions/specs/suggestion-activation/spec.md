# suggestion-activation Delta Specification

## ADDED Requirements

### Requirement: Wizard mode is a distinct suggestion-active sub-state

The system SHALL recognise a `WizardActive` sub-state within suggestion mode, separate from normal `SuggestionActive`. In `WizardActive`, Tab/Enter/Esc are routed exclusively through wizard step logic rather than standard completion acceptance or dropdown cycling. All other keypresses exit wizard mode and restore normal suggestion behaviour.

#### Scenario: Tab in WizardActive advances wizard step, not cycling

- **WHEN** the suggestion dropdown is visible and wizard mode is active
- **AND** the user presses Tab
- **THEN** the current wizard slot is accepted and the next slot's suggestions are shown
- **AND** normal Tab cycling behaviour (cycling through candidates) does NOT occur

#### Scenario: Enter in WizardActive fast-forwards wizard, not standard accept

- **WHEN** wizard mode is active and the user presses Enter
- **THEN** the wizard applies defaults for all remaining slots and closes the clause
- **AND** normal Enter acceptance (accepting selected item and closing dropdown) does NOT occur

#### Scenario: Esc in WizardActive steps back, not closing dropdown

- **WHEN** wizard mode is active and the user presses Esc
- **THEN** the wizard reverts to the previous step's suggestions
- **AND** normal Esc behaviour (closing the suggestion dropdown) does NOT occur, unless wizard mode exits

#### Scenario: Any other keypress exits wizard mode

- **WHEN** wizard mode is active and the user presses any key other than Tab, Enter, or Esc
- **THEN** wizard mode is deactivated and the keypress is handled by normal input and suggestion logic
