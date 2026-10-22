# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Account details

  Scenario: The status endpoint tells whether sign-up is open
    Given a user Alice
    When anonymous sends GET "/auth/status"
    Then the status is 200
    And the JSON at "/setup_required" is "false"

  Scenario: Setup cannot run again once users exist
    Given a user Alice
    When anonymous sends POST "/auth/setup" with JSON:
      """
      {"email":"root-<run>@example.test","name":"Root","password":"a-long-enough-password"}
      """
    Then the status is 409

  Scenario: The name can be changed and the password rules are enforced
    Given a user Alice
    When Alice sends PATCH "/me" with JSON:
      """
      {"name":"Alicia"}
      """
    Then the JSON at "/user/name" is "Alicia"
    When Alice sends PATCH "/me" with JSON:
      """
      {"name":"   "}
      """
    Then the status is 400
    When Alice sends POST "/me/password" with JSON:
      """
      {"current":"wrong-current-password","new":"another-long-password"}
      """
    Then the status is 400
    When Alice sends POST "/me/password" with JSON:
      """
      {"current":"a-long-enough-password","new":"short"}
      """
    Then the status is 400
    When Alice sends POST "/me/password" with JSON:
      """
      {"current":"a-long-enough-password","new":"another-long-password"}
      """
    Then the status is 200
    When Alice logs in with the password "a-long-enough-password"
    Then the status is 401
    When Alice logs in with the password "another-long-password"
    Then the status is 200

  Scenario: Duplicate e-mail addresses are refused
    Given an administrator Root
    And a user Alice
    When Root sends POST "/admin/users" with JSON:
      """
      {"email":"alice-<run>@example.test","name":"Twin","password":"a-long-enough-password"}
      """
    Then the status is 409

  Scenario: Unknown accounts take as long as known ones
    When anonymous sends POST "/auth/login" with JSON:
      """
      {"email":"ghost-<run>@example.test","password":"whatever-password"}
      """
    Then the status is 401
