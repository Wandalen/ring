//! Tests for `ring_align`'s cache-line padding.
//!
//! Claims the padding half of `docs/feature/169_padded_cursor.md`. In the
//! feature's own words, "the padding is the whole point", because two cursors
//! sharing a line make every write by either invalidate the other's cached copy.
//! A test can decide the structural assertion, line-sized size and alignment. The
//! throughput claim the feature also makes needs a number nobody has stated yet.
//!
//! The cursor type that consumes these wrappers is `ring_cursor`; this crate
//! owns only the constant and the wrapper.

use ring_align::{CACHE_LINE, CacheAligned, on_distinct_lines};

/// The constant follows the per-target table
/// (-> `docs/decisions/002_cache_line_follows_the_target_architecture.md`).
///
/// Restated here from `std::env::consts::ARCH` and `OS`, a second source for
/// the target than the crate's `cfg!`s. So a wrong row fails on the machine
/// that runs it rather than agreeing with itself. Apple's operating systems
/// stand in for `target_vendor = "apple"`.
#[test]
fn cache_line_follows_the_architecture_table() {
  use std::env::consts::{ARCH, OS};

  let apple = matches!(OS, "macos" | "ios" | "tvos" | "watchos" | "visionos");
  let expected = match ARCH {
    "x86_64" | "powerpc64" => 128,
    "aarch64" if apple => 128,
    "arm" | "mips" | "mips32r6" | "mips64" | "mips64r6" | "sparc" | "hexagon" => 32,
    "m68k" => 16,
    "s390x" => 256,
    _ => 64,
  };
  assert_eq!(CACHE_LINE, expected, "on {ARCH} / {OS}");
}

/// The direction of a future change to `CACHE_LINE` matters, and before this
/// test nothing said so.
///
/// Raising it only spends memory. Lowering it silently defeats every guarantee
/// in this crate, because `on_distinct_lines` stays calibrated to the same
/// wrong number it divides by
/// (-> `docs/decisions/002_cache_line_follows_the_target_architecture.md`).
/// Pinned as a floor rather than an exact value, because
/// `cache_line_follows_the_architecture_table` above already pins the value. So
/// a deliberate raise still passes and only a decrease trips this.
///
/// Only on the 128-byte rows the family runs on: x86-64, and AArch64 on Apple,
/// whose 128-byte line is the reason this table exists.
#[cfg(any(target_arch = "x86_64", all(target_arch = "aarch64", target_vendor = "apple")))]
#[test]
fn cache_line_must_not_shrink_below_the_current_known_minimum() {
  // `core::hint::black_box` defeats the compiler's constant-folding of
  // `CACHE_LINE >= 128` (both operands are literals today), which otherwise
  // trips `clippy::assertions_on_constants`. The comparison is a genuine
  // runtime regression guard against a future *decrease* of the constant,
  // not dead code the lint should silence.
  assert!(
    core::hint::black_box(CACHE_LINE) >= 128,
    "CACHE_LINE dropped below 128 - every assertion in this suite would still \
     pass while the padding guarantee silently stopped holding, see \
     docs/pitfall/001_a_constant_too_small_buys_nothing.md"
  );
}

/// A wrapped value occupies exactly one line, for every payload smaller than
/// one. Both size and alignment, because either alone is insufficient. An
/// aligned type smaller than a line still lets a second one share it.
#[test]
fn a_wrapped_value_occupies_exactly_one_line() {
  assert_eq!(core::mem::align_of::<CacheAligned<u8>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<u8>>(), CACHE_LINE);
  assert_eq!(core::mem::align_of::<CacheAligned<u64>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<u64>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<[u8; CACHE_LINE - 1]>>(), CACHE_LINE);
}

/// A payload larger than a line still gets whole lines, so two of them never
/// share one.
#[test]
fn an_oversized_payload_rounds_up_to_whole_lines() {
  let size = core::mem::size_of::<CacheAligned<[u8; CACHE_LINE + 1]>>();
  assert_eq!(size % CACHE_LINE, 0, "size {size} is not a whole number of lines");
  assert_eq!(size, 2 * CACHE_LINE);
}

/// The observable consequence, and the one the feature cares about, is that two
/// wrapped fields in one struct land on different cache lines. Asserted on real
/// addresses, not on `size_of` alone.
#[test]
fn two_wrapped_fields_land_on_different_lines() {
  #[derive(Debug)]
  struct TwoCursors {
    producer: CacheAligned<u64>,
    consumer: CacheAligned<u64>,
  }

  let pair = TwoCursors {
    producer: CacheAligned::new(0),
    consumer: CacheAligned::new(0),
  };

  let a = core::ptr::from_ref(&pair.producer) as usize;
  let b = core::ptr::from_ref(&pair.consumer) as usize;
  assert!(a.abs_diff(b) >= CACHE_LINE, "fields are {} bytes apart", a.abs_diff(b));
  assert!(on_distinct_lines(a, b));
}

/// Two *unwrapped* fields do share a line.
///
/// This is the negative control. It shows the test above measures the padding
/// rather than something the layout would have done anyway.
#[test]
fn two_unwrapped_fields_share_a_line() {
  #[derive(Debug)]
  struct Naive {
    producer: u64,
    consumer: u64,
  }

  // Pin `pair` to the start of a cache line so the assertion measures
  // adjacency, not allocator luck. A bare `Naive` on the stack can land
  // with `producer` at the tail of one line and `consumer` in the next.
  // The wrapper pads around the pair, never between its fields.
  #[repr(align(128))]
  struct Aligned(Naive);

  let pair = Aligned(Naive {
    producer: 0,
    consumer: 0,
  });
  let a = core::ptr::from_ref(&pair.0.producer) as usize;
  let b = core::ptr::from_ref(&pair.0.consumer) as usize;
  assert!(a.abs_diff(b) < CACHE_LINE);
  assert!(!on_distinct_lines(a, b), "the unpadded pair should share a line");
}

/// `on_distinct_lines` reads line boundaries, not raw distance. Two addresses
/// 1 byte apart can straddle a boundary, and two a line minus one apart can
/// share a line.
#[test]
fn distinct_lines_follows_boundaries_not_distance() {
  assert!(!on_distinct_lines(0, CACHE_LINE - 1), "both in line 0");
  assert!(on_distinct_lines(CACHE_LINE - 1, CACHE_LINE), "the two straddle the boundary");
  assert!(on_distinct_lines(0, CACHE_LINE));
  assert!(!on_distinct_lines(CACHE_LINE, 2 * CACHE_LINE - 1));
  assert!(on_distinct_lines(2 * CACHE_LINE - 1, 2 * CACHE_LINE));
  assert!(!on_distinct_lines(100, 100));
}

/// The wrapper is transparent to its payload. What goes in comes out, through
/// every accessor.
#[test]
fn the_wrapper_round_trips_its_payload() {
  let mut wrapped = CacheAligned::new(7u64);
  assert_eq!(*wrapped.get(), 7);

  *wrapped.get_mut() = 9;
  assert_eq!(*wrapped.get(), 9);
  assert_eq!(wrapped.into_inner(), 9);

  assert_eq!(CacheAligned::<u32>::default().into_inner(), 0);
  assert_eq!(CacheAligned::new(3u16), CacheAligned::new(3u16));
  assert_ne!(CacheAligned::new(3u16), CacheAligned::new(4u16));
}

/// The wrapper is `Copy` when its payload is, so a cursor read does not have to
/// borrow.
#[test]
fn the_wrapper_is_copy() {
  let a = CacheAligned::new(1u64);
  let b = a;
  assert_eq!(*a.get(), *b.get());
}
