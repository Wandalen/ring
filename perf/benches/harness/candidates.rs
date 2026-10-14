//! The queues under test, each behind the same two ends. An adapter uses its crate's fastest
//! idiomatic API and names it in its doc comment.

use std::collections::VecDeque;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
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

/// `N` words, every one the key: a payload the queue has to copy whole, and whose check reads every word.
///
/// The check used to compare the first and last word only. At a 264-byte stride that never touched the middle
/// 128-byte line of a 256-byte record, so the consumer pulled one line fewer per record than at a 256-byte stride,
/// and the payload bench measured the checker's footprint instead of the queue's.
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
    self.0.iter().all(|word| *word == self.0[0])
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

/// A candidate of `capacity` slots, on the heap at a chosen byte of a 256-byte block.
///
/// A ring holds its cursors inline, so built on the stack its layout against the cache lines moved
/// with the bench thread's stack: the same commit measured 25 or 72 M/s from one process to the
/// next, and later 24 or 200, depending on nothing but the length of the environment. Placed here,
/// every process sees the same layout, and `RING_OFFSET` chooses which one.
///
/// Two layouts exist on a 128-byte-line host for a ring whose cursors sit 64 bytes apart (`ring_align`
/// pads to 64): at byte 0 of a line both cursors share it, at byte 64 they sit on two. The suite
/// defaults to byte 0. Set `RING_OFFSET=64` to measure the other layout; anything else is byte 0.
/// `rtrb` and `ArrayQueue` pad their own indices to 128 and are indifferent to the choice.
pub fn build<C: Candidate>(capacity: usize) -> Placed<C> {
  build_at(placement_from_env(), capacity)
}

/// Where [`build`] puts a candidate inside its 256-byte block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
  /// Byte 0: on a 128-byte-line host the two cursors of a ring share one line.
  LineStart,
  /// Byte 64: the two cursors sit on two lines.
  LineMiddle,
}

/// `RING_OFFSET=64` selects [`Placement::LineMiddle`]; anything else, including unset, is
/// [`Placement::LineStart`].
pub fn placement_from_env() -> Placement {
  match std::env::var("RING_OFFSET").as_deref() {
    Ok("64") => Placement::LineMiddle,
    _ => Placement::LineStart,
  }
}

/// [`build`] with the placement chosen by the caller, for a test or a sweep over both layouts.
pub fn build_at<C: Candidate>(placement: Placement, capacity: usize) -> Placed<C> {
  match placement {
    Placement::LineStart => Placed::LineStart(Box::new(At {
      _pad: [],
      candidate: C::new(capacity),
    })),
    Placement::LineMiddle => Placed::LineMiddle(Box::new(At {
      _pad: [0; 64],
      candidate: C::new(capacity),
    })),
  }
}

/// What [`build`] returns: the candidate at byte 0 or byte 64 of a 256-byte-aligned block, with cold
/// padding on both sides so nothing else shares its lines.
#[derive(Debug)]
pub enum Placed<C> {
  /// See [`Placement::LineStart`].
  LineStart(Box<At<0, C>>),
  /// See [`Placement::LineMiddle`].
  LineMiddle(Box<At<64, C>>),
}

/// `PAD` bytes, then the candidate, in a 256-byte-aligned block.
#[derive(Debug)]
#[repr(C, align(256))]
pub struct At<const PAD: usize, C> {
  _pad: [u8; PAD],
  candidate: C,
}

impl<C> Deref for Placed<C> {
  type Target = C;

  fn deref(&self) -> &C {
    match self {
      Self::LineStart(at) => &at.candidate,
      Self::LineMiddle(at) => &at.candidate,
    }
  }
}

