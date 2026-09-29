//! Flush policies deciding when thread-local staging reaches the ring.
//!
//! One of the ring family's 33 crates — the concurrency write-path implementation.
//! Depends on `ring_tls`, `ring_core`.
//!
//! One of the five crates on the family's export Contract — and the only one
//! that is a *decision* rather than a thing a consumer holds. `ring_factory`
//! makes things, `ring_handle` and `ring_tls` are things, `ring_types` is
//! vocabulary. Importing this crate acquires no capability; it accepts three
//! obligations.
//!
//! **The crate owns no ring.** It supplies the one thing `ring_tls`
//! deliberately withheld — a trigger — so that publication happens at a moment
//! somebody chose rather than at whatever moment a buffer happens to fill.
//! [`FlushPolicy`] is `Copy` and consulted by value on a path constrained to
//! perform zero atomics and zero allocations.
//!
//! **Three obligations fall on the caller and none is enforceable here:** it
//! must drive (nothing self-fires, and there is no `Drop` impl); it must
//! announce barriers truthfully if it chose [`FlushPolicy::OnBarrier`]; it must
//! retry a rejected final drain. The third loses data when neglected, because
//! Rust has no linear types and a [`Flusher`] can always be dropped with
//! records staged.
//!
//! **`OnBarrier` cannot observe a barrier**, and adding a dependency edge would
//! not change that. What is missing is the barrier *instance* the consumer is
//! gated on and a *notification* when it advances, and no arrangement of
//! `[dependencies]` lines supplies either.
//!
//! # What the implementation settled
//!
//! Three questions the pre-implementation instances left open were answered by
//! building, and the answers are recorded where the questions were asked.
//!
//! | Question | Settled as | Recorded in |
//! |---|---|---|
//! | The flush log's compilation boundary | An opt-in [`FlushLog`] the [`Flusher`] owns — no cargo feature, no `cfg` | `docs/data_structure/002_the_flush_log.md` |
//! | How a record reaches the buffer at all | [`Flusher::append`], absent from both API instances | `docs/api/001_the_policy_surface.md` |
//! | `ConfigError::AlreadyBound` | Unreachable — `new` takes the buffer by value, so ownership enforces N3 | `docs/type/001_flush_policy.md` |
//!
//! # The seam this crate is built on
//!
//! `ring_tls`'s pre-implementation surface specified `seal`/`drain`/`reset` as
//! three calls, and what was built is `flush_into` — claim and drain fused,
//! emptying the buffer whether or not the records land. That shape cannot
//! satisfy this crate's O3/O4: a rejected batch would already be gone.
//! `TlsBuffer::drain` was added there so the check can happen before the buffer
//! is touched (→ `docs/algorithm/002_sequencing_seal_drain_reset.md`).
//!
//! Feature 176 is Reached when each of the three policies fires at exactly its
//! stated trigger and at no other point, asserted by a scripted sequence
//! against a recorded flush log — see `tests/flush_test.rs`.

#![ deny( missing_docs ) ]

use ring_core::Producer;
use ring_tls::TlsBuffer;
use ring_types::RingError;

/// When a buffer's staged records are published to the ring.
///
/// Three variants, one carrying a parameter, and the asymmetry is load-bearing:
/// `OnFull` and `OnBarrier` name conditions determined elsewhere — by the
/// buffer's capacity and by the consumer's schedule. `OnBatch( n )` names a
/// condition this crate owns outright, which is why it is the only variant with
/// a parameter, the only one with state, and the only one that can be
/// misconfigured.
///
/// # No `Default`
///
/// Withheld deliberately, and it is the most likely trait to be added by
/// mistake. A default policy is a policy nobody chose, applied wherever someone
/// wrote `..Default::default()` — which recreates, in one derive, exactly the
/// "publication point nobody designed" state feature 176 exists to prevent.
///
/// ```
/// use ring_flush::FlushPolicy;
/// assert_eq!( FlushPolicy::OnBatch( 8 ), FlushPolicy::OnBatch( 8 ) );
/// assert_ne!( FlushPolicy::OnFull, FlushPolicy::OnBarrier );
/// ```
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum FlushPolicy
{
  /// Flush when the buffer cannot accept another record.
  OnFull,
  /// Flush when the driver is told the stage barrier has been reached.
  OnBarrier,
  /// Flush when `n` records have accumulated since the last flush.
  OnBatch( usize ),
}

