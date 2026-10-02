//! Must not compile: there is no second producer.
//!
//! A breach of the rule that capability follows the handle, and **not part
//! of the stated acceptance criterion**. The criterion names two cases and
//! this is a third. It is here because this is the violation with the worst
//! consequence. Two producers against an SPSC ring is a data race,
//! and `ring_spsc`'s correctness argument assumes it cannot happen.
//!
//! # The receiver must stay owned
//!
//! `producer` is bound by value on purpose. Rust's method resolution tries the
//! receiver type, then `&` and `&mut` of it, so with an owned `Producer` there
//! is no `clone` to find. Bind a `&Producer` instead and `.clone()` resolves to
//! `<&Producer as Clone>::clone`, which copies the *reference* and compiles.
//!
//! That rewrite does not silently weaken the case. It makes the file compile,
//! and a `compile_fail` case that compiles is a test failure, not a pass. The
//! failure this case cannot see is a different one: failing for the wrong
//! reason, which is what the pinned `.stderr` is for.

use ring_config::RingConfig;
use ring_core::Ring;
use ring_handle::Split;

fn main()
{
  let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
  let mut split = Split::new( ring );
  let mut ends = split.ends();
  let ( producer, _consumer ) = ends.split();

  let _second = producer.clone();
}
