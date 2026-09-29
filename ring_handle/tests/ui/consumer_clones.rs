//! Must not compile: there is no second consumer.
//!
//! C5 of `docs/type/002_consumer.md`, and the mirror of
//! [`producer_clones`](producer_clones.rs). Also outside feature 179's stated
//! two-case criterion.
//!
//! Two consumers is the same class of defect as two producers and is easier to
//! talk yourself into — "it's only reading" — so the symmetric case is written
//! rather than left implied. A second consumer takes records the first will
//! never see, and on an SPSC ring both advance the same read cursor.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( _producer, consumer ) = ends.split();

  let _second = consumer.clone();
}
