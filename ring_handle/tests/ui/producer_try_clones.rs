//! Must not compile: `ring_core`'s duplication capability is not forwarded.
//!
//! N2 of `docs/decisions/001_what_this_crate_is_for.md` — the narrowing that is
//! most of this crate's justification. `ring_core::Producer::try_clone` exists
//! and *succeeds* on an MPSC backend; on an SPSC ring it refuses at runtime.
//! A handle that cannot name the method refuses at compile time instead.
//!
//! # What this case detects that `producer_clones.rs` does not
//!
//! `producer_clones.rs` catches a `#[derive(Clone)]` added here. This one
//! catches a route *through* to the wrapped handle — specifically
//! `impl Deref for Producer { type Target = ring_core::Producer<'a, T> }`,
//! which would make every `ring_core` capability reachable by autoderef and
//! turn the whole partition advisory (F2 of
//! `docs/pitfall/001_a_convenience_method_undoes_the_crate.md`).
//!
//! **It does not catch an explicit accessor.** A `pub fn inner( &self ) ->
//! &ring_core::Producer` would leave this case rejected exactly as it is now,
//! because autoderef does not fire through a method call. F3 stays open, and
//! the reason is structural: a compile-fail case names one thing that must not
//! exist, and F3 is "any route", which has no name.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( producer, _consumer ) = ends.split();

  let _second = producer.try_clone();
}
