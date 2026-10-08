//! The queues under test, each behind the same two ends. An adapter uses its crate's fastest
//! idiomatic API and names it in its doc comment.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, MutexGuard};

use ring_slot::{CopySlot, TypedSlot};
use ring_types::Capacity;

/// A record's key is its producer's id above these bits and the producer's sequence number in them.
pub const SEQ_BITS: u32 = 48;

/// What travels through a queue: built from a key, giving it back on arrival.
pub trait Record: Copy + Send + 'static {
  /// A record carrying `key`.
  fn new(key: u64) -> Self;

  /// The key it carries.
  fn key(&self) -> u64;

  /// Whether it arrived whole rather than torn.
  fn whole(&self) -> bool {
    true
  }
}

impl Record for u64 {
  fn new(key: u64) -> Self {
    key
  }

  fn key(&self) -> u64 {
    *self
  }
}

/// `N` words with the key in the first and the last: a payload the queue has to copy whole.
#[derive(Clone, Copy, Debug)]
pub struct Wide<const N: usize>(pub [u64; N]);

// By hand: std's `Default` for arrays stops at 32 elements, so a derive could not cover every `N`.
// `CopySlot` needs it to fill a ring before the first write.
impl<const N: usize> Default for Wide<N> {
  fn default() -> Self {
    Self([0; N])
  }
}

impl<const N: usize> Record for Wide<N> {
  fn new(key: u64) -> Self {
    Self([key; N])
  }

  fn key(&self) -> u64 {
    self.0[0]
  }

  fn whole(&self) -> bool {
    self.0[0] == self.0[N - 1]
  }
}

/// A sending end.
pub trait Tx<R> {
  /// Push one record; `false` when the queue is full, and nothing was taken.
  fn try_push(&mut self, record: R) -> bool;

  /// Push a prefix of `records` in one operation; how many went in. Called only when
  /// [`Candidate::PUSH_BATCH`] is set.
  fn push_batch(&mut self, _records: &[R]) -> usize {
    unreachable!("this candidate has no batch push")
  }
}

/// The receiving end.
pub trait Rx<R> {
  /// Pop one record into `sink`; `false` when the queue is empty.
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool;

  /// Pop what is available, at most `max`, into `sink`; how many records.
  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize;
}

/// A queue under test, as the drivers see it: built per run, split into ends.
pub trait Candidate {
  /// What it carries.
  type Record: Record;
  /// The row label.
  const NAME: &'static str;
  /// Whether the crate has a batch push of its own; without one there is no batch row.
  const PUSH_BATCH: bool;
  /// How many producers the queue takes.
  const MAX_PRODUCERS: usize;

  /// A queue of `capacity` slots, a power of two.
  fn new(capacity: usize) -> Self;

  /// Split into `producers` sending ends and the receiving end, and hand them to `run`.
  fn split<V: Run<Self::Record>>(&mut self, producers: usize, run: V) -> V::Output;
}

/// What is done with a split queue. A trait rather than a closure because the ends borrow the
/// queue and their types differ per candidate.
pub trait Run<R> {
  /// What the run returns.
  type Output;

  /// Use the ends.
  fn run<T: Tx<R> + Send, X: Rx<R> + Send>(self, producers: Vec<T>, consumer: X) -> Self::Output;
}

fn capacity(slots: usize) -> Capacity {
  Capacity::new(slots).expect("a power-of-two capacity")
}

/// `ring_spsc` direct: `try_push`; `drain_up_to`, read through `TypedSlot::get`.
#[derive(Debug)]
pub struct Spsc<R = u64>(ring_spsc::Ring<TypedSlot<R>>);

impl<R: Record> Candidate for Spsc<R> {
  type Record = R;

  const NAME: &'static str = "spsc";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = 1;

  fn new(slots: usize) -> Self {
    Self(ring_spsc::Ring::new(capacity(slots)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    assert_eq!(producers, 1, "spsc takes one producer");
    let (tx, rx) = self.0.split();

    run.run(vec![SpscTx(tx)], SpscRx(rx))
  }
}

struct SpscTx<'a, R>(ring_spsc::Producer<'a, TypedSlot<R>>);

impl<R: Record> Tx<R> for SpscTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_push(record).is_ok()
  }
}

struct SpscRx<'a, R>(ring_spsc::Consumer<'a, TypedSlot<R>>);