impl<C> DerefMut for Placed<C> {
  fn deref_mut(&mut self) -> &mut C {
    match self {
      Self::LineStart(at) => &mut at.candidate,
      Self::LineMiddle(at) => &mut at.candidate,
    }
  }
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

/// `crossbeam-channel` bounded — `try_send`; `try_recv`. What users reach for
/// when they need MPMC rather than our MPSC: the same algorithm family as
/// `sync_channel` above plus the multi-consumer and selection machinery.
#[derive(Debug)]
pub struct CrossbeamChannel<R = u64>(Option<(crossbeam_channel::Sender<R>, crossbeam_channel::Receiver<R>)>);

impl<R: Record> Candidate for CrossbeamChannel<R> {
  type Record = R;

  const NAME: &'static str = "crossbeam_channel";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Some(crossbeam_channel::bounded(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let (tx, rx) = self.0.take().expect("split once");
    // `Sender: Clone`, one handle per producer thread.
    let producers = (0..producers).map(|_| ChannelTx(tx.clone())).collect();
    run.run(producers, ChannelRx(rx))
  }
}

#[derive(Clone)]
struct ChannelTx<R>(crossbeam_channel::Sender<R>);

impl<R: Record> Tx<R> for ChannelTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_send(record).is_ok()
  }
}

struct ChannelRx<R>(crossbeam_channel::Receiver<R>);

impl<R: Record> Rx<R> for ChannelRx<R> {
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

/// `flume` bounded — `try_send`; `try_recv`. An MPMC channel with its own
/// adaptive backoff, no allocators beyond the queue itself at these features.
#[derive(Debug)]
pub struct Flume<R = u64>(Option<(flume::Sender<R>, flume::Receiver<R>)>);

impl<R: Record> Candidate for Flume<R> {
  type Record = R;

  const NAME: &'static str = "flume";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Some(flume::bounded(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let (tx, rx) = self.0.take().expect("split once");
    let producers = (0..producers).map(|_| FlumeTx(tx.clone())).collect();
    run.run(producers, FlumeRx(rx))
  }
}

#[derive(Clone)]
struct FlumeTx<R>(flume::Sender<R>);

impl<R: Record> Tx<R> for FlumeTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_send(record).is_ok()
  }
}

struct FlumeRx<R>(flume::Receiver<R>);

impl<R: Record> Rx<R> for FlumeRx<R> {
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

/// `ringbuf::HeapRb` — `try_push`; `try_pop`. A heap SPSC ring with a runtime
/// capacity, the closest off-the-shelf shape to ours.
#[derive(Debug)]
pub struct RingBuf<R = u64> {
  capacity: usize,
  record: PhantomData<R>,
}

impl<R: Record> Candidate for RingBuf<R> {
  type Record = R;

  const NAME: &'static str = "ringbuf";
  const PUSH_BATCH: bool = true;
  const MAX_PRODUCERS: usize = 1;

  fn new(capacity: usize) -> Self {
    Self {
      capacity,
      record: PhantomData,
    }
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    use ringbuf::traits::Split;
    assert_eq!(producers, 1, "ringbuf takes one producer");
    let (tx, rx) = ringbuf::HeapRb::new(self.capacity).split();

    run.run(vec![RingBufTx(tx)], RingBufRx(rx))
  }
}

struct RingBufTx<R>(<ringbuf::HeapRb<R> as ringbuf::traits::Split>::Prod);

impl<R: Record> Tx<R> for RingBufTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    use ringbuf::traits::Producer;
    self.0.try_push(record).is_ok()
  }

  fn push_batch(&mut self, records: &[R]) -> usize {
    use ringbuf::traits::Producer;
    self.0.push_slice(records)
  }
}

struct RingBufRx<R>(<ringbuf::HeapRb<R> as ringbuf::traits::Split>::Cons);

impl<R: Record> Rx<R> for RingBufRx<R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    use ringbuf::traits::Consumer;
    self.0.try_pop().map(sink).is_some()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    use ringbuf::traits::Consumer;
    let mut n = 0;
    while n < max {
      match self.0.try_pop() {
        Some(record) => sink(record),
        None => return n,
      }
      n += 1;
    }

    n
  }
}