/// Why a flush fired.
///
/// Separate from [`FlushPolicy`], and the separation is the point: a log
/// recording only "a flush occurred under policy `OnBatch( 64 )`" cannot detect
/// an `OnBatch` policy that *also* fires when full — the entries are
/// indistinguishable from correct ones. Recording why each flush fired is what
/// turns the log from a count into evidence.
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum FlushCause
{
  /// The buffer could accept no more records.
  Full,
  /// A barrier was announced to the driver.
  Barrier,
  /// The configured batch size was reached.
  Batch,
  /// A final drain, which ignores the policy.
  Shutdown,
}

/// What a drive call reports.
///
/// The first two variants are the reason this type exists. A `bool` return, or
/// a bare count, collapses `NotTriggered` and `TriggeredEmpty` into the same
/// observation — nothing moved — and they mean opposite things: under
/// `NotTriggered` there may be any number of records waiting, and **a consumer
/// that only ever sees it has a misconfigured [`FlushPolicy::OnBarrier`]**.
/// Making that misconfiguration observable is the whole design intent, since
/// the crate cannot prevent it.
///
/// Not a `Result`: three of the four are ordinary outcomes. `Rejected` is the
/// backpressure the design expects, not a bug.
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
pub enum FlushOutcome
{
  /// The policy was consulted and its trigger did not hold.
  NotTriggered,
  /// The trigger held; the buffer held no records.
  TriggeredEmpty,
  /// The trigger held; `count` records were moved into the ring.
  Flushed
  {
    /// How many records landed.
    count : usize,
  },
  /// The trigger held; the ring could not accept the batch. The records are
  /// **still staged** and the call is safe to retry.
  Rejected
  {
    /// How many records remain staged.
    staged : usize,
  },
}

/// Why binding a policy to a buffer was refused.
///
/// Every variant is a configuration error surfaced at binding time, which is
/// the design intent: an invalid configuration should fail before any record is
/// appended, not degrade into a different working policy after a million of
/// them. Each unvalidated case would degrade into *a different, working policy*
/// — and a program producing a benchmark verdict about the wrong policy is
/// worse than one that refuses to start.
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub enum ConfigError
{
  /// `OnBatch( 0 )` would fire on every append — a fourth policy by accident.
  ZeroBatch,
  /// `OnBatch( n )` with `n` above the buffer's capacity could never fire, and
  /// would degrade into `OnFull`.
  BatchExceedsCapacity
  {
    /// The requested batch size.
    requested : usize,
    /// The buffer's capacity in records.
    capacity : usize,
  },
}

