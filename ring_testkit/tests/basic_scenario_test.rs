//! Basic scenario: a ring built through `ring_factory` delivers what was pushed.
#![cfg(not(loom))]

use ring_factory::{Factory, RingConfig};

#[test]
fn factory_ring_transfers_one_value() {
  let config = RingConfig::new(8).expect("valid ring capacity");

  let mut split = Factory.build::<u32>(config).expect("factory should build the ring");

  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  producer.try_push(7).expect("ring should accept the value");

  assert_eq!(consumer.try_recv(), Some(7));
}
