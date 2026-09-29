//! Must not compile: the draining end cannot publish.
//!
//! Feature 179's second compile-fail case, and V2 of
//! `docs/invariant/001_capability_follows_the_handle.md`.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( _producer, mut consumer ) = ends.split();

  let _published = consumer.try_push( 1u32 );
}