/// `heapless::spsc::Queue` — `enqueue`; `dequeue`. Statically allocated, no
/// allocator at all: the capacity is a const generic, so each size is its own
/// candidate and `new` asserts the runtime capacity matches it.
/// Usable capacity is `N - 1` (one sentinel slot), so the aliases size one
/// up from the row capacity. Fixed-size aliases: the `every_candidate!`
/// macro in micro.rs only accepts plain identifiers, not generic paths,
/// so the sizes used there go by name.
pub type Heapless64 = Heapless<u64, 65>;
/// Fixed 1024 usable slots for the suites that run one size: `spsc` payload
/// and pinned rows, micro rows, latency rows.
pub type Heapless1024 = Heapless<u64, 1025>;

#[derive(Debug)]
/// Statically allocated SPSC queue behind a const-generic capacity.
pub struct Heapless<R = u64, const N: usize = 1024> {
  queue: heapless::spsc::Queue<R, N>,
}

impl<R: Record, const N: usize> Candidate for Heapless<R, N> {
  type Record = R;

  const NAME: &'static str = "heapless";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = 1;

  fn new(capacity: usize) -> Self {
    assert_eq!(capacity + 1, N, "heapless holds N - 1: size one up from the row capacity");
    Self {
      queue: heapless::spsc::Queue::new(),
    }
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    assert_eq!(producers, 1, "heapless takes one producer");
    // Borrowed ends, not owned: heapless hands out handles into the queue,
    // so they borrow the candidate for the run's duration (which outlives it).
    let (tx, rx) = self.queue.split();

    run.run(vec![HeaplessTx(tx)], HeaplessRx(rx))
  }
}

struct HeaplessTx<'a, R>(heapless::spsc::Producer<'a, R>);

impl<R: Record> Tx<R> for HeaplessTx<'_, R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.enqueue(record).is_ok()
  }
}

struct HeaplessRx<'a, R>(heapless::spsc::Consumer<'a, R>);

impl<R: Record> Rx<R> for HeaplessRx<'_, R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.0.dequeue().map(sink).is_some()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let mut n = 0;
    while n < max {
      match self.0.dequeue() {
        Some(record) => sink(record),
        None => return n,
      }
      n += 1;
    }

    n
  }
}

/// `concurrent_queue::ConcurrentQueue` bounded — `push`; `pop`. MPMC with a
/// single shared handle: one `Arc` serves every producer thread.
#[derive(Debug)]
pub struct ConcurrentQueue<R = u64>(Arc<concurrent_queue::ConcurrentQueue<R>>);

impl<R: Record> Candidate for ConcurrentQueue<R> {
  type Record = R;

  const NAME: &'static str = "concurrent_queue";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Arc::new(concurrent_queue::ConcurrentQueue::bounded(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    // One handle, shared by every producer thread.
    let producers = (0..producers).map(|_| QueueTx(self.0.clone())).collect();
    run.run(producers, QueueRx(self.0.clone()))
  }
}

#[derive(Clone)]
struct QueueTx<R>(Arc<concurrent_queue::ConcurrentQueue<R>>);

impl<R: Record> Tx<R> for QueueTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }
}

struct QueueRx<R>(Arc<concurrent_queue::ConcurrentQueue<R>>);

impl<R: Record> Rx<R> for QueueRx<R> {
  fn try_pop(&mut self, sink: &mut impl FnMut(R)) -> bool {
    self.0.pop().map(sink).is_ok()
  }

  fn pop_batch(&mut self, max: usize, sink: &mut impl FnMut(R)) -> usize {
    let mut n = 0;
    while n < max {
      match self.0.pop() {
        Ok(record) => sink(record),
        Err(_) => return n,
      }
      n += 1;
    }

    n
  }
}

