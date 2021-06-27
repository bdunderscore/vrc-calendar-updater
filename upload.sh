#!/bin/bash

export RUST_LOG='debug,hyper::proto::h1::io=info,hyper::proto::h1::decode=info,calendar_updater=trace'
export TZ=Japan
export RUST_BACKTRACE=1

set -euxo pipefail

echo AccessKey:${BCDN_APIKEY:-UNSET} > /root/.bcdn_apikey
echo AccessKey:${BCDN_STORAGEKEY:-UNSET} > /root/.bcdn_storagekey

BRANCH_NAME=${BRANCH_NAME:-TEST}

cd /calendar-updater

./calendar-updater -t template.png -h header.png -o calendar-$BRANCH_NAME.png -b calendar-$BRANCH_NAME || echo "Updater failed: $?"

if [ "${DRYRUN:-}" != "" ]; then
    cp calendar-$BRANCH_NAME.png /output/dryrun.png
    exit 0
fi

if [ "${BCDN_STORAGE_ENDPOINT:-}" != "" ]; then
    curl -f -XPUT -H@/root/.bcdn_storagekey $BCDN_STORAGE_ENDPOINT/calendar-$BRANCH_NAME.png -T calendar-$BRANCH_NAME.png
    echo
    curl -f -XPOST -H@/root/.bcdn_apikey https://bunnycdn.com/api/pullzone/$BDCN_PULLZONE_ID/purgeCache -T /dev/null
    echo
fi

aws s3 cp --acl public-read calendar-$BRANCH_NAME.png s3://vrc-calendar.fushizen.net/calendar-$BRANCH_NAME.png
