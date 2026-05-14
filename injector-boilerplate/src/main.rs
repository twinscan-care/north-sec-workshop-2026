use clap::Parser;
use libc::pid_t;

mod asm;
mod constants;

#[derive(clap::Parser)]
struct Cli {
    /// The PID of the process to inject into
    pid: pid_t,
}

fn main() {
    // Interpret the asm code as byte slices
    // ASM that jumps to the backdoor
    let payload = unsafe {
        std::slice::from_raw_parts(
            asm::hook_start as *const u8,
            asm::payload_start as *const u8 as usize - asm::hook_start as *const u8 as usize,
        )
    };

    let cli = Cli::parse();

    let pid = cli.pid;

    todo!("inject payload into process with pid {}", pid);
}
