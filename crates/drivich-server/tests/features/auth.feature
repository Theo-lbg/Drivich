# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Authentication and account
  Sessions are cookie based and every state-changing request needs the CSRF header.

  Scenario: The health endpoint answers without a session
    When anonymous sends GET "/health"
    Then the status is 200
    And the JSON at "/status" is "ok"

  Scenario: The OpenAPI description is public
    When anonymous sends GET "/openapi.json"
    Then the status is 200
    And the JSON at "/openapi" is not null

  Scenario: Anonymous requests are refused
    When anonymous sends GET "/nodes"
    Then the status is 401

  Scenario: A user logs in and reads their profile
    Given a user Alice
    When Alice sends GET "/me"
    Then the status is 200
    And the JSON at "/user/name" is "Alice"

  Scenario: A wrong password is refused
    Given a user Alice
    When Alice logs in with the password "nope-nope-nope"
    Then the status is 401

  Scenario: Repeated failures are throttled
    Given a user Alice
    When Alice fails to log in 9 times
    Then the status is 429

  Scenario: A session ends at logout
    Given a user Alice
    When Alice signs out
    And Alice sends GET "/nodes"
    Then the status is 401

  Scenario: The language is validated and saved
    Given a user Alice
    When Alice sends PATCH "/me" with JSON:
      """
      {"locale":"fr_FR"}
      """
    Then the status is 400
    When Alice sends PATCH "/me" with JSON:
      """
      {"locale":"pt-BR"}
      """
    Then the status is 200
    And the JSON at "/user/locale" is "pt-BR"

  Scenario: A javascript: shortcut address is refused
    Given a user Alice
    When Alice sends PATCH "/me" with JSON:
      """
      {"immich_url":"javascript:alert(1)"}
      """
    Then the status is 400
    When Alice sends PATCH "/me" with JSON:
      """
      {"immich_url":"https://photos.example.test/"}
      """
    Then the status is 200
    And the JSON at "/user/immich_url" is "https://photos.example.test"
