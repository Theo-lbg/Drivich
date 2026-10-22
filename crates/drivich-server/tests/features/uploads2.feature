# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Upload sessions

  Background:
    Given a user Alice

  Scenario: An upload can be inspected, resumed and cancelled
    When Alice sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"name":"big.bin","size":10}
      """
    Then the JSON at "/upload/id" is saved as "U"
    When Alice sends HEAD "/uploads/<U>"
    Then the response header "upload-offset" starts with "0"
    When Alice sends GET "/uploads/<U>"
    Then the status is 200
    And the JSON at "/upload/received" is "0"
    When Alice sends DELETE "/uploads/<U>"
    Then the status is 200
    When Alice sends DELETE "/uploads/<U>"
    Then the status is 404

  Scenario: Upload validation
    When Alice sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"size":10}
      """
    Then the status is 400
    When Alice sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"name":"neg.bin","size":-1}
      """
    Then the status is 400

  Scenario: Only files that are not trashed get new versions
    Given Alice owns a file "a.txt" containing "alpha" saved as "A"
    And Alice owns a folder "Docs" saved as "F"
    When Alice sends POST "/uploads" with JSON:
      """
      {"replace_node_id":"<F>","size":3}
      """
    Then the status is 400
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends POST "/uploads" with JSON:
      """
      {"replace_node_id":"<A>","size":3}
      """
    Then the status is 400

  Scenario: Version housekeeping
    Given Alice owns a file "a.txt" containing "alpha" saved as "A"
    And Alice owns a folder "Docs" saved as "F"
    When Alice replaces file "A" with "second"
    And Alice sends GET "/nodes/<A>/versions"
    Then the JSON at "/items/0/id" is saved as "V"
    When Alice sends DELETE "/nodes/<A>/versions/<V>"
    Then the status is 200
    When Alice sends DELETE "/nodes/<A>/versions/<V>"
    Then the status is 404
    When Alice sends GET "/nodes/<F>/versions"
    Then the status is 400
    When Alice sends POST "/nodes/<A>/trash"
    And Alice sends POST "/nodes/<A>/versions/<V>/restore"
    Then the status is 400
