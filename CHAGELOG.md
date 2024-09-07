# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2024-09-07

### Added

- introduce evdev library for scancode reading (check evdev_events.yaml example)
- arm-unknown-linux-gnueabihf binary build

### Changed

- use debian buster for deb builds

### Fixed

- restore all time events instead of the ones defined in startup

## [0.3.0] - 2024-09-01

### Added

- state field for all events to manipulate data between events
- metadata field for all events to return additional information
- merge policy to control when data should be merged
- command event can replace arguments with the template data

### Changed

- reduce amount of required fields for yaml definitions
- renamed mqtt_publish.template to mqtt_publish.body
- renamed api_listen.template to api_listen.response_body

### Fixed

- prevent self referencing events
- event.data can be in yaml

## [0.2.0] - 2024-08-10

### Added

- period event - for allowing execution within period defined
- repeat event - repeat event at the time specified

### Changed

- time event contains only execution time
- time event does not reschedule

### Fixed

- time event, http event queue not removing events
- event queue do not block on command and api call events

## [0.1.1] - 2024-08-04

### Changed

- build time options

### Fixed

- sunset/sunrise parser

## [0.1.0] - 2024-08-03

### Added

- Initial project
