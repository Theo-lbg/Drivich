# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Sharing and public links

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Projet" saved as "F1"
    And Alice owns a file "a.txt" containing "alpha" in folder "F1" saved as "A"

  Scenario: A viewer can read but not modify
    When Alice sends POST "/nodes/<F1>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    Then the status is 200
    When Bob sends GET "/nodes/<A>"
    Then the status is 200
    And the JSON at "/role" is "viewer"
    When Bob sends GET "/nodes/<A>/download"
    Then the response body is "alpha"
    When Bob sends PATCH "/nodes/<A>" with JSON:
      """
      {"name":"x"}
      """
    Then the status is 403
    When Bob sends GET "/nodes/<F1>/shares"
    Then the status is 403
    When Bob sends GET "/nodes?view=shared"
    Then the JSON at "/items" has at least 1 items

  Scenario: Revoking a share removes access immediately
    When Alice sends POST "/nodes/<F1>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Alice sends GET "/nodes/<F1>/shares"
    Then the JSON at "/users/0/id" is saved as "S"
    When Alice sends DELETE "/shares/<S>"
    And Bob sends GET "/nodes/<A>"
    Then the status is 404

  Scenario: A public link works without a login until revoked
    When Alice sends POST "/nodes/<A>/links" with JSON:
      """
      {}
      """
    Then the status is 200
    And the JSON at "/token" is saved as "T"
    And the JSON at "/link/id" is saved as "L"
    When anonymous sends GET "/public/<T>"
    Then the status is 200
    And the JSON at "/root/name" is "a.txt"
    When anonymous sends GET "/public/<T>/nodes/<A>/download"
    Then the response body is "alpha"
    When Alice sends DELETE "/links/<L>"
    And anonymous sends GET "/public/<T>"
    Then the status is 404

  Scenario: A public link cannot reach files outside its scope
    Given Alice owns a file "other.txt" containing "zzz" saved as "O"
    When Alice sends POST "/nodes/<A>/links" with JSON:
      """
      {}
      """
    Then the JSON at "/token" is saved as "T"
    When anonymous sends GET "/public/<T>/nodes/<O>/download"
    Then the status is 404
