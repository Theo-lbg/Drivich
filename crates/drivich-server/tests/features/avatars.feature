# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Profile pictures

  Scenario: A profile picture is re-encoded, served and removed
    Given a user Alice
    And a user Bob
    When Alice sends GET "/me"
    Then the user Alice id is saved as "AID"
    When anonymous sends GET "/users/<AID>/avatar"
    Then the status is 401
    When Bob sends GET "/users/<AID>/avatar"
    Then the status is 404
    When Alice sends PUT "/me/avatar" with the raw body "this is not an image"
    Then the status is 400
    When Alice uploads a profile picture
    Then the status is 200
    And the JSON at "/user/avatar_version" is "1"
    When Bob sends GET "/users/<AID>/avatar"
    Then the status is 200
    And the response header "content-type" starts with "image/jpeg"
    When Alice sends DELETE "/me/avatar"
    Then the status is 200
    And the JSON at "/user/avatar_version" is "0"
    When Bob sends GET "/users/<AID>/avatar"
    Then the status is 404
