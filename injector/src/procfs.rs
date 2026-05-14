use std::{
    fs::OpenOptions,
    io::{Seek, Write},
};

use libc::pid_t;

use crate::constants::*;

pub fn inject(pid: pid_t, hook_payload: &[u8], payload: &[u8]) {
    println!("Injecting into process {} via procfs", pid);

    let mut file = OpenOptions::new()
        .write(true)
        .open(format!("/proc/{}/mem", pid))
        .unwrap();

    file.seek(std::io::SeekFrom::Start(HOOK_ADDRESS as u64))
        .unwrap();
    file.write_all(hook_payload).unwrap();

    file.seek(std::io::SeekFrom::Start(PAYLOAD_ADDRESS as u64))
        .unwrap();
    file.write_all(payload).unwrap();
}
