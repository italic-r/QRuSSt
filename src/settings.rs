#![allow(unused_variables)]
#![allow(non_camel_case_types)]

use std::io;
use std::io::prelude::*;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::convert::Infallible;

use clap;
use clap::clap_app;

use shellexpand as se;
use config::{Config, ConfigError, File as cFile};

use toml;
use serde::{Serialize, Deserialize};

use cpal;
use cpal::traits::*;


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
        (@arg format:          -f --format          [TYPE]       display_order(2) number_of_values(1)
             possible_values(&["i16", "u16", "f32"])
             "Audio device sample format")
        (@arg rate:            -r --rate            [SAMPLES]    display_order(2) number_of_values(1)
             possible_values(&["16000", "32000", "44100", "48000", "96000", "192000"])
             "Audio device sample rate")
    ).get_matches()
}

#[derive(Debug)]
pub (crate) enum SettingsError {
    ConfigError(ConfigError),    // config::ConfigError
    ReadError(io::Error),   // file read error
    WriteError(io::Error),  // file write error
    DeserError(toml::de::Error), // data deserialize error
    SerError(toml::ser::Error),  // data serialize error
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) enum FftWindowType {
    Rectangle,
    Cosine,
    Triangle,
    Hann,
    Blackman,
    Hamming,
    Nuttall,
    Flat,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Fft {
    pub fft_size: u32,
    pub window_type: FftWindowType,
}

impl Default for Fft {
    fn default() -> Self {
        Fft {
            fft_size: 32768,
            window_type: FftWindowType::Hann,
        }
    }
}

/*
fn hann_window(window_length: usize) -> Vec<f32> {
    (0..window_length).map(|n|
        (0.5 - (0.5 * (PI * n as f32 * 2. / (window_length as f32 - 1.)).sin())) * 2.
    ).collect()
}
*/

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) enum AudioFormat {
    i16,
    u16,
    f32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Audio {
    pub device:      String,
    pub rate:        u32,
    pub format:      AudioFormat,
    pub freq_range: (u16, u16),
}

impl Default for Audio {
    fn default() -> Self {
        Audio {
            device:     "default".to_string(),
            rate:        48000,
            format:      AudioFormat::i16,
            freq_range: (100, 2800),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub (crate) struct Image {
    pub brightness:    u8,
    pub contrast:      u8,
    pub dimensions:   (u16, u16),
    pub use_window_xy: bool,
}

impl Default for Image {
    fn default() -> Self {
        Image {
            brightness:    50,
            contrast:      50,
            dimensions:   (1280, 720),
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
    pub verbose: u8,
    pub config:  PathBuf,
    pub fft:     Fft,
    pub audio:   Audio,
    pub image:   Image,
    pub export:  Export,
    pub names:   Names,
}

impl Settings {
    fn validate_export_path(&mut self) -> () {
        if !self.export.path.starts_with("file://") {
            // TODO: unwrap()
            self.export.path = format!("file://{}", self.export.path.to_str().unwrap()).into();
        }
    }

    pub fn read_config(&mut self) -> Result<(), SettingsError> {
        let mut s = Config::new();
        s.merge(Config::try_from(&self)
            .map_err(SettingsError::ConfigError)?)
            .map_err(SettingsError::ConfigError)?;
        if let Some(buf) = &self.config.to_str() {
            let c_file = cFile::with_name(buf);
            s.merge(c_file).map_err(SettingsError::ConfigError)?;
        }
        *self = s.try_into().map_err(SettingsError::ConfigError)?;
        self.validate_export_path();
        Ok(())
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

    pub fn arg_override(&mut self, cli: &clap::ArgMatches) -> Result<(), Infallible> {
        self.verbose = match cli.occurrences_of("verbose") {
            0     => 0,
            1     => 1,
            2 | _ => 2,
        };

        // TODO: process outside of this method
        // if cli.is_present("save_prefs") { }

        if let Some(c) = cli.value_of("config") {
            self.config = c.into();
        }

        self.image.use_window_xy = cli.is_present("window");

        if let Some(d) = cli.values_of("dimensions") {
            // requires two args, so direct conversion is ok here
            let d: Vec<u16> = d.map(|x| x.parse().unwrap()).collect();
            self.image.dimensions = (d[0], d[1]);
        }

        if let Some(b) = cli.value_of("brightness") {
            self.image.brightness = b.parse().unwrap();
        }

        if let Some(c) = cli.value_of("contrast") {
            self.image.contrast = c.parse().unwrap();
        }

        self.export.export_enable = cli.is_present("export_images");

        if let Some(path) = cli.value_of("export_path") {
            self.export.path = path.into();
            self.validate_export_path();
        }

        if let Some(dev) = cli.value_of("device") {
            self.audio.device = dev.into();
        }

        // Value already checked against parse. Safe to unwrap.
        if let Some(freq) = cli.values_of("frequency_range") {
            let mut freq: Vec<u16> = freq.map(|x| x.parse().unwrap()).collect();
            freq.sort_unstable();
            self.audio.freq_range = (freq[0], freq[1]);
        }

        // Valid options given in help message. Clap prevents others.
        if let Some(f) = cli.value_of("format") {
            self.audio.format = match f {
                "u16" => AudioFormat::u16,
                "i16" => AudioFormat::i16,
                "f32" => AudioFormat::f32,
                _     => unreachable!(),
            };
        }

        // Valid options given in help message. Parse directly into u32.
        if let Some(r) = cli.value_of("rate") {
            self.audio.rate = r.parse().unwrap();
        }

        Ok(())
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            verbose: 0,
            config:  (*se::full("~/.config/QRuSSt/config.toml").unwrap()).into(),
            fft:     Fft::default(),
            audio:   Audio::default(),
            image:   Image::default(),
            export:  Export::default(),
            names:   Names::default(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_settings() {
        let config_path: PathBuf = (*se::full("~/.config/QRuSSt/config.toml").unwrap()).into();
        let export_path: PathBuf = (*se::full("~/.local/share/QRuSSt/export/").unwrap()).into();
        let def = Settings::default();
        assert_eq!(def,
            Settings {
                verbose: 0,
                config: config_path.into(),
                fft: Fft {
                    fft_size: 32768,
                    window_type: FftWindowType::Hann,
                },
                audio: Audio {
                    device: "default".to_string(),
                    rate: 48000,
                    format: AudioFormat::i16,
                    freq_range: (100, 2800),
                },
                image: Image {
                    brightness: 50,
                    contrast: 50,
                    dimensions: (1280, 720),
                    use_window_xy: false,
                },
                export: Export {
                    path: export_path.into(),
                    export_enable: true,
                    single: true,
                    average: true,
                    peak: true,
                    hour: true,
                    day: true,
                },
                names: Names {
                    single: "single".to_string(),
                    average: "avg".to_string(),
                    peak: "pk".to_string(),
                    hour: "hr".to_string(),
                    day: "day".to_string(),
                },
            }
        );
    }
    #[test]
    fn config_read() {
        let mut def = Settings::default();
        def.config = "assets/default.toml".into();
        let read_ok = def.read_config();
        assert!(read_ok.is_ok());

        def.config = "assets/no_file.toml".into();
        let read_err = def.read_config();
        assert!(read_err.is_err());
    }
    #[test]
    fn config_write() {
        let mut def = Settings::default();
        def.config = "assets/write_test_ok.toml".into();
        let write_ok = def.write_config();
        assert!(write_ok.is_ok());

        def.config = "/write_test_err.toml".into();
        let write_err = def.write_config();
        assert!(write_err.is_err());
    }
    #[test]
    fn arg_override() {
        let mut set = Settings::default();
        let args = clap_args();
        assert!(set.arg_override(&args).is_ok());
    }
}
