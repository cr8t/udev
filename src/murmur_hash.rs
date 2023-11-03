/// MurmurHash2 was written by Austin Appleby, and is placed in the public
/// domain. The author hereby disclaims copyright to this source code.
///
/// Note - This code makes a few assumptions about how your machine behaves -
///
/// 1. We can read a 4-byte value from any address without crashing
/// 2. sizeof(int) == 4
///
/// And it has a few limitations -
///
/// 1. It will not work incrementally.
/// 2. It will not produce the same results on little-endian and big-endian
///    machines.
pub fn murmur_hash2(key: &[u8], seed: u32) -> u32 {
    // 'm' and 'r' are mixing constants generated offline.
    // They're not really 'magic', they just happen to work well.

    const M: u32 = 0x5bd1e995;
    const R: u32 = 24;

    // Initialize the hash to a 'random' value

    let mut h = seed ^ (key.len() as u32);

    // Mix 4 bytes at a time into the hash
    key.chunks_exact(4).for_each(|data| {
        let mut k = u32::from_ne_bytes(data.try_into().unwrap_or([0u8; 4]));

        k = k.saturating_mul(M);
        k ^= k >> R;
        k = k.saturating_mul(M);

        h = h.saturating_mul(M);
        h ^= k;
    });

    let key_len = key.len();
    let mod_len = key_len % 4;

    // Handle the last few bytes of the input array

    match mod_len {
        3 => {
            h ^= (key[key_len - 1] as u32) << 16;
            h ^= (key[key_len - 2] as u32) << 8;
            h ^= key[key_len - 3] as u32;
        }
        2 => {
            h ^= (key[key_len - 1] as u32) << 8;
            h ^= key[key_len - 2] as u32;
        }
        1 => h ^= key[key_len - 1] as u32,
        _ => (),
    }

    h = h.saturating_mul(M);

    // Do a few final mixes of the hash to ensure the last few
    // bytes are well-incorporated.

    h ^= h >> 13;
    h = h.saturating_mul(M);
    h ^= h >> 15;

    h
}
