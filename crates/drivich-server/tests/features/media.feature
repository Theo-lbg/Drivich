# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Thumbnails, previews and inline content

  Background:
    Given a user Alice
    And a user Bob

  Scenario: Thumbnails appear once the worker made them
    Given Alice owns an image "pic.png" saved as "P"
    When Alice sends GET "/nodes/<P>/thumbnail"
    Then the status is 404
    Given the worker has made the thumbnail of "P"
    When Alice sends GET "/nodes/<P>/thumbnail"
    Then the status is 200
    And the response header "content-type" starts with "image/jpeg"
    And the response header "etag" is saved as "ET"
    When Alice sends GET "/nodes/<P>/thumbnail" with header "If-None-Match" set to "<ET>"
    Then the status is 304
    When Bob sends GET "/nodes/<P>/thumbnail"
    Then the status is 404

  Scenario: Folders have no thumbnail
    Given Alice owns a folder "Docs" saved as "F"
    When Alice sends GET "/nodes/<F>/thumbnail"
    Then the status is 400
    When Alice sends GET "/nodes/<F>/download"
    Then the status is 400

  Scenario: Office previews
    Given Alice owns a file "Rapport.docx" containing "PK" saved as "D"
    And Alice owns a file "notes.txt" containing "hello" saved as "T"
    And Alice owns a folder "Docs" saved as "F"
    When Alice sends POST "/nodes/<D>/preview"
    Then the JSON at "/state" is "pending"
    When Alice sends POST "/nodes/<D>/preview"
    Then the JSON at "/state" is "pending"
    When Alice sends GET "/nodes/<D>/preview"
    Then the status is 404
    Given the worker has made the PDF preview of "D"
    When Alice sends GET "/nodes/<D>/preview"
    Then the status is 200
    And the response header "content-type" starts with "application/pdf"
    And the response header "etag" is saved as "ET"
    When Alice sends GET "/nodes/<D>/preview" with header "If-None-Match" set to "<ET>"
    Then the status is 304
    When Alice sends POST "/nodes/<T>/preview"
    Then the JSON at "/state" is "unsupported"
    When Alice sends POST "/nodes/<F>/preview"
    Then the status is 400

  Scenario: Inline content policy and range edge cases
    Given Alice owns a file "logo.svg" containing "<svg/>" saved as "S"
    And Alice owns a file "data.json" containing "{}" saved as "J"
    And Alice owns a file "notes.txt" containing "0123456789" saved as "T"
    And Alice owns a file "blob.bin" containing "x" saved as "B"
    When Alice sends GET "/nodes/<S>/content"
    Then the status is 200
    And the response header "content-security-policy" starts with "sandbox"
    When Alice sends GET "/nodes/<J>/content"
    Then the response header "content-type" starts with "application/json"
    When Alice sends GET "/nodes/<B>/content"
    Then the response header "content-disposition" starts with "attachment"
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=-4"
    Then the response body is "6789"
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=-0"
    Then the status is 416
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=abc-"
    Then the status is 416
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "lines=1-2"
    Then the response body is "0123456789"
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=0-1,3-4"
    Then the response body is "0123456789"
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=5"
    Then the response body is "0123456789"
    When Alice sends GET "/nodes/<T>/download" with header "Range" set to "bytes=3-1"
    Then the status is 416
