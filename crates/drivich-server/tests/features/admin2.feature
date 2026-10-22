# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: More administration

  Background:
    Given an administrator Root
    And a user Bob

  Scenario: Settings and job controls can be read
    When Root sends GET "/admin/settings"
    Then the status is 200
    When Root sends GET "/admin/jobs/config"
    Then the status is 200
    When Root sends POST "/admin/jobs/retry"
    Then the status is 200
    When Root sends POST "/admin/backups"
    Then the status is 200
    And the JSON at "/queued" is "true"

  Scenario: The default quota is validated
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"default_quota_bytes":-5}
      """
    Then the status is 400
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"default_quota_bytes":1000000000}
      """
    Then the status is 200
    When Root sends PUT "/admin/settings" with JSON:
      """
      {"default_quota_bytes":null}
      """
    Then the status is 200

  Scenario: Job limits are validated
    When Root sends PUT "/admin/jobs/config" with JSON:
      """
      {"paused":[],"limits":{"thumbnail":99}}
      """
    Then the status is 400

  Scenario: Backups cannot be downloaded when absent or badly named
    When Root sends GET "/admin/backups/drivich-20200101-000000.dump/download"
    Then the status is 404
    When Root sends GET "/admin/backups/evil.sh/download"
    Then the status is 404

  Scenario: An administrator cannot demote or disable themselves
    When Root sends GET "/me"
    Then the user Root id is saved as "RID"
    When Root sends PATCH "/admin/users/<RID>" with JSON:
      """
      {"is_admin":false}
      """
    Then the status is 400
    When Root sends PATCH "/admin/users/<RID>" with JSON:
      """
      {"disabled":true}
      """
    Then the status is 400
    When Root sends PATCH "/admin/users/<RID>" with JSON:
      """
      {"quota_bytes":-1}
      """
    Then the status is 400
    When Root sends PATCH "/admin/users/<RID>" with JSON:
      """
      {"quota_bytes":null,"is_admin":true}
      """
    Then the status is 200