// Added late, by the crate that first consumed this one from outside its own
// tests. `ring_flush` is on the family's export Contract and this was the only
// error type on that Contract implementing neither trait — `ring_types::RingError`
// and `ring_factory::BuildError` both do — so a consumer could not render a
// binding refusal or fold it into a `Box< dyn Error >` alongside the other two.
// Found by `ring_bench`, which is the first crate to hold all three at once;
// see that crate's `docs/integration/001`.
impl core::fmt::Display for ConfigError
{
  fn fmt( &self, f : &mut core::fmt::Formatter< '_ > ) -> core::fmt::Result
  {
    match self
    {
      Self::ZeroBatch => f.write_str( "OnBatch( 0 ) fires on every append" ),
      Self::BatchExceedsCapacity { requested, capacity } =>
      {
        write!( f, "OnBatch( {requested} ) never fires in a {capacity}-record buffer" )
      }
    }
  }
}

impl core::error::Error for ConfigError {}

/// One recorded flush.
///
/// The third field is the [`FlushOutcome`] the same call returned, not a
/// separate count. The pre-implementation shape was `policy | cause | count`,
/// and a count cannot tell [`FlushOutcome::Rejected`] from
/// [`FlushOutcome::TriggeredEmpty`] — both moved nothing, and they mean
/// opposite things about whether the ring had room. Carrying the outcome makes
/// the count derivable ([`FlushEntry::count`]) and makes the requirement that
/// log and outcome agree true by construction rather than asserted and hoped.
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct FlushEntry
{
  /// Which policy was in effect.
  pub policy : FlushPolicy,
  /// Why this flush fired.
  pub cause : FlushCause,
  /// What the drive call that produced this entry returned.
  pub outcome : FlushOutcome,
}

impl FlushEntry
{
  /// How many records moved. Zero for a trigger that found the buffer empty or
  /// one the ring refused.
  // Fix(flush_entry_count_catchall_not_exhaustive): was `_ => 0` behind the one
  //   named `Flushed` arm, so a future `FlushOutcome` variant carrying its own
  //   moved-record count (e.g. a partial flush) would silently report 0 moved
  //   records instead of failing to compile.
  // Root cause: `_` in a `match` accepts every variant it was not told about,
  //   current or future, with the same arm — indistinguishable at the call
  //   site from a variant that was actually considered and is genuinely zero.
  // Pitfall: a caller summing `count()` across a log to reconcile how much data
  //   moved would silently under-count the moment such a variant existed,
  //   with nothing at this call site pointing at why the total came up short.
  #[ must_use ]
  pub const fn count( &self ) -> usize
  {
    match self.outcome
    {
      FlushOutcome::Flushed { count } => count,
      FlushOutcome::NotTriggered
      | FlushOutcome::TriggeredEmpty
      | FlushOutcome::Rejected { .. } => 0,
    }
  }
}

/// A record of every flush and its cause.
///
/// # Why this exists
///
/// Feature 176's criterion is partly negative: each policy fires at its trigger
/// **and at no other point**. Proving a flush happened needs no log — observe
/// the ring. Proving no *other* flush happened requires a record of every flush
/// that did. You cannot observe an absence; you can only enumerate the
/// presences and find the set complete.
///
/// # Why it has no compilation boundary
///
/// A log the driver always maintains would allocate on the flush path in
/// production and grow without bound in a long-running process — so the
/// pre-implementation instance weighed `cfg(test)` (invisible to integration
/// tests), `cfg(debug_assertions)` (coarse), a `flush-log` cargo feature (the
/// acceptance criterion then holds only under a non-default feature), and a
/// caller-supplied sink (a generic parameter on an exported type).
///
/// **None was needed.** The log is opt-in per [`Flusher`]: absent unless
/// [`Flusher::with_log`] is called, one `Option` branch on the cold path when
/// present. The criterion holds under default features, `cargo check` passes
/// with and without them, and a release build that never opts in allocates
/// nothing — which is what all four options were trying to buy.
///
/// # Completeness
///
/// Entries carry the [`FlushOutcome`] the drive call returned, written inside
/// that call rather than beside it, so a flush that produced an outcome and no
/// entry is unrepresentable and an entry that disagrees with its outcome is
/// unrepresentable too. That is stronger than the acceptance row asks for: the
/// row mandates a log and asserts against it, but requires nothing of its
/// completeness, and a flush path that bypassed logging would satisfy every
/// assertion while proving nothing.
///
/// **A `NotTriggered` call is not logged, and that is the design.** The log
/// records firings; `is_empty()` is then the assertion that discharges "and at
/// no other point" directly, rather than a filter over entries that would have
/// to be written correctly to mean anything.
#[ derive( Debug, Clone, Default, PartialEq, Eq ) ]
pub struct FlushLog
{
  entries : Vec< FlushEntry >,
}

impl FlushLog
{
  /// An empty log.
  #[ must_use ]
  pub const fn new() -> Self
  {
    Self { entries : Vec::new() }
  }

  /// Every flush recorded so far, in the order it happened.
  ///
  /// Ordering is `Vec` position. There is no timestamp, deliberately: reading a
  /// clock is forbidden anywhere near this path, and position already carries
  /// the ordering a test needs.
  #[ must_use ]
  pub fn entries( &self ) -> &[ FlushEntry ]
  {
    &self.entries
  }

  /// How many flushes have been recorded.
  #[ must_use ]
  pub fn len( &self ) -> usize
  {
    self.entries.len()
  }

  /// Whether no flush has been recorded.
  ///
  /// The assertion that discharges the "and at no other point" clause.
  #[ must_use ]
  pub fn is_empty( &self ) -> bool
  {
    self.entries.is_empty()
  }

