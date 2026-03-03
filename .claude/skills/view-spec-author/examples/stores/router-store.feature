@STR-router-store
Feature: Router Store
  As the application
  I want a router store that tracks navigation state
  So that components can reactively display the current path and consumers are notified of route changes

  Background:
    Given the store "STR-router-store" is initialized

  # ---------------------------------------------------------------------------
  # Initial state
  # ---------------------------------------------------------------------------

  Scenario: Router store initializes currentPath to "/"
    Then the state field "currentPath" should be "/"

  Scenario: Router store initializes previousPath to null
    Then the state field "previousPath" should be null

  Scenario: Router store initializes params to an empty object
    Then the state field "params" should be an empty object

  Scenario: Router store initializes query to an empty object
    Then the state field "query" should be an empty object

  Scenario Outline: Router store initializes "<field>" to <value>
    Then the state field "<field>" should be <value>

    Examples:
      | field        | value |
      | currentPath  | "/"   |
      | previousPath | null  |

  # ---------------------------------------------------------------------------
  # Reducer: EVT-route-changed
  # ---------------------------------------------------------------------------

  Scenario: EVT-route-changed sets previousPath to the current currentPath
    Given the state field "currentPath" is "/home"
    When the event "EVT-route-changed" is dispatched with payload:
      | field | value  |
      | path  | /about |
    Then the state field "previousPath" should be "/home"

  Scenario: EVT-route-changed sets currentPath to the payload path
    When the event "EVT-route-changed" is dispatched with payload:
      | field | value  |
      | path  | /about |
    Then the state field "currentPath" should be "/about"

  Scenario: EVT-route-changed sets params from the payload
    When the event "EVT-route-changed" is dispatched with payload:
      | field  | value           |
      | path   | /users/42       |
      | params | {"userId":"42"} |
    Then the state field "params" should contain key "userId" with value "42"

  Scenario: EVT-route-changed defaults params to empty object when not provided
    When the event "EVT-route-changed" is dispatched with payload:
      | field | value |
      | path  | /home |
    Then the state field "params" should be an empty object

  Scenario: EVT-route-changed reducer defines exactly 3 field operations
    Then the reducer for "EVT-route-changed" should define 3 operations

  Scenario Outline: EVT-route-changed reducer operation <index> targets field "<field>" with operation "<operation>"
    Then the reducer for "EVT-route-changed" operation <index> should target field "<field>"
    And the reducer for "EVT-route-changed" operation <index> should use operation "<operation>"

    Examples:
      | index | field        | operation |
      | 1     | previousPath | set       |
      | 2     | currentPath  | set       |
      | 3     | params       | set       |

  # ---------------------------------------------------------------------------
  # Selectors
  # ---------------------------------------------------------------------------

  Scenario: The currentPath selector returns the current path
    Given the state field "currentPath" is "/about"
    Then the selector "currentPath" should return "/about"

  Scenario: The previousPath selector returns the previous path
    Given the state field "previousPath" is "/home"
    Then the selector "previousPath" should return "/home"

  Scenario: The isHome selector returns true when currentPath is "/"
    Given the state field "currentPath" is "/"
    Then the selector "isHome" should return true

  Scenario: The isHome selector returns false when currentPath is not "/"
    Given the state field "currentPath" is "/about"
    Then the selector "isHome" should return false

  Scenario: Router store defines exactly 3 selectors
    Then the store should define 3 selectors

  Scenario Outline: Selector "<name>" computes "<expression>"
    Then the selector "<name>" should compute "<expression>"

    Examples:
      | name         | expression                |
      | currentPath  | state.currentPath         |
      | previousPath | state.previousPath        |
      | isHome       | state.currentPath === '/' |

  # ---------------------------------------------------------------------------
  # Consumer notification
  # ---------------------------------------------------------------------------

  Scenario: Router store notifies the header component
    Then the store should notify consumer "CMP-header"

  Scenario: Router store notifies exactly 1 consumer
    Then the store should notify 1 consumer

  # ---------------------------------------------------------------------------
  # Persistence
  # ---------------------------------------------------------------------------

  Scenario: Router store has persistence disabled
    Then the store persistence should be disabled

  Scenario: Router store persistence storage type is session-storage
    Then the store persistence storage should be "session-storage"

  Scenario: Router store persistence key is "router-state"
    Then the store persistence key should be "router-state"
