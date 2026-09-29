//! Optional sequence-operation trace log.
//!
//! Tier 2 of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_types`.
//!
//! Claims `docs/feature/185_ring_stats.md`. Its acceptance criterion, filed at
//! `ring/bench_harness/docs/acceptance/001_feature_reached_tests.md`, is a
//! pair of numbers: `ring_trace` "records one entry per sequence operation
//! when enabled and **zero when not**." Both halves are assertions, and the
//! second is the harder one — a trace that costs something when disabled is a
//! trace nobody leaves compiled in, and the family's whole output is a measured
//! comparison that an always-on trace would distort.
//!
//! ## Trace against stats
//!
//! `ring_stats` counts; this records. A counter answers "how many publishes"
//! in constant space and tells you nothing about which sequences or in what
//! order; a trace answers "which operations, in what order" and grows without
//! bound — "order" here is the order producers reached the log's lock, which
//! is operation order for one producer and an interleaving for more, not
//! reconstructed sequence order under contention (→ `docs/invariant/002`).
//! Unbounded is not abstract: one producer recording as fast as it can has
//! been measured to grow the log at roughly 667 MiB/s of live entries — this
//! machine's figure, its order of magnitude rather than its exact number is
//! what should be trusted (→ `docs/lifecycle/001`).
//! They are not two implementations of one thing — `ring_stats` is
//! always on and cheap, `ring_trace` is off by default and expensive, and a
//! diagnosis usually starts at the counter and only then reaches for the log.
//! "Expensive" measured: roughly 7× the disabled cost per call uncontended,
//! and 30–40× under four producers sharing one trace — this machine's ratio,
//! not its nanoseconds, is what should be trusted (→ `docs/decisions/002`).
//!
//! ## Why a `Mutex` is the right cost here
//!
//! A shared trace across producers needs some form of exclusion, and a lock is
//! the honest one. The alternative — a lock-free log — would make the disabled
//! path no cheaper (it is already a branch on a `bool`) while making the
//! enabled path a second concurrency problem inside the crate that exists to
//! debug the first. The lock is affordable precisely because the feature is off
//! whenever the measurement matters.
//!
//! Every access recovers from poisoning rather than propagating it, via
//! `Trace::entries_guard`. A `Vec<TraceEntry>` has no invariant a panic could
//! leave half-established — a push either landed or it did not — so there is
//! nothing for poisoning to protect. The two alternatives are both worse and
//! both were briefly in this file: panicking on `record` would let a diagnostic
//! kill the producer thread it was added to observe, and silently reporting
//! zero on the read side would make "nothing happened" and "the log broke"
//! indistinguishable, which is exactly the misreading a diagnostic must not
//! invite.

#![ deny( missing_docs ) ]

use core::fmt;
use std::sync::Mutex;
use ring_types::Seq;

/// The kind of sequence operation an entry records.
///
/// Deliberately about *sequences*, not payloads: a trace of what the cursors
/// did is what diagnoses a stall or a lap, and a trace of what the payloads
/// were is a different tool with different costs.
///
/// ```
/// use ring_trace::TraceOp;
/// assert_eq!( TraceOp::ALL.len(), 5 );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
pub enum TraceOp
{
  /// A producer took ownership of one or more sequences.
  Claim,
  /// A producer made a claimed sequence visible to consumers.
  Publish,
  /// A consumer read a published sequence.
  Consume,
  /// A consumer's cursor advanced, freeing slots for reuse.
  Commit,
  /// A publish was refused and the overflow policy ran.
  Drop,
}

impl TraceOp
{
  /// Every discriminant, for a test that must cover all of them.
  pub const ALL : [ Self; 5 ] =
  [
    Self::Claim,
    Self::Publish,
    Self::Consume,
    Self::Commit,
    Self::Drop,
  ];

  /// This operation's name.
  ///
  /// Written as an exhaustive `match` rather than a derive so that adding a
  /// discriminant fails to compile here, where a human then has to say what the
  /// new operation is called.
  ///
  /// ```
  /// use ring_trace::TraceOp;
  /// assert_eq!( TraceOp::Claim.name(), "claim" );
  /// ```
  #[ must_use ]
  pub const fn name( self ) -> &'static str
  {
    match self
    {
      Self::Claim => "claim",
      Self::Publish => "publish",
      Self::Consume => "consume",
      Self::Commit => "commit",
      Self::Drop => "drop",
    }
  }
}

