use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::path::PathBuf;

use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use ron::{
    ser::{PrettyConfig, to_string_pretty},
    de::{from_reader, from_str},
};


/// QRuSSt is a QRSS processor using audio input
/// from a sound card or SDR demodulator.
#[derive(StructOpt, Debug, Clone, PartialEq)]
pub struct Opts {
    #[structopt(short, long)]
    /// Write given arguments to config file
    /// (ie make permanent)
    pub (crate) save_prefs: bool,

    /// Path to configuration file
    /// (default: ~/.config/QRuSSt/config)
    #[structopt(short, long, parse(from_os_str))]
    pub (crate) config: Option<PathBuf>,

    /// Audio device to use
    #[structopt(short, long)]
    device: Option<String>,

    /// Audio device sample rate
    #[structopt(short, long)]
    rate: Option<u32>,

    /// Audio device bit depth
    #[structopt(short, long)]
    format: Option<u8>,

    /// Audio frequency range to process and display
    /// <low high>
    #[structopt(
        short="F", long,
        min_values=2,
        max_values=2)
    ]
    frequency_range: Option<Vec<u32>>,

    /// Image brightness
    #[structopt(short="B", long)]
    brightness: Option<u8>,

    /// Image contrast
    #[structopt(short="C", long)]
    contrast: Option<u8>,

    /// Use window dimensions for image export
    #[structopt(short="w", long="window")]
    use_window_dimensions: bool,

    /// Pixel dimensions for image export if
    /// not using window dimensions <width height>
    /// (see: --window)
    #[structopt(
        short="D",
        long="dimensions",
        min_values=2,
        max_values=2,
        required_if("use_window_dimensions", "false")
    )]
    image_dimensions: Option<Vec<u32>>,

    /// Enable image export
    #[structopt(short="i", long="images")]
    export_images: bool,

    /// Image export directory
    #[structopt(short="E", long, parse(from_os_str))]
    export_path: Option<PathBuf>,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            save_prefs: false,
            config: None,
            device: None,
            rate: None,
            format: None,
            frequency_range: None,
            brightness: None,
            contrast: None,
            use_window_dimensions: false,
            image_dimensions: None,
            export_images: false,
            export_path: None,
        }
    }
}

impl Opts {
    pub fn is_default(&self) -> bool {
        self == &Opts::default()
    }
}

#[derive(Debug)]
pub enum SettingsError {
    FileError(io::Error),
    SerialReadError(ron::de::Error),
    SerialWriteError(ron::ser::Error),
    OverrideError(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Settings {
    pub config_path:                 PathBuf,
    pub audio_device:                String,
    pub ad_rate:                     u32,
    pub ad_depth:                    u8,
    pub freq_range:                 (u32, u32),
    pub brightness:                  u8,
    pub contrast:                    u8,
    pub image_use_window_dimensions: bool,
    pub image_dimensions:           (u32, u32),
    pub export_images:               bool,
    pub export_single:               bool,
    pub export_avg:                  bool,
    pub export_pk:                   bool,
    pub export_hr:                   bool,
    pub export_day:                  bool,
    pub single_name:                 String,
    pub avg_name:                    String,
    pub pk_name:                     String,
    pub hr_name:                     String,
    pub day_name:                    String,
    pub export_path:                 PathBuf,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            config_path:                 PathBuf::from("~/.config/QRuSSt/config"),
            audio_device:                String::new(),
            ad_rate:                     48000,
            ad_depth:                    16,
            freq_range:                 (400, 800),
            brightness:                  50,
            contrast:                    50,
            image_use_window_dimensions: false,
            image_dimensions:           (1280, 720),
            export_images:               true,
            export_single:               true,
            export_avg:                  true,
            export_pk:                   true,
            export_hr:                   true,
            export_day:                  true,
            single_name:                 String::from("single"),
            avg_name:                    String::from("avg"),
            pk_name:                     String::from("pk"),
            hr_name:                     String::from("hour"),
            day_name:                    String::from("day"),
            export_path:                 PathBuf::from("~/.local/share/QRuSSt/export/"),
        }
    }
}

/// Implement read/write/override methods for settings.
/// Serialize to RON format for storage.
impl Settings {
    pub fn read_config(&mut self) ->  Result<(), SettingsError> {
        let f = File::open(&self.config_path)
            .map_err(SettingsError::FileError)?;
        let d = from_reader(f)
            .map_err(SettingsError::SerialReadError)?;
        *self = d;
        Ok(())
    }

    pub fn write_config(&self) -> Result<(), SettingsError> {
        let mut f = File::open(&self.config_path)
            .map_err(SettingsError::FileError)?;
        let s = to_string_pretty(&self, PrettyConfig::default())
            .map_err(SettingsError::SerialWriteError)?;
        f.write_all(&s.as_bytes())
            .map_err(SettingsError::FileError)?;
        Ok(())
    }

    pub fn set_override(&mut self, opts: Opts) -> Result<(), SettingsError> { // Use SettingsError?
        if opts.is_default() {
            return Err(SettingsError::OverrideError(
                "Settings override error. Overrides are default. Skipping."
                .to_string()));
        }
        *self = Settings {
            config_path:                 opts.config.unwrap_or(self.config_path.clone()),
            audio_device:                opts.device.unwrap_or(self.audio_device.clone()),
            ad_rate:                     opts.rate.unwrap_or(self.ad_rate),
            ad_depth:                    opts.format.unwrap_or(self.ad_depth),
            freq_range: {                // Quicker way to make tuple?
                match opts.frequency_range {
                    Some(vec) => (vec[0], vec[1]),
                    None => self.freq_range}},
            brightness:                  opts.brightness.unwrap_or(self.brightness),
            contrast:                    opts.contrast.unwrap_or(self.contrast),
            image_use_window_dimensions: opts.use_window_dimensions,
            image_dimensions: {          // Quicker way to make tuple
                match opts.image_dimensions {
                    Some(vec) => (vec[0], vec[1]),
                    None => self.image_dimensions}},
            export_images:               opts.export_images,
            export_single:               self.export_single,
            export_avg:                  self.export_avg,
            export_pk:                   self.export_pk,
            export_hr:                   self.export_hr,
            export_day:                  self.export_day,
            single_name:                 self.single_name.clone(),
            avg_name:                    self.avg_name.clone(),
            pk_name:                     self.pk_name.clone(),
            hr_name:                     self.hr_name.clone(),
            day_name:                    self.day_name.clone(),
            export_path:                 opts.export_path.unwrap_or(self.export_path.clone()),
        };
        Ok(())
    }
}

// SETTINGS INIT
// set default settings
// read config settings
// read cli args (+ make permanent?)
// OUTPUT settings struct

// AUDIO
// get audio device
// open audio file
// send stream to fft processor
// fft process

// IMAGE OUTPUT
// write to image
// save image


// PROGRAM OP
// init gtk
// set prefs (following settings init above)
// populate gtk fields/options
// open gtk window
