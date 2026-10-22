# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Comments follow sharing roles

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Projet" saved as "F1"
    And Alice owns a file "a.txt" containing "alpha" in folder "F1" saved as "A"

  Scenario: A viewer cannot comment, a commenter can
    When Alice sends POST "/nodes/<F1>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Bob sends POST "/nodes/<A>/comments" with JSON:
      """
      {"body":"hi"}
      """
    Then the status is 403
    When Alice sends GET "/nodes/<F1>/shares"
    Then the JSON at "/users/0/id" is saved as "S"
    When Alice sends PATCH "/shares/<S>" with JSON:
      """
      {"role":"commenter"}
      """
    And Bob sends POST "/nodes/<A>/comments" with JSON:
      """
      {"body":"  bonjour  "}
      """
    Then the status is 200
    And the JSON at "/comment/id" is saved as "C"
    When Alice sends GET "/nodes/<A>/comments"
    Then the JSON at "/items/0/body" is "bonjour"
    When Alice sends PATCH "/comments/<C>" with JSON:
      """
      {"body":"x"}
      """
    Then the status is 403
    When Bob sends PATCH "/comments/<C>" with JSON:
      """
      {"body":"corrigé"}
      """
    Then the JSON at "/comment/edited_at" is not null
    When Bob sends POST "/nodes/<A>/comments" with JSON:
      """
      {"body":"   "}
      """
    Then the status is 400
    When Alice sends DELETE "/comments/<C>"
    Then the status is 200
