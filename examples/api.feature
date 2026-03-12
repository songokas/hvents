Feature: API request examples

  Scenario: Listen for an api call
    Given http "/status" is called respond with "OK"
    Then print to stdout "Status called"

  Scenario: Listen for an api call
    Given http "/api" is called with post and text respond with "Hey"
    Then print to stdout "Data received {{data}}"

  Scenario: Fetch status
    Given every "in 10 seconds"
    Then http get "http://127.0.0.1:23323/status"
    And print to stdout "Response received from status {{data}}"

  Scenario: Send request
    Given every "in 12 seconds"
    Then http post "Hello" to "http://127.0.0.1:23323/api" expect text
    And print to stdout "Response received from api {{data}}"
