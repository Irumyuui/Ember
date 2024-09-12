#![allow(unused)]

use krocore::cli::Cli;
use krocore::data::Config;
use krocore::error::default_error_handler;
use krocore::error::Result;

fn run() -> Result<bool> {
    let config: Config = Cli::new()?.get_json_config()?.try_into()?;
    dbg!(&config);

    Ok(true)
}

fn main() {
    let result = run();

    match result {
        Err(err) => {
            let stderr = std::io::stderr();
            default_error_handler(&err, &mut stderr.lock());
            std::process::exit(1);
        }
        Ok(false) => {
            std::process::exit(1);
        }
        Ok(true) => {
            std::process::exit(0);
        }
    }
}
