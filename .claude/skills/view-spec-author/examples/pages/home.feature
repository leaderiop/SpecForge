@PG-home
Feature: Home Page
  As a user
  I want to land on the home page of the Search App
  So that I can view the search interface and begin searching immediately

  Background:
    Given the page "PG-home" is loaded
    And the wireframe "WF-search-app" is active

  # ---------------------------------------------------------------------------
  # Route matching
  # ---------------------------------------------------------------------------

  Scenario: Home page matches the root path
    Then the page route path should be "/"

  Scenario: Home page does not define any path params
    Then the page route should have 0 params

  Scenario: Home page does not use hash routing
    Then the page route hash should be disabled

  # ---------------------------------------------------------------------------
  # Query param extraction
  # ---------------------------------------------------------------------------

  Scenario: Home page extracts the "q" query parameter
    When the URL is "/?q=hello"
    Then the query parameter "q" should be "hello"

  Scenario: The "q" query parameter defaults to an empty string
    When the URL is "/"
    Then the query parameter "q" should be ""

  Scenario: The "q" query parameter is typed as string
    Then the query parameter "q" should have type "string"

  # ---------------------------------------------------------------------------
  # Auth guard
  # ---------------------------------------------------------------------------

  Scenario: Home page requires no authentication guard
    Then the page guard should be null

  # ---------------------------------------------------------------------------
  # Component rendering order
  # ---------------------------------------------------------------------------

  Scenario: Home page renders components in the correct order
    Then the page should render the following components in order:
      | position | component-ref |
      | 1        | CMP-header    |
      | 2        | CMP-search-bar |

  Scenario: Home page renders exactly 2 components
    Then the page should render 2 components

  # ---------------------------------------------------------------------------
  # Store subscriptions
  # ---------------------------------------------------------------------------

  Scenario Outline: Home page subscribes to store "<store>"
    Then the page should subscribe to store "<store>"

    Examples:
      | store            |
      | STR-router-store |
      | STR-search-store |

  Scenario: Home page subscribes to exactly 2 stores
    Then the page should subscribe to 2 stores

  # ---------------------------------------------------------------------------
  # Meta tags
  # ---------------------------------------------------------------------------

  Scenario: Home page sets the correct title meta tag
    Then the page meta title should be "Search App - Home"

  Scenario: Home page sets the correct description meta tag
    Then the page meta description should be "Search anything with the Search App"

  # ---------------------------------------------------------------------------
  # Layout
  # ---------------------------------------------------------------------------

  Scenario: Home page uses a single-column layout
    Then the page layout should be "single-column"
