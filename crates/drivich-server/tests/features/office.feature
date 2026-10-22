# SPDX-License-Identifier: AGPL-3.0-or-later
# Copyright (c) 2026 Theo-lbg

Feature: Editing Office documents with ONLYOFFICE

  Background:
    Given a user Alice
    And a user Bob
    And Alice owns a file "Rapport.docx" containing "original" saved as "D"

  Scenario: Editing is refused when it is not enabled
    Given document editing is disabled
    When Alice sends GET "/nodes/<D>/office"
    Then the status is 400
    When Alice sends POST "/nodes/<D>/office/title"
    Then the status is 400
    When anonymous sends GET "/office/file/whatever"
    Then the status is 400

  Scenario: The owner gets an editable configuration, a viewer a read-only one
    When Alice sends GET "/nodes/<D>/office"
    Then the status is 200
    And the JSON at "/server" is "/onlyoffice"
    And the JSON at "/config/editorConfig/mode" is "edit"
    And the JSON at "/config/documentType" is "word"
    And the JSON at "/config/token" is not null
    When Alice sends POST "/nodes/<D>/shares" with JSON:
      """
      {"email":"bob-<run>@example.test","role":"viewer"}
      """
    And Bob sends GET "/nodes/<D>/office"
    Then the JSON at "/config/editorConfig/mode" is "view"
    When Bob sends POST "/nodes/<D>/office/title"
    Then the status is 403

  Scenario: Only documents can be opened
    Given Alice owns a folder "Dossier" saved as "F"
    And Alice owns a file "data.bin" containing "x" saved as "B"
    When Alice sends GET "/nodes/<F>/office"
    Then the status is 400
    When Alice sends GET "/nodes/<B>/office"
    Then the status is 400
    When Alice sends POST "/nodes/<F>/office/title"
    Then the status is 400
    When Bob sends GET "/nodes/<D>/office"
    Then the status is 404

  Scenario: Spreadsheets and presentations map to their editors
    Given Alice owns a file "t.xlsx" containing "x" saved as "X"
    And Alice owns a file "s.pptx" containing "x" saved as "P"
    When Alice sends GET "/nodes/<X>/office"
    Then the JSON at "/config/documentType" is "cell"
    When Alice sends GET "/nodes/<P>/office"
    Then the JSON at "/config/documentType" is "slide"

  Scenario: The Document Server downloads the file with its signed link
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/document/url" is saved as "FT"
    When anonymous sends GET "/office/file/<FT>"
    Then the status is 200
    And the response body is "original"
    When anonymous sends GET "/office/file/not-a-token"
    Then the status is 401
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    When anonymous sends GET "/office/file/<CB>"
    Then the status is 401

  Scenario: A saved document becomes a new version
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    And the user Alice id is saved as "AID"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/doc","users":["<AID>"]}
      """
    Then the JSON at "/error" is "0"
    And the document "D" contains "edited by the Document Server"
    When Alice sends GET "/nodes/<D>/versions"
    Then the JSON at "/items" has 1 items

  Scenario: The signature may travel in the body and the editor defaults to the owner
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    When anonymous sends POST "/office/callback/<CB>" with JSON carrying its own signature:
      """
      {"status":6,"url":"https://drive.example/onlyoffice/cache/doc"}
      """
    Then the JSON at "/error" is "0"
    And the document "D" contains "edited by the Document Server"

  Scenario: Statuses that carry nothing are acknowledged
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":1}
      """
    Then the JSON at "/error" is "0"
    And the document "D" contains "original"

  Scenario: Bad callbacks are reported as errors and change nothing
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":2}
      """
    Then the JSON at "/error" is "1"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":2,"url":"not a url"}
      """
    Then the JSON at "/error" is "1"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/empty"}
      """
    Then the JSON at "/error" is "1"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed by the Document Server:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/missing"}
      """
    Then the JSON at "/error" is "1"
    And the document "D" contains "original"

  Scenario: Unsigned, wrongly signed or mistyped callbacks are refused
    When Alice sends GET "/nodes/<D>/office"
    Then the last path segment of the JSON at "/config/editorConfig/callbackUrl" is saved as "CB"
    And the last path segment of the JSON at "/config/document/url" is saved as "FT"
    When anonymous sends POST "/office/callback/<CB>" with JSON:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/doc"}
      """
    Then the JSON at "/error" is "1"
    When anonymous sends POST "/office/callback/<CB>" with JSON signed with the wrong secret:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/doc"}
      """
    Then the JSON at "/error" is "1"
    When anonymous sends POST "/office/callback/<FT>" with JSON signed by the Document Server:
      """
      {"status":2,"url":"https://drive.example/onlyoffice/cache/doc"}
      """
    Then the JSON at "/error" is "1"
    And the document "D" contains "original"

  Scenario: Renaming works even when the Document Server is unreachable
    When Alice sends POST "/nodes/<D>/office/title"
    Then the JSON at "/ok" is "true"
    Given the Document Server is unreachable
    When Alice sends POST "/nodes/<D>/office/title"
    Then the JSON at "/ok" is "true"
