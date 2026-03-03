use sysinfo::System;

const BYTES_PER_MB: u64 = 1_048_576;

/// Returns `(total_mb, available_mb)`.
pub fn query(sys: &mut System) -> (u64, u64) {
    sys.refresh_memory();
    let total = sys.total_memory() / BYTES_PER_MB;
    let avail = sys.available_memory() / BYTES_PER_MB;
    (total, avail)
}
