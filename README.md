This repository contains the backend code for generating the textures that power
the [VRC Scroll
Calendar](https://github.com/bdunderscore/vrchat-scroll-calendar).

While I kept meaning to clean this up and get it to a state where it was a bit more maintainable, I've not found the time, so, well, here's my prototype. Perhaps it will be of some use to you...?

I've written some quick docs on the format in [docs/format.md](docs/format.md).

# Usage

```
cargo build --release

export TZ=Japan # or your timezone

# Recommended debugging options
export RUST_LOG='debug,hyper::proto::h1::io=info,hyper::proto::h1::decode=info,calendar_updater=trace'
export RUST_BACKTRACE=1

./target/release/calendar-updater -b branch-name -t template.png -h header.png -o output.png
```

"branch-name" is displayed in the bottom debug text on the calendar.

template.png contains a full-scale template image of the calendar. This should be 1024x1447 pixels in size; if you want to adjust this, you'll probably need to adjust the constants in src/config.rs.

header.png contains a transparent sample of the date header that is used to divide events up by date.

The calendar URL is currently hardcoded in src/calendar.rs to point to Kakkou's VRChat Event Calendar, via the google calendar ICS export.

## Misc scripts

I've also included some of the scripts I've been using to maintain the backend.

* build.ps1 - builds a docker container containing the updater (see also Dockerfile.build, Dockerfile.deploy)
* dryrun.ps1 - builds a test calendar. You'll need to update the paths in dryrun.ps1 to use this.
* upload.sh - Generates and uploads the calendar to bunnycdn.

Currently, the updater itself is deployed to AWS Fargate.