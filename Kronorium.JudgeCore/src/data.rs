#![allow(unused)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonDeConfig {
    lang: String,
    exe_file_path: String,

    input_file_path: Option<String>,
    output_file_path: Option<String>,
    error_file_path: Option<String>,

    cpu_time_limit: Option<u64>,
    real_time_limit: Option<u64>,

    memory_limit: Option<u64>,
    stack_limit: Option<u64>,

    max_output_size: Option<u64>,

    args: Option<Vec<String>>,
    env: Option<Vec<String>>,

    uid: Option<u32>,
    gid: Option<u32>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LanguageType {
    C,
    Cpp,
}

#[derive(Debug)]
pub struct Config {
    pub lang: LanguageType,
    pub exe_file_path: String,

    pub input_file_path: String,
    pub output_file_path: String,
    pub error_file_path: String,

    pub cpu_time_limit: Option<u64>,
    pub real_time_limit: Option<u64>,

    pub memory_limit: Option<u64>,
    pub stack_limit: Option<u64>,

    pub output_size_limit: Option<u64>,

    pub args: Vec<String>,
    pub env: Vec<String>,

    pub uid: Option<nix::unistd::Uid>,
    pub gid: Option<nix::unistd::Gid>,
}

impl TryFrom<JsonDeConfig> for Config {
    type Error = crate::error::Error;

    fn try_from(json_cfg: JsonDeConfig) -> Result<Self, Self::Error> {
        let lang = match json_cfg.lang.as_str() {
            "C" => LanguageType::C,
            "C++" => LanguageType::Cpp,
            _ => return Err(crate::error::Error::InvalidLanguage(json_cfg.lang)),
        };

        fn check_file_path(path: &String) -> Result<(), crate::error::Error> {
            if !std::path::Path::new(&path).exists() {
                Err(crate::error::Error::InvalidFilePath(path.clone()))
            } else {
                Ok(())
            }
        }

        let exe_file_path = json_cfg.exe_file_path;
        let input_file_path = json_cfg.input_file_path.unwrap_or("/dev/stdin".into());
        let output_file_path = json_cfg.output_file_path.unwrap_or("/dev/stdout".into());
        let error_file_path = json_cfg.error_file_path.unwrap_or("/dev/stderr".into());

        check_file_path(&exe_file_path)?;
        check_file_path(&input_file_path)?;
        check_file_path(&output_file_path)?;
        check_file_path(&error_file_path)?;

        let cpu_time_limit = match json_cfg.cpu_time_limit {
            Some(limit) => match limit {
                0 => return Err("CPU time limit cannot be zero".into()),
                _ => Some(limit),
            },
            _ => None,
        };

        let real_time_limit = match json_cfg.real_time_limit {
            Some(limit) => match limit {
                0 => return Err("Real time limit cannot be zero".into()),
                _ => Some(limit),
            },
            _ => None,
        };

        let memory_limit = match json_cfg.memory_limit {
            Some(limit) => match limit {
                0 => return Err("Memory limit cannot be zero".into()),
                _ => Some(limit),
            },
            _ => None,
        };

        let stack_limit = match json_cfg.stack_limit {
            Some(limit) => match limit {
                0 => return Err("Stack limit cannot be zero".into()),
                _ => Some(limit),
            },
            _ => None,
        };

        let output_size_limit = match json_cfg.max_output_size {
            Some(limit) => match limit {
                0 => return Err("Output size limit cannot be zero".into()),
                _ => Some(limit),
            },
            _ => None,
        };

        let args = json_cfg.args.unwrap_or_default();
        let env = json_cfg.env.unwrap_or_default();

        let uid = match json_cfg.uid {
            Some(uid) => Some(nix::unistd::Uid::from_raw(uid)),
            _ => None,
        };
        let gid = match json_cfg.gid {
            Some(gid) => Some(nix::unistd::Gid::from_raw(gid)),
            _ => None,
        };

        Ok(Self {
            lang,
            exe_file_path,
            input_file_path,
            output_file_path,
            error_file_path,
            cpu_time_limit,
            real_time_limit,
            memory_limit,
            stack_limit,
            output_size_limit,
            args,
            env,
            uid,
            gid,
        })
    }
}

#[derive(Debug, Serialize)]
pub enum JudgeState {
    SystemError,
    MemoryLimitExceeded,
    RealTimeLimitExceeded,
    CpuTimeLimitExceeded,
    RuntimeError,
}

#[derive(Debug, Serialize)]
pub struct JudgeResult {
    pub cpu_time: u64,
    pub real_time: u64,
    pub memory: u64,
    pub state: JudgeState,
    pub exit_code: i32,
    // core_state: JudgeCoreState,
}
