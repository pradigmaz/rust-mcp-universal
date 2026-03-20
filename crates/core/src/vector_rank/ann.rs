use super::{ANN_BITS_PER_FAMILY, ANN_BUCKET_FAMILIES};

pub(super) fn ann_bucket_keys_impl(vector: &[f32]) -> Vec<(i64, String)> {
    if vector.is_empty() {
        return Vec::new();
    }

    let mut keys = Vec::with_capacity(ANN_BUCKET_FAMILIES);
    for family in 0..ANN_BUCKET_FAMILIES {
        let mut bits = 0_u64;
        for bit in 0..ANN_BITS_PER_FAMILY {
            let slot = ann_projection_index(vector.len(), family, bit);
            if vector[slot].is_finite() && vector[slot] >= 0.0 {
                bits |= 1_u64 << bit;
            }
        }
        keys.push((
            i64::try_from(family).unwrap_or(i64::MAX),
            format!("{bits:04x}"),
        ));
    }
    keys
}

fn ann_projection_index(dim: usize, family: usize, bit: usize) -> usize {
    let seed = (family.wrapping_mul(131)).wrapping_add(bit.wrapping_mul(47));
    let mixed = seed
        .wrapping_mul(seed.wrapping_add(17))
        .wrapping_add(bit * 13);
    mixed % dim
}