/// `thingbuf` MPSC channel — `try_send`; `try_recv`. Static-capable MPSC with
/// recycled storage; records must be `Default + Sync` for the channel
/// constructor, which all harness records are.
#[derive(Debug)]
pub struct ThingBuf<R = u64>(Option<(thingbuf::mpsc::blocking::Sender<R>, thingbuf::mpsc::blocking::Receiver<R>)>);

impl<R: Record + Default + Sync> Candidate for ThingBuf<R> {
  type Record = R;

  const NAME: &'static str = "thingbuf";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self(Some(thingbuf::mpsc::blocking::channel(capacity)))
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    let (tx, rx) = self.0.take().expect("split once");
    let producers = (0..producers).map(|_| ThingBufTx(tx.clone())).collect();
    run.run(producers, ThingBufRx(rx))
  }
}

#[derive(Clone)]
struct ThingBufTx<R>(thingbuf::mpsc::blocking::Sender<R>);

impl<R: Record + Default + Sync> Tx<R> for ThingBufTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.try_send(record).is_ok()
  }
}

struct ThingBufRx<R>(thingbuf::mpsc::blocking::Receiver<R>);

impl<R: Record + Default + Sync> Rx<R> for ThingBufRx<R> {
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

/// `quetzalcoatl` SPSC ring — `push`; `pop`. Arc-backed with owned ends.
#[derive(Debug)]
pub struct QuetzalSpsc<R = u64> {
  capacity: usize,
  record: PhantomData<R>,
}

impl<R: Record> Candidate for QuetzalSpsc<R> {
  type Record = R;

  const NAME: &'static str = "quetzal_spsc";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = 1;

  fn new(capacity: usize) -> Self {
    Self {
      capacity,
      record: PhantomData,
    }
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    use quetzalcoatl::capacity::Capacity;
    assert_eq!(producers, 1, "quetzal_spsc takes one producer");
    let (tx, rx) = quetzalcoatl::spsc::RingBuffer::new(Capacity::at_least(self.capacity)).split();

    run.run(vec![QuetzalSpscTx(tx)], QuetzalSpscRx(rx))
  }
}

struct QuetzalSpscTx<R>(quetzalcoatl::spsc::Producer<R>);

impl<R: Record> Tx<R> for QuetzalSpscTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }
}

struct QuetzalSpscRx<R>(quetzalcoatl::spsc::Consumer<R>);

impl<R: Record> Rx<R> for QuetzalSpscRx<R> {
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

/// `quetzalcoatl` MPSC ring — `push`; `pop`. Cloneable producer handles.
#[derive(Debug)]
pub struct QuetzalMpsc<R = u64> {
  capacity: usize,
  record: PhantomData<R>,
}

impl<R: Record> Candidate for QuetzalMpsc<R> {
  type Record = R;

  const NAME: &'static str = "quetzal_mpsc";
  const PUSH_BATCH: bool = false;
  const MAX_PRODUCERS: usize = usize::MAX;

  fn new(capacity: usize) -> Self {
    Self {
      capacity,
      record: PhantomData,
    }
  }

  fn split<V: Run<R>>(&mut self, producers: usize, run: V) -> V::Output {
    use quetzalcoatl::capacity::Capacity;
    let (tx, rx) = quetzalcoatl::mpsc::RingBuffer::new(Capacity::at_least(self.capacity)).split();
    let producers = (0..producers).map(|_| QuetzalMpscTx(tx.clone())).collect();
    run.run(producers, QuetzalMpscRx(rx))
  }
}

#[derive(Clone)]
struct QuetzalMpscTx<R>(quetzalcoatl::mpsc::Producer<R>);

impl<R: Record> Tx<R> for QuetzalMpscTx<R> {
  fn try_push(&mut self, record: R) -> bool {
    self.0.push(record).is_ok()
  }
}

struct QuetzalMpscRx<R>(quetzalcoatl::mpsc::Consumer<R>);

impl<R: Record> Rx<R> for QuetzalMpscRx<R> {
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
