Feature: MQTT examples

  Scenario: Subscribe with fuzzy match
    Given mqtt message from topic "sensors/+/temp" and body fuzzy matches "temperature"
    Then print to stdout "Temperature received: {{data}}"

  Scenario: Subscribe with fuzzy match any
    Given mqtt message from topic "sensors/+/status" and body fuzzy matches any "on,off,idle"
    Then print to stdout "Status: {{data}}"

  Scenario: Subscribe with body type and pool id
    Given mqtt message from topic "home/+/state" and body contains "active" using "secondary"
    Then print to stdout "Active: {{data}}"

  Scenario: Subscribe with pool id
    Given mqtt message from topic "home/alerts" using "alerts_pool"
    Then print to stdout "Alert: {{data}}"

  Scenario: Publish with retain
    Given every "in 5 seconds"
    Then mqtt publish to topic "home/status" with body "online" retain

  Scenario: Publish with pool id
    Given every "in 10 seconds"
    Then mqtt publish to topic "secondary/status" with body "ok" using "secondary"

  Scenario: Unsubscribe
    Given mqtt message from topic "home/shutdown"
    Then mqtt unsubscribe "home/alerts" using "alerts_pool"

  Scenario: Active during daytime only
    Given mqtt message from topic "home/sensor"
    When between "08:00" and "22:00"
    Then print to stdout "Daytime sensor: {{data}}"