impl<R: Record> Rx<R> for SpscRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.pop_batch(1, sink) == 1
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let batch = self.0.drain_up_to(max);
    // An empty slot — a dropped `ReservedBatch` publishes one per unwritten offset — is not a
    // record: the count is what `sink` saw, or the checker and the driver drift apart.
    batch.iter().filter_map(TypedSlot::get).map(|&record| sink(record)).count()
  }
}

/// `ring_mpsc` direct: `push`, and `claim_batch`-backed `push_batch` for the batch modes;
/// `drain_up_to`, read through `TypedSlot::get`.
///
/// One split per ring. A second `Ring::ends` used to start a fresh claim cursor at zero under a
/// consumer cursor that had moved on, and what it pushed was never delivered — the cursor moved
/// into the ring (`ring_mpsc/docs/pitfall/003`), and the second generation continues instead.
#[derive(Debug)]
pub struct Mpsc<R = u64>(ring_mpsc::Ring<TypedSlot<R>>);

impl<R: Record> Candidate for Mpsc<R> {
  type Record = R;

  const NAME: &'static str = "mpsc";
  const PUSH_BATCH: bool = true;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(slots: usize) -> Self {
    Self(ring_mpsc::Ring::new(capacity(slots)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let mut ends = self.0.ends();
    let (tx, rx) = ends.split();

    run.run(vec![MpscTx(tx); producers], MpscRx(rx))
  }
}

#[derive(Clone)]
struct MpscTx<'a, R>(ring_mpsc::Producer<'a, TypedSlot<R>>);

impl<R: Record> Tx<R> for MpscTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }

  fn push_batch(&mut self, records: &[R]) -> usize {
    let Ok(mut guard) = self.0.claim_batch(records.len()) else {
      return 0;
    };
    let granted = guard.len();
    for (offset, &record) in records.iter().enumerate().take(granted) {
      guard.slot_mut(offset).expect("within the grant").set(record);
    }
    drop(guard);

    granted
  }
}

struct MpscRx<'a, R>(ring_mpsc::Consumer<'a, TypedSlot<R>>);

impl<R: Record> Rx<R> for MpscRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.pop_batch(1, sink) == 1
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let batch = self.0.drain_up_to(max);
    // An empty slot — a dropped `ReservedBatch` publishes one per unwritten offset — is not a
    // record: the count is what `sink` saw, or the checker and the driver drift apart.
    batch.iter().filter_map(TypedSlot::get).map(|&record| sink(record)).count()
  }
}

/// `ring_mpsc` through the exclusive primary handle: the spike's cached-cursor claim fast path
/// (`ring_mpsc/docs/decisions/003`), one producer by construction. The receiving end is the
/// ordinary `MpscRx` — the primary changes the producer's claim path only.
#[derive(Debug)]
pub struct MpscPrimary<R = u64>(ring_mpsc::Ring<TypedSlot<R>>);

impl<R: Record> Candidate for MpscPrimary<R> {
  type Record = R;

  const NAME: &'static str = "mpsc-primary";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = 1;

  fn new(slots: usize) -> Self {
    Self(ring_mpsc::Ring::new(capacity(slots)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    assert_eq!(producers, 1, "the primary handle takes one producer");
    let mut ends = self.0.ends();
    let (mut tx, rx) = ends.split();
    let primary = tx.primary();

    run.run(vec![MpscPrimaryTx(primary)], MpscRx(rx))
  }
}

struct MpscPrimaryTx<'a, R>(ring_mpsc::PrimaryProducer<'a, TypedSlot<R>>);

impl<R: Record> Tx<R> for MpscPrimaryTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_push(record).is_ok()
  }
}

/// `ring_spsc` over `CopySlot`, the record stored without `TypedSlot`'s tag: `try_push`;
/// `drain_up_to`, read through `CopySlot::get`.
#[derive(Debug)]
pub struct SpscPlain<R = u64>(ring_spsc::Ring<CopySlot<R>>);

impl<R: Record + Default> Candidate for SpscPlain<R> {
  type Record = R;

  const NAME: &'static str = "spsc-plain";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = 1;

  fn new(slots: usize) -> Self {
    Self(ring_spsc::Ring::new(capacity(slots)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    assert_eq!(producers, 1, "spsc takes one producer");
    let (tx, rx) = self.0.split();

    run.run(vec![SpscPlainTx(tx)], SpscPlainRx(rx))
  }
}

struct SpscPlainTx<'a, R>(ring_spsc::Producer<'a, CopySlot<R>>);

impl<R: Record> Tx<R> for SpscPlainTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_push(record).is_ok()
  }
}