  /// Forget every entry, keeping the allocation. Used between scenarios.
  pub fn clear( &mut self )
  {
    self.entries.clear();
  }
}

/// A policy bound to a buffer and a ring producer.
///
/// # Driven, not self-firing
///
/// There is no thread, no timer, no callback registration, and no `Drop` impl.
/// Nothing here flushes unless [`Flusher::drive`], [`Flusher::drive_at_barrier`]
/// or [`Flusher::drain_final`] is called. That is a deliberate commitment: the
/// publication point is the caller's, chosen at a cadence no crate in this
/// family owns.
///
/// # The unverifiable argument
///
/// [`Flusher::drive_at_barrier`] asserts a fact this crate has no way to check
/// — that the stage barrier really was reached. This crate knows that the call
/// was made; it does not know whether a barrier was crossed, which barrier the
/// ring's consumer is gated on, or whether the caller holds the right barrier
/// at all. Every consequence of a wrong announcement is the caller's, and
/// stating that plainly is the only mitigation available.
///
/// Two named methods rather than `drive( at_barrier : bool )`, because a reader
/// auditing whether barriers are announced correctly can grep for
/// `drive_at_barrier` and cannot grep for `true`.
#[ derive( Debug ) ]
pub struct Flusher< 'a, T >
{
  buffer : TlsBuffer< T >,
  producer : Producer< 'a, T >,
  policy : FlushPolicy,
  log : Option< FlushLog >,
}

/// The `Send` bound is inherited, not local. Every `ring_core::Producer` method
/// requires it, so a `Flusher` over a non-`Send` record type can be constructed
/// but can do nothing — putting the bound here makes that a compile error at
/// the type rather than a puzzling `E0599` at the call.
impl< 'a, T : Send > Flusher< 'a, T >
{
  /// Bind a policy to a buffer and the producer its records will reach.
  ///
  /// **This is where validation happens** — a [`FlushPolicy`] value alone
  /// carries no guarantee it is valid for the buffer it is about to be used
  /// with, because the batch-size rule needs the buffer's capacity to check.
  ///
  /// The `producer` argument is not in the pre-implementation surface, which
  /// specified `Flusher::new( buffer, policy )`. A `drive( &mut self )` with no
  /// arguments cannot reach a ring it was never given, so the ring is bound
  /// here or it is passed to every drive call; binding it once is what makes
  /// the driver surface argument-free.
  ///
  /// # Errors
  ///
  /// [`ConfigError::ZeroBatch`] when `OnBatch( 0 )` — it would fire on every
  /// append, which is a fourth policy nobody chose.
  /// [`ConfigError::BatchExceedsCapacity`] when the batch size is above the
  /// buffer's capacity — it could never fire, and the buffer would fill and
  /// behave as `OnFull`.
  ///
  /// There is no `AlreadyBound` variant. Binding a second policy to a bound
  /// buffer is the rule "a buffer has exactly one policy", and this signature
  /// takes the buffer **by value** — so ownership enforces it and no runtime
  /// check is reachable. An error variant that cannot be constructed is worse
  /// than no variant: it invites a caller to write a match arm for a case that
  /// will never arrive.
  ///
  /// ```
  /// use ring_flush::{ ConfigError, FlushPolicy, Flusher };
  /// use ring_tls::TlsBuffer;
  ///
  /// # use ring_config::RingConfig;
  /// # use ring_core::Ring;
  /// # let mut ring : Ring< u32 > = Ring::new( &RingConfig::new( 16 ).unwrap() ).unwrap();
  /// # let mut ends = ring.ends();
  /// # let ( producer, _consumer ) = ends.split();
  /// let buffer = TlsBuffer::< u32 >::with_capacity( 4 );
  /// let refused = Flusher::new( buffer, producer, FlushPolicy::OnBatch( 0 ) );
  /// assert_eq!( refused.unwrap_err(), ConfigError::ZeroBatch );
  /// ```
  pub fn new
  (
    buffer : TlsBuffer< T >,
    producer : Producer< 'a, T >,
    policy : FlushPolicy,
  )
  -> Result< Self, ConfigError >
  {
    if let FlushPolicy::OnBatch( n ) = policy
    {
      if n == 0
      {
        return Err( ConfigError::ZeroBatch );
      }

      if n > buffer.capacity()
      {
        return Err
        (
          ConfigError::BatchExceedsCapacity { requested : n, capacity : buffer.capacity() }
        );
      }
    }

    Ok( Self { buffer, producer, policy, log : None } )
  }

  /// Record every flush this driver performs.
  ///
  /// Opt-in, and absent by default — see [`FlushLog`] for why that replaced a
  /// compilation boundary rather than needing one.
  pub fn with_log( mut self ) -> Self
  {
    self.log = Some( FlushLog::new() );
    self
  }

  /// The bound policy. `Copy`; no interior state is exposed.
  #[ must_use ]
  pub const fn policy( &self ) -> FlushPolicy
  {
    self.policy
  }

