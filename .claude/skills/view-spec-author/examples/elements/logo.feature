@ELM-logo
Feature: Logo Element
  As a user
  I want a clickable logo image in the header
  So that I can navigate back to the home page by clicking or pressing Enter on it

  Background:
    Given the element "ELM-logo" is rendered
    And the parent component "CMP-header" is active

  # ---------------------------------------------------------------------------
  # Element type
  # ---------------------------------------------------------------------------

  Scenario: Logo element is of type image
    Then the element type should be "image"

  # ---------------------------------------------------------------------------
  # States - default
  # ---------------------------------------------------------------------------

  Scenario: Logo has a default width of 120
    When the element is in the "default" state
    Then the element width should be 120

  Scenario: Logo has a default height of 40
    When the element is in the "default" state
    Then the element height should be 40

  Scenario: Logo has a pointer cursor in the default state
    When the element is in the "default" state
    Then the element cursor should be "pointer"

  # ---------------------------------------------------------------------------
  # States - hover
  # ---------------------------------------------------------------------------

  Scenario: Logo reduces opacity on hover
    When the element is in the "hover" state
    Then the element opacity should be 0.8

  Scenario: Logo retains pointer cursor on hover
    When the element is in the "hover" state
    Then the element cursor should be "pointer"

  # ---------------------------------------------------------------------------
  # States - outline (all defined states)
  # ---------------------------------------------------------------------------

  Scenario Outline: Logo applies correct styles in the "<state>" state
    When the element is in the "<state>" state
    Then the element cursor should be "<cursor>"

    Examples:
      | state   | cursor  |
      | default | pointer |
      | hover   | pointer |

  # ---------------------------------------------------------------------------
  # Action triggers
  # ---------------------------------------------------------------------------

  Scenario: Clicking the logo triggers the navigate action
    When the user clicks the element
    Then the action "ACT-navigate" should be triggered

  # ---------------------------------------------------------------------------
  # Accessibility
  # ---------------------------------------------------------------------------

  Scenario: Logo has the img ARIA role
    Then the element ARIA role should be "img"

  Scenario: Logo has an accessible label
    Then the element ARIA label should be "Search App logo - navigate to home"

  Scenario: Pressing Enter on the logo triggers the navigate action
    When the user presses "Enter" on the element
    Then the action "ACT-navigate" should be triggered

  # ---------------------------------------------------------------------------
  # Keyboard interactions (exhaustive)
  # ---------------------------------------------------------------------------

  Scenario: Logo defines exactly 1 keyboard shortcut
    Then the element should define 1 keyboard shortcut
