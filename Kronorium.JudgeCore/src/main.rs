#![allow(unused)]

use krocore::cli::Cli;
use krocore::data::Config;
use krocore::error::default_error_handler;
use krocore::error::Result;
use krocore::judge::run_judge;

fn run() -> Result<bool> {
    let config: Config = Cli::new()?.get_json_config()?.try_into()?;
    let result = run_judge(&config)?;

    let result_json = serde_json::to_string_pretty(&result)?;
    println!("{}", result_json);

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
