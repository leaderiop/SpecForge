@ELM-search-button
Feature: Search Button Element
  As a user
  I want a button to submit my search query
  So that I can trigger a search by clicking or pressing Enter/Space on the button

  Background:
    Given the element "ELM-search-button" is rendered
    And the parent component "CMP-search-bar" is active

  # ---------------------------------------------------------------------------
  # Element type
  # ---------------------------------------------------------------------------

  Scenario: Search button element is of type button
    Then the element type should be "button"

  # ---------------------------------------------------------------------------
  # States
  # ---------------------------------------------------------------------------

  Scenario: Search button has correct default state styles
    When the element is in the "default" state
    Then the element background should be "#1A73E8"
    And the element color should be "#FFFFFF"
    And the element border should be "none"
    And the element border-radius should be 24
    And the element padding should be "12px 24px"
    And the element font-size should be 14
    And the element font-weight should be 600
    And the element cursor should be "pointer"

  Scenario: Search button has correct hover state styles
    When the element is in the "hover" state
    Then the element background should be "#1557B0"

  Scenario: Search button has correct active state styles
    When the element is in the "active" state
    Then the element background should be "#174EA6"

  Scenario: Search button has correct disabled state styles
    When the element is in the "disabled" state
    Then the element background should be "#DADCE0"
    And the element color should be "#80868B"
    And the element cursor should be "not-allowed"

  Scenario: Search button has correct loading state styles
    When the element is in the "loading" state
    Then the element background should be "#1A73E8"
    And the element opacity should be 0.7
    And the element cursor should be "wait"

  Scenario Outline: Search button applies the correct styles in the "<state>" state
    When the element is in the "<state>" state
    Then the element background should be "<background>"

    Examples:
      | state    | background |
      | default  | #1A73E8    |
      | hover    | #1557B0    |
      | active   | #174EA6    |
      | disabled | #DADCE0    |
      | loading  | #1A73E8    |

  Scenario Outline: Search button applies the correct cursor in the "<state>" state
    When the element is in the "<state>" state
    Then the element cursor should be "<cursor>"

    Examples:
      | state    | cursor      |
      | default  | pointer     |
      | disabled | not-allowed |
      | loading  | wait        |

  # ---------------------------------------------------------------------------
  # Action triggers
  # ---------------------------------------------------------------------------

  Scenario: Clicking the search button triggers the submit-search action
    When the user clicks the element
    Then the action "ACT-submit-search" should be triggered

  Scenario: Search button defines exactly 1 action trigger
    Then the element should define 1 action trigger

  # ---------------------------------------------------------------------------
  # Store binding
  # ---------------------------------------------------------------------------

  Scenario: Search button is bound to the search store
    Then the element should be bound to store "STR-search-store"

  Scenario: Search button is bound to the isLoading field
    Then the element store-binding field should be "isLoading"

  # ---------------------------------------------------------------------------
  # Accessibility
  # ---------------------------------------------------------------------------

  Scenario: Search button has the button ARIA role
    Then the element ARIA role should be "button"

  Scenario: Search button has an accessible label
    Then the element ARIA label should be "Submit search"

  # ---------------------------------------------------------------------------
  # Keyboard interactions
  # ---------------------------------------------------------------------------

  Scenario: Pressing Enter on the button triggers the submit-search action
    When the user presses "Enter" on the element
    Then the action "ACT-submit-search" should be triggered

  Scenario: Pressing Space on the button triggers the submit-search action
    When the user presses "Space" on the element
    Then the action "ACT-submit-search" should be triggered

  Scenario: Search button defines exactly 2 keyboard shortcuts
    Then the element should define 2 keyboard shortcuts

  Scenario Outline: Pressing "<key>" triggers the submit-search action
    When the user presses "<key>" on the element
    Then the action "ACT-submit-search" should be triggered

    Examples:
      | key   |
      | Enter |
      | Space |
