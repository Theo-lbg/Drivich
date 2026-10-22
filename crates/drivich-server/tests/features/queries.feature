# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Listing and search filters

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Docs" saved as "F"
    And Alice owns a file "one.txt" containing "alpha beta" in folder "F" saved as "A"
    And Alice owns a file "two.txt" containing "gamma" in folder "F" saved as "B"

  Scenario: Every view and sort is accepted, bad ones are refused
    When Alice sends GET "/nodes?view=drive"
    Then the status is 200
    When Alice sends GET "/nodes?view=recent"
    Then the status is 200
    When Alice sends GET "/nodes?view=starred"
    Then the status is 200
    When Alice sends GET "/nodes?view=shared"
    Then the status is 200
    When Alice sends GET "/nodes?view=nope"
    Then the status is 400
    When Alice sends GET "/nodes?sort=modified"
    Then the status is 200
    When Alice sends GET "/nodes?sort=created&order=desc"
    Then the status is 200
    When Alice sends GET "/nodes?sort=size"
    Then the status is 200
    When Alice sends GET "/nodes?sort=nope"
    Then the status is 400
    When Alice sends GET "/nodes?order=sideways"
    Then the status is 400
    When Alice sends GET "/nodes?parent=not-a-uuid"
    Then the status is 400
    When Alice sends GET "/nodes?parent=root"
    Then the status is 200

  Scenario: Type, owner and date filters on the listing
    When Alice sends GET "/nodes?parent=<F>&type=text"
    Then the JSON at "/items" has 2 items
    When Alice sends GET "/nodes?parent=<F>&type=image"
    Then the JSON at "/items" has 0 items
    When Alice sends GET "/nodes?parent=<F>&type=any"
    Then the JSON at "/items" has 2 items
    When Alice sends GET "/nodes?parent=<F>&owner=me"
    Then the JSON at "/items" has 2 items
    When Alice sends GET "/nodes?parent=<F>&owner=others"
    Then the JSON at "/items" has 0 items
    When Alice sends GET "/nodes?owner=nobody"
    Then the status is 400
    When Alice sends GET "/nodes?location=drive"
    Then the status is 200
    When Alice sends GET "/nodes?modified_from=2000-01-01T00:00:00Z"
    Then the status is 200
    When Alice sends GET "/nodes?type=folder"
    Then the status is 200

  Scenario: The listing is paginated
    When Alice sends GET "/nodes?parent=<F>&limit=1"
    Then the JSON at "/items" has 1 items
    And the JSON at "/next_cursor" is saved as "CUR"
    When Alice sends GET "/nodes?parent=<F>&limit=1&cursor=<CUR>"
    Then the JSON at "/items" has 1 items
    When Alice sends GET "/nodes?cursor=nope"
    Then the status is 400

  Scenario: Search filters
    When Alice sends GET "/search?name=one"
    Then the JSON at "/items" has 1 items
    When Alice sends GET "/search?name=100%25_"
    Then the JSON at "/items" has 0 items
    When Alice sends GET "/search?q=txt&type=text"
    Then the status is 200
    When Alice sends GET "/search?q=txt&type=nope"
    Then the status is 400
    When Alice sends GET "/search?q=txt&location=drive"
    Then the status is 200
    When Alice sends GET "/search?q=txt&location=shared"
    Then the status is 200
    When Alice sends GET "/search?q=txt&location=starred"
    Then the status is 200
    When Alice sends GET "/search?q=txt&location=trash"
    Then the status is 200
    When Alice sends GET "/search?q=txt&location=elsewhere"
    Then the status is 400
    When Alice sends GET "/search?q=txt&owner=me"
    Then the status is 200
    When Alice sends GET "/search?q=txt&owner=others"
    Then the status is 200
    When Alice sends GET "/search?q=txt&owner=nobody"
    Then the status is 400
    When Alice sends GET "/search?q=txt&modified_from=2000-01-01T00:00:00Z&modified_to=2100-01-01T00:00:00Z"
    Then the JSON at "/items" has at least 2 items
    When Alice sends GET "/search?q=txt&in_folder=<F>"
    Then the JSON at "/items" has 2 items
    When Alice sends GET "/search?q=txt&limit=1"
    Then the JSON at "/next_cursor" is saved as "CUR"
    When Alice sends GET "/search?q=txt&limit=1&cursor=<CUR>"
    Then the status is 200
    When Alice sends GET "/search?q=txt&cursor=nope"
    Then the status is 400

  Scenario: Content search needs the indexer, but the query is valid
    When Alice sends GET "/search?words=alpha"
    Then the status is 200

  Scenario: Shared items appear in Bob's views and searches
    When Alice sends POST "/nodes/<F>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"editor"}
      """
    And Bob sends GET "/nodes?view=shared"
    Then the status is 200
    When Bob sends GET "/search?q=one&location=shared"
    Then the JSON at "/items" has 1 items
    When Bob sends GET "/search?q=one&owner=others"
    Then the JSON at "/items" has 1 items
    When Bob sends GET "/nodes/<A>/path"
    Then the status is 200
    When Bob sends GET "/nodes?parent=<F>&owner=others"
    Then the JSON at "/items" has 2 items

  Scenario: Starred and trash views
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"starred":true}
      """
    And Alice sends GET "/nodes?view=starred"
    Then the JSON at "/items" has 1 items
    When Alice sends GET "/search?q=one&location=starred"
    Then the JSON at "/items" has 1 items
    When Alice sends POST "/nodes/<B>/trash"
    And Alice sends GET "/search?q=two&location=trash"
    Then the JSON at "/items" has 1 items
