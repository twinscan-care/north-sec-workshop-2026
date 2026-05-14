use std::arch::global_asm;

use crate::constants::*;

global_asm!(
    "
payload:
    mov rax, {example}
payload_end:
",
    example = const EXAMPLE_CONSTANT,
);

// These exports points to the memory location of the labels
unsafe extern "C" {
    pub fn payload();
}
