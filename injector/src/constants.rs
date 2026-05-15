pub const HOOK_ADDRESS: usize = 0x009c0188;
pub const RETURN_AFTER_HOOK: usize = 0x009c0195;
pub const PAYLOAD_ADDRESS: usize = 0x005db980;

// Little endian of "405af337". Any guid starting with this will trigger the backdoor.
pub const BACKDOOR_KEY: u64 = 0x3733336661353034;

pub const PAGE_MASK: usize = 0xfffffffffffff000;
