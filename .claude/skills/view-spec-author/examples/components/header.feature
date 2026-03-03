@CMP-header
Feature: Header Component
  As a user
  I want a persistent header at the top of the page
  So that I can see the application title and navigate via the logo

  Background:
    Given the component "CMP-header" is rendered
    And the page "PG-home" is active

  # ---------------------------------------------------------------------------
  # Props
  # ---------------------------------------------------------------------------

  Scenario: Header accepts a title prop with a default value
    When no title prop is provided
    Then the title prop should be "Search App"

  Scenario: Header accepts a custom title prop
    When the title prop is "My Custom App"
    Then the title prop should be "My Custom App"

  Scenario: The title prop is not required
    Then the prop "title" should not be required

  Scenario: The title prop is typed as string
    Then the prop "title" should have type "string"

  # ---------------------------------------------------------------------------
  # Children rendering
  # ---------------------------------------------------------------------------

  Scenario: Header renders the logo element
    Then the component should render child "ELM-logo"

  Scenario: Header renders exactly 1 child
    Then the component should render 1 child

  # ---------------------------------------------------------------------------
  # Store binding
  # ---------------------------------------------------------------------------

  Scenario: Header binds to the router store
    Then the component should bind to store "STR-router-store"

  Scenario: Header maps the currentPath selector to activePath
    Then the store binding for "STR-router-store" should map selector "currentPath" to local property "activePath"

  # ---------------------------------------------------------------------------
  # Responsive visibility
  # ---------------------------------------------------------------------------

  Scenario Outline: Header is visible on the "<viewport>" viewport
    When the viewport is "<viewport>"
    Then the component should be visible

    Examples:
      | viewport |
      | desktop  |
      | tablet   |
      | mobile   |
