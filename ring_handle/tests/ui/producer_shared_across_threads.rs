//! Must not compile: a handle cannot be *shared* between threads, only moved.
//!
//! Q5 of `docs/non_functional_requirement/002_send_without_sync.md`. `Send`
//! and `Sync` answer two different questions and the requirement wants both
//! answered deliberately: `Send` is asserted present in
//! `handle_test.rs::both_handles_are_send`, `Sync` is asserted absent here.
//!
//! # Where the absence comes from
//!
//! Not from this crate. `ring_spsc`'s handles carry a
//! `PhantomData< Cell< () > >` — a deliberate `!Sync` opt-out placed in the
//! crate that owns the one-producer-one-consumer invariant — and `ring_core`
//! and then `ring_handle` inherit it structurally.
//!
//! **That inheritance is the reason this case is worth writing rather than
//! assuming.** The property holds today for a reason two crates away, and
//! nothing in this crate's source mentions it. A backend change that dropped
//! the marker would make `&Producer` sendable, and nothing here would say so.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( producer, _consumer ) = ends.split();

  let shared = &producer;
  std::thread::scope
  ( | scope |
  {
    scope.spawn( move || shared.is_full() );
  } );
}
