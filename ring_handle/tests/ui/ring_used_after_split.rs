//! Must not compile: the ring is gone once it has been split.
//!
//! The first step of splitting a ring into two ends, and the narrowing that
//! `ring_core::Ring::ends` does not provide, because it borrows rather than
//! consumes.
//!
//! Without this, the problem the handles exist to solve is only half
//! addressed. The caller still holds the ring, can split it a second time
//! after the first pair drops, and has a third route to a value that is
//! supposed to have exactly two.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let _split = Split::new( ring );

  let _capacity = ring.capacity();
}
