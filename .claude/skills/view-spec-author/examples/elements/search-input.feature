@ELM-search-input
Feature: Search Input Element
  As a user
  I want a text input for entering search queries
  So that I can type my search terms and submit them via keyboard or trigger change-based search

  Background:
    Given the element "ELM-search-input" is rendered
    And the parent component "CMP-search-bar" is active

  # ---------------------------------------------------------------------------
  # Element type
  # ---------------------------------------------------------------------------

  Scenario: Search input element is of type input
    Then the element type should be "input"

  # ---------------------------------------------------------------------------
  # States
  # ---------------------------------------------------------------------------

  Scenario: Search input has correct default state styles
    When the element is in the "default" state
    Then the element border should be "1px solid #DADCE0"
    And the element border-radius should be 24
    And the element padding should be "12px 16px"
    And the element font-size should be 16
    And the element width should be "100%"

  Scenario: Search input has correct hover state styles
    When the element is in the "hover" state
    Then the element border-color should be "#B0B3B8"

  Scenario: Search input has correct focused state styles
    When the element is in the "focused" state
    Then the element border-color should be "#1A73E8"
    And the element box-shadow should be "0 0 0 2px rgba(26,115,232,0.2)"

  Scenario: Search input has correct disabled state styles
    When the element is in the "disabled" state
    Then the element background should be "#F1F3F4"
    And the element color should be "#80868B"
    And the element cursor should be "not-allowed"

  Scenario: Search input has correct error state styles
    When the element is in the "error" state
    Then the element border-color should be "#D93025"
    And the element box-shadow should be "0 0 0 2px rgba(217,48,37,0.2)"

  Scenario: Search input has correct loading state styles
    When the element is in the "loading" state
    Then the element background should be "#F8F9FA"
    And the element cursor should be "wait"

  Scenario Outline: Search input applies the correct cursor in the "<state>" state
    When the element is in the "<state>" state
    Then the element cursor should be "<cursor>"

    Examples:
      | state    | cursor      |
      | disabled | not-allowed |
      | loading  | wait        |

  # ---------------------------------------------------------------------------
  # Action triggers
  # ---------------------------------------------------------------------------

  Scenario: Submitting the input triggers the submit-search action
    When the user submits the element
    Then the action "ACT-submit-search" should be triggered

  Scenario: Changing the input triggers the submit-search action
    When the user changes the element value
    Then the action "ACT-submit-search" should be triggered

  Scenario: Search input defines exactly 2 action triggers
    Then the element should define 2 action triggers

  # ---------------------------------------------------------------------------
  # Store binding
  # ---------------------------------------------------------------------------

  Scenario: Search input is bound to the search store
    Then the element should be bound to store "STR-search-store"

  Scenario: Search input is bound to the query field
    Then the element store-binding field should be "query"

  # ---------------------------------------------------------------------------
  # Validation
  # ---------------------------------------------------------------------------

  Scenario: Search input enforces a max-length of 256 characters
    Then the element should have a "max-length" validation rule with value 256

  Scenario: Search input shows a validation message when max-length is exceeded
    When the input value exceeds 256 characters
    Then the validation message should be "Search query must be 256 characters or less"

  # ---------------------------------------------------------------------------
  # Accessibility
  # ---------------------------------------------------------------------------

  Scenario: Search input has the searchbox ARIA role
    Then the element ARIA role should be "searchbox"

  Scenario: Search input has an accessible label
    Then the element ARIA label should be "Search input"

  # ---------------------------------------------------------------------------
  # Keyboard interactions
  # ---------------------------------------------------------------------------

  Scenario: Pressing Enter on the input triggers the submit-search action
    When the user presses "Enter" on the element
    Then the action "ACT-submit-search" should be triggered

  Scenario: Pressing Escape on the input triggers the navigate action
    When the user presses "Escape" on the element
    Then the action "ACT-navigate" should be triggered

  Scenario: Search input defines exactly 2 keyboard shortcuts
    Then the element should define 2 keyboard shortcuts

  Scenario Outline: Pressing "<key>" triggers the correct action
    When the user presses "<key>" on the element
    Then the action "<action>" should be triggered

    Examples:
      | key    | action            |
      | Enter  | ACT-submit-search |
      | Escape | ACT-navigate      |
