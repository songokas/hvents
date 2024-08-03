#!/bin/bash

set -eEu -o pipefail

trap "jobs -p | xargs -r kill" SIGINT
trap '[[ $? != 0 ]] && jobs -p | xargs -r kill' EXIT

BIN="./target/debug/hvents"

assert_file_content() {
    cmp -s "$1" <(echo -n "$2") || (echo "`cat "$1"` != $2" && diff -u "$1" <(echo -n "$2") && exit 1)
}

test_reschedule() {
    echo "test_reschedule"
    local data="test_reschedule"
    local file="/tmp/_test_write1"
    local config=$(cat <<EOF
events:
    run_in_seconds:
        time:
            execute_time: in 5 seconds
        data: $data
        next_event: write_to_file
    write_to_file:
        file_write:
            file: $file
            mode: append
start_with:
    - run_in_seconds
EOF
)

    local pids=()

    rm "$file" || true

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 5
    assert_file_content "$file" "$data"
    sleep 5
    assert_file_content "$file" "$data$data"
    kill "$pids"

}

test_schedule_on_crash() {
    echo "test_schedule_on_crash"
    local data="test_schedule_on_crash"
    local file="/tmp/_test_write2"
    local restoreDir="/tmp/_data_hevents"
    local config=$(cat <<EOF
events:
    run_in_seconds:
        time:
            execute_time: in 5 seconds
        data: $data
        next_event: write_to_file
    write_to_file:
        file_write:
            file: $file
            mode: append
start_with:
    - run_in_seconds
restore: $restoreDir
EOF
)

    local pids=()

    rm "$file" || true
    rm -rf "$restoreDir" || true

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 2
    kill "$pids"
    if [ -f "$file" ]; then
        echo "File should not exist"
        exit 1
    fi

    local pids=()
    $BIN <(echo -n "$config") & pids+=($!)
    sleep 3
    assert_file_content "$file" "$data"
    kill "$pids"
}

test_mqtt() {
    echo "test_mqtt"
    local data="Hi PeterHi DavidJohn"
    local file="/tmp/_test_write3"
    local config=$(cat <<EOF
events:
    subscribe:
        mqtt_subscribe:
            topic: test/people-names/#
            contains_string: Hi
        next_event: write_to_file
    publish_peter:
        mqtt_publish:
            topic: test/people-names/peter
            template: Hi Peter
        next_event: publish_john
    publish_john:
        mqtt_publish:
            topic: test/people-names/john
        data: John
        next_event: publish_david
    publish_david:
        mqtt_publish:
            topic: test/people-names/david
        data: Hi David
    write_to_file:
        file_write:
            file: $file
            mode: append
start_with:
    - subscribe
    - publish_peter
mqtt:
    default:
        host: localhost
EOF
)

    local pids=()

    rm "$file" || true

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 5
    assert_file_content "$file" "$data"
    kill "$pids"

}

test_api() {
    echo "test_api"
    local file="/tmp/_test_write_api"
    local config=$(cat <<EOF
events:
    call_endpoint_a:
        api_call:
            url: http://127.0.0.1:8911/clients/1
            method: post
            request_content: text
            response_content: text
        data: call_endpoint_a
        next_event: write_to_file
    call_endpoint_b:
        api_call:
            url: http://127.0.0.1:8912/clients/1
            method: post
            request_content: text
            response_content: text
        data: call_endpoint_b
        next_event: write_to_file
    return_response_a:
        api_listen:
            path: /clients/1
            method: post
            request_content: text
            response_content: text
        data: return_response_a
        next_event: write_to_file
    return_response_b:
        api_listen:
            path: /clients/1
            method: post
            request_content: text
            response_content: text
            pool_id: b
        data: return_response_b
        next_event: write_to_file
    
    write_to_file:
        file_write:
            file: $file
            mode: append
http:
    a: 127.0.0.1:8911
    b: 127.0.0.1:8912
start_with:
    - return_response_a
    - return_response_b
    - call_endpoint_a
    - call_endpoint_b
EOF
)

    local pids=()

    rm "$file" || true

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 5
    assert_file_content "$file" "call_endpoint_areturn_response_acall_endpoint_areturn_response_acall_endpoint_breturn_response_bcall_endpoint_breturn_response_b"
    kill "$pids"
}

test_file_changes() {
    echo "test_file_changes"
    local dir="/tmp/_test_file_changes"
    local file="$dir/_content"
    local config=$(cat <<EOF
events:
    file_a_changed:
        file_changed:
            path: $dir/a
            when: created
        data: file_a_changed
        next_event: write_to_file
    file_b_changed:
        file_changed:
            path: $dir/b
            when: written
        data: file_b_changed
        next_event: write_to_file
    file_b_removed:
        file_changed:
            path: $dir/b
            when: removed
        data: file_b_removed
        next_event: write_to_file
    watch_dir:
        watch:
            path: $dir
        data: watch_dir
        next_event: write_to_file
    write_to_file:
        file_write:
            file: $file
            mode: append
start_with:
    - watch_dir
EOF
)

    local pids=()

    rm -rf "$dir"
    mkdir -p "$dir"

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 1
    touch $dir/a
    echo "content" > $dir/b
    rm $dir/b
    sleep 3
    assert_file_content "$file" "watch_dirfile_a_changedfile_b_changedfile_b_removed"
    kill "$pids"

}

test_command() {
    echo "test_command"
    local file="/tmp/_test_write_command"
    local data="`rustc --version`
"
    local config=$(cat <<EOF
events:
    run_command:
        execute:
            command: rustc
            args: [--version]
        next_event: write_to_file
    write_to_file:
        file_write:
            file: $file
            mode: append
start_with:
    - run_command
EOF
)

    local pids=()

    rm "$file" || true

    $BIN <(echo -n "$config") & pids+=($!)
    sleep 1
    assert_file_content "$file" "$data"
    kill "$pids"
}

test() {
    cargo build
    cargo test
    test_command
    test_file_changes
    test_api
    test_mqtt
    test_reschedule
    test_schedule_on_crash
}

"${1:-"test"}"
