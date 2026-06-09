use clap::{Parser, Subcommand, ValueEnum};

/// Tool for bulk-downloading recordings from unifi protect.
#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    propagate_version = true,
    help_template = "{before-help}{name} {version} by {author}\n{about-with-newline}\n{usage-heading} {usage}\n\n{all-args}{after-help}"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Download footage from the UniFi Protect server.
    Download(DownloadArgs),
}

#[derive(clap::Args, Debug)]
pub struct DownloadArgs {
    /// The URI of the UniFi Protect server.
    pub uri: String,
    /// The username for logging into the UniFi Protect server.
    pub username: String,
    /// The password for logging into the UniFi Protect server.
    pub password: String,
    /// The path to the directory to download files to.
    pub out_path: String,
    /// The mode to download files in.
    pub mode: DownloadMode,
    /// The type of recording to download.
    pub recording_type: RecordingType,
    /// The start date/time to download files from (YYYY-MM-DD or YYYY-MM-DD-HH).
    pub start_date: String,
    /// The end date/time to download files to (YYYY-MM-DD or YYYY-MM-DD-HH).
    pub end_date: String,
    /// Optional daily hour window to download (START-END, end-exclusive, e.g. 07-19).
    #[arg(long, value_name = "START-END")]
    pub hours: Option<String>,
    /// Timelapse speed factor. Matches UniFi Protect UI options.
    #[arg(long, value_enum, default_value = "60x")]
    pub timelapse_factor: TimelapseFactor,
    /// Comma-separated list of camera names/ids, or `all` / `*`.
    #[arg(value_delimiter = ',')]
    pub cameras: Vec<String>,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum DownloadMode {
    Daily,
    Hourly,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum RecordingType {
    Rotating,
    Timelapse,
}

impl RecordingType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rotating => "rotating",
            Self::Timelapse => "timelapse",
        }
    }
}

#[derive(Clone, Debug, ValueEnum)]
pub enum TimelapseFactor {
    #[value(name = "60x")]
    X60,
    #[value(name = "120x")]
    X120,
    #[value(name = "300x")]
    X300,
    #[value(name = "600x")]
    X600,
}

impl TimelapseFactor {
    pub fn as_fps(&self) -> u32 {
        match self {
            Self::X60 => 4,
            Self::X120 => 8,
            Self::X300 => 20,
            Self::X600 => 40,
        }
    }
}

pub fn parse_args() -> Cli {
    Cli::parse()
}
