@EVT-route-changed
Feature: Route Changed Event
  As the application
  I want a route-changed event with validated payload
  So that stores update navigation state and side effects like analytics are triggered

  Background:
    Given the event "EVT-route-changed" is defined

  # ---------------------------------------------------------------------------
  # Payload validation - path (required)
  # ---------------------------------------------------------------------------

  Scenario: Route changed event requires a path in the payload
    Then the payload field "path" should be required

  Scenario: The path payload field is typed as string
    Then the payload field "path" should have type "string"

  Scenario: Route changed event is valid with only the path field
    When the event is dispatched with payload:
      | field | value  |
      | path  | /about |
    Then the event payload should be valid

  Scenario: Route changed event is invalid without the path field
    When the event is dispatched with an empty payload
    Then the event payload should be invalid

  # ---------------------------------------------------------------------------
  # Payload validation - params (optional)
  # ---------------------------------------------------------------------------

  Scenario: Route changed event accepts an optional params field
    Then the payload field "params" should not be required

  Scenario: The params payload field is typed as object
    Then the payload field "params" should have type "object"

  Scenario: Route changed event is valid with both path and params
    When the event is dispatched with payload:
      | field  | value          |
      | path   | /users/42      |
      | params | {"userId":"42"} |
    Then the event payload should be valid

  # ---------------------------------------------------------------------------
  # Payload field count
  # ---------------------------------------------------------------------------

  Scenario: Route changed event defines exactly 2 payload fields
    Then the event should define 2 payload fields

  # ---------------------------------------------------------------------------
  # Store targeting
  # ---------------------------------------------------------------------------

  Scenario: Route changed event targets the router store
    Then the event should target store "STR-router-store"

  Scenario: Route changed event targets exactly 1 store
    Then the event should target 1 store

  # ---------------------------------------------------------------------------
  # Side effects - navigation
  # ---------------------------------------------------------------------------

  Scenario: Route changed event triggers a navigation side effect
    When the event is dispatched with payload:
      | field | value |
      | path  | /home |
    Then a side effect of type "navigation" should be triggered
    And the navigation path should be "payload.path"

  # ---------------------------------------------------------------------------
  # Side effects - analytics
  # ---------------------------------------------------------------------------

  Scenario: Route changed event triggers an analytics side effect
    When the event is dispatched with payload:
      | field | value |
      | path  | /home |
    Then a side effect of type "analytics" should be triggered
    And the analytics event-name should be "page_view"
    And the analytics property "path" should be "payload.path"

  # ---------------------------------------------------------------------------
  # Side effects count
  # ---------------------------------------------------------------------------

  Scenario: Route changed event triggers exactly 2 side effects
    Then the event should trigger 2 side effects

  Scenario Outline: Route changed event triggers a "<type>" side effect
    Then the event should trigger a side effect of type "<type>"

    Examples:
      | type       |
      | navigation |
      | analytics  |
