#!/bin/bash

#  Copyright 2020-2021 bd_
# 
#  Permission is hereby granted, free of charge, to any person obtaining a copy
#  of this software and associated documentation files (the "Software"), to deal
#  in the Software without restriction, including without limitation the rights
#  to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
#  copies of the Software, and to permit persons to whom the Software is
#  furnished to do so, subject to the following conditions: The above copyright
#  notice and this permission notice shall be included in all copies or
#  substantial portions of the Software.
# 
#  THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
#  IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
#  FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
#  AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
#  LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
#  OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
#  SOFTWARE.

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
