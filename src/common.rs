use std::io::{BufRead, BufReader};

#[macro_export]
macro_rules! mem_size {
    ($value:expr, B) => {
        $value
    };

    ($value:expr, KB) => {
        $value * 1024
    };

    ($value:expr, MB) => {
        $value * 1024 * 1024
    };

    ($value:expr, GB) => {
        $value * 1024 * 1024 * 1024
    };
}

pub fn get_from_config(variable: &str) -> Option<String> {
    let conf_file = std::fs::OpenOptions::new()
                                                .read(true)
                                                .open("./container.conf")
                                                .expect("Opening .conf file");

    let conf_file = BufReader::new(conf_file);
    for line in conf_file.lines() {
        let line = line.expect("Reading line from .conf");

        if let Some((key, val)) = line.split_once('=') {
            if key == variable && !val.trim().starts_with('#') {
                return Some(val.trim().to_string());
            }
        }
    }

    None
}