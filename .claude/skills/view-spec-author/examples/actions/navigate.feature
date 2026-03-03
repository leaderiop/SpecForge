@ACT-navigate
Feature: Navigate Action
  As a user
  I want clicking the logo to trigger a navigation
  So that the app dispatches a route-changed event and I am redirected to the home page

  Background:
    Given the action "ACT-navigate" is defined
    And the action type is "navigate"

  # ---------------------------------------------------------------------------
  # Action type
  # ---------------------------------------------------------------------------

  Scenario: Navigate action is of type navigate
    Then the action type should be "navigate"

  # ---------------------------------------------------------------------------
  # Trigger firing
  # ---------------------------------------------------------------------------

  Scenario: Navigate action is triggered by clicking the logo element
    Then the trigger element should be "ELM-logo"
    And the trigger interaction should be "click"

  # ---------------------------------------------------------------------------
  # Preconditions
  # ---------------------------------------------------------------------------

  Scenario: Navigate action has no preconditions
    Then the action should have 0 preconditions

  Scenario: Navigate action always executes when triggered
    When the action is triggered
    Then the action should not be blocked by any precondition

  # ---------------------------------------------------------------------------
  # Event dispatch
  # ---------------------------------------------------------------------------

  Scenario: Navigate action dispatches the route-changed event
    When the action executes
    Then the event "EVT-route-changed" should be dispatched

  Scenario: Navigate action dispatches exactly 1 event
    Then the action should dispatch 1 event
