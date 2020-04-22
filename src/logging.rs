use std::fs::OpenOptions;
use std::io::BufReader;
use std::io::prelude::*;

use slog;
use sloggers::{
    Build,
    Config,
    LoggerConfig,
};

use toml;
use serde::{Serialize, Deserialize};

use shellexpand as se;

const DEFAULT_LOGGER: &'static str = r#"
type =            "terminal"
format =          "compact"
source_location = "module_and_line"
timezone =        "utc"
level =           "debug"
destination =     "stderr"
"#;

pub (crate) fn set_logger() -> slog::Logger {
    let mut cfg = String::new();
    let file_path: String = (*se::full("~/.config/QRuSSt/logger.toml").unwrap()).into();
    let file_obj = OpenOptions::new()
        .read(true)
        .open(file_path);
    if file_obj.is_err() ||
        file_obj.unwrap().read_to_string(&mut cfg).is_err() ||
            cfg.len() == 0 {
        println!("Logging config not found. Using defaults.");
        cfg = DEFAULT_LOGGER.into();
    }

    let config: LoggerConfig = toml::from_str(&cfg).unwrap();
    let builder = config.try_to_builder().unwrap();
    let logger = builder.build().unwrap();
    logger
}
