# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Administration

  Scenario: A normal user cannot reach admin routes
    Given a user Bob
    When Bob sends GET "/admin/overview"
    Then the status is 403
    When Bob sends GET "/admin/backups"
    Then the status is 403
    When Bob sends GET "/admin/jobs/config"
    Then the status is 403

  Scenario: The administrator sees the overview
    Given an administrator Root
    When Root sends GET "/admin/overview"
    Then the status is 200
    And the JSON at "/database/ok" is "true"

  Scenario: The administrator creates a user who can then log in
    Given an administrator Root
    When Root sends POST "/admin/users" with JSON:
      """
      {"email":"carol-<run>@example.test","name":"Carol","password":"a-long-enough-password"}
      """
    Then the status is 200

  Scenario: Job kinds can be paused and resumed
    Given an administrator Root
    When Root sends PUT "/admin/jobs/config" with JSON:
      """
      {"paused":["thumbnail"],"limits":{"thumbnail":1}}
      """
    Then the status is 200
    And the JSON at "/paused/0" is "thumbnail"
    When Root sends PUT "/admin/jobs/config" with JSON:
      """
      {"paused":[],"limits":{}}
      """
    Then the status is 200
    And the JSON at "/paused" has 0 items

  Scenario: The backups listing is available to administrators
    Given an administrator Root
    When Root sends GET "/admin/backups"
    Then the status is 200

  Scenario: The administrator manages users
    Given an administrator Root
    And a user Dave
    When Dave sends GET "/me"
    Then the user Dave id is saved as "DID"
    When Root sends GET "/admin/users"
    Then the status is 200
    When Root sends PATCH "/admin/users/<DID>" with JSON:
      """
      {"disabled":true}
      """
    Then the status is 200
    When Dave logs in with the password "a-long-enough-password"
    Then the status is 401
    When Root sends PATCH "/admin/users/<DID>" with JSON:
      """
      {"disabled":false,"quota_bytes":1000000}
      """
    Then the status is 200
    When Root sends POST "/admin/users/<DID>/password" with JSON:
      """
      {"password":"a-brand-new-password"}
      """
    Then the status is 200
    When Dave logs in with the password "a-brand-new-password"
    Then the status is 200

  Scenario: Sign-up follows the admin setting
    Given an administrator Root
    When anonymous sends POST "/auth/signup" with JSON:
      """
      {"email":"eve-<run>@example.test","name":"Eve","password":"a-long-enough-password"}
      """
    Then the status is 403
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"allow_signup":true}
      """
    Then the status is 200
    When anonymous sends POST "/auth/signup" with JSON:
      """
      {"email":"eve-<run>@example.test","name":"Eve","password":"a-long-enough-password"}
      """
    Then the status is 200
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"allow_signup":false}
      """
    Then the status is 200

  Scenario: The server-wide Immich address is the default for users
    Given an administrator Root
    And a user Bob
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"immich_url":"https://default.example.test"}
      """
    Then the status is 200
    When Bob sends GET "/me"
    Then the JSON at "/immich_url" is "https://default.example.test"
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"immich_url":null}
      """
    Then the status is 200
