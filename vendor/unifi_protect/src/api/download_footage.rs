use crate::camera::UnifiProtectCameraSimple;
use crate::{ErrorResponse, UnifiProtectServer};
use reqwest::Client;
use tokio::io::AsyncWriteExt;

const DOWNLOAD_ATTEMPTS: usize = 3;

impl UnifiProtectServer {
    pub async fn download_footage(
        &self,
        camera: &UnifiProtectCameraSimple,
        output_path: &str,
        recording_type: &str,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<bool, String> {
        self.download_footage_with_fps(
            camera,
            output_path,
            recording_type,
            None,
            start_unix,
            end_unix,
        )
        .await
    }

    pub async fn download_footage_with_fps(
        &self,
        camera: &UnifiProtectCameraSimple,
        output_path: &str,
        recording_type: &str,
        timelapse_fps: Option<u32>,
        start_unix: i64,
        end_unix: i64,
    ) -> Result<bool, String> {
        let client = Client::builder()
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|err| format!("Failed to build HTTP client: {}", err))?;

        for channel in 0..4 {
            let timelapse_fps_param = if recording_type == "timelapse" {
                format!("&fps={}", timelapse_fps.unwrap_or(4))
            } else {
                String::new()
            };
            let endpoint = format!(
                "{}/proxy/protect/api/video/export?camera={}\
                {}\
                &channel={}\
                &filename={}.mp4\
                &lens=0\
                &start={}\
                &end={}\
                &type={}",
                self.uri,
                camera.id,
                timelapse_fps_param,
                channel,
                camera.mac,
                start_unix,
                end_unix,
                recording_type
            );

            let mut retryable_error = None;

            for attempt in 1..=DOWNLOAD_ATTEMPTS {
                let mut response = match client
                    .get(&endpoint)
                    .headers(self.headers.clone())
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        retryable_error = Some(format!(
                            "failed to send download request for channel {} on attempt {}/{}: {}",
                            channel, attempt, DOWNLOAD_ATTEMPTS, err
                        ));
                        if attempt < DOWNLOAD_ATTEMPTS {
                            eprintln!(
                                "{}. Retrying...",
                                retryable_error.as_ref().unwrap()
                            );
                            continue;
                        }
                        break;
                    }
                };
                retryable_error = None;

                if !response.status().is_success() {
                    eprintln!("Error: {:?}", response);
                    let status_code = response.status();
                    let error_msg = response.json::<ErrorResponse>().await.ok().map(|x| x.error);
                    if error_msg.is_some()
                        && (error_msg.as_ref().unwrap().contains("o files found")
                            || error_msg
                                .as_ref()
                                .unwrap()
                                .contains("track information is not valid"))
                    {
                        break;
                    } else {
                        if error_msg.is_some() {
                            eprintln!("Error: {}", error_msg.unwrap());
                        } else {
                            eprintln!("Unknown Error - Status Code: {}", status_code);
                        }
                        eprintln!("Failed to download video.");
                        break;
                    }
                }

                let mut file = tokio::fs::File::create(output_path)
                    .await
                    .map_err(|err| format!("Failed to create file '{}': {}", output_path, err))?;

                loop {
                    match response.chunk().await {
                        Ok(Some(chunk)) => file.write_all(&chunk).await.map_err(|err| {
                            format!("Failed to write video chunk to '{}': {}", output_path, err)
                        })?,
                        Ok(None) => return Ok(true),
                        Err(err) => {
                            let _ = tokio::fs::remove_file(output_path).await;
                            retryable_error = Some(format!(
                                "failed to read response chunk for channel {} on attempt {}/{}: {}",
                                channel, attempt, DOWNLOAD_ATTEMPTS, err
                            ));
                            if attempt < DOWNLOAD_ATTEMPTS {
                                eprintln!(
                                    "{}. Retrying...",
                                    retryable_error.as_ref().unwrap()
                                );
                                break;
                            }
                            return Err(retryable_error.unwrap());
                        }
                    }
                }
            }

            if let Some(error) = retryable_error {
                return Err(error);
            }
        }

        Ok(false)
    }
}
