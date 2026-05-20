use super::{Args, show_heatmap};

pub fn show_debug_heatmap(rows: usize, cols: usize) {
    let n = rows * cols;
    let mut rng = crate::utils::rand::Lcg::new(42);
    let mut buckets = Vec::with_capacity(n);
    for _ in 0..n {
        buckets.push(rng.next_u8() % 11); // 0..=10
    }
    show_heatmap(Args {
        buckets,
        rows,
        cols,
        terminal_width: None,
    });
}
