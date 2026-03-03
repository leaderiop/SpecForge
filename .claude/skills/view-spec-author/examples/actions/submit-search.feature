@ACT-submit-search
Feature: Submit Search Action
  As a user
  I want submitting the search input to trigger a search
  So that the app validates the query, dispatches a search event, and debounces rapid submissions

  Background:
    Given the action "ACT-submit-search" is defined
    And the action type is "search-submit"

  # ---------------------------------------------------------------------------
  # Action type
  # ---------------------------------------------------------------------------

  Scenario: Submit search action is of type search-submit
    Then the action type should be "search-submit"

  # ---------------------------------------------------------------------------
  # Trigger firing
  # ---------------------------------------------------------------------------

  Scenario: Submit search action is triggered by submitting the search input
    Then the trigger element should be "ELM-search-input"
    And the trigger interaction should be "submit"

  # ---------------------------------------------------------------------------
  # Preconditions - pass
  # ---------------------------------------------------------------------------

  Scenario: Submit search action executes when the query is non-empty
    Given the query value is "hello world"
    When the action is triggered
    Then the action should not be blocked by any precondition
    And the event "EVT-search-submitted" should be dispatched

  Scenario Outline: Submit search action executes for valid query "<query>"
    Given the query value is "<query>"
    When the action is triggered
    Then the action should not be blocked by any precondition

    Examples:
      | query          |
      | hello          |
      | a              |
      | search term    |
      |  leading space |

  # ---------------------------------------------------------------------------
  # Preconditions - fail
  # ---------------------------------------------------------------------------

  Scenario: Submit search action is blocked when the query is empty
    Given the query value is ""
    When the action is triggered
    Then the action should be blocked
    And the event "EVT-search-submitted" should not be dispatched

  Scenario: Submit search action is blocked when the query is only whitespace
    Given the query value is "   "
    When the action is triggered
    Then the action should be blocked

  Scenario: The precondition evaluates query.trim().length > 0
    Then the action should have 1 precondition
    And precondition 1 condition should be "query.trim().length > 0"
    And precondition 1 fail-action should be "block"

  # ---------------------------------------------------------------------------
  # Event dispatch
  # ---------------------------------------------------------------------------

  Scenario: Submit search action dispatches the search-submitted event
    Given the query value is "test"
    When the action executes
    Then the event "EVT-search-submitted" should be dispatched

  Scenario: Submit search action dispatches exactly 1 event
    Then the action should dispatch 1 event

  # ---------------------------------------------------------------------------
  # Debounce behavior
  # ---------------------------------------------------------------------------

  Scenario: Submit search action is debounced with a 300ms wait
    Then the debounce wait should be 300

  Scenario: Submit search action debounce fires on trailing edge
    Then the debounce trailing should be true

  Scenario: Submit search action debounce does not fire on leading edge
    Then the debounce leading should be false

  Scenario: Rapid submissions within 300ms result in a single dispatch
    Given the query value is "test"
    When the action is triggered 5 times within 300 milliseconds
    Then the event "EVT-search-submitted" should be dispatched exactly 1 time

  Scenario: A submission after the debounce window triggers a new dispatch
    Given the query value is "test"
    When the action is triggered
    And 300 milliseconds elapse
    And the action is triggered again
    Then the event "EVT-search-submitted" should be dispatched exactly 2 times
