# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Renaming, moving, describing and opening

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Docs" saved as "F"
    And Alice owns a file "one.txt" containing "alpha" saved as "A"

  Scenario: Rename rules
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"name":"one.md"}
      """
    Then the status is 400
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"name":"   "}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"name":"renamed.txt"}
      """
    Then the status is 400

  Scenario: Descriptions
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"description":"  a note  "}
      """
    Then the status is 200
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"description":null}
      """
    Then the status is 200
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {}
      """
    Then the status is 200

  Scenario: Move rules
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"parent_id":"<F>"}
      """
    Then the status is 200
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"parent_id":"<F>"}
      """
    Then the status is 200
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"parent_id":null}
      """
    Then the status is 200
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"parent_id":"<F>"}
      """
    Then the status is 400

  Scenario: Items cannot move into someone else's drive
    Given Bob owns a folder "Bobs" saved as "BF"
    When Alice sends PATCH "/nodes/<A>" with JSON:
      """
      {"parent_id":"<BF>"}
      """
    Then the status is 404

  Scenario: Opening a file marks it as recent
    When Alice sends POST "/nodes/<A>/open"
    Then the status is 204
    When Alice sends GET "/nodes?view=recent"
    Then the JSON at "/items" has 1 items
    When Bob sends POST "/nodes/<A>/open"
    Then the status is 404

  Scenario: Batch limits and partial failures
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":[],"action":"trash"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>","00000000-0000-0000-0000-000000000000"],"action":"unstar"}
      """
    Then the status is 200
    And the JSON at "/failed" has 1 items
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>"],"action":"move","parent_id":"<F>"}
      """
    Then the status is 200

  Scenario: Duplicate sign-up and user lookups
    When Alice sends GET "/users/lookup?q=bob"
    Then the status is 200
    When Alice sends GET "/users/lookup?q="
    Then the status is 200

  Scenario: Trash is emptied for good
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends POST "/trash/empty"
    Then the JSON at "/deleted" is "1"
