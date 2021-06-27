#./build.ps1
$pwd = Get-Location
$pwd = $pwd -replace "\\", "/"
docker run -i -e DRYRUN=yes -v "c:/users/bd/unity-2/calendar/Assets:/output" calendar-updater