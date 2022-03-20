#![allow(unused_variables)]
#![allow(non_camel_case_types)]

use std::io;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::path::PathBuf;

use clap;
use clap::clap_app;

use shellexpand as se;
use config::{Config, ConfigError, File as cFile};

use toml;
use serde::{Serialize, Deserialize};

use cpal;
use cpal::traits::*;

use super::windows;


pub (crate) fn clap_args() -> clap::ArgMatches<'static> {
    let path_exists = |path: String| {
        if se::full(&path).is_ok() {
            Ok(())
        } else {
            Err(String::from("File does not exist"))
        }
    };
    let f_range = |range: String| {
        if let Ok(val) = range.parse::<u16>() {
            if val <= 3000 && val >= 50 {
                Ok(())
            } else {
                Err(String::from("Maximum range: 50-3000"))
            }
        } else {
            Err(String::from("Positive integer inputs only"))
        }
    };
    let d_range = |range: String| {
        if let Ok(val) = range.parse::<u32>() {
            if val >= 480 && val <= 3000 {
                Ok(())
            } else {
                Err(String::from("Width range: 640-3000. Height range: 480-2000."))
            }
        } else {
            Err(String::from("Integer values only"))
        }
    };
    let c_b_range = |val: String| {
        if let Ok(v) = val.parse::<u8>() {
            if v <= 100 {
                Ok(())
            } else {
                Err(String::from("Range: 0-100"))
            }
        } else {
            Err(String::from("Integer range only"))
        }
    };
    let aud_exists = |device: String| {
        if cpal::default_host().devices().unwrap().any(|x| x.name().unwrap() == device) {
            Ok(())
        } else {
            Err(String::from("Device unavailable"))
        }
    };

    clap_app!(QRuSSt =>
        (about: "A QRSS processor using audio input from a sound card or SDR demodulator")
        (@arg verbose:         -v --verbose         ...                                                             "stdout verbosity (can be passed up to twice)"                    )
        (@arg save_prefs:      -s --("save-prefs")               display_order(1)                                   "Write given arguments to config file"                            )
        (@arg config:          -c --config          [FILE]       display_order(1) number_of_values(1) {path_exists} "Path to config file (default: ~/.config/QRuSSt/config.toml)"     )

        (@arg window:          -w --window                       display_order(4)                                   "Use window dimensions for image export"                          )
        (@arg dimensions:      -D --dimensions      [X] [Y]      display_order(3) number_of_values(2) {d_range}     "Pixel dimensions for export (see --window)"                      )
        (@arg brightness:      -B --brightness      [NUM]        display_order(3) number_of_values(1)               "Image brightness (0-100)"                                        )
        (@arg contrast:        -C --contrast        [NUM]        display_order(3) number_of_values(1) {c_b_range}   "Image contrast (0-100)"                                          )

        (@arg export_images:   -i --images                       display_order(3)                                   "Enable image export"                                             )
        (@arg export_path:     -E --("export-path") [DIR]        display_order(4) number_of_values(1) {path_exists} "Image export directory (default: ~/.local/share/QRuSSt/export/)" )

        (@arg device:          -d --device          [NAME]       display_order(2) number_of_values(1) {aud_exists}  "Audio device to use (use device name from `arecord -L`)"         )
        (@arg frequency_range: -F --("f-range")     [LOW] [HIGH] display_order(2) number_of_values(2) {f_range}     "Audio frequency range to process/display (maximum range: 0-3000)")
        (@arg rate:            -r --rate            [SAMPLES]    display_order(2) number_of_values(1)
             possible_values(&["16000", "32000", "44100", "48000", "96000", "192000"])
             "Audio device sample rate")
    ).get_matches()
}

#[derive(Debug)]
pub (crate) enum SettingsError {
    ConfigError(ConfigError),    // config::ConfigError
    ReadError(io::Error),        // file read error
    WriteError(io::Error),       // file write error
    DeserError(toml::de::Error), // data deserialize error
    SerError(toml::ser::Error),  // data serialize error
}