  /// How many records are staged.
  ///
  /// **Advisory.** Between this read and any action taken on it, an append on
  /// the owning thread can change the count. It is useful for diagnostics and
  /// for deciding whether a drive is worthwhile; it is not a basis for a
  /// correctness decision.
  #[ must_use ]
  pub fn staged( &self ) -> usize
  {
    self.buffer.len()
  }

  /// How many records the staging buffer can hold before it refuses.
  ///
  /// Fixed for the driver's lifetime — the buffer is sized once and never
  /// grows, which is what makes [`Flusher::append`] able to refuse rather than
  /// reallocate.
  ///
  /// **The companion to [`Flusher::staged`], and the reason both are needed.**
  /// An `OnBarrier` caller has no trigger of its own between announcements, so
  /// its buffer can fill and start refusing appends
  /// (`docs/algorithm/001`'s option 2). `capacity() - staged()` is how many
  /// more records it may stage before that happens — the one number that lets
  /// a caller drive early instead of discovering the refusal from an `Err`.
  ///
  /// Advisory in the same way `staged` is: an append on the owning thread can
  /// consume the headroom between the read and the use.
  #[ must_use ]
  pub fn buffer_capacity( &self ) -> usize
  {
    self.buffer.capacity()
  }

  /// The recorded flushes, if this driver was built with a log.
  #[ must_use ]
  pub fn log( &self ) -> Option< &FlushLog >
  {
    self.log.as_ref()
  }

  /// Forget every recorded entry, if there is a log. Used between scenarios.
  pub fn clear_log( &mut self )
  {
    if let Some( log ) = self.log.as_mut()
    {
      log.clear();
    }
  }

  /// Stage one record. No atomic, no lock, no allocation, and **no flush**.
  ///
  /// Appending never publishes: that is the driven-not-self-firing commitment.
  /// A full buffer refuses the record rather than flushing to make room, so an
  /// `OnBarrier` buffer that fills before a barrier is announced applies
  /// backpressure to the writer instead of publishing at a point nobody chose.
  ///
  /// **There is no batch counter**, and the append path is one `push` because
  /// of it. The pre-implementation algorithm called for a `usize` incremented
  /// here for `OnBatch` — but records staged since the last flush is exactly
  /// what the buffer's own occupancy already is, on every path: this driver
  /// owns the buffer privately, only `append` adds to it, and only a flush that
  /// succeeds empties it. A rejected flush leaves both untouched together. The
  /// counter could never disagree with `TlsBuffer::len`, so it was redundant
  /// state and an increment on the one path this crate is not allowed to make
  /// expensive.
  ///
  /// The redundancy was found by probe, not by reading: see
  /// `tests/manual/readme.md`'s F3.
  ///
  /// # Errors
  ///
  /// [`RingError::Full`] when the buffer already holds `capacity()` records.
  /// **The record is not returned on refusal — it is dropped.**
  /// `TlsBuffer::push` (`ring_tls`) has no channel to hand it back: its
  /// signature is `Result< (), RingError >` and `RingError::Full` carries
  /// nothing, so `record` drops silently inside `push` before this function
  /// returns. Retrying with the same value is therefore not possible; the fix
  /// is `ring_tls`'s (a widened `push( item : T ) -> Result< (), ( RingError,
  /// T ) >`), which this signature would need to widen with.
  pub fn append( &mut self, record : T ) -> Result< (), RingError >
  {
    self.buffer.push( record )
  }

  /// Drive without announcing a barrier.
  ///
  /// Evaluates [`FlushPolicy::OnFull`] and [`FlushPolicy::OnBatch`].
  /// [`FlushPolicy::OnBarrier`] always returns [`FlushOutcome::NotTriggered`]
  /// here — this is the only route by which a misconfigured `OnBarrier` becomes
  /// observable, and a `drive` that fired for it would be the silent
  /// degeneration the whole design guards against.
  pub fn drive( &mut self ) -> FlushOutcome
  {
    match self.trigger( false )
    {
      Some( cause ) => self.run( cause ),
      None => FlushOutcome::NotTriggered,
    }
  }

  /// Drive announcing that the stage barrier has been reached.
  ///
  /// The only route by which [`FlushPolicy::OnBarrier`] can fire. The other two
  /// policies are still evaluated, so a caller that announces barriers does not
  /// have to know which policy is bound.
  ///
  /// **This crate believes the announcement and cannot check it** — see
  /// [`Flusher`]'s own note.
  pub fn drive_at_barrier( &mut self ) -> FlushOutcome
  {
    match self.trigger( true )
    {
      Some( cause ) => self.run( cause ),
      None => FlushOutcome::NotTriggered,
    }
  }

