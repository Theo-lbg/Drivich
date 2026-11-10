# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Making a copy of a file

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Projet" saved as "F1"
    And Alice owns a file "a.txt" containing "alpha" in folder "F1" saved as "A"

  Scenario: The owner's copy sits next to the original and is independent of it
    When Alice sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 200
    And the JSON at "/name" is "Copie de a.txt"
    And the JSON at "/parent_id" is "<F1>"
    And the JSON at "/size" is "5"
    And the JSON at "/role" is "owner"
    And the JSON at "/id" is saved as "C"
    When Alice sends GET "/nodes/<C>/download"
    Then the response body is "alpha"
    When Alice replaces file "C" with "beta"
    And Alice sends GET "/nodes/<A>/download"
    Then the response body is "alpha"
    When Alice sends GET "/nodes?parent=<F1>"
    Then the JSON at "/items" has 2 items

  Scenario: Only a file with the same extension can be copied
    When Alice sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"a.exe"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"a/b.txt"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<F1>/copy" with JSON:
      """
      {"name":"Copie de Projet"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 400

  Scenario: A viewer copies a shared file into their own Drive
    When Alice sends POST "/nodes/<F1>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Bob sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 200
    And the JSON at "/parent_id" is "null"
    And the JSON at "/role" is "owner"
    And the JSON at "/id" is saved as "C"
    When Bob sends GET "/nodes/<C>/download"
    Then the response body is "alpha"
    When Alice sends GET "/nodes?parent=<F1>"
    Then the JSON at "/items" has 1 items

  Scenario: An editor's copy stays in the shared folder
    When Alice sends POST "/nodes/<F1>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"editor"}
      """
    And Bob sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 200
    And the JSON at "/parent_id" is "<F1>"
    And the JSON at "/role" is "editor"
    When Alice sends GET "/nodes?parent=<F1>"
    Then the JSON at "/items" has 2 items

  Scenario: Someone without access cannot copy, and the quota still applies
    When Bob sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 404
    Given an administrator Root
    Then the user Alice id is saved as "AID"
    When Root sends PATCH "/admin/users/<AID>" with JSON:
      """
      {"quota_bytes":5}
      """
    And Alice sends POST "/nodes/<A>/copy" with JSON:
      """
      {"name":"Copie de a.txt"}
      """
    Then the status is 507
