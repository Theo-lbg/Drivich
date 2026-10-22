# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Public links in depth

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a folder "Docs" saved as "F"
    And Alice owns a file "one.txt" containing "alpha" in folder "F" saved as "A"
    And Alice owns a file "pic.png" containing "x" saved as "X"

  Scenario: A shared folder can be browsed through its link
    When Alice sends POST "/nodes/<F>/links" with JSON:
      """
      {}
      """
    Then the JSON at "/token" is saved as "T"
    When anonymous sends GET "/public/<T>/nodes/<F>"
    Then the status is 200
    When anonymous sends GET "/public/<T>/nodes/<F>/children"
    Then the JSON at "/items" has 1 items
    When anonymous sends GET "/public/<T>/nodes/<F>/children?limit=1&cursor=0"
    Then the status is 200
    When anonymous sends GET "/public/<T>/nodes/<A>/children"
    Then the status is 400
    When anonymous sends GET "/public/<T>/nodes/<A>"
    Then the status is 200
    When anonymous sends GET "/public/<T>/nodes/<A>/content"
    Then the status is 200
    When anonymous sends GET "/public/<T>/nodes/<F>/zip"
    Then the status is 200
    When anonymous sends GET "/public/<T>/nodes/<A>/zip"
    Then the status is 400
    When anonymous sends GET "/public/<T>/nodes/<X>"
    Then the status is 404

  Scenario: Public thumbnails
    Given Alice owns an image "pic2.png" saved as "P"
    When Alice sends POST "/nodes/<P>/links" with JSON:
      """
      {}
      """
    Then the JSON at "/token" is saved as "T"
    When anonymous sends GET "/public/<T>/nodes/<P>/thumbnail"
    Then the status is 404
    Given the worker has made the thumbnail of "P"
    When anonymous sends GET "/public/<T>/nodes/<P>/thumbnail"
    Then the status is 200

  Scenario: Link options and sharing errors
    When Alice sends POST "/nodes/<F>/links" with JSON:
      """
      {"expires_in_days":1}
      """
    Then the status is 200
    When Alice sends POST "/nodes/<F>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"owner"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<F>/shares" with JSON:
      """
      {"email":"alice-<run>@example.test","role":"viewer"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<F>/shares" with JSON:
      """
      {"email":"nobody-<run>@example.test","role":"viewer"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<F>/trash"
    And Alice sends POST "/nodes/<F>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<F>/links" with JSON:
      """
      {}
      """
    Then the status is 400
