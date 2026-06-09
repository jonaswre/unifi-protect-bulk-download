# Unifi-Protect footage bulk download tool
This CLI-tool allows you to download all footage from your Unifi-Protect NVR. It is written in Rust and uses the [unifi-protect](https://github.com/xlfpx/unifi-protect-rust) crate to communicate with the Unifi-Protect API.

# Installation
1. Install rust & cargo if not installed: https://rust-lang.org/tools/install
2. Install this CLI-tool: `cargo install unifi-protect-bulk-download`

### Docker
Alternatively, you can also use Docker to run the tool without installing Rust: `docker run -it unifiprotect/unifi-protect-bulk-download download`

# Usage
`unifi_protect_bulk_download download <uri> <username> <password> <path> <mode> <recording_type> <start_date> <end_date> [cameras]`

Arguments:
- \<uri>             The uri of the unifi protect server
- \<username>        The username for logging into the unifi protect server
- \<password>        The password for logging into the unifi protect server
- \<path>            The path to the directory to download the files to
- \<mode>            The mode to download the files in (daily or hourly) [possible values: daily, hourly]
- \<recording_type>  The type of recording to download (rotating or timelapse) [possible values: rotating, timelapse]
- \<start_date>      The start date/time to download files from (YYYY-MM-DD or YYYY-MM-DD-HH)
- \<end_date>        The end date/time to download files to (YYYY-MM-DD or YYYY-MM-DD-HH)
- \[cameras]         Optional comma-separated list of camera names or camera ids to download. Omit it, use `all`, or use `*` to download every camera.

Options:
- `--hours <START-END>` limits each day to an end-exclusive hour window, for example `--hours 07-19` downloads from 07:00 up to, but not including, 19:00 each day.
- `--timelapse-factor <FACTOR>` sets the timelapse speed factor. Supported values are `60x`, `120x`, `300x`, and `600x`. The default is `60x`.
- `--timelapse-duration <DURATION>` targets a fixed output duration for each timelapse export, for example `--timelapse-duration 5m`. Supported units are `s`, `m`, and `h`. This overrides `--timelapse-factor`.
- `--force` overwrites existing output files. Without this flag, existing files are skipped.


# Example
For example, to download all footage from your Unifi-Protect NVR, for all cameras, for the months of June and July 2023, run the following command:
```bash
download https://<Unifi-Protect-IP-Addr> <username> <password> /path/to/destination/folder daily rotating 2023-06-01 2023-07-31
```
In the above example, replace:
1. __\<Unifi-Protect-IP-Addr\>__ with the IP-Address of your unifi-protect system
2. __\<username\>__ with the username of your unifi-protect account
3. __\<password\>__ with the password of your unifi-protect account
4. __/path/to/destination/folder__ with the path to the folder where you want to download the footage to
5. __daily__ with __hourly__ in case you want one video per camera per hour, rather than per day of footage
6. __rotating__ with __timelapse__ in case you want to download timelapse footage rather than real time recordings
6. __2023-06-01__ (or for hourly precision __2023-06-01-08__) with the start date/time of the footage you want to download
6. __2023-07-31__ (or for hourly precision __2023-07-31-18__) with the end date/time of the footage you want to download

To download only selected cameras, append a comma-separated list of camera names or ids:
```bash
download https://<Unifi-Protect-IP-Addr> <username> <password> /path/to/destination/folder daily rotating 2023-06-01 2023-07-31 "Front Door,Garage"
```

To download timelapse footage only during daytime hours each day:
```bash
download https://<Unifi-Protect-IP-Addr> <username> <password> /path/to/destination/folder hourly timelapse 2023-06-01 2023-07-31 --hours 07-19 --timelapse-factor 300x "Front Door"
```

To target a five-minute timelapse export for each selected time frame:
```bash
download https://<Unifi-Protect-IP-Addr> <username> <password> /path/to/destination/folder daily timelapse 2023-06-01 2023-07-31 --hours 07-19 --timelapse-duration 5m "Front Door"
```

To download only specific hours (for example daylight hours), specify the start and end date/time in the format __YYYY-MM-DD-HH__ (for example __2023-06-01-08__).

# CI/CD

GitHub Actions runs formatting, clippy, tests, and Docker build validation on pushes and pull requests.

To create release binaries for Linux, macOS, and Windows, push a version tag:

```bash
git tag v0.6.1
git push fork v0.6.1
```

The release workflow builds:
- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

## GPL3 LICENSE SYNOPSIS
TL;DR* Here's what the license entails:

1. Anyone can copy, modify and distribute this software.
2. You have to include the license and copyright notice with each and every distribution.
3. You can use this software privately.
4. You can use this software for commercial purposes.
5. If you dare build your business solely from this code, you risk open-sourcing the whole code base.
6. If you modify it, you have to indicate changes made to the code.
7. Any modifications of this code base MUST be distributed with the same license, GPLv3.
8. This software is provided without warranty.
9. The software author or license can not be held liable for any damages inflicted by the software.
   More information on about the LICENSE can be found [here](https://www.gnu.org/licenses/gpl-3.0.en.html)
