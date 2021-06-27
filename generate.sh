#!/bin/sh

export RUST_LOG='debug,hyper::proto::h1::io=info,hyper::proto::h1::decode=info,calendar_updater=trace'
export TZ=Japan
export RUST_BACKTRACE=1

VARIANT=$1
shift


./target/$VARIANT/calendar-updater -t template.png -h header.png "$@"
