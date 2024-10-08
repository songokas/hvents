# About

Very simple event automation system to manage home events easily by defining it in a configuration file.

Supports:

* time events
* mqtt events
* http events
* file events
* external commands

# How to install

## Install deb

amd64

```
wget https://github.com/songokas/hvents/releases/download/v0.3.1/hvents_0.3.1_amd64.deb \
  && sudo apt install ./hvents_0.3.1_amd64.deb
```

armhf

```
wget https://github.com/songokas/hvents/releases/download/v0.3.1/hvents_0.3.1_armhf.deb \
  && sudo apt install ./hvents_0.3.1_armhf.deb
```

## Download binary

https://github.com/hvens/hvents/releases


## Install from source

```bash
cargo install --bins --root=. --git=https://github.com/songokas/hvents
```

# How to configure and run

## Start with a minimal configuration

```yaml
# events.yaml
events:
    schedule_print:
        time: in 5 seconds
        data: Executed every 5 seconds
        next_event: print_to_stdout
    print_to_stdout:
        print: stdout

start_with:
  - schedule_print
```

Run

```
hvents events.yaml
```

Add more events as needed

## Configuration options

Create a global configuration file:

```yaml
# events.yaml

# events are loaded from specified files with prefix from the key
# optional
groups:
  hall: events/hall.yaml
  weather: events/weather.yaml

# events are loaded from specified files
# optional
event_files:
    - doors.yaml

# events defined in the same configuration file
# optional
events:
    movement:
        mqtt_subscribe:
            topic: security/hall/movement
            body: "True"
        next_event: light_on
    light_on:
        mqtt_publish:
            topic: cmnd/hall/Power
            body: on

# specify which events to start with
start_with:
  - movement

# configure mqtt clients
# optional
mqtt:
  default: # pool_id - defines which client to use for mqtt events
    host: host
    port: 1883 # optional
    user: user # optional
    pass: pass # optional
    client_id: homeevents # optional

# host and port to listen on for api_listen events
# optional
http:
    # default is the pool id used for api_listen events
    default: 127.0.0.1:8991 

# restore events from the directory specified, between startups
# optional, no restore by default
restore: data/

# specify location for sunrise, sunset calculations
# optional
location:
    latitude: 52.37403
    longitude: 4.88969

# specify devices to read scancodes from
# optional
devices:
    default: /dev/input/event0
```

## Run 

### Manually

```bash
hvents events.yaml
```

### With systemd

Working directory /opt/hvents

```bash
systemctl start hvents
```

## Available events

### Publish to mqtt topic

Publish to topic with body from even.data

```yaml
  mqtt_publish: announce/back-door
```

```yaml
  mqtt_publish:
    topic: announce/back-door
    body: back door open # optional event.data will be used if template is not defined
    pool_id: default # optional client to use for publishing events
```

Publish event can use handlebar templates to define a body as well

```yaml
  mqtt_publish:
    topic: announce/weather
    body: '{{#each forecastTimestamps}}{{#if (eq forecastTimeUtc (date-time-format ../forecastToShow "%Y-%m-%d %H:%M:%S"))}}Air temperature {{airTemperature}} degrees{{/if}}{{/each}}'
```

### Subscribe to mqtt topic


```yaml
  mqtt_subscribe: security/back-door/open
```

Mqtt request body must match exactly

```yaml
  mqtt_subscribe:
    topic: security/back-door/open
    body: "True"
    pool_id: default # optional, client to use for publishing events
```

Mqtt request body must contain a string to match

```yaml
  mqtt_subscribe:
    topic: security/back-door/open
    body_contains: "special string"
```

### Read from file

```yaml
  file_read: /tmp/file
```

```yaml
  file_read: 
    file: /tmp/file
    # options: string,json,bytes
    # optional
    data_type: string
```

### Write to file

File will be written with data provided by the previous event or event.data defined in its own configuration

```yaml
  file_write: /tmp/file
```

```yaml
  file_write:
    file: /tmp/file
    # options: truncate,append
    mode: truncate # default
```

### Call API endpoint

```yaml
    api_call: https://api.meteo.lt/v1/places/vilnius/forecasts/long-term
```

```yaml
    api_call: 
        url: https://api.meteo.lt/v1/places/vilnius/forecasts/long-term
        # optional
        headers:
            X-HEADER: value
        # options: get,post,put,delete
        method: get # optional
        # options: json,text,bytes
        request_content: json # optional
        # options: json,text,bytes
        response_content: json # optional
```

 ### Listen for API call

 Listen for an http call

 event.data or response_body can be used to control what to return as a response

