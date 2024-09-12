use std::io::Read;

use clap::Parser;

use crate::data::JsonDeConfig;
use crate::error::Result;

#[derive(Debug, clap::Parser)]
pub struct Cli {
    config_json_path: String,
}

fn get_json_from_file(path: &str) -> Result<String> {
    let mut file = std::fs::File::open(path)?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    Ok(buffer)
}

fn get_config_from_json(json: &String) -> Result<JsonDeConfig> {
    let config: JsonDeConfig = serde_json::from_str(&json)?;
    Ok(config)
}

impl Cli {
    pub fn new() -> Result<Self> {
        let cli = Cli::parse();
        Ok(cli)
    }

    pub fn get_json_config(&self) -> Result<JsonDeConfig> {
        let cfg_json = get_json_from_file(&self.config_json_path)?;
        let cfg = get_config_from_json(&cfg_json)?;
        Ok(cfg)
    }
}
