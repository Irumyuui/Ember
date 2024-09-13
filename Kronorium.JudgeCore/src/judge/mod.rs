#![allow(unused)]
#![allow(dead_code)]

pub mod rules;

use std::{
    os::fd::{AsFd, AsRawFd},
    process::{Command, ExitStatus},
    thread::JoinHandle,
    time::Duration,
};

use nix::{
    libc::{
        dup2, fileno, fopen, fopen64, pthread_cancel, rlimit64, rusage, wait4, RLIMIT_STACK,
        SIGUSR1, WEXITSTATUS, WIFCONTINUED, WTERMSIG,
    },
    sys::resource::{setrlimit, Resource},
    unistd::{fork, getuid, setgid, setgroups, setuid, ForkResult, Pid},
};

use crate::{
    data::{Config, JudgeResult, JudgeState, LanguageType},
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

    if let Some(gid) = config.gid {
        if let Err(e) = setgid(gid) {
            setgroups(&[gid])?;
        }
    }

    if let Some(uid) = config.uid {
        setuid(uid)?;
    }

    // TODO: rules
    match &config.lang {
        _ => return Err("Unsupported language. ".into()),
    };

    Ok(())
}

fn kill_process(pid: Pid) -> Result<ExitStatus> {
    match Command::new("kill").arg("-9").arg(pid.to_string()).status() {
        Ok(status) => Ok(status),
        Err(e) => Err(e.into()),
    }
}

fn check_timeout(pid: Pid, timeout: u64) -> JoinHandle<()> {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(timeout));
        kill_process(pid);
    })
}

fn parent_process(config: &Config, child_pid: Pid) -> Result<JudgeResult> {
    let mut time_start: nix::libc::timeval = unsafe { std::mem::zeroed() };
    let mut time_end: nix::libc::timeval = unsafe { std::mem::zeroed() };

    unsafe {
        nix::libc::gettimeofday(&mut time_start as _, std::ptr::null_mut());
    }

    let mut handle: Option<JoinHandle<()>> = None;
    if let Some(timeout) = config.real_time_limit {
        handle.insert(check_timeout(child_pid, timeout));
    }

    let mut child_status = 0;
    let mut resource_usage: rusage = unsafe { std::mem::zeroed() };

    unsafe {
        let result = wait4(
            child_pid.as_raw(),
            &mut child_status as _,
            nix::libc::WSTOPPED,
            &mut resource_usage as _,
        );

        if result == -1 {
            kill_process(child_pid)?;
            return Err("Failed to wait for child process.".into());
        }
    }

    unsafe {
        nix::libc::gettimeofday(&mut time_end as _, std::ptr::null_mut());
    }

    let mut result: JudgeResult = unsafe { std::mem::zeroed() };
    result.real_time = (time_end.tv_sec * 1000 + time_end.tv_usec / 1000
        - time_start.tv_sec * 1000
        - time_start.tv_usec / 1000) as u64;

    if let Some(handle) = handle.take() {
        unsafe {
            let pid = std::os::unix::thread::JoinHandleExt::as_pthread_t(&handle);
            let _result = pthread_cancel(pid);
        }
    }

    let single = if WIFCONTINUED(child_status) {
        Some(WTERMSIG(child_status))
    } else {
        None
    };

    if let Some(single) = single {
        if single == SIGUSR1 {
            result.state = JudgeState::SystemError;
        }
        return Ok(result);
    }

    result.exit_code = WEXITSTATUS(child_status);
    result.cpu_time =
        (resource_usage.ru_utime.tv_sec * 1000 + resource_usage.ru_utime.tv_usec / 1000) as u64;
    result.memory = (resource_usage.ru_maxrss * 1024) as u64;

    if result.exit_code != 0 || single.is_some() {
        result.state = JudgeState::RuntimeError;
    }

    if let Some(mem_limit) = config.memory_limit {
        if result.memory > mem_limit {
            result.state = JudgeState::MemoryLimitExceeded;
        }
    }

    if let Some(time_limit) = config.real_time_limit {
        if result.real_time > time_limit {
            result.state = JudgeState::RealTimeLimitExceeded;
        }
    }

    if let Some(time_limit) = config.cpu_time_limit {
        if result.cpu_time > time_limit {
            result.state = JudgeState::CpuTimeLimitExceeded;
        }
    }

    Ok(result)
}

pub fn run_judge(config: &Config) -> Result<JudgeResult> {
    if !getuid().is_root() {
        return Err("Permission denied. Please run as root.".into());
    }

    let fork_result = unsafe { fork() };

    match fork_result {
        Ok(ForkResult::Parent { child, .. }) => {
            return parent_process(&config, child);
        }
        Ok(ForkResult::Child) => {
            child_process(&config)?;
            unreachable!();
        }
        Err(e) => return Err(e.into()),
    }

    unreachable!()
}
