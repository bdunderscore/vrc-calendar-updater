Set-StrictMode -Version latest
$ErrorActionPreference = "Stop"

docker build -t calbuild -f Dockerfile.build .
if ($lastexitcode -ne 0) {
    throw ("Build failure")
}
docker rm calbuild
docker run --name calbuild -t calbuild true
docker cp calbuild:/calendar-updater-src/target/release/calendar-updater .
if ($lastexitcode -ne 0) {
    throw ("Build failure")
}

docker build -t calendar-updater -f Dockerfile.deploy .
if ($lastexitcode -ne 0) {
    throw ("Build failure")
}
