//! Shared harness for the fuzz targets: the byte-stream reader, the
//! capacities and op budget, and the drain-and-compare helper.
//!
//! Lives beside the targets and is pulled in with `#[path]`, not as a
//! `src/` library crate: G22 reads a `pub` item under `src/` as the end of
//! this directory's exemption, so promoting this file there would turn the
//! exemption stale and fail the gate. Keep it here.

use std::collections::VecDeque;

/// Byte stream reader that never runs dry: past the end every read is zero,
/// so long inputs keep exercising wrap-around instead of stopping early.
pub struct Reader<'a> {
  data: &'a [u8],
  pos: usize,
}

impl<'a> Reader<'a> {
  /// New reader over `data`.
  pub fn new(data: &'a [u8]) -> Self {
    Self { data, pos: 0 }
  }

  /// Next byte, or zero past the end.
  pub fn byte(&mut self) -> u8 {
    let byte = *self.data.get(self.pos).unwrap_or(&0);
    self.pos += 1;
    byte
  }

  /// Next eight bytes as a little-endian `u64`.
  pub fn intake(&mut self) -> u64 {
    let mut value = 0u64;
    for shift in 0..8 {
      value |= (self.byte() as u64) << (8 * shift);
    }
    value
  }
}

/// Capacities under test, picked by the first input byte.
pub const CAPACITIES: [usize; 6] = [1, 2, 4, 8, 32, 128];
/// Operations per input; the stream past the end reads as zeroes.
pub const OPS: usize = 1024;

/// Drain-check against the oracle, in order. Takes the batch length plus a
/// take-by-offset closure instead of a consumer: the two consumer types
/// share method names but no trait, so a generic over the consumer would
/// need a new trait for one call site. The closure keeps the FIFO assertion
/// in exactly one place.
pub fn drain_all(
  len: usize,
  mut take: impl FnMut(usize) -> Option<u64>,
  oracle: &mut VecDeque<u64>,
) {
  for offset in 0..len {
    assert_eq!(take(offset), oracle.pop_front(), "records must come back FIFO");
  }
}
