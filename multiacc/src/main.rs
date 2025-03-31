use std::{
    collections::HashMap,
    env,
    ffi::CStr,
    io,
    os::{fd::RawFd, unix::process::CommandExt},
    process::Command,
};

use nix::{
    libc::{
        AT_FDCWD, ENOSYS, F_SETLK, PATH_MAX, SYS_fcntl, SYS_openat, iovec, process_vm_readv,
        user_regs_struct,
    },
    sys::{
        ptrace::{self, Options},
        signal::Signal,
        wait::{WaitStatus, waitpid},
    },
    unistd::Pid,
};
use syscalls::{Sysno, SysnoMap, SysnoSet};

const FCNTL_ID: u64 = 72;

fn log_syscall(pid: Pid, regs: user_regs_struct) {
    // let event = ptrace::getevent(pid)?;

    println!(
        "[pid: {}] {}({:x}, {:x}, {:x}, ...) = {:x}",
        pid.as_raw(),
        // regs.orig_rax,
        Sysno::from(regs.orig_rax as u32),
        regs.rdi,
        regs.rsi,
        regs.rdx,
        regs.rax,
    );

    // if regs.rsi == F_SETLK as u64 {
    //     println!("setlk!!");
    // }
}

fn read_remote_str(pid: Pid, remote_addr: usize, maxlen: usize) -> Result<String, std::io::Error> {
    let mut buffer = vec![0u8; maxlen];

    // TODO cache this!! (or maybe not?? benchmark!!)
    // let pagesize = unsafe { sysconf(_SC_PAGESIZE) };
    // println!("pagesize: {}", pagesize);

    let local_iov = iovec {
        iov_base: buffer.as_mut_ptr() as *mut _,
        iov_len: maxlen,
    };

    let remote_iov = iovec {
        iov_base: remote_addr as *mut _,
        iov_len: maxlen,
    };

    let bytes_read = unsafe { process_vm_readv(pid.as_raw(), &local_iov, 1, &remote_iov, 1, 0) };
    if bytes_read < 0 {
        return Err(nix::Error::last().into());
    }

    match CStr::from_bytes_until_nul(&buffer) {
        Ok(s) => s
            .to_str()
            .map(String::from)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8")),
        Err(_) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No null terminator found",
        )),
    }

    // Ok("none".to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");

    let args: Vec<String> = env::args().skip(1).collect();
    println!("args: {args:?}");

    let mut lockfile_fd: Option<(Pid, RawFd)> = None;

    let lockfile_path =
        env::var("HOME")? + "/.var/app/org.vinegarhq.Sober/data/sober/assets/base.apk";

    let mut command = Command::new(args[0].clone());
    command.args(&args[1..]);

    unsafe {
        command.pre_exec(|| nix::sys::ptrace::traceme().map_err(|e| e.into()));
    }

    let child = command.spawn()?;
    let child_pid = Pid::from_raw(child.id() as _);
    println!("child pid: {child_pid}");

    let mut fd_map: HashMap<(Pid, RawFd), String> = HashMap::new();

    // waitpid(None, None)?;
    ptrace::setoptions(
        child_pid,
        Options::PTRACE_O_TRACESYSGOOD
            | Options::PTRACE_O_TRACEEXIT
            | Options::PTRACE_O_TRACEEXEC
            | Options::PTRACE_O_TRACEFORK
            | Options::PTRACE_O_TRACEVFORK
            | Options::PTRACE_O_TRACECLONE,
    )?;

    // let res = waitpid(child_pid, None)?;
    // println!("first wait: {res:?}");

    // ptrace::syscall(child_pid, None)?;

    // loop {
    //     let status = waitpid(Pid::from_raw(-1), None)?;
    //     let pid = status.pid().unwrap();
    //     let regs = ptrace::getregs(pid)?;
    //     // if let WaitStatus::PtraceEvent(pid, Signal::SIGTRAP, PTRACE_EVENT_FORK) = status {
    //     //     let new_child = ptrace::getevent(pid)?;
    //     //     let new_child_pid = Pid::from_raw(new_child as _);
    //     //     println!("new child: {new_child}");
    //     //     // ptrace::getevent(new_child_pid)?;
    //     //     //
    //     //     // ptrace::syscall(new_child_pid, None)?;
    //     // };
    //
    //     if regs.orig_rax == SYS_fcntl as u64 {
    //         eprintln!(
    //             "[pid: {}] {}({:x}, {:x}, {:x}, ...) = {:x}",
    //             pid.as_raw(),
    //             regs.orig_rax,
    //             regs.rdi,
    //             regs.rsi,
    //             regs.rdx,
    //             regs.rax,
    //         );
    //         if regs.rsi == F_SETLK as u64 {
    //             println!("setlk!!");
    //         }
    //     }
    //
    //     ptrace::syscall(pid, None)?;
    // }

    loop {
        let status = waitpid(None, None)?;

        match status {
            WaitStatus::Stopped(pid, signal) => {
                // child called exec
                if signal == Signal::SIGTRAP {
                    println!("sigtrap");

                    let regs = ptrace::getregs(pid)?;
                    log_syscall(pid, regs);

                    ptrace::syscall(pid, None)?;
                    continue;
                }

                // process created child, child will stop with SIGSTOP
                if signal == Signal::SIGSTOP {
                    // TODO log child creation here maybe
                    ptrace::syscall(pid, None)?;
                    continue;
                }

                // child process terminates, gets interrupted or or resumes after interruption,
                // this stops the parent and we need to continue execution with PTRACE_SYSCALL
                if signal == Signal::SIGCHLD {
                    ptrace::syscall(pid, Some(signal))?;
                    continue;
                }

                ptrace::cont(pid, signal)?;
            }
            WaitStatus::Exited(pid, _) => {
                if pid == child_pid {
                    break;
                } else {
                    continue;
                }
            }
            WaitStatus::PtraceEvent(pid, _, code) => {
                ptrace::syscall(pid, None)?;
            }
            WaitStatus::PtraceSyscall(pid) => {
                let event = ptrace::getevent(pid)?;

                let regs = ptrace::getregs(pid)?;
                let syscall_nr = regs.orig_rax;

                // log_syscall(pid, regs);

                // get the fd for base.apk
                if (regs.rax as i32 != -ENOSYS) // -ENOSYS indicates syscall enter on x86
                    && (syscall_nr == SYS_openat as u64)
                    && regs.rdi as i32 == AT_FDCWD
                {
                    println!("openat at_fdcwd!!");
                    // log_syscall(pid, regs);

                    let arg2 = regs.rsi; // filename address
                    let name = read_remote_str(pid, arg2 as usize, PATH_MAX as usize)?;
                    println!("name: {name}");
                    // if name == lockfile_path {
                    if name.starts_with("/home/xun/.var/app/org.vinegarhq.Sober/") {
                        println!("is lockfile!!");
                        lockfile_fd = Some((pid, regs.rax as i32));
                        fd_map.insert((pid, regs.rax as i32), name);
                    }
                }

                'b: {
                    // if let Some((fd_pid, fd)) = lockfile_fd {
                    if let Some((fd_pid, fd)) = lockfile_fd {
                        if fd_pid != pid {
                            break 'b;
                        }

                        // runs on syscall exit since -ENOSYS means syscall enter
                        if (regs.rax as i32 != -ENOSYS)
                            && (syscall_nr == SYS_fcntl as u64)
                            && (regs.rdi as i32 == fd)
                            && (regs.rsi == F_SETLK as u64)
                        {
                            // log_syscall(pid, regs);
                            println!("fcntl F_SETLK on lockfile fd");
                            let mut regs = regs;
                            regs.rax = 0;
                            ptrace::setregs(pid, regs)?;
                        }
                    }
                }

                // if syscall_nr == SYS_fcntl as u64 {
                // }

                log_syscall(pid, regs);
                ptrace::syscall(pid, None)?;
            }
            WaitStatus::Signaled(pid, signal, coredump) => {
                println!(
                    "Child {} terminated with signal {:?} {}",
                    pid,
                    signal,
                    if coredump { "(core dumped)" } else { "" }
                );
                break;
            }
            WaitStatus::Continued(_) | WaitStatus::StillAlive => {
                continue;
            }
        }
    }

    Ok(())
}
