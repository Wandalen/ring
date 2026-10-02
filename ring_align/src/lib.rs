//! Cache-line padding constants and alignment wrappers.
//!
//! Tier 1 of the ring family's 33 crates, which implement the concurrency write-path.
//! Depends on `ring_types`.
//!
//! `docs/feature/169_padded_cursor.md` states the problem this crate exists to
//! solve. A producer cursor and a consumer cursor that share a cache line make
//! every write by either invalidate the other's cached copy, so two cores
//! contend on a line neither is sharing data through. The fix is to give each
//! its own line. This crate holds the constant and the wrapper;
//! `ring_cursor` holds the cursors that use them.
//!
//! No `unsafe` is needed for any of it, because `#[ repr( align( 64 ) ) ]` is a
//! safe attribute. So this crate compiles under the workspace-wide
//! `unsafe-code = "deny"`, like most of the family. It once held an entry in
//! `ring/bench_harness/gate/declared/ring/unsafe_allowlist.txt` permitting an
//! opt-out it never took.
//! [decision 123](../../../docs/decision/123_ring_shared_slot_storage_unsafe_sited.md)
//! removed it, on the ground that a permission nobody exercises is a bound
//! looser than the code actually is.

#![deny(missing_docs)]

/// Bytes in a cache line on the family's target platforms.
///
/// 64 on x86-64 and on AArch64's common configuration. Apple Silicon uses 128.
/// A value too small is the failure that matters, because two cursors 64 bytes
/// apart still share a 128-byte line. So a future port raises this rather than
/// making it conditional per crate.
///
/// ```
/// assert_eq!( ring_align::CACHE_LINE, 64 );
/// ```
pub const CACHE_LINE: usize = 64;

// `on_distinct_lines` below computes `a / CACHE_LINE` as a line index. That
// equals the true line index only because lines are naturally aligned, which
// holds only when `CACHE_LINE` is a power of two. Every value this family has
// used (64, 128) is one, and nothing checked it until now (-> docs/algorithm/001
// AL1). `ring_types::Capacity::new` asserts the analogous precondition for
// its own number. This is the compile-time form, since a `pub const` needs no
// runtime `Result`.
const _: () = assert!(
  CACHE_LINE.is_power_of_two(),
  "CACHE_LINE must be a power of two: on_distinct_lines divides by it to compute a line index"
);

/// A value given a whole cache line to itself.
///
/// The padding is the point. `size_of::<CacheAligned<T>>()` is a multiple of
/// [`CACHE_LINE`] for any non-zero-sized `T` that fits, so two of them in one
/// struct are guaranteed to land on different lines. (A zero-sized `T` makes
/// the wrapper itself zero-sized, so two of them coincide at one address.
/// Nothing in this family wraps one.)
///
/// ```
/// use ring_align::{ CacheAligned, CACHE_LINE };
///
/// let padded = CacheAligned::new( 7u64 );
/// assert_eq!( *padded.get(), 7 );
/// assert_eq!( core::mem::align_of::< CacheAligned< u64 > >(), CACHE_LINE );
/// assert_eq!( core::mem::size_of::< CacheAligned< u64 > >(), CACHE_LINE );
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(align(64))]
pub struct CacheAligned<T>(T);

impl<T> CacheAligned<T> {
  /// Wrap a value so it occupies a cache line alone.
  ///
  /// ```
  /// use ring_align::CacheAligned;
  /// assert_eq!( *CacheAligned::new( 1u32 ).get(), 1 );
  /// ```
  pub const fn new(value: T) -> Self {
    Self(value)
  }

  /// Borrow the wrapped value.
  ///
  /// ```
  /// use ring_align::CacheAligned;
  /// let a = CacheAligned::new( 5u8 );
  /// assert_eq!( *a.get(), 5 );
  /// ```
  pub const fn get(&self) -> &T {
    &self.0
  }

  /// Borrow the wrapped value mutably.
  ///
  /// ```
  /// use ring_align::CacheAligned;
  /// let mut a = CacheAligned::new( 5u8 );
  /// *a.get_mut() = 9;
  /// assert_eq!( *a.get(), 9 );
  /// ```
  pub const fn get_mut(&mut self) -> &mut T {
    &mut self.0
  }

  /// Unwrap, discarding the padding.
  ///
  /// ```
  /// use ring_align::CacheAligned;
  /// assert_eq!( CacheAligned::new( 3u16 ).into_inner(), 3 );
  /// ```
  pub fn into_inner(self) -> T {
    self.0
  }
}

/// Whether two addresses fall on different cache lines.
///
/// The observable form of what [`CacheAligned`] buys. A test asserts this over
/// two real fields rather than trusting `size_of` alone.
///
/// Takes addresses as plain integers rather than references, because the
/// question is where two *fields* sit. The caller already has the addresses,
/// and two stack locals in a doc example would say nothing.
///
/// ```
/// use ring_align::on_distinct_lines;
///
/// assert!( !on_distinct_lines( 0, 63 ) );    // both in line 0
/// assert!( on_distinct_lines( 63, 64 ) );    // straddling the boundary
/// assert!( !on_distinct_lines( 128, 130 ) ); // both in line 2
/// ```
#[must_use]
pub const fn on_distinct_lines(a: usize, b: usize) -> bool {
  a / CACHE_LINE != b / CACHE_LINE
}
