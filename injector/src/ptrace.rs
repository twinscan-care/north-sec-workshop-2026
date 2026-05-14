use libc::{
    PTRACE_ATTACH, PTRACE_CONT, PTRACE_DETACH, PTRACE_GETREGS, PTRACE_PEEKDATA, PTRACE_POKETEXT,
    PTRACE_SETREGS, SIGSTOP, WUNTRACED, pid_t, ptrace, size_t, waitpid,
};

use crate::constants::*;

struct AttachedProcess(pid_t);

pub fn inject(pid: pid_t, hook_payload: &[u8], payload: &[u8]) {
    println!("Injecting into process {} via PTRACE", pid);

    let mut payload = payload.to_vec();
    pad(&mut payload);

    // Attach to the process.
    let process = AttachedProcess::new(pid);

    // Wait for the process
    println!("Waiting for SIGSTOP...");
    process.wait_sig(SIGSTOP);
    println!("SIGSTOP received");

    // Read page and add hook
    let prefix_pad = HOOK_ADDRESS - HOOK_ADDRESS / 8 * 8;
    let payload_end = prefix_pad + hook_payload.len();
    let suffix_pad = 8 - (payload_end % 8);
    let mut page_backup = vec![0u8; prefix_pad + hook_payload.len() + suffix_pad];
    process.read(HOOK_ADDRESS / 8 * 8, &mut page_backup);

    page_backup[prefix_pad..prefix_pad + hook_payload.len()].copy_from_slice(&hook_payload);

    // Inject the hook
    process.write(HOOK_ADDRESS / 8 * 8, &page_backup);

    // Inject the payload. Here we don't care about what's being overwritten as we don't intend to restore it
    process.write(PAYLOAD_ADDRESS, &payload);

    // Continue the process
    process.cont();
}

/// SAFETY
/// Functions here only uses safe data structure and only the target process is unsafe, not the main process
impl AttachedProcess {
    fn new(pid: pid_t) -> Self {
        // Attach to the process.
        let res = unsafe { ptrace(PTRACE_ATTACH, pid, 0, 0) };
        if res == -1 {
            panic!("Failed to attach to process");
        }

        AttachedProcess(pid)
    }

    fn write(&self, address: size_t, data: &[u8]) {
        for i in (0..data.len()).step_by(std::mem::size_of::<size_t>()) {
            let word = size_t::from_ne_bytes(
                data[i..i + std::mem::size_of::<size_t>()]
                    .try_into()
                    .unwrap(),
            );
            unsafe { ptrace(PTRACE_POKETEXT, self.0, address + (i as size_t), word) };
        }
    }

    fn read(&self, address: size_t, buffer: &mut [u8]) {
        for i in (0..buffer.len()).step_by(std::mem::size_of::<size_t>()) {
            let word = unsafe { ptrace(PTRACE_PEEKDATA, self.0, address + (i as size_t), 0) };
            buffer[i..i + std::mem::size_of::<size_t>()].copy_from_slice(&word.to_ne_bytes());
        }
    }

    fn _get_regs(&self) -> libc::user_regs_struct {
        let mut regs: libc::user_regs_struct = unsafe { std::mem::zeroed() };
        unsafe { ptrace(PTRACE_GETREGS, self.0, 0, &mut regs) };
        regs
    }

    fn _set_regs(&self, regs: libc::user_regs_struct) {
        unsafe { ptrace(PTRACE_SETREGS, self.0, 0, &regs) };
    }

    fn wait_sig(&self, signal: libc::c_int) {
        loop {
            let mut status = 0;
            unsafe { waitpid(self.0, &raw mut status, WUNTRACED) };

            if status != 0 {
                println!("Status: {:x}", status);

                if libc::WIFEXITED(status) {
                    panic!(
                        "Attached process exited with status {}",
                        libc::WEXITSTATUS(status)
                    );
                }

                if libc::WIFSTOPPED(status) {
                    let sig = libc::WSTOPSIG(status);
                    if sig == signal {
                        break;
                    };
                }
            }
        }
    }

    fn cont(&self) {
        unsafe { ptrace(PTRACE_CONT, self.0, 0, 0) };
    }
}

impl Drop for AttachedProcess {
    fn drop(&mut self) {
        // Detach from the process.
        let _ = unsafe { ptrace(PTRACE_DETACH, self.0, 0, 0) };
    }
}

// Pad de buffer so it's aligned to qwords for read and writing
fn pad(buffer: &mut Vec<u8>) {
    let new_size =
        ((buffer.len() - 1) / std::mem::size_of::<size_t>() + 1) * std::mem::size_of::<size_t>();
    buffer.resize(new_size, 0x90);
}
