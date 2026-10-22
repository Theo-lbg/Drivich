# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Request hardening

  Scenario: A state-changing request without the CSRF header is refused
    Given a user Alice
    When Alice sends POST "/nodes/folder" without the CSRF header
    Then the status is 403

  Scenario: Responses carry baseline security headers
    When anonymous sends GET "/health"
    Then the response header "x-content-type-options" starts with "nosniff"
    And the response header "x-frame-options" starts with "SAMEORIGIN"
