# pitfall

Two ways to be wrong with a function that has no error path. The first is to
reach the end of the sequence space, where `run`'s additions do something the
build profile decides and the upstream doc, when this was filed, described
incorrectly. The second is to not call this crate at all, and get the arithmetic
right while getting its provenance wrong.

Neither is a bug in `of`. `of` is total, in both profiles, at every input the
types permit — which is what makes these two worth separating out: the hazards
in this crate are all in what surrounds the fold, never in the fold.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_run_that_panics_in_debug_and_wraps_in_release.md) | The Run That Panics in Debug and Wraps in Release | `run` at `u64::MAX`, the inherited `+`, and a `ring_types` comment that described saturation — since corrected |
| [002](002_the_second_fold_nobody_noticed.md) | The Second Fold Nobody Noticed | `ring_mpsc`'s hand-written fold, and the two fields it reads its length from |

## The Build Profile Is Part of the Contract, and Nothing Says So

`run( Seq( u64::MAX ), 2, cap )` panics under `cargo test` and returns two slots
under `cargo test --release`. When this was filed, neither `run` nor
`Seq::advanced_by` carried any overflow documentation, so a reader had no way to
learn this — and the one comment in the family that addressed `Seq` overflow
stated the release behaviour as saturation, which it is not.

`Seq::advanced_by` has since been given an overflow paragraph, and it names this
exact case: the reachability argument that covers `next` does not carry over,
because `n` comes from the caller. `run` — this crate's own half — still carries
nothing, and no test drives it across the boundary.

The measured consequence matters more than the panic. `u64::MAX + 1` wraps to
`0`, so a wrapped `Seq` folds to slot 0 and compares as *less than* every
sequence before it. That is exactly the "silently violate the monotonicity every
gate in the family relies on" outcome the `Seq::next` comment said the design
avoided by not wrapping — a claim since replaced by the accurate one, that it
wraps and the wrap point is 584 years away.

## Duplication Is the Symptom; Split Provenance Is the Disease

`ring_mpsc`'s `stamp()` writes the fold by hand — the recipe above prints the
line, and the crate has exactly one. Read as duplication, the fix is to call
`ring_index::of` and move on. Read carefully, that fix is insufficient: the line
masks with `self.consumers.capacity()` and indexes `self.stamps`, which was
sized from a constructor argument stored nowhere. Calling `of` would leave both
halves exactly where they are.