struct SpscPlainRx<'a, R>(ring_spsc::Consumer<'a, CopySlot<R>>);

impl<R: Record> Rx<R> for SpscPlainRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.pop_batch(1, sink) == 1
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    self.0.drain_up_to(max).iter().map(|slot| sink(slot.get())).count()
  }
}

/// `ring_mpsc` over `CopySlot`: `claim` and `CopySlot::set`, `claim_batch` for the batch modes;
/// `drain_up_to`, read through `CopySlot::get`.
#[derive(Debug)]
pub struct MpscPlain<R = u64>(ring_mpsc::Ring<CopySlot<R>>);

impl<R: Record + Default> Candidate for MpscPlain<R> {
  type Record = R;

  const NAME: &'static str = "mpsc-plain";
  const PUSH_BATCH: bool = true;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(slots: usize) -> Self {
    Self(ring_mpsc::Ring::new(capacity(slots)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let mut ends = self.0.ends();
    let (tx, rx) = ends.split();

    run.run(vec![MpscPlainTx(tx); producers], MpscPlainRx(rx))
  }
}

#[derive(Clone)]
struct MpscPlainTx<'a, R>(ring_mpsc::Producer<'a, CopySlot<R>>);

impl<R: Record> Tx<R> for MpscPlainTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    let Ok(mut slot) = self.0.claim() else {
      return false;
    };
    slot.set(record);

    true
  }

  fn push_batch(&mut self, records: &[R]) -> usize {
    let Ok(mut guard) = self.0.claim_batch(records.len()) else {
      return 0;
    };
    let granted = guard.len();
    for (offset, &record) in records.iter().enumerate().take(granted) {
      guard.slot_mut(offset).expect("within the grant").set(record);
    }
    drop(guard);

    granted
  }
}

struct MpscPlainRx<'a, R>(ring_mpsc::Consumer<'a, CopySlot<R>>);

impl<R: Record> Rx<R> for MpscPlainRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.pop_batch(1, sink) == 1
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    self.0.drain_up_to(max).iter().map(|slot| sink(slot.get())).count()
  }
}

/// `std::sync::Mutex< VecDeque< R > >` bounded at the capacity: one lock per push, per pop, or per
/// batch.
#[derive(Debug)]
pub struct MutexDeque<R = u64> {
  queue: Mutex<VecDeque<R>>,
  capacity: usize,
}

impl<R: Record> Candidate for MutexDeque<R> {
  type Record = R;

  const NAME: &'static str = "mutex";
  const PUSH_BATCH: bool = true;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self {
      queue: Mutex::new(VecDeque::with_capacity(capacity)),
      capacity,
    }
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let tx = MutexTx {
      queue: &self.queue,
      capacity: self.capacity,
    };
    let rx = MutexRx {
      queue: &self.queue,
      scratch: Vec::with_capacity(self.capacity),
    };

    run.run(vec![tx; producers], rx)
  }
}

fn lock<R>(queue: &Mutex<VecDeque<R>>) -> MutexGuard<'_, VecDeque<R>> {
  queue.lock().expect("a thread panicked holding the queue lock")
}

#[derive(Clone)]
struct MutexTx<'a, R> {
  queue: &'a Mutex<VecDeque<R>>,
  capacity: usize,
}

impl<R: Record> Tx<R> for MutexTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    let mut queue = lock(self.queue);
    let room = queue.len() < self.capacity;
    if room {
      queue.push_back(record);
    }

    room
  }

  fn push_batch(&mut self, records: &[R]) -> usize {
    let mut queue = lock(self.queue);
    let n = records.len().min(self.capacity - queue.len());
    queue.extend(&records[..n]);

    n
  }
}

struct MutexRx<'a, R> {
  queue: &'a Mutex<VecDeque<R>>,
  /// A throwaway reuse buffer: records land here under the lock and leave it at the next call.
  /// Nothing reads its contents between calls.
  scratch: Vec<R>,
}