impl fmt::Display for TraceOp
{
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
  {
    f.write_str( self.name() )
  }
}

/// One recorded operation.
///
/// `count` is what makes the log readable against a batch: a claim of 64 is one
/// entry saying 64, not 64 entries, because feature 177's whole point is that
/// it *was* one operation. A trace that expanded it would contradict the thing
/// it is meant to be evidence of.
///
/// `count` is a `usize`, not a narrower `u32`, though every count is bounded by
/// a ring's [`Capacity`](ring_types::Capacity) — a `usize` newtype with no
/// declared ceiling. Narrowing here would mean asserting a bound the type it is
/// checked against does not itself assert, so the twenty-nine per cent of this
/// struct that is alignment padding (measured: 24 bytes for 17 of fields) is
/// accepted deliberately, not an oversight.
///
/// ```
/// use ring_trace::{ TraceEntry, TraceOp };
/// use ring_types::Seq;
///
/// let entry = TraceEntry { op : TraceOp::Claim, seq : Seq( 8 ), count : 64 };
/// assert_eq!( entry.end(), Seq( 72 ) );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct TraceEntry
{
  /// What happened.
  pub op : TraceOp,
  /// The first sequence involved.
  pub seq : Seq,
  /// How many consecutive sequences the operation covered.
  pub count : usize,
}

impl TraceEntry
{
  /// One past the last sequence this entry covers.
  ///
  /// Saturates rather than wrapping: a bare `+` here would print a range that
  /// reads backwards, or panic under debug assertions, the moment a caller
  /// traces the one `Seq` the family publishes by name —
  /// `ring_mpsc::UNSTAMPED` (`Seq(u64::MAX)`). See `pitfall/001` TR41.
  #[ must_use ]
  pub const fn end( &self ) -> Seq
  {
    Seq( self.seq.0.saturating_add( self.count as u64 ) )
  }
}

impl fmt::Display for TraceEntry
{
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
  {
    write!( f, "{} {}..{}", self.op, self.seq.0, self.end().0 )
  }
}

/// A log of sequence operations, off unless deliberately switched on.
///
/// The enabled flag is fixed at construction and never mutable afterwards. A
/// trace that could be switched on mid-run would produce a log with a silent
/// hole at the front, which reads exactly like a run where nothing happened
/// early — the one misreading a diagnostic tool must not invite.
///
/// ```
/// use ring_trace::{ Trace, TraceOp };
/// use ring_types::Seq;
///
/// let off = Trace::disabled();
/// off.record( TraceOp::Publish, Seq( 0 ), 1 );
/// assert_eq!( off.len(), 0, "disabled records nothing" );
///
/// let on = Trace::enabled();
/// on.record( TraceOp::Publish, Seq( 0 ), 1 );
/// assert_eq!( on.len(), 1 );
/// ```
#[ derive( Debug ) ]
pub struct Trace
{
  enabled : bool,
  entries : Mutex< Vec< TraceEntry > >,
}

impl Trace
{
  /// A trace that records.
  #[ must_use ]
  pub const fn enabled() -> Self
  {
    Self { enabled : true, entries : Mutex::new( Vec::new() ) }
  }

  /// A trace that discards — the default a ring is built with.
  #[ must_use ]
  pub const fn disabled() -> Self
  {
    Self { enabled : false, entries : Mutex::new( Vec::new() ) }
  }

  /// Whether this trace records.
  ///
  /// ```
  /// use ring_trace::Trace;
  /// assert!( Trace::enabled().is_enabled() );
  /// assert!( !Trace::disabled().is_enabled() );
  /// ```
  #[ must_use ]
  pub const fn is_enabled( &self ) -> bool
  {
    self.enabled
  }

  /// Record one operation, or do nothing if disabled.
  ///
  /// Takes `&self`, not `&mut self`, for the same reason `ring_stats` does: the
  /// producers holding this concurrently cannot each have a unique reference.
  ///
  /// ```
  /// use ring_trace::{ Trace, TraceOp };
  /// use ring_types::Seq;
  ///
  /// let trace = Trace::enabled();
  /// trace.record( TraceOp::Claim, Seq( 4 ), 64 );
  /// assert_eq!( trace.entries()[ 0 ].count, 64, "one entry for the whole batch" );
  /// ```
  ///
  /// Carries `#[ inline ]` so the disabled path — a single `bool` read and a
  /// return — can be inlined into cross-crate call sites instead of paying for
  /// an un-inlined call on every producer/consumer step (→ `algorithm/001`).
  #[ inline ]
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  {
    if !self.enabled
    {
      return;
    }
    self.entries_guard().push( TraceEntry { op, seq, count } );
  }