What `ring_store` does differently is not "calls the shared function" but
"keeps the capacity in the same struct as the storage it sizes." One number, one
owner, nothing to drift. That is the property the one-owner rule is protecting,
and it is not the property that a `grep` for duplicated expressions finds.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# both methods entire — doc paragraph and body — since the paragraphs are what
# this file's two claims turn on and a fixed-length window silently truncates them
command grep -m1 -A16 -F '  /// The next sequence after this one.' ring_types/src/id.rs
command grep -m1 -A18 -F '  /// This sequence advanced by `n`.' ring_types/src/id.rs
command grep -r 'u64::MAX' ring_*/src/*.rs | sed 's/:  */: /'
command grep -r '( seq.0 as usize ) &' ring_*/src/*.rs | sed 's/:  */: /'
sed -n '/^    let stamps = ( 0 \.\. capacity\.get() )$/,/^    }$/p;/^  pub fn capacity( &self ) -> Capacity$/,/^  }$/p;/^  fn stamp( &self, seq : Seq ) -> &AtomicSeq$/,/^  }$/p' ring_mpsc/src/lib.rs
sed -n '/^pub struct Buffer< S >$/,/^}$/p;/^  pub fn new( capacity : Capacity ) -> Self$/,/^  }$/p' ring_store/src/lib.rs
```

Live output:

```
  /// The next sequence after this one.
  ///
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 41 ).next(), Seq( 42 ) );
  /// ```
  #[ must_use ]
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  }
  /// This sequence advanced by `n`.
  ///
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
  /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
  /// takes `n` from the caller and reaches it in a single call from any
  /// position. A caller deriving `n` from a batch length or a configured
  /// count owns that bound; nothing here checks it.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 10 ).advanced_by( 5 ), Seq( 15 ) );
  /// assert_eq!( Seq( 10 ).advanced_by( 0 ), Seq( 10 ) );
  /// ```
  #[ must_use ]
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
ring_atomic/src/lib.rs: /// `u64`, and the addition wraps: `fetch_add( u64::MAX )` moves the cursor
ring_bench/src/lib.rs: // wrap to a near-`u64::MAX` stat instead of panicking, exactly the failure
ring_debug/src/lib.rs: // build wraps to roughly `u64::MAX`, which exceeds every capacity and so
ring_index/src/lib.rs:/// from `ring_mpsc::UNSTAMPED`, the family's `Seq( u64::MAX )` sentinel,
ring_mpsc/src/lib.rs:/// assert_eq!( ring_mpsc::UNSTAMPED, Seq( u64::MAX ) );
ring_mpsc/src/lib.rs:pub const UNSTAMPED : Seq = Seq( u64::MAX );
ring_stats/src/lib.rs: // takes an unbounded `n`, so two calls whose counts summed past `u64::MAX`
ring_stats/src/lib.rs: // Root cause: the fold assumed its three inputs would never sum past `u64::MAX`,
ring_trace/src/lib.rs: /// `ring_mpsc::UNSTAMPED` (`Seq(u64::MAX)`). See `pitfall/001` TR41.
ring_index/src/lib.rs: SlotIndex( ( seq.0 as usize ) & capacity.mask() )
ring_mpsc/src/lib.rs: let index = ( seq.0 as usize ) & self.capacity().mask();
    let stamps = ( 0 .. capacity.get() )
      .map( | _ | AtomicSeq::new( UNSTAMPED ) )
      .collect::< Vec< _ > >()
      .into_boxed_slice();

    Self
    {
      slots : Buffer::new( capacity ),
      stamps,
      consumers : GatingSet::new( capacity, 1 ),
    }
  pub fn capacity( &self ) -> Capacity
  {
    self.consumers.capacity()
  }
  fn stamp( &self, seq : Seq ) -> &AtomicSeq
  {
    let index = ( seq.0 as usize ) & self.capacity().mask();
    // The mask is `capacity - 1` for a power-of-two capacity, which `Capacity`
    // enforces at construction, so the index is always in range.
    &self.stamps[ index ]
  }
pub struct Buffer< S >
{
  slots : Box< [ S ] >,
  capacity : Capacity,
}
  pub fn new( capacity : Capacity ) -> Self
  {
    let mut slots = Vec::with_capacity( capacity.get() );
    slots.resize_with( capacity.get(), S::default );
    Self { slots : slots.into_boxed_slice(), capacity }
  }
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX16 | `ring_index` | n/a — doc gap | `run` inherits overflow behaviour from a bare `+` in `ring_types` and documents no `# Panics`. **Still open** — `advanced_by` upstream now documents it, `run` does not, and no test drives the boundary |
| IX17 | `ring_types` | **wrong doc** | `Seq::next`'s comment said release builds saturate; they wrap, producing the exact monotonicity violation the same comment said the design avoids. The comment now states debug panic / release wrap, and rests the case on the 584-year reachability bound instead |
| IX18 | `ring_mpsc` | **latent hazard** | `UNSTAMPED` publishes `Seq( u64::MAX )`, making the overflow boundary reachable in one expression from a public constant |
| IX19 | `ring_mpsc` | **latent hazard** | `stamp()` masks with `consumers`' capacity and indexes `stamps`, sized from an unstored constructor argument; the comment above it asserts the wrong guarantee |
| IX20 | `ring_mpsc` | n/a — coverage | All three `stamps().len()` assertions compare against hard-coded integers, never against `capacity()` — the field the fold actually uses |