```yaml
    api_listen:
        path: /clients/1
        # options: get,post,put,delete
        method: get # optional
        # options: json,text,bytes
        request_content: json # optional
        # options: json,text,bytes
        response_content: json # optional
        # response template to be rendered 
        response_body: "{{client_id}}" #optional
        pool_id: default # optional references which http server handles the request
```

Keys available in a response body template:

- request
- url
- segments (http request url split by /)
- data

### File changes

```yaml
    file_changed:
        path: /tmp/a
        # options: created, written, removed
        when: created # optional
    watch:
        path: /tmp
        # options: start, stop
        action: start # optional
        recursive: false # optional
```

### Schedule at specific time

Execute event at 8:00:00

```yaml
  time: 8:00
```

Execute event at 8:00:00 with event id

```yaml
  time:
    execute_time: 8:00
    event_id: time_events # event id can be used to overwrite a previous event with the same id
```

Scheduling the same event will overwrite the previous event.

All times are in local timezone.

Available date time format can be found on https://lib.rs/crates/human-date-parser#readme-formats

Additional formats supported:
* sunset
* sunset in 1 hours
* sunrise
* sunrise in 20 seconds

### Schedule at specific time and repeat

Execute event at 8:00:00 and repeat tomorrow 8:00:00

```yaml
  repeat: 8:00
```

### Allow event only for specific times

Allow event execution only at specific times

```yaml
  period: 
    from: 8:00
    to: 10:00
```

### Execute command

Execute external command

Command takes input from the previous event data

```yaml
  execute:
    command: date
    # optional
    args: ["--utc"]
    # render template and replace arguments by index
    # optional
    replace_args:
        0: "--local"
    # options: string,json,bytes
    # optional
    data_type: string
    # provide environment variables
    # optional
    vars:
        ENV_VARIABLE_KEY: value 
```

### Read scan codes from the device

```yaml
  scan_code_read: 0x7a1a
```

devices needs to be defined globally

## Template data

Unless otherwise stated per command keys available in templates

- data
- metadata
- state

## Event references and data

Each event can reference next event and define data, which is merged together
as it goes through the chain

example:

```yaml
subscribe:
  mqtt_subsribe:
    topic: weather/data
    body_contains: "forecast"
  next_event: schedule_writing
schedule_writing:
  time: 8:00
  next_event: write_to_file
  data: schedule_writing_data
write_to_file:
  file_write: /tmp/test3
  data: write_to_file_data
```

would write

```
#/tmp/test3
forecast 22.2
schedule_writing_datawrite_to_file_data
```

## Event examples

```yaml
# events/hall.yaml
movement:
  mqtt_subscribe:
    topic: security/hall/movement
    body: "True"
  next_event: schedule_light_on

schedule_light_on:
  period:
      from: 23:00
      to: 05:00
  next_event: light_on

schedule_light_off:
  time: in 20 seconds
  next_event: light_off

light_on:
  mqtt_publish:
    topic: cmnd/hall/Power
    body: on
  next_event: schedule_light_off

light_off:
  mqtt_publish:
    topic: cmnd/hall/Power
    body: off
```

```yaml
# events/weather.yaml
schedule:
  time: 07:59
  next_event: retrieve
retrieve:
  api_call: 
    url: https://api.meteo.lt/v1/places/vilnius/forecasts/long-term
  next_event: store_file
store_file:
  file_write: events/data.json
  next_event: announce
announce_8:
  time: 8:00
  next_event: announce_from_file
  data: {"forecastToShow":"today 8:00:00"}
announce_from_file:
  file_read:
    file: events/data.json
  next_event: announce
announce:
  mqtt_publish:
    topic: announce/weather
    body: '{{#each forecastTimestamps}}{{#if (eq forecastTimeUtc (date-time-format ../forecastToShow "%Y-%m-%d %H:%M:%S"))}}Air temperature {{airTemperature}} degrees{{/if}}{{/each}}'
```

```yaml
# events/doors.yaml
door_front_open:
  mqtt_subscribe:
    topic: security/front-door/open
    body: "True"
  next_event: announce_front
door_back_open:
  mqtt_subscribe:
    topic: security/back-door/open
    body: "True"
  next_event: announce_back
door_announce_front:
  mqtt_publish:
    topic: announce/front-door
    body: front door open
door_announce_back:
  mqtt_publish:
    topic: announce/back-door
    body: back door open
```
