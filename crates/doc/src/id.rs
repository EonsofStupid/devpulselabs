//! Slide identity (LS2): a ULID — 26 Crockford base32 characters holding
//! 48 bits of milliseconds and 80 random bits. Sortable by creation time
//! (the newest draft really is the last one), never a path or a folder
//! name. Hand-rolled so this crate keeps its dependency ceiling
//! (coldlight + serde).

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A fresh slide id.
pub fn new_id() -> String {
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
        & ((1 << 48) - 1);
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    // `RandomState` is seeded from the OS per instance; two of them give
    // the 80 random bits without a crate.
    let random = |salt: u64| {
        let mut hasher = RandomState::new().build_hasher();
        hasher.write_u64(salt);
        hasher.write_u64(sequence);
        hasher.write_u64(millis);
        hasher.finish()
    };
    let random80 = (u128::from(random(1) & 0xFFFF) << 64) | u128::from(random(2));
    encode((u128::from(millis) << 80) | random80)
}

fn encode(mut value: u128) -> String {
    let mut out = [0u8; 26];
    for slot in out.iter_mut().rev() {
        *slot = CROCKFORD[(value & 31) as usize];
        value >>= 5;
    }
    out.iter().map(|&b| char::from(b)).collect()
}

/// Is this a well-formed slide id?
pub fn is_id(value: &str) -> bool {
    value.len() == 26
        && value.bytes().all(|b| CROCKFORD.contains(&b))
        // 26 × 5 = 130 bits: the top character carries only 3 of the 128.
        && value.as_bytes()[0] <= b'7'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_well_formed_unique_and_sort_by_time() {
        let first = new_id();
        let second = new_id();
        assert!(is_id(&first) && is_id(&second), "{first} {second}");
        assert_ne!(first, second);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let later = new_id();
        assert!(later > first, "{later} sorts after {first}");
        assert!(!is_id("authored-draft-2"));
    }
}
