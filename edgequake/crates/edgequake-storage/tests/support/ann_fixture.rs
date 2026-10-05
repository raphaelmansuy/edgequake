//! Deterministic independent vector components shared by ANN measurements.

pub const DIM: usize = 32;

pub fn embedding(seed: usize) -> Vec<f32> {
    let mut state = seed as u64 + 1;
    (0..DIM)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state % 100_000) as f32 / 50_000.0 - 1.0
        })
        .collect()
}

pub fn embedding_text(seed: usize) -> String {
    format!(
        "[{}]",
        embedding(seed)
            .iter()
            .map(f32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}
