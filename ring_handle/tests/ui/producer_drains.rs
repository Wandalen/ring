//! Must not compile: the publishing end has no drain.
//!
//! Feature 179's first compile-fail case, and V1 of
//! `docs/invariant/001_capability_follows_the_handle.md`. If this file ever
//! compiles, a drain-shaped method has appeared on `Producer` and the
//! capability partition is no longer a partition.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( mut producer, _consumer ) = ends.split();

  let _drained = producer.try_recv();
}
