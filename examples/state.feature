Feature: Home temperature

  Scenario: Turn on ac periodically
    Given every "in 20 seconds"
    When state key "<room>_temperature" is below "<temperature>"
    And state key "<room>_internal_state" is equal to "off"
    Then mqtt publish to topic "rooms/<room>/set" with body "1"
    And replace state key "<room>_internal_state" with value "on"
    And print to stdout "Ac is on"

    Examples:
      | room     | temperature |
      | bathroom |          20 |
      | ballroom |          22 |

  Scenario: Turn off ac periodically
    Given every "in 20 seconds"
    When state key "<room>_temperature" is above "20"
    And state key "<room>_internal_state" is equal to "on"
    Then mqtt publish to topic "rooms/<room>/set" with body "0"
    And replace state key "<room>_internal_state" with value "off"
    And print to stdout "Ac is off"

    Examples:
      | room     |
      | bathroom |
      | ballroom |

  Scenario: Store temperatures
    Given mqtt message from topic "temperatures/+/get"
    Then replace state key "{{ metadata.topic_segments.1 }}_temperature" with value "{{ data }}"

  Scenario: Store power status
    Given mqtt message from topic "rooms/+/get"
    Then replace state key "{{ metadata.topic_segments.1 }}_external_state" with value "{{ data }}"
    And once "in 1 hour"
    And replace state key "{{ metadata.topic_segments.1 }}_internal_state" with value "{{ data }}"

  Scenario: Send temperature periodically
    Given periodically "in 10 seconds"
    Then execute "shuf" with args "-i 10-22 -n 1"
    Then mqtt publish "{{data}}" to "temperatures/bathroom/get"
    And print to stdout "{{#each state}}{{@key}}={{this}} {{/each}}"
