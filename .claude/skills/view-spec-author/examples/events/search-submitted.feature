@EVT-search-submitted
Feature: Search Submitted Event
  As the application
  I want a search-submitted event with validated payload
  So that the search store updates, an API call is made, analytics are tracked, and success/failure chains are handled

  Background:
    Given the event "EVT-search-submitted" is defined

  # ---------------------------------------------------------------------------
  # Payload validation - query (required)
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event requires a query in the payload
    Then the payload field "query" should be required

  Scenario: The query payload field is typed as string
    Then the payload field "query" should have type "string"

  Scenario: Search submitted event is valid with only the query field
    When the event is dispatched with payload:
      | field | value    |
      | query | test     |
    Then the event payload should be valid

  Scenario: Search submitted event is invalid without the query field
    When the event is dispatched with an empty payload
    Then the event payload should be invalid

  # ---------------------------------------------------------------------------
  # Payload validation - timestamp (optional)
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event accepts an optional timestamp field
    Then the payload field "timestamp" should not be required

  Scenario: The timestamp payload field is typed as number
    Then the payload field "timestamp" should have type "number"

  Scenario: Search submitted event is valid with both query and timestamp
    When the event is dispatched with payload:
      | field     | value        |
      | query     | hello world  |
      | timestamp | 1700000000000 |
    Then the event payload should be valid

  # ---------------------------------------------------------------------------
  # Payload field count
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event defines exactly 2 payload fields
    Then the event should define 2 payload fields

  # ---------------------------------------------------------------------------
  # Store targeting
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event targets the search store
    Then the event should target store "STR-search-store"

  Scenario: Search submitted event targets exactly 1 store
    Then the event should target 1 store

  # ---------------------------------------------------------------------------
  # Side effects - API call
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event triggers an API call side effect
    When the event is dispatched with payload:
      | field | value |
      | query | test  |
    Then a side effect of type "api-call" should be triggered
    And the API call method should be "GET"
    And the API call URL should be "/api/search?q={query}"

  # ---------------------------------------------------------------------------
  # Side effects - analytics
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event triggers an analytics side effect
    When the event is dispatched with payload:
      | field | value |
      | query | test  |
    Then a side effect of type "analytics" should be triggered
    And the analytics event-name should be "search"
    And the analytics property "query" should be "payload.query"

  # ---------------------------------------------------------------------------
  # Side effects count
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event triggers exactly 2 side effects
    Then the event should trigger 2 side effects

  Scenario Outline: Search submitted event triggers a "<type>" side effect
    Then the event should trigger a side effect of type "<type>"

    Examples:
      | type      |
      | api-call  |
      | analytics |

  # ---------------------------------------------------------------------------
  # Success chain
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event dispatches search-results-received on success
    When the API call returns a successful response
    Then the event "EVT-search-results-received" should be dispatched

  Scenario: The on-success dispatch target is EVT-search-results-received
    Then the on-success dispatch should be "EVT-search-results-received"

  # ---------------------------------------------------------------------------
  # Failure chain
  # ---------------------------------------------------------------------------

  Scenario: Search submitted event dispatches search-failed on failure
    When the API call returns an error response
    Then the event "EVT-search-failed" should be dispatched

  Scenario: The on-failure dispatch target is EVT-search-failed
    Then the on-failure dispatch should be "EVT-search-failed"
