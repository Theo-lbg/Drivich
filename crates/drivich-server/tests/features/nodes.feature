# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Blank documents and multi-selection

  Background:
    Given a user Alice

  Scenario: Blank documents can be created
    When Alice sends POST "/nodes/file" with JSON:
      """
      {"parent_id":null,"kind":"txt","name":"notes"}
      """
    Then the status is 200
    When Alice sends POST "/nodes/file" with JSON:
      """
      {"parent_id":null,"kind":"exe","name":"bad"}
      """
    Then the status is 400

  Scenario: Batch actions star, trash, restore and delete
    Given Alice owns a folder "One" saved as "A"
    And Alice owns a folder "Two" saved as "B"
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>","<B>"],"action":"star"}
      """
    Then the status is 200
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>","<B>"],"action":"trash"}
      """
    Then the status is 200
    When Alice sends GET "/nodes?view=trash"
    Then the JSON at "/items" has 2 items
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>"],"action":"restore"}
      """
    Then the status is 200
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<A>"],"action":"explode"}
      """
    Then the status is 400
    When Alice sends POST "/trash/empty"
    Then the status is 200
    When Alice sends GET "/nodes?view=trash"
    Then the JSON at "/items" has 0 items

  Scenario: Moving into a folder and listing it
    Given Alice owns a folder "One" saved as "A"
    And Alice owns a file "f.txt" containing "x" saved as "F"
    When Alice sends POST "/nodes/batch" with JSON:
      """
      {"ids":["<F>"],"action":"move","parent_id":"<A>"}
      """
    Then the status is 200
    When Alice sends GET "/nodes?parent=<A>"
    Then the JSON at "/items" has 1 items
    When Alice sends GET "/nodes?view=drive&sort=size&order=desc&type=folder"
    Then the status is 200
    When Alice sends GET "/users/lookup?q=alice"
    Then the status is 200
