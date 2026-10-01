//! Batch size: one producer pushing 1, 8, 32 or 128 records per operation into 1024 slots, the
//! consumer taking everything available. A candidate without a batch push has the 1 column only.

mod harness;

use criterion::{Criterion, criterion_group, criterion_main};
use harness::bench::ThreadedGroup;
use harness::candidates::{Candidate, Mpsc, MutexDeque, Rtrb, Spsc};
use harness::driver::{Mode, Shape};

/// Records per run.
const RECORDS: u64 = 1 << 20;

fn batch(c: &mut Criterion) {
  let mut group = ThreadedGroup::new(c, "batch");
  for size in [1, 8, 32, 128] {
    sized::<Spsc>(&mut group, size);
    sized::<Rtrb>(&mut group, size);
    sized::<Mpsc>(&mut group, size);
    sized::<MutexDeque>(&mut group, size);
  }
  group.finish();
}

fn sized<C: Candidate>(group: &mut ThreadedGroup<'_>, size: usize) {
  if size > 1 && !C::PUSH_BATCH {
    return;
  }
  let mode = if size == 1 { Mode::PUSH1_POPN } else { Mode::batch(size) };
  group.bench::<C>(C::NAME, &Shape::new(1024, 1, RECORDS, mode), size);
}

criterion_group!(benches, batch);
criterion_main!(benches);
