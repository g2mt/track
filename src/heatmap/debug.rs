use super::{Args, show_heatmap};

/// Simple LCG PRNG
struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u8(&mut self) -> u8 {
        self.state = self
            .state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.state >> 33) as u8
    }
}

pub fn show_debug_heatmap(rows: usize, cols: usize) {
    let n = rows * cols;
    let mut rng = Lcg::new(42);
    let mut buckets = Vec::with_capacity(n);
    for _ in 0..n {
        buckets.push(rng.next_u8() % 11); // 0..=10
    }
    show_heatmap(Args {
        buckets,
        rows,
        cols: Some(cols),
    });
}
