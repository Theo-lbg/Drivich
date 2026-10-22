# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: File versions

  Scenario: Uploading over a file creates a version that can be restored
    Given a user Alice
    And Alice owns a file "a.txt" containing "alpha" saved as "A"
    When Alice replaces file "A" with "alpha two"
    Then the JSON at "/node/size" is "9"
    When Alice sends GET "/nodes/<A>/versions"
    Then the status is 200
    And the JSON at "/items" has 1 items
    And the JSON at "/items/0/id" is saved as "V"
    When Alice sends GET "/nodes/<A>/versions/<V>/download"
    Then the response body is "alpha"
    When Alice sends POST "/nodes/<A>/versions/<V>/restore"
    Then the status is 200
    When Alice sends GET "/nodes/<A>/download"
    Then the response body is "alpha"
