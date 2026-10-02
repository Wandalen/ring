//! Must not compile: the publishing end has no drain.
//!
//! The first compile-fail case of the acceptance criterion, and a breach of
//! the rule that capability follows the handle. If this file ever
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
