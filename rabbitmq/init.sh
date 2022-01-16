#!/bin/sh

log() {
    # Logs a message using an easily grep-able format
    # Usage:
    # log [message]

    echo >&2 "*** init.sh: ${1-} ***"
}

create_user() {
    # Continuously creates Rabbitmq user until it succeeds
    # Usage:
    # create_user

    initial_delay="5"
    backoff="5"

    sleep "$initial_delay"
    log "Starting to configure RabbitMQ server."
    until try_create_user "$RABBITMQ_USER" "$RABBITMQ_PASSWORD"
    do
        log "Creating user failed; waiting $backoff seconds to try again."
        sleep 5
    done
    log "User '$RABBITMQ_USER' with password '$RABBITMQ_PASSWORD' completed."
    log "Log in the WebUI at port 15672 (example: http:/localhost:15672)"
}

try_create_user() {
    # Tries to create the RabbitMQ user once.
    # If successful, returns a zero exit code.
    # Otherwise, it should be retried.
    # Usage:
    # try_create_user [user] [password]

    user="$1"
    password="$2"
    # Use `&&\` at the end of each command
    # to make the function's return code non-zero if any of them fail,
    # and also for them to short-circuit
    # (i.e. if an earlier one fails,
    # the function returns early without execuitng the later ones)
    rabbitmqctl await_startup >/dev/null 2>&1 &&\
    rabbitmqctl add_user "$user" "$password" 2>/dev/null &&\
    rabbitmqctl set_user_tags "$user" administrator &&\
    rabbitmqctl set_permissions -p / "$password" ".*" ".*" ".*"
}

# $@ is used to pass arguments to the rabbitmq-server command.
# For example if you use it like this: docker run -d rabbitmq arg1 arg2,
# it will be as you run in the container rabbitmq-server arg1 arg2
log "Starting RabbitMQ server and waiting to configure."
(create_user & rabbitmq-server "$@")