  /// The log, with poisoning recovered rather than propagated.
  ///
  /// The single access point every method below goes through, so no two of
  /// them can disagree about what a poisoned lock means — see the module
  /// documentation for why recovery is the right answer here.
  ///
  /// Safe only because no caller-supplied code ever runs while this guard is
  /// held: every method below creates and drops it inside a few plain
  /// statements, so the poisoning this recovers from cannot actually occur.
  /// A future method that hands out the guard itself, or takes a callback to
  /// invoke under the lock, would change that and needs its own reachability
  /// argument before it can rely on the same recovery. See `pitfall/002` TR43.
  fn entries_guard( &self ) -> std::sync::MutexGuard< '_, Vec< TraceEntry > >
  {
    self.entries.lock().unwrap_or_else( std::sync::PoisonError::into_inner )
  }

  /// How many entries have been recorded.
  ///
  /// Zero forever on a disabled trace — the second half of feature 185's
  /// `ring_trace` clause, readable without draining the log.
  #[ must_use ]
  pub fn len( &self ) -> usize
  {
    self.entries_guard().len()
  }

  /// Whether nothing has been recorded.
  #[ must_use ]
  pub fn is_empty( &self ) -> bool
  {
    self.len() == 0
  }

  /// Every entry, in the order recorded.
  ///
  /// A copy rather than a borrow: handing out a guard would let a caller hold
  /// the lock across arbitrary code, and the lock is on the path producers take.
  ///
  /// ```
  /// use ring_trace::{ Trace, TraceOp };
  /// use ring_types::Seq;
  ///
  /// let trace = Trace::enabled();
  /// trace.record( TraceOp::Claim, Seq( 0 ), 1 );
  /// trace.record( TraceOp::Publish, Seq( 0 ), 1 );
  ///
  /// let ops : Vec< _ > = trace.entries().iter().map( |e| e.op ).collect();
  /// assert_eq!( ops, vec![ TraceOp::Claim, TraceOp::Publish ] );
  /// ```
  #[ must_use ]
  pub fn entries( &self ) -> Vec< TraceEntry >
  {
    self.entries_guard().clone()
  }

  /// How many entries record `op`.
  ///
  /// O(n) in the log's current length, and the scan runs under the same lock
  /// `record` takes — unlike [`len`](Self::len), which is a field read. Do not
  /// poll this in a loop alongside a producer whose timing matters; a single
  /// scanning reader has been measured to cut a producer's throughput by two
  /// orders of magnitude at a 100,000-entry log (→ `docs/algorithm/002`).
  ///
  /// ```
  /// use ring_trace::{ Trace, TraceOp };
  /// use ring_types::Seq;
  ///
  /// let trace = Trace::enabled();
  /// trace.record( TraceOp::Drop, Seq( 1 ), 1 );
  /// assert_eq!( trace.count_of( TraceOp::Drop ), 1 );
  /// assert_eq!( trace.count_of( TraceOp::Claim ), 0 );
  /// ```
  #[ must_use ]
  pub fn count_of( &self, op : TraceOp ) -> usize
  {
    self.entries_guard().iter().filter( |e| e.op == op ).count()
  }

  /// Discard every entry, keeping the enabled state.
  ///
  /// Takes `&mut self`, unlike every other method here: erasing what another
  /// holder recorded is not a read. `record` returns `()`, so a producer
  /// sharing this trace has no signal that its entries are gone — giving
  /// `clear` a receiver only the owner (or someone who can prove exclusivity,
  /// e.g. via `Arc::get_mut`) can obtain forecloses a second holder of `&Trace`
  /// from erasing another's work silently. See `api/002` TR7.
  ///
  /// ```
  /// use ring_trace::{ Trace, TraceOp };
  /// use ring_types::Seq;
  ///
  /// let mut trace = Trace::enabled();
  /// trace.record( TraceOp::Commit, Seq( 0 ), 1 );
  /// trace.clear();
  /// assert!( trace.is_empty() );
  /// assert!( trace.is_enabled(), "clearing is not disabling" );
  /// ```
  pub fn clear( &mut self )
  {
    self.entries_guard().clear();
  }
}

impl Default for Trace
{
  /// Disabled — the state a ring that was never asked to trace must be in.
  fn default() -> Self
  {
    Self::disabled()
  }
}
