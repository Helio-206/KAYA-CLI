pub const MESH_VERSION: u16 = 1;
pub const DEFAULT_MESH_TTL: u8 = 5;

pub fn decrement_ttl(ttl: u8) -> Option<u8> {
    ttl.checked_sub(1).filter(|value| *value > 0)
}
