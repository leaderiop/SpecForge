@WF-search-app
Feature: Search App Wireframe
  As a designer
  I want a responsive wireframe for the Search App
  So that the layout adapts correctly across all viewport sizes and applies the design theme consistently

  Background:
    Given the wireframe "WF-search-app" is loaded
    And the wireframe defines viewports "desktop", "tablet", and "mobile"
    And the wireframe theme is applied

  # ---------------------------------------------------------------------------
  # Viewport rendering
  # ---------------------------------------------------------------------------

  Scenario Outline: Viewport renders at the correct breakpoint
    When the viewport width is <width> pixels
    Then the active viewport should be "<viewport>"

    Examples:
      | width | viewport |
      | 1440  | desktop  |
      | 1024  | desktop  |
      | 1023  | tablet   |
      | 900   | tablet   |
      | 768   | tablet   |
      | 767   | mobile   |
      | 375   | mobile   |
      | 320   | mobile   |

  Scenario: Desktop viewport has a minimum width of 1024
    When the viewport is "desktop"
    Then the minimum width should be 1024

  Scenario: Tablet viewport has a width range of 768 to 1023
    When the viewport is "tablet"
    Then the minimum width should be 768
    And the maximum width should be 1023

  Scenario: Mobile viewport has a maximum width of 767
    When the viewport is "mobile"
    Then the maximum width should be 767

  # ---------------------------------------------------------------------------
  # Page inclusion
  # ---------------------------------------------------------------------------

  Scenario: Wireframe includes the home page
    Then the wireframe should include the page "PG-home"

  Scenario: Wireframe includes exactly one page
    Then the wireframe should include 1 page

  # ---------------------------------------------------------------------------
  # Theme colors
  # ---------------------------------------------------------------------------

  Scenario Outline: Theme defines the correct color for "<token>"
    Then the theme color "<token>" should be "<value>"

    Examples:
      | token      | value   |
      | primary    | #1A73E8 |
      | secondary  | #34A853 |
      | background | #FFFFFF |
      | surface    | #F8F9FA |
      | text       | #202124 |
      | error      | #D93025 |

  # ---------------------------------------------------------------------------
  # Theme typography
  # ---------------------------------------------------------------------------

  Scenario: Theme applies the correct font family
    Then the theme font-family should be "Inter, system-ui, sans-serif"

  Scenario: Theme applies the correct base font size
    Then the theme base-size should be "16px"
