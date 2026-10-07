//! Helpers shared by the integration test crates.

/// Ends that outlive a loom model's threads.
///
/// `loom::thread::spawn` requires `'static` closures, so nothing a model
/// spawns may borrow a local. The ring and its ends are therefore leaked
/// rather than scoped. That is one leak per execution, which is what loom's
/// own test runner expects and why its models are kept to two slots.
#[cfg(loom)]
pub fn leaked_ends() -> &'static mut ring_mpsc::Ends<'static, ring_slot::TypedSlot<u8>> {
  let capacity = ring_types::Capacity::new(2).expect("a power of two");
  let ring: &'static mut ring_mpsc::Ring<ring_slot::TypedSlot<u8>> = Box::leak(Box::new(ring_mpsc::Ring::new(capacity)));

  Box::leak(Box::new(ring.ends()))
}
