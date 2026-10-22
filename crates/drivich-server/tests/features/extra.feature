# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Limits, errors and odd corners

  Scenario: Quota and file size limits are enforced
    Given an administrator Root
    And a user Bob
    Then the user Bob id is saved as "BID"
    When Root sends PATCH "/admin/users/<BID>" with JSON:
      """
      {"quota_bytes":3}
      """
    And Bob sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"name":"a.bin","size":10}
      """
    Then the status is 507

  Scenario: Files above the configured size are refused
    Given the server accepts files up to 5 bytes
    And a user Alice
    When Alice sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"name":"a.bin","size":10}
      """
    Then the status is 413

  Scenario: A wrong upload offset is reported with the right one
    Given a user Alice
    When Alice sends POST "/uploads" with JSON:
      """
      {"parent_id":null,"name":"a.bin","size":10}
      """
    Then the JSON at "/upload/id" is saved as "U"
    When Alice sends PATCH "/uploads/<U>" with header "Upload-Offset" set to "5"
    Then the status is 409
    And the response header "upload-offset" starts with "0"

  Scenario: Proxy headers are used for throttling only when trusted
    Given the server trusts proxy headers
    And a user Alice
    When Alice logs in with the password "wrong-password-1" using header "X-Real-IP" set to "203.0.113.7"
    And Alice logs in with the password "wrong-password-2" using header "X-Forwarded-For" set to "203.0.113.8, 10.0.0.1"
    Then the status is 401

  Scenario: Blank documents of every kind
    Given a user Alice
    When Alice sends POST "/nodes/file" with JSON:
      """
      {"parent_id":null,"kind":"docx","name":"Report"}
      """
    Then the status is 200
    When Alice sends POST "/nodes/file" with JSON:
      """
      {"parent_id":null,"kind":"md","name":"Notes.md"}
      """
    Then the status is 200

  Scenario: Breadcrumbs, double trash and double restore
    Given a user Alice
    And Alice owns a folder "Docs" saved as "F"
    When Alice sends GET "/nodes/<F>/path"
    Then the JSON at "/items" has 1 items
    When Alice sends POST "/nodes/<F>/restore"
    Then the status is 200
    When Alice sends POST "/nodes/<F>/trash"
    And Alice sends POST "/nodes/<F>/trash"
    Then the status is 200
    When Alice sends PATCH "/nodes/<F>" with JSON:
      """
      {"starred":true}
      """
    Then the status is 400

  Scenario: Pagination of shared views and zip selections
    Given a user Alice
    And a user Bob
    And Alice owns a folder "One" saved as "A"
    And Alice owns a folder "Two" saved as "B"
    When Alice sends POST "/nodes/<A>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Alice sends POST "/nodes/<B>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Bob sends GET "/nodes?view=shared&limit=1&sort=created"
    Then the JSON at "/items" has 1 items
    And the JSON at "/next_cursor" is saved as "CUR"
    When Bob sends GET "/nodes?view=shared&limit=1&sort=created&cursor=<CUR>"
    Then the JSON at "/items" has 1 items
    When Bob sends GET "/nodes?view=shared&owner=me"
    Then the status is 200
    When Bob sends GET "/nodes?view=shared&owner=others&location=any"
    Then the status is 200
    When Bob sends GET "/nodes?view=shared&owner=nobody"
    Then the status is 400
    When Bob sends GET "/nodes/zip?ids="
    Then the status is 400
    When Alice sends GET "/nodes/zip?ids=<A>"
    Then the status is 200
    When Alice sends POST "/nodes/<A>/links" with JSON:
      """
      {}
      """
    And Alice sends POST "/nodes/<A>/links" with JSON:
      """
      {}
      """
    Then the status is 200

  Scenario: Public listings paginate
    Given a user Alice
    And Alice owns a folder "Docs" saved as "F"
    And Alice owns a file "a.txt" containing "a" in folder "F" saved as "A"
    And Alice owns a file "b.txt" containing "b" in folder "F" saved as "B"
    When Alice sends POST "/nodes/<F>/links" with JSON:
      """
      {}
      """
    Then the JSON at "/token" is saved as "T"
    When anonymous sends GET "/public/<T>/nodes/<F>/children?limit=1"
    Then the JSON at "/next_cursor" is not null

  Scenario: Backups can be downloaded by administrators
    Given an administrator Root
    And a user Bob
    And a database backup exists
    When Root sends GET "/admin/backups/drivich-20260101-000000.dump/download"
    Then the status is 200
    And the response body is "dump"
    When Bob sends GET "/admin/backups/drivich-20260101-000000.dump/download"
    Then the status is 403

  Scenario: Avatars support revalidation
    Given a user Alice
    And a user Bob
    When Alice sends GET "/me"
    Then the user Alice id is saved as "AID"
    When Alice uploads a profile picture
    And Bob sends GET "/users/<AID>/avatar"
    Then the response header "etag" is saved as "ET"
    When Bob sends GET "/users/<AID>/avatar" with header "If-None-Match" set to "<ET>"
    Then the status is 304
