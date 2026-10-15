//! Must not compile: the draining end cannot publish.
//!
//! The second compile-fail case of the acceptance criterion, and a breach of
//! the rule that capability follows the handle.

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
