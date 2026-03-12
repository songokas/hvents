Feature: File change examples

  Scenario: Listen for changes
    Given execute "mkdir" with args "-p /tmp/_file_changes"
    And watch directory "/tmp/_file_changes" recursive

  Scenario: File created
    Given on file change "/tmp/_file_changes/a" when created
    Then print to stdout "File created"

  Scenario: File changed
    Given on file change "/tmp/_file_changes/a" when written
    Then print to stdout "File changed"

  Scenario: File removed
    Given on file change "/tmp/_file_changes/a" when removed
    Then print to stdout "File removed"

  Scenario: Write changes
    Given every "in 10 seconds"
    And execute "ls" with args "/tmp/"
    Then write file "/tmp/_file_changes/a"

  Scenario: Remove file every 15 seconds
    Given every "in 15 seconds"
    And execute "rm" with args "/tmp/_file_changes/a"
