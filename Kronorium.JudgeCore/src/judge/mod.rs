#![allow(unused)]
#![allow(dead_code)]

pub mod rules;

use std::os::fd::{AsFd, AsRawFd};

use nix::{
    libc::{dup2, fileno, fopen, fopen64, rlimit64, RLIMIT_STACK},
    sys::resource::{setrlimit, Resource},
    unistd::{fork, getuid, ForkResult},
};

use crate::{
    data::{Config, JudgeResult},
    error::Result,
};

fn child_process(config: &Config) -> Result<()> {
    use nix::sys::resource::Resource as Res;

    if let Some(limit) = config.stack_limit {
        setrlimit(Res::RLIMIT_STACK, limit, limit)?;
    }

    if let Some(limit) = config.memory_limit {
        setrlimit(Res::RLIMIT_AS, limit, limit)?;
    }

    if let Some(limit) = config.output_size_limit {
        setrlimit(Res::RLIMIT_FSIZE, limit, limit)?;
    }

    if let Some(limit) = config.cpu_time_limit {
        setrlimit(Res::RLIMIT_CPU, limit, limit)?;
    }

    unsafe {
        let input_file = fopen(config.input_file_path.as_ptr() as _, b"r".as_ptr() as _);
        if input_file.is_null() {
            return Err("Failed to open input file.".into());
        }
        if dup2(
            fileno(input_file),
            fileno(std::io::stdin().as_raw_fd() as _),
        ) == -1
        {
            return Err("Failed to redirect input file to stdin.".into());
        }
    }

    unsafe {
        let output_file = fopen64(config.output_file_path.as_ptr() as _, b"w".as_ptr() as _);
        if output_file.is_null() {
            return Err("Failed to open output file.".into());
        }
        if dup2(
            fileno(output_file),
            fileno(std::io::stdout().as_raw_fd() as _),
        ) == -1
        {
            return Err("Failed to redirect output file to stdout.".into());
        }
    }

    unsafe {
        let error_file = fopen64(config.error_file_path.as_ptr() as _, b"w".as_ptr() as _);
        if error_file.is_null() {
            return Err("Failed to open error file.".into());
        }
        if dup2(
            fileno(error_file),
            fileno(std::io::stderr().as_raw_fd() as _),
        ) == -1
        {
            return Err("Failed to redirect error file to stderr.".into());
        }
    }

    Ok(())
}

pub fn run_judge(config: &Config) -> Result<JudgeResult> {
    if !getuid().is_root() {
        return Err("Permission denied. Please run as root.".into());
    }

    let fork_result = unsafe { fork() };

    match fork_result {
        Ok(ForkResult::Parent { child, .. }) => {
            // Child process
            // rules::run_rules(config)?;
            // std::process::exit(0);
        }
        Ok(ForkResult::Child) => {
            child_process(&config);
        }
        Err(e) => return Err(e.into()),
    }

    todo!();
}