impl<R: Record> Rx<R> for MutexRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    let record = lock(self.queue).pop_front();
    record.map(sink).is_some()
  }

  // Copied out under the lock into `scratch` and handed to `sink` after it: the lock guards the
  // queue, not the consumer's work.
  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    {
      let mut queue = lock(self.queue);
      let n = max.min(queue.len());
      self.scratch.extend(queue.drain(..n));
    }
    let n = self.scratch.len();
    self.scratch.drain(..).for_each(sink);

    n
  }
}

/// `rtrb` 0.4, the reference SPSC ring: `push`, `push_partial_slice`; `pop`, `read_chunk`.
#[derive(Debug)]
pub struct Rtrb<R = u64> {
  capacity: usize,
  record: PhantomData<R>,
}

impl<R: Record> Candidate for Rtrb<R> {
  type Record = R;

  const NAME: &'static str = "rtrb";
  const PUSH_BATCH: bool = true;
  const MAX_PRODUCERS: usize = 1;

  fn new(capacity: usize) -> Self {
    Self {
      capacity,
      record: PhantomData,
    }
  }

  // `rtrb` allocates when it splits, so the ring is built here — still outside the clock.
  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    assert_eq!(producers, 1, "rtrb takes one producer");
    let (tx, rx) = rtrb::RingBuffer::new(self.capacity);

    run.run(vec![RtrbTx(tx)], RtrbRx(rx))
  }
}

struct RtrbTx<R>(rtrb::Producer<R>);

impl<R: Record> Tx<R> for RtrbTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }

  fn push_batch(&mut self, records: &[R]) -> usize {
    self.0.push_partial_slice(records).0.len()
  }
}

struct RtrbRx<R>(rtrb::Consumer<R>);

impl<R: Record> Rx<R> for RtrbRx<R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.0.pop().map(sink).is_ok()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let n = self.0.slots().min(max);
    if n == 0 {
      return 0;
    }
    let chunk = self.0.read_chunk(n).expect("`slots` counted them");
    let (first, second) = chunk.as_slices();
    first.iter().chain(second).for_each(|&record| sink(record));
    chunk.commit_all();

    n
  }
}

/// `std::sync::mpsc::sync_channel` — `try_send`; `try_recv`. The standard library's bounded
/// channel, the crossbeam-channel algorithm since 1.67, and the first thing a user reaches for.
#[derive(Debug)]
pub struct SyncChannel<R = u64>(Option<(SyncSender<R>, Receiver<R>)>);

impl<R: Record> Candidate for SyncChannel<R> {
  type Record = R;

  const NAME: &'static str = "sync_channel";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Some(std::sync::mpsc::sync_channel(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let (tx, rx) = self.0.take().expect("split once");

    run.run(vec![SyncTx(tx); producers], SyncRx(rx))
  }
}

#[derive(Clone)]
struct SyncTx<R>(SyncSender<R>);

impl<R: Record> Tx<R> for SyncTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_send(record).is_ok()
  }
}

struct SyncRx<R>(Receiver<R>);

impl<R: Record> Rx<R> for SyncRx<R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.0.try_recv().map(sink).is_ok()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let mut n = 0;
    while n < max {
      match self.0.try_recv() {
        Ok(record) => sink(record),
        Err(_) => return n,
      }
      n += 1;
    }

    n
  }
}

/// `crossbeam_queue::ArrayQueue` — `push`; `pop`. A bounded MPMC array queue with a stamp per
/// slot: the layout P6 proposes, and `ring_core`'s interim `crossbeam` backend. Already a
/// workspace dependency.
#[derive(Debug)]
pub struct CrossbeamQueue<R = u64>(Arc<crossbeam_queue::ArrayQueue<R>>);

impl<R: Record> Candidate for CrossbeamQueue<R> {
  type Record = R;

  const NAME: &'static str = "arrayqueue";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Arc::new(crossbeam_queue::ArrayQueue::new(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    run.run(vec![CrossbeamTx(self.0.clone()); producers], CrossbeamRx(self.0.clone()))
  }
}

#[derive(Clone)]
struct CrossbeamTx<R>(Arc<crossbeam_queue::ArrayQueue<R>>);

impl<R: Record> Tx<R> for CrossbeamTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }
}

struct CrossbeamRx<R>(Arc<crossbeam_queue::ArrayQueue<R>>);

impl<R: Record> Rx<R> for CrossbeamRx<R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.0.pop().map(sink).is_some()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let mut n = 0;
    while n < max {
      match self.0.pop() {
        Some(record) => sink(record),
        None => return n,
      }
      n += 1;
    }

    n
  }
}
