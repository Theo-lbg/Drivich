# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Folders, files and trash

  Background:
    Given a user Alice
    And a user Bob

  Scenario: Folders can be renamed and are private
    Given Alice owns a folder "Projet" saved as "F1"
    When Alice sends PATCH "/nodes/<F1>" with JSON:
      """
      {"name":"Projet 2026"}
      """
    Then the status is 200
    And the JSON at "/name" is "Projet 2026"
    When Bob sends GET "/nodes/<F1>"
    Then the status is 404

  Scenario: Invalid names and loops are refused
    Given Alice owns a folder "Projet" saved as "F1"
    When Alice sends PATCH "/nodes/<F1>" with JSON:
      """
      {"name":"a/b"}
      """
    Then the status is 400
    When Alice sends PATCH "/nodes/<F1>" with JSON:
      """
      {"parent_id":"<F1>"}
      """
    Then the status is 400

  Scenario: A file downloads with its exact bytes and supports ranges
    Given Alice owns a file "hello.txt" containing "0123456789" saved as "N"
    When Alice sends GET "/nodes/<N>/download"
    Then the status is 200
    And the response body is "0123456789"
    When Alice sends GET "/nodes/<N>/download" with header "Range" set to "bytes=2-4"
    Then the status is 206
    And the response body is "234"
    When Alice sends GET "/nodes/<N>/download" with header "Range" set to "bytes=100-"
    Then the status is 416

  Scenario: Another user cannot download the file
    Given Alice owns a file "hello.txt" containing "secret" saved as "N"
    When Bob sends GET "/nodes/<N>/download"
    Then the status is 404

  Scenario: HTML is served as plain text, never executed
    Given Alice owns a file "page.html" containing "<b>x</b>" saved as "H"
    When Alice sends GET "/nodes/<H>/content"
    Then the status is 200
    And the response header "content-type" starts with "text/plain"

  Scenario: Trash, restore and permanent delete
    Given Alice owns a folder "Projet" saved as "F1"
    And Alice owns a file "a.txt" containing "alpha" in folder "F1" saved as "A"
    When Alice sends POST "/nodes/<F1>/trash"
    Then the status is 200
    When Alice sends GET "/nodes?view=trash"
    Then the JSON at "/items" has at least 1 items
    When Alice sends POST "/nodes/<F1>/restore"
    Then the status is 200
    When Alice sends POST "/nodes/<F1>/trash"
    And Alice sends DELETE "/nodes/<F1>"
    Then the status is 200
    When Alice sends GET "/nodes/<A>"
    Then the status is 404

  Scenario: Folders download as a zip
    Given Alice owns a folder "Projet" saved as "F1"
    And Alice owns a file "a.txt" containing "alpha" in folder "F1" saved as "A"
    When Alice sends GET "/nodes/<F1>/zip"
    Then the status is 200
    And the response header "content-disposition" starts with "attachment"
    When Bob sends GET "/nodes/<F1>/zip"
    Then the status is 404
    When Alice sends GET "/nodes/<A>/zip"
    Then the status is 400
    When Alice sends GET "/nodes/zip?ids=<A>,<F1>"
    Then the status is 200
    When Alice sends GET "/nodes/zip?ids=nope"
    Then the status is 400