  /// Publish everything staged, ignoring the policy.
  ///
  /// A named lifecycle phase — called once, at teardown, by the owner — and not
  /// a general escape hatch. An unconditional `flush_now()` available to any
  /// caller is the exact capability this crate exists to remove: it puts the
  /// publication decision back at the call site. The distinction between the
  /// two lives in naming and documentation rather than in the type system, and
  /// is therefore a convention that can erode.
  ///
  /// **A [`FlushOutcome::Rejected`] here must be retried**, and nothing in this
  /// crate will do it. Rust has no linear types; a [`Flusher`] can be dropped
  /// with records staged, and they are gone.
  pub fn drain_final( &mut self ) -> FlushOutcome
  {
    self.run( FlushCause::Shutdown )
  }

  /// Which cause fires, if any. Reads the policy by value; touches no atomic.
  // Fix(flush_policy_trigger_catchall_not_exhaustive): was a guard-based match
  //   ending `_ => None`, so a fourth `FlushPolicy` variant would silently
  //   never trigger — compiling cleanly while never firing, regardless of its
  //   own condition. Restructured to match the variant exhaustively first and
  //   apply each guard inside its own arm, preserving every existing
  //   true/false outcome exactly.
  // Root cause: a match guard (`if …`) does not participate in exhaustiveness
  //   checking, so `_` was covering two different things at once — "the named
  //   variant's condition is false" and "a variant this match was never told
  //   about" — and a new variant silently took the same `None` as the first.
  // Pitfall: `trigger` returning `None` for a real policy is indistinguishable
  //   from that policy's condition simply not holding yet; a caller has no
  //   signal that the policy was never wired up at all.
  fn trigger( &self, at_barrier : bool ) -> Option< FlushCause >
  {
    match self.policy
    {
      FlushPolicy::OnFull => if self.buffer.is_full() { Some( FlushCause::Full ) } else { None },
      FlushPolicy::OnBarrier => if at_barrier { Some( FlushCause::Barrier ) } else { None },
      FlushPolicy::OnBatch( n ) => if self.buffer.len() >= n { Some( FlushCause::Batch ) } else { None },
    }
  }

  /// Seal, claim, drain, reset, report — the cold path, run once per flush.
  ///
  /// The ordering obligations are structural rather than conditional. The
  /// capacity check happens **before the buffer is touched at all**, so the
  /// rejection path cannot reach the drain; and the drain is a single
  /// expression, so no later edit can move a reset above it.
  fn run( &mut self, cause : FlushCause ) -> FlushOutcome
  {
    let staged = self.buffer.len();

    if staged == 0
    {
      return self.record( cause, FlushOutcome::TriggeredEmpty );
    }

    // Step 2 — Claim. The only step that can fail, and it fails before the
    // buffer is read. `free_capacity` is a snapshot, but on a ring this
    // `Flusher` is the sole producer of it can only grow between here and the
    // push: the consumer's drain frees space and nothing else consumes it.
    if self.producer.free_capacity() < staged
    {
      return self.record( cause, FlushOutcome::Rejected { staged } );
    }

    // Steps 1, 3 and 4 — seal, drain, reset. `TlsBuffer::drain` empties the
    // buffer as the iterator drops, so the reset is not a separate statement
    // that could be reordered above the push.
    let count = self.producer.try_push_batch( &mut self.buffer.drain() );

    // `count < staged` is not asserted here, and the omission is deliberate.
    // It is reachable only by violating this crate's contract — a second
    // producer on the same ring taking the space between the check above and
    // this push — and at this point the shortfall is already unrecoverable:
    // `try_push_batch` consumes the record it fails to place. A
    // `debug_assert!` would promise a guarantee that evaporates in exactly the
    // build where the race is likely, and no runtime check can restore records
    // that are already gone. What is reported is what landed.
    self.record( cause, FlushOutcome::Flushed { count } )
  }

  /// Derive the log entry from the outcome, so the two cannot disagree.
  fn record( &mut self, cause : FlushCause, outcome : FlushOutcome ) -> FlushOutcome
  {
    if let Some( log ) = self.log.as_mut()
    {
      log.entries.push( FlushEntry { policy : self.policy, cause, outcome } );
    }

    outcome
  }
}
