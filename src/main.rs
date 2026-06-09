use crate::app_error::{api_error, to_local, AppError};
use crate::parse_args::{parse_args, Commands, DownloadArgs, DownloadMode, RecordingType};
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, NaiveTime, Timelike};
use std::path::{Path, PathBuf};
use unifi_protect::*;

mod app_error;
mod parse_args;

#[tokio::main]
async fn main() {
    let args = parse_args();

    match args.command {
        Commands::Download(download_args) => {
            if let Err(error) = download(&download_args).await {
                eprintln!("Download failed: {}", error);
                std::process::exit(1);
            }
        }
    }
}

async fn download(args: &DownloadArgs) -> Result<(), AppError> {
    let start_date = parse_date_or_hour(&args.start_date, true)
        .map_err(|source| AppError::parse_date(&args.start_date, source))?;
    let end_date = parse_date_or_hour(&args.end_date, false)
        .map_err(|source| AppError::parse_date(&args.end_date, source))?;

    if end_date < start_date {
        return Err(AppError::InvalidDateRange {
            start: start_date,
            end: end_date,
        });
    }

    let cameras = args.cameras.clone();
    let hour_window = args.hours.as_deref().map(parse_hour_window).transpose()?;

    println!("Cameras to Download: {:?}", cameras);

    let mut server = UnifiProtectServer::new(&args.uri);
    println!("Logging in...");
    server
        .login(&args.username, &args.password)
        .await
        .map_err(|source| api_error(format!("failed to login to '{}'", args.uri), source))?;
    println!("Logged in!");
    println!("Fetching cameras...");
    server
        .fetch_cameras(false)
        .await
        .map_err(|source| api_error("failed to fetch cameras", source))?;

    println!("Found {} cameras", server.cameras_simple.len());
    for camera in server.cameras_simple.iter() {
        println!(
            "Camera: {} {} {} '{}'",
            (if camera.is_connected {
                "<online>"
            } else {
                "<offline>"
            }),
            &camera.mac,
            &camera.id,
            &camera.name
        );
    }

    let time_frames = build_time_frames(&args.mode, start_date, end_date, hour_window)?
        .into_iter()
        .map(|(frame_start, frame_end)| {
            Ok((
                to_local(frame_start, "frame start".to_string())?,
                to_local(frame_end, "frame end".to_string())?,
            ))
        })
        .collect::<Result<Vec<(DateTime<Local>, DateTime<Local>)>, AppError>>()?;
    let timelapse_fps = if matches!(args.recording_type, RecordingType::Timelapse) {
        Some(args.timelapse_factor.as_fps())
    } else {
        None
    };

    println!("Downloading videos...");
    for time_frame in time_frames {
        println!(
            "Downloading video for time frame '{}' to '{}'",
            time_frame.0, time_frame.1
        );
        for camera in server.cameras_simple.iter() {
            if !should_download_camera(&camera.name, &camera.id, &cameras) {
                continue;
            }
            let mut file_name = format!(
                "{}-{}-{}.mp4",
                time_frame.0.format("%Y-%m-%d-%H"),
                camera.name,
                args.recording_type.as_str()
            );
            // sanitize filename using sanitize-filename and drop non-ascii symbols
            let options = sanitize_filename::Options {
                truncate: true,  // true by default, truncates to 255 bytes
                windows: true, // default value depends on the OS, removes reserved names like `con` from start of strings on Windows
                replacement: "", // str to replace sanitized chars/strings
            };
            file_name = sanitize_filename::sanitize_with_options(file_name, options)
                .chars()
                .filter(|s| s.is_ascii())
                .collect::<String>();

            let file_path: PathBuf = Path::new(&args.out_path).join(file_name);
            let file_path_display = file_path.display().to_string();
            let file_path_lossy = file_path.to_string_lossy().to_string();

            // check if file exists
            if file_path.exists() {
                println!("File '{}' already exists, skipping...", file_path_display);
                continue;
            }
            println!(
                "Downloading {} video for camera '{}' (file path: {})",
                args.recording_type.as_str(),
                camera.name,
                file_path_display
            );
            if !server
                .download_footage_with_fps(
                    camera,
                    &file_path_lossy,
                    args.recording_type.as_str(),
                    timelapse_fps,
                    time_frame.0.timestamp_millis(),
                    time_frame.1.timestamp_millis(),
                )
                .await
                .map_err(|source| {
                    api_error(
                        format!(
                            "failed to download {} video for camera '{}' ({}) for timeframe '{}' to '{}' into '{}'",
                            args.recording_type.as_str(),
                            camera.name,
                            camera.id,
                            time_frame.0,
                            time_frame.1,
                            file_path_display
                        ),
                        source,
                    )
                })?
            {
                println!(
                    "No video found for time frame '{}' to '{}' for camera '{}'",
                    time_frame.0, time_frame.1, camera.name
                );
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HourWindow {
    start: NaiveTime,
    end: NaiveTime,
}

fn parse_hour_window(input: &str) -> Result<HourWindow, AppError> {
    let (start, end) = input
        .split_once('-')
        .ok_or_else(|| AppError::InvalidHourWindow {
            input: input.to_string(),
            reason: "expected START-END, for example 07-19".to_string(),
        })?;
    let start = parse_hour_component(input, start)?;
    let end = parse_hour_component(input, end)?;

    if end <= start {
        return Err(AppError::InvalidHourWindow {
            input: input.to_string(),
            reason: "end must be after start".to_string(),
        });
    }

    Ok(HourWindow { start, end })
}

fn parse_hour_component(window: &str, component: &str) -> Result<NaiveTime, AppError> {
    let component = component.trim();
    let hour = component
        .parse::<u32>()
        .map_err(|_| AppError::InvalidHourWindow {
            input: window.to_string(),
            reason: format!("'{}' is not an hour from 00 to 23", component),
        })?;

    NaiveTime::from_hms_opt(hour, 0, 0).ok_or_else(|| AppError::InvalidHourWindow {
        input: window.to_string(),
        reason: format!("'{}' is not an hour from 00 to 23", component),
    })
}

fn build_time_frames(
    mode: &DownloadMode,
    start_date: NaiveDateTime,
    end_date: NaiveDateTime,
    hour_window: Option<HourWindow>,
) -> Result<Vec<(NaiveDateTime, NaiveDateTime)>, AppError> {
    let mut time_frames = vec![];

    if matches!(mode, DownloadMode::Hourly) {
        let mut cursor = start_date;
        while cursor <= end_date {
            let hour_start = cursor
                .date()
                .and_hms_opt(cursor.time().hour(), 0, 0)
                .ok_or_else(|| AppError::DateConstruction {
                    context: format!("failed to build hour start from '{}'", cursor),
                })?;

            if let Some(window) = hour_window {
                let hour = hour_start.time();
                if hour < window.start || hour >= window.end {
                    cursor = hour_start + Duration::hours(1);
                    continue;
                }
            }

            let hour_end = hour_start + Duration::hours(1) - Duration::seconds(1);
            time_frames.push((start_date.max(hour_start), end_date.min(hour_end)));
            cursor = hour_start + Duration::hours(1);
        }
    } else if matches!(mode, DownloadMode::Daily) {
        let mut date = start_date.date();
        while date <= end_date.date() {
            let (day_start, day_end) = if let Some(window) = hour_window {
                (
                    date.and_time(window.start),
                    date.and_time(window.end) - Duration::seconds(1),
                )
            } else {
                (
                    date.and_hms_opt(0, 0, 0)
                        .ok_or_else(|| AppError::DateConstruction {
                            context: format!("failed to build day start from '{}'", date),
                        })?,
                    date.and_hms_opt(23, 59, 59)
                        .ok_or_else(|| AppError::DateConstruction {
                            context: format!("failed to build day end from '{}'", date),
                        })?,
                )
            };

            let frame_start = start_date.max(day_start);
            let frame_end = end_date.min(day_end);
            if frame_start <= frame_end {
                time_frames.push((frame_start, frame_end));
            }

            date = date.succ_opt().ok_or_else(|| AppError::DateOverflow {
                context: format!("failed to calculate next day after '{}'", date),
            })?;
        }
    } else {
        return Err(AppError::InvalidMode {
            mode: format!("{:?}", mode),
        });
    }

    Ok(time_frames)
}

fn should_download_camera(
    camera_name: &str,
    camera_id: &str,
    requested_cameras: &[String],
) -> bool {
    let mut has_filter = false;

    for requested_camera in requested_cameras
        .iter()
        .map(|camera| camera.trim())
        .filter(|camera| !camera.is_empty())
    {
        has_filter = true;

        if requested_camera == "*"
            || requested_camera.eq_ignore_ascii_case("all")
            || requested_camera == camera_name
            || requested_camera == camera_id
        {
            return true;
        }
    }

    !has_filter
}

fn parse_date_or_hour(
    date_or_hour: &str,
    is_start: bool,
) -> Result<NaiveDateTime, chrono::ParseError> {
    // try to parse as date-time (YYYY-MM-DD-HH)
    if let Ok(date_time) =
        NaiveDateTime::parse_from_str(&format!("{}-00", date_or_hour), "%Y-%m-%d-%H-%M")
    {
        return Ok(date_time);
    }

    // hourly parsing failed, try to parse as date (YYYY-MM-DD)
    let date = NaiveDate::parse_from_str(date_or_hour, "%Y-%m-%d")?;
    Ok(if is_start {
        date.and_hms_opt(0, 0, 0)
    } else {
        date.and_hms_opt(23, 59, 59)
    }
    .expect("hard-coded time should be valid"))
}

#[cfg(test)]
mod tests {
    use super::{build_time_frames, parse_hour_window, should_download_camera, HourWindow};
    use crate::parse_args::DownloadMode;
    use chrono::{NaiveDate, NaiveTime};

    #[test]
    fn selects_all_cameras_when_no_filter_is_provided() {
        let requested_cameras = Vec::new();

        assert!(should_download_camera(
            "Front Door",
            "camera-id-1",
            &requested_cameras
        ));
    }

    #[test]
    fn selects_all_cameras_for_all_or_wildcard_filter() {
        assert!(should_download_camera(
            "Front Door",
            "camera-id-1",
            &[String::from("all")]
        ));
        assert!(should_download_camera(
            "Front Door",
            "camera-id-1",
            &[String::from("*")]
        ));
    }

    #[test]
    fn selects_only_cameras_matching_requested_name_or_id() {
        let requested_cameras = vec![String::from("Front Door"), String::from("camera-id-2")];

        assert!(should_download_camera(
            "Front Door",
            "camera-id-1",
            &requested_cameras
        ));
        assert!(should_download_camera(
            "Back Yard",
            "camera-id-2",
            &requested_cameras
        ));
        assert!(!should_download_camera(
            "Garage",
            "camera-id-3",
            &requested_cameras
        ));
    }

    #[test]
    fn parses_end_exclusive_hour_window() {
        let window = parse_hour_window("07-19").expect("valid hour window");

        assert_eq!(window.start, NaiveTime::from_hms_opt(7, 0, 0).unwrap());
        assert_eq!(window.end, NaiveTime::from_hms_opt(19, 0, 0).unwrap());
    }

    #[test]
    fn rejects_hour_windows_where_end_is_not_after_start() {
        assert!(parse_hour_window("19-07").is_err());
        assert!(parse_hour_window("07-07").is_err());
    }

    #[test]
    fn builds_hourly_frames_only_inside_daily_hour_window() {
        let start = NaiveDate::from_ymd_opt(2026, 5, 5)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 5, 6)
            .unwrap()
            .and_hms_opt(23, 59, 59)
            .unwrap();
        let window = HourWindow {
            start: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        };

        let frames = build_time_frames(&DownloadMode::Hourly, start, end, Some(window)).unwrap();

        assert_eq!(frames.len(), 24);
        assert_eq!(
            frames.first().unwrap().0,
            start.date().and_hms_opt(7, 0, 0).unwrap()
        );
        assert_eq!(
            frames.first().unwrap().1,
            start.date().and_hms_opt(7, 59, 59).unwrap()
        );
        assert_eq!(
            frames.last().unwrap().0,
            NaiveDate::from_ymd_opt(2026, 5, 6)
                .unwrap()
                .and_hms_opt(18, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn builds_daily_frames_clipped_to_hour_window_and_outer_range() {
        let start = NaiveDate::from_ymd_opt(2026, 5, 5)
            .unwrap()
            .and_hms_opt(16, 0, 0)
            .unwrap();
        let end = NaiveDate::from_ymd_opt(2026, 5, 6)
            .unwrap()
            .and_hms_opt(10, 0, 0)
            .unwrap();
        let window = HourWindow {
            start: NaiveTime::from_hms_opt(7, 0, 0).unwrap(),
            end: NaiveTime::from_hms_opt(19, 0, 0).unwrap(),
        };

        let frames = build_time_frames(&DownloadMode::Daily, start, end, Some(window)).unwrap();

        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].0, start);
        assert_eq!(
            frames[0].1,
            NaiveDate::from_ymd_opt(2026, 5, 5)
                .unwrap()
                .and_hms_opt(18, 59, 59)
                .unwrap()
        );
        assert_eq!(
            frames[1].0,
            NaiveDate::from_ymd_opt(2026, 5, 6)
                .unwrap()
                .and_hms_opt(7, 0, 0)
                .unwrap()
        );
        assert_eq!(frames[1].1, end);
    }

    #[test]
    fn maps_timelapse_factor_to_export_fps() {
        assert_eq!(crate::parse_args::TimelapseFactor::X60.as_fps(), 4);
        assert_eq!(crate::parse_args::TimelapseFactor::X120.as_fps(), 8);
        assert_eq!(crate::parse_args::TimelapseFactor::X300.as_fps(), 20);
        assert_eq!(crate::parse_args::TimelapseFactor::X600.as_fps(), 40);
    }
}
