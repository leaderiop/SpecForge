@CMP-search-bar
Feature: Search Bar Component
  As a user
  I want a search bar with an input field and submit button
  So that I can type a query and trigger a search

  Background:
    Given the component "CMP-search-bar" is rendered
    And the page "PG-home" is active

  # ---------------------------------------------------------------------------
  # Props
  # ---------------------------------------------------------------------------

  Scenario: Search bar accepts a placeholder prop with a default value
    When no placeholder prop is provided
    Then the placeholder prop should be "Type to search..."

  Scenario: The placeholder prop is not required
    Then the prop "placeholder" should not be required

  Scenario: The placeholder prop is typed as string
    Then the prop "placeholder" should have type "string"

  Scenario: Search bar accepts a maxLength prop with a default value
    When no maxLength prop is provided
    Then the maxLength prop should be 256

  Scenario: The maxLength prop is not required
    Then the prop "maxLength" should not be required

  Scenario: The maxLength prop is typed as number
    Then the prop "maxLength" should have type "number"

  # ---------------------------------------------------------------------------
  # Children rendering order
  # ---------------------------------------------------------------------------

  Scenario: Search bar renders children in the correct order
    Then the component should render the following children in order:
      | position | child-ref        |
      | 1        | ELM-search-input |
      | 2        | ELM-search-button |

  Scenario: Search bar renders exactly 2 children
    Then the component should render 2 children

  # ---------------------------------------------------------------------------
  # Store bindings
  # ---------------------------------------------------------------------------

  Scenario: Search bar binds to the search store
    Then the component should bind to store "STR-search-store"

  Scenario: Search bar maps the query selector to searchValue
    Then the store binding for "STR-search-store" should map selector "query" to local property "searchValue"

  Scenario: Search bar maps the isLoading selector to loading
    Then the store binding for "STR-search-store" should map selector "isLoading" to local property "loading"

  # ---------------------------------------------------------------------------
  # Responsive visibility
  # ---------------------------------------------------------------------------

  Scenario Outline: Search bar is visible on the "<viewport>" viewport
    When the viewport is "<viewport>"
    Then the component should be visible

    Examples:
      | viewport |
      | desktop  |
      | tablet   |
      | mobile   |
