//! Tests for `ring_align` — cache-line padding.
//!
//! Claims the padding half of `docs/feature/169_padded_cursor.md`. The feature's
//! own words: "the padding is the whole point", because two cursors sharing a
//! line make every write by either invalidate the other's cached copy. The
//! structural assertion — 64-byte size and alignment — is what a test can
//! decide; the throughput claim the feature also makes needs a number nobody has
//! stated yet, which is why `docs/plan/008_ring_write_path_staged.md` splits
//! stage S3 out and applies scale-invariance recursion inside it.
//!
//! The cursor type that consumes these wrappers is `ring_cursor`; this crate
//! owns only the constant and the wrapper.

use ring_align::{CACHE_LINE, CacheAligned, on_distinct_lines};

/// The constant is 64 — the line size on the family's stated target platforms.
#[test]
fn cache_line_is_sixty_four() {
  assert_eq!(CACHE_LINE, 64);
}

/// The direction of a future change to `CACHE_LINE` matters and nothing but
/// this test said so before now: raising it (-> docs/decisions/001 E1) only
/// spends memory, but lowering it silently defeats every guarantee in this
/// crate, because `on_distinct_lines` stays calibrated to the same wrong
/// number it divides by (-> docs/pitfall/001, docs/decisions/001 AL14).
/// Pinned as a floor rather than an exact value — `cache_line_is_sixty_four`
/// above already pins the value — so a deliberate raise still passes and
/// only a decrease trips this.
#[test]
fn cache_line_must_not_shrink_below_the_current_known_minimum() {
  // `core::hint::black_box` defeats the compiler's constant-folding of
  // `CACHE_LINE >= 64` (both operands are literals today), which otherwise
  // trips `clippy::assertions_on_constants` — the comparison is a genuine
  // runtime regression guard against a future *decrease* of the constant,
  // not dead code the lint should silence.
  assert!(
    core::hint::black_box(CACHE_LINE) >= 64,
    "CACHE_LINE dropped below 64 - every assertion in this suite would still \
     pass while the padding guarantee silently stopped holding, see \
     docs/pitfall/001_a_constant_too_small_buys_nothing.md"
  );
}

/// A wrapped value occupies exactly one line, for every payload smaller than
/// one. Both size and alignment, because either alone is insufficient: an
/// aligned type smaller than a line still lets a second one share it.
#[test]
fn a_wrapped_value_occupies_exactly_one_line() {
  assert_eq!(core::mem::align_of::<CacheAligned<u8>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<u8>>(), CACHE_LINE);
  assert_eq!(core::mem::align_of::<CacheAligned<u64>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<u64>>(), CACHE_LINE);
  assert_eq!(core::mem::size_of::<CacheAligned<[u8; 63]>>(), CACHE_LINE);
}

/// A payload larger than a line still gets whole lines, so two of them never
/// share one.
#[test]
fn an_oversized_payload_rounds_up_to_whole_lines() {
  let size = core::mem::size_of::<CacheAligned<[u8; 65]>>();
  assert_eq!(size % CACHE_LINE, 0, "size {size} is not a whole number of lines");
  assert_eq!(size, 128);
}

/// The observable consequence, and the one the feature actually cares about:
/// two wrapped fields in one struct land on different cache lines. Asserted on
/// real addresses, not on `size_of` alone.
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

/// Two *unwrapped* fields do share a line — the negative control that shows the
/// test above is measuring the padding rather than something the layout would
/// have done anyway.
#[test]
fn two_unwrapped_fields_share_a_line() {
  #[derive(Debug)]
  struct Naive {
    producer: u64,
    consumer: u64,
  }

  let pair = Naive {
    producer: 0,
    consumer: 0,
  };
  let a = core::ptr::from_ref(&pair.producer) as usize;
  let b = core::ptr::from_ref(&pair.consumer) as usize;
  assert!(a.abs_diff(b) < CACHE_LINE);
  assert!(!on_distinct_lines(a, b), "the unpadded pair should share a line");
}

/// `on_distinct_lines` reads line boundaries, not raw distance: two addresses
/// 2 bytes apart can straddle a boundary and two 62 apart can share a line.
#[test]
fn distinct_lines_follows_boundaries_not_distance() {
  assert!(!on_distinct_lines(0, 63), "0 and 63 are both in line 0");
  assert!(on_distinct_lines(63, 64), "63 and 64 straddle the boundary");
  assert!(on_distinct_lines(0, 64));
  assert!(!on_distinct_lines(64, 127));
  assert!(on_distinct_lines(127, 128));
  assert!(!on_distinct_lines(100, 100));
}

/// The wrapper is transparent to its payload: what goes in comes out, through
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
