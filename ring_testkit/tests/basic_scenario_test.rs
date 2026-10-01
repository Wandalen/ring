//! Basic scenario: a ring built through `ring_factory` delivers what was pushed.
#![cfg(not(loom))]

use ring_factory::{Factory, RingConfig};
use ring_testkit::audit_received;
use ring_types::OverflowPolicy;

#[test]
fn factory_ring_transfers_one_value() {
  let config = RingConfig::new(8).expect("valid ring capacity");

  let mut split = Factory.build::<u32>(config).expect("factory should build the ring");

  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  producer.try_push(7).expect("ring should accept the value");

  assert_eq!(consumer.try_recv(), Some(7));
}

#[test]
fn factory_built_ring_delivers_what_was_pushed() {
  let cfg = RingConfig::new(4)
    .expect("a power of two")
    .with_overflow(OverflowPolicy::Fail);
  let mut split = Factory.build::<u32>(cfg).expect("Fail is an accepted policy");
  let mut ends = split.ends();
  let (mut producer, mut consumer) = ends.split();

  for record in 0..4 {
    assert_eq!(producer.try_push(record), Ok(()));
  }
  assert_eq!(producer.try_push(4), Err(4), "a full ring under Fail hands the record back");

  let mut received = Vec::new();
  while let Some(record) = consumer.try_recv() {
    received.push(record);
  }

  assert_eq!(received, [0, 1, 2, 3]);
  assert_eq!(audit_received(&received, 4), Ok(()));
}
