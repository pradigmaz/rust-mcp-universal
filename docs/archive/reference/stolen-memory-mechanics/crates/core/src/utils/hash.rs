use sha2::{Digest, Sha256};

pub(crate) fn hash_bytes(value: &[u8]) -> String {
    let digest = Sha256::digest(value);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}
