# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Search

  Scenario: Search finds by name and never leaks other users' files
    Given a user Alice
    And a user Bob
    And Alice owns a file "rapport.txt" containing "alpha" saved as "A"
    When Alice sends GET "/search?q=rapport"
    Then the status is 200
    And the JSON at "/items" has 1 items
    When Alice sends GET "/search/suggest?q=rapport"
    Then the status is 200
    And the JSON at "/items" has at least 1 items
    When Bob sends GET "/search?q=rapport"
    Then the JSON at "/items" has 0 items

  Scenario: Trashed files leave search results
    Given a user Alice
    And Alice owns a file "gone.txt" containing "x" saved as "A"
    When Alice sends GET "/search?q=gone"
    Then the JSON at "/items" has 1 items
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends GET "/search?q=gone"
    Then the JSON at "/items" has 0 items
