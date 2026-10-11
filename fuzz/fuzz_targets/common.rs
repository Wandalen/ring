//! Shared byte-stream reader for the fuzz targets.
//!
//! Lives beside the targets and is pulled in with `#[path]`, not as a
//! `src/` library crate: G22 reads a `pub` item under `src/` as the end of
//! this directory's exemption, so promoting this file there would turn the
//! exemption stale and fail the gate. Keep it here.

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
