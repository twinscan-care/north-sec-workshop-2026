pub const HOOK_ADDRESS: usize = 0x009c253a;
pub const RETURN_AFTER_HOOK: usize = 0x009c2547;
pub const PAYLOAD_ADDRESS: usize = 0x00a5c9d8;

// Little endian of "405af337". Any guid starting with this will trigger the backdoor.
pub const BACKDOOR_KEY: u64 = 0x3733336661353034;

pub const PAGE_MASK: usize = 0xfffffffffffff000;