impl From<ConfigError> for SettingsError {
    fn from(e: ConfigError) -> Self {
        SettingsError::ConfigError(e)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub (crate) enum FftWindowType {
    Rectangle,
    Cosine,
    Triangle,
    Hamming,
    Hann,
    Blackman,
    Nuttall,
    Flat,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct FftWindow {
    pub window_type: FftWindowType,
    pub length: usize,
    pub window_func: Vec<f32>,
}

impl Default for FftWindow {
    fn default() -> Self {
        FftWindow::new(32768, &FftWindowType::Hann)
    }
}

impl FftWindow {
    pub (crate) fn new(length: usize, window_type: &FftWindowType) -> Self {
        FftWindow {
            window_type: *window_type,
            length,
            window_func: match *window_type {
                FftWindowType::Rectangle => windows::rectangle(length),
                FftWindowType::Cosine    => windows::cosine(length),
                FftWindowType::Triangle  => windows::triangle(length),
                FftWindowType::Hann      => windows::hann(length),
                FftWindowType::Blackman  => windows::blackman(length),
                FftWindowType::Hamming   => windows::hamming(length),
                FftWindowType::Nuttall   => windows::nuttall(length),
                FftWindowType::Flat      => windows::flat(length),
            }
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Audio {
    pub device:     String,
    pub rate:       u32,
    pub freq_range: Vec<u32>,
}

impl Default for Audio {
    fn default() -> Self {
        Audio {
            device:    "default".to_string(),
            rate:       48000,
            freq_range: vec![100, 2800],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Image {
    pub brightness:    u8,
    pub contrast:      u8,
    pub dimensions:    Vec<u32>,
    pub use_window_xy: bool,
}

impl Default for Image {
    fn default() -> Self {
        Image {
            brightness:    50,
            contrast:      50,
            dimensions:    vec![1280, 720],
            use_window_xy: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Export {
    pub path:          PathBuf,
    pub export_enable: bool,
    pub single:        bool,
    pub average:       bool,
    pub peak:          bool,
    pub hour:          bool,
    pub day:           bool,
}

impl Default for Export {
    fn default() -> Self {
        Export {
            path: (*se::full("~/.local/share/QRuSSt/export/").unwrap()).into(),
            export_enable: true,
            single:        true,
            average:       true,
            peak:          true,
            hour:          true,
            day:           true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Names {
    pub single:  String,
    pub average: String,
    pub peak:    String,
    pub hour:    String,
    pub day:     String,
}

impl Default for Names {
    fn default() -> Self {
        Names {
            single:  "single".to_string(),
            average: "avg"   .to_string(),
            peak:    "pk"    .to_string(),
            hour:    "hr"    .to_string(),
            day:     "day"   .to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Settings {
    pub verbose:    u8,
    pub config:     PathBuf,
    pub fft_window: FftWindow,
    pub audio:      Audio,
    pub image:      Image,
    pub export:     Export,
    pub names:      Names,
}

impl Settings {
    pub fn read_config_file(&mut self) -> Result<(), SettingsError> {
        let file = OpenOptions::new()
            .read(true).write(false).create(false)
            .open(&self.config)
            .map_err(SettingsError::ReadError)?;
        Ok(())
    }

    pub fn load_config(&mut self, cli: &clap::ArgMatches) -> Result<Self, SettingsError> {
        let mut b = Config::builder();
            // XXX: Need default serialized in file when defaults are created when object is created?
            //.add_source(&toml::to_string(&Self::default()).unwrap())
        if self.read_config_file().is_ok() {
            b = b.add_source(cFile::with_name(&self.config.to_str().unwrap()));
        } else {
            println!("Error reading existing config.");
        }

        // Parse and save CLI args
        b = b.set_override("verbose", match cli.occurrences_of("verbose") {
            0 => 0,
            1 => 1,
            2 | _ => 2,
        })?;

        if let Some(c) = cli.value_of("config") {
            b = b.set_override("config", c)?;
        }

        if cli.is_present("window") {
            b = b.set_override("image.use_window_xy", true)?;
        }

        if let Some(d) = cli.values_of("dimensions") {
            // requires two args, so direct conversion is ok here
            let d: Vec<u32> = d.map(|x| x.parse().unwrap()).collect();
            b = b.set_override::<&str, Vec<i32>>("image.dimensions", vec![d[0] as i32, d[1] as i32])?;
        }

        if let Some(x) = cli.value_of("brightness") {
            b = b.set_override::<&str, i8>("image.brightness", x.parse().unwrap())?;
        }

        if let Some(c) = cli.value_of("contrast") {
            b = b.set_override::<&str, i8>("image.contrast", c.parse().unwrap())?;
        }

        if cli.is_present("export_images") {
            b = b.set_override("export_images", true)?;
        }

        if let Some(path) = cli.value_of("export_path") {
            if !path.starts_with("file://") {
                // XXX: unwrap()
                self.export.path = format!("file://{}", self.export.path.to_str().unwrap()).into();
            }
            b = b.set_override("export.path", path)?;
        }

        if let Some(dev) = cli.value_of("device") {
            b = b.set_override("audio.device", dev)?;
        }

        // Value already checked against parse. Safe to unwrap.
        if let Some(freq) = cli.values_of("frequency_range") {
            let mut freq: Vec<i32> = freq.map(|x| x.parse().unwrap()).collect();
            freq.sort_unstable();
            println!("freq: {:?}", &freq);
            b = b.set_override::<&str, Vec<i32>>("audio.freq_range", vec![freq[0], freq[1]])?;
        }

        // Valid options given in help message. Parse directly into u32.
        if let Some(r) = cli.value_of("rate") {
            b = b.set_override::<&str, i32>("audio.rate", r.parse().unwrap())?;
        }

        // Read files and finalize config for use
        let s = b.build()?;
        s.try_deserialize().map_err(SettingsError::ConfigError)
    }

    pub fn write_config(&self) -> Result<(), SettingsError> {
        let mut file = OpenOptions::new()
            .write(true).create(true)
            .open(&self.config)
            .map_err(SettingsError::WriteError)?;
        let coded = toml::to_string(self)
            .map_err(SettingsError::SerError)?;
        file.write_all(format!("{}", coded).as_bytes())
            .map_err(SettingsError::WriteError)?;
        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            verbose:    0,
            config:     (*se::full("~/.config/QRuSSt/config.toml").unwrap()).into(),
            fft_window: FftWindow::default(),
            audio:      Audio::default(),
            image:      Image::default(),
            export:     Export::default(),
            names:      Names::default(),
        }
    }
}
