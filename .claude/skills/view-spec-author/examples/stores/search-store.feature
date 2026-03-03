@STR-search-store
Feature: Search Store
  As the application
  I want a search store that tracks query state, results, and loading status
  So that search components reactively display current search state and results are persisted across sessions

  Background:
    Given the store "STR-search-store" is initialized

  # ---------------------------------------------------------------------------
  # Initial state
  # ---------------------------------------------------------------------------

  Scenario: Search store initializes query to an empty string
    Then the state field "query" should be ""

  Scenario: Search store initializes results to an empty array
    Then the state field "results" should be an empty array

  Scenario: Search store initializes isLoading to false
    Then the state field "isLoading" should be false

  Scenario: Search store initializes error to null
    Then the state field "error" should be null

  Scenario: Search store initializes totalResults to 0
    Then the state field "totalResults" should be 0

  Scenario Outline: Search store initializes "<field>" to <value>
    Then the state field "<field>" should be <value>

    Examples:
      | field        | value |
      | query        | ""    |
      | isLoading    | false |
      | error        | null  |
      | totalResults | 0     |

  # ---------------------------------------------------------------------------
  # Reducer: EVT-search-submitted
  # ---------------------------------------------------------------------------

  Scenario: EVT-search-submitted sets query to the payload query
    When the event "EVT-search-submitted" is dispatched with payload:
      | field | value       |
      | query | hello world |
    Then the state field "query" should be "hello world"

  Scenario: EVT-search-submitted sets isLoading to true
    When the event "EVT-search-submitted" is dispatched with payload:
      | field | value |
      | query | test  |
    Then the state field "isLoading" should be true

  Scenario: EVT-search-submitted clears the error
    Given the state field "error" is "Previous error"
    When the event "EVT-search-submitted" is dispatched with payload:
      | field | value |
      | query | test  |
    Then the state field "error" should be null

  Scenario: EVT-search-submitted clears the results
    Given the state field "results" is a non-empty array
    When the event "EVT-search-submitted" is dispatched with payload:
      | field | value |
      | query | test  |
    Then the state field "results" should be an empty array

  Scenario: EVT-search-submitted reducer defines exactly 4 field operations
    Then the reducer for "EVT-search-submitted" should define 4 operations

  Scenario Outline: EVT-search-submitted reducer operation <index> targets field "<field>" with operation "<operation>"
    Then the reducer for "EVT-search-submitted" operation <index> should target field "<field>"
    And the reducer for "EVT-search-submitted" operation <index> should use operation "<operation>"

    Examples:
      | index | field     | operation |
      | 1     | query     | set       |
      | 2     | isLoading | set       |
      | 3     | error     | clear     |
      | 4     | results   | clear     |

  # ---------------------------------------------------------------------------
  # Selectors
  # ---------------------------------------------------------------------------

  Scenario: The query selector returns the current query
    Given the state field "query" is "hello"
    Then the selector "query" should return "hello"

  Scenario: The results selector returns the results array
    Given the state field "results" contains items
    Then the selector "results" should return the results array

  Scenario: The isLoading selector returns the loading state
    Given the state field "isLoading" is true
    Then the selector "isLoading" should return true

  Scenario: The hasResults selector returns true when results exist
    Given the state field "results" contains items
    Then the selector "hasResults" should return true

  Scenario: The hasResults selector returns false when results are empty
    Given the state field "results" is an empty array
    Then the selector "hasResults" should return false

  Scenario: The hasError selector returns true when an error exists
    Given the state field "error" is "Something went wrong"
    Then the selector "hasError" should return true

  Scenario: The hasError selector returns false when error is null
    Given the state field "error" is null
    Then the selector "hasError" should return false

  Scenario: The resultCount selector returns the totalResults value
    Given the state field "totalResults" is 42
    Then the selector "resultCount" should return 42

  Scenario: Search store defines exactly 6 selectors
    Then the store should define 6 selectors

  Scenario Outline: Selector "<name>" computes "<expression>"
    Then the selector "<name>" should compute "<expression>"

    Examples:
      | name        | expression              |
      | query       | state.query             |
      | results     | state.results           |
      | isLoading   | state.isLoading         |
      | hasResults  | state.results.length > 0 |
      | hasError    | state.error !== null    |
      | resultCount | state.totalResults      |

  # ---------------------------------------------------------------------------
  # Consumer notification
  # ---------------------------------------------------------------------------

  Scenario Outline: Search store notifies consumer "<consumer>"
    Then the store should notify consumer "<consumer>"

    Examples:
      | consumer          |
      | CMP-search-bar    |
      | ELM-search-input  |
      | ELM-search-button |

  Scenario: Search store notifies exactly 3 consumers
    Then the store should notify 3 consumers

  # ---------------------------------------------------------------------------
  # Persistence
  # ---------------------------------------------------------------------------

  Scenario: Search store has persistence enabled
    Then the store persistence should be enabled

  Scenario: Search store persists to session-storage
    Then the store persistence storage should be "session-storage"

  Scenario: Search store uses the key "search-state"
    Then the store persistence key should be "search-state"

  Scenario: Search store saves state on every change
    When the store state changes
    Then the state should be saved to persistence

  Scenario: Search store restores state on initialization
    Given session-storage contains a value for key "search-state"
    When the store is initialized
    Then the state should be restored from persistence
