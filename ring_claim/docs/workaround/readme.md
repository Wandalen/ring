# workaround

Two files for two kinds of constraint the crate did not choose. The first is the
language's: a derived `PartialOrd` two crates away is not `const`, so two methods
that could be `const` are not, and a sibling crate escapes the same constraint by
reaching through a public field. The second is the project's own: tests live in
`tests/`, integration tests are separate crates, and so a constructor that has no
external caller must be public anyway.

They differ in what "workaround" means, and the difference is worth stating
because it decides what to do about each. A language constraint is a fact to be
routed around, and both routes here are cheap, proven, and available. A project
constraint is a trade already made deliberately elsewhere — the `tests/`
convention buys 37 honest integration suites and costs 32 public constructors,
and that is a good bargain. Nothing in this definition argues for changing it.

What both files find instead is that neither constraint is recorded where it
acts. `Claim` is missing two `const fn`s and does not say why. `Claim::new` is
public because of a project rule, and its own doc gives a different reason
([`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md)
§ CL12, which owns that finding and is not repeated here). In both cases the code
is defensible and the documentation is not, which is the same shape
[`type/`](../type/readme.md) found in the signatures and
[`pitfall/`](../pitfall/readme.md) found in the warnings.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Const Functions the Compiler Refuses](001_two_const_functions_the_compiler_refuses.md) | CL51, CL52 — E0015 on a derive two crates away, the `.0` escape a sibling uses, the edition-2024 `use< >` escape pointing the other way, and the free combination `ring_claim` now takes |
| 002 | [The Constructor That Exists for the Test Directory](002_the_constructor_that_exists_for_the_test_directory.md) | CL53, CL54 — the two of four preconditions a forgeable `Claim` could carry, and the convention held 33 of 33 times that keeps it forgeable |

### The Two Escapes, Side by Side

Both files turn on the same observation from opposite ends: `ring_claim` and
`ring_batch` implement the same range type and each escaped exactly one of the
two language constraints, in opposite directions.

| | `ring_claim::Claim` | `ring_batch::BatchClaim` |
|--|---------------------|--------------------------|
| Receiver | `self` — a 16-byte `Copy` value | `&self` |
| Needs `+ use< >` on `sequences` | **no** | **yes** |
| Comparison inside `contains`/`overlaps` | `.0`, raw `u64` | `.0`, raw `u64` |
| `const fn` of those achievable | **7 of 7** | **7 of 7** |
| Newtype held through the predicate | no | no |

The comparison logic in each body is otherwise identical — same operators,
same `.0` field access — though `ring_claim`'s bodies also carry rationale
comments `ring_batch`'s lack, and neither crate's source mentions that the
other receiver choice exists. The combination that pays neither cost —
`self` by value *and* `.0` — is what `ring_claim` now has, after CL52; it was
what neither crate had when these two documents were written.

### Why Each Is Hard to Notice

| Constraint | Where it bites | Why it goes unseen |
|------------|----------------|--------------------|
| `E0015` on a derived operator | only if someone adds `const` | the methods are correct without it; nothing fails |
| edition-2024 RPIT capture | at a **call site in another crate** | the library compiles clean; the diagnostic names `tests/t.rs`, not the signature |
| integration tests are separate crates | at the visibility of every constructor | a `pub` that costs nothing to write and cannot be taken back |

The middle row is the one worth carrying out of this definition. Omitting
`use< >` is not a local error — `ring_batch` would build, ship, and only fail
for a downstream caller who tried to keep the iterator, with a note pointing back
at a signature in a crate they do not own.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# const-ness of every public method on the three range types
for c in ring_claim ring_batch ring_consume; do
  printf '\n--- %s ---\n' "$c"
  grep -E '^  pub (const )?fn' ring/$c/src/lib.rs
done

# the two bodies -- comparison logic identical, ring_claim also carries
# rationale comments the window below must be wide enough to include
awk '/^  pub const fn contains\( self, seq : Seq \) -> bool$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 6 { print } /^  pub const fn overlaps\( self, other : Self \) -> bool$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 9 { print }' ring_claim/src/lib.rs
awk '/^  pub const fn contains\( &self, seq : Seq \) -> bool$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 4 { print } /^  pub const fn overlaps\( &self, other : &Self \) -> bool$/{ n2 = NR } n2 && NR >= n2 && NR <= n2 + 5 { print }' ring_batch/src/lib.rs

# the derive the E0015 note blames
command grep -m1 -B1 -F 'pub struct Seq( pub u64 );' ring_types/src/id.rs

# every iterator-returning method with a receiver, and which carries `use<…>`
command grep -rE 'fn [a-z_]+\( *(&?self|&?mut self)[^)]*\) *-> *impl Iterator' ring_*/src/*.rs

# the dependency edge that makes ring_publish a test-time consumer only
sed -n '/\[dev-dependencies\]/,/^\[/p' ring_publish/Cargo.toml

# the convention, checked for exceptions
for f in ring_*/src/*.rs; do
  n=$( grep -vE "^[[:space:]]*//" "$f" | grep -cE 'cfg\( *test *\)' )
  [ "$n" -gt 0 ] && printf '%-42s %s\n' "$f" "$n"
done
ls -d ring_* | wc -l ; ls -d ring_*/tests | wc -l

# the three escapes the family has never used
grep -rc 'doc( hidden )'       ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
grep -rc 'pub( crate ) fn new' ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
grep -rcE '^  pub (const )?fn new\(' ring_*/src/*.rs | awk -F: '{s+=$2} END {print s}'
```

Live output:

```

--- ring_claim ---
  pub const fn new( start : Seq, len : usize ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> usize
  pub const fn is_empty( self ) -> bool
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn overlaps( self, other : Self ) -> bool
  pub fn new( consumers : &'a GatingSet ) -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub const fn consumers( &self ) -> &'a GatingSet
  pub fn claimed( &self ) -> Seq
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >

--- ring_batch ---
  pub const fn new( start : Seq, count : usize ) -> Self
  pub const fn start( &self ) -> Seq
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub const fn end( &self ) -> Seq
  pub const fn contains( &self, seq : Seq ) -> bool
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  pub const fn overlaps( &self, other : &Self ) -> bool

--- ring_consume ---
  pub const fn new( start : Seq, len : u64 ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> u64
  pub const fn is_empty( self ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn new( cursor : &'a PaddedCursor, barrier : Barrier< 'a > ) -> Self
  pub const fn cursor( &self ) -> &'a PaddedCursor
  pub const fn barrier( &self ) -> Barrier< 'a >
  pub fn position( &self ) -> Seq
  pub fn available( &self ) -> Available
  pub fn available_up_to( &self, max : u64 ) -> Available
  pub fn commit( &self, through : Seq ) -> Result< Seq, RingError >
  pub fn commit_available( &self ) -> Seq
  pub const fn contains( self, seq : Seq ) -> bool
  {
    // Compared as raw `u64` rather than through `Seq`'s operators: `PartialOrd`
    // is not callable in a `const fn`, and reaching through the newtype for two
    // comparisons is the entire cost of having this one at compile time.
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }
  pub const fn overlaps( self, other : Self ) -> bool
  {
    // Raw `u64` comparisons for the same reason `contains` uses them, and with
    // more at stake: this predicate is the one the exclusivity tests assert
    // with, so having it answerable at compile time is worth reaching through
    // the newtype for.
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0
      && other.start.0 < self.end().0
  }
  pub const fn contains( &self, seq : Seq ) -> bool
  {
    seq.0 >= self.start.0 && seq.0 < self.end().0
  }

  pub const fn overlaps( &self, other : &Self ) -> bool
  {
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  }
}
#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default ) ]
pub struct Seq( pub u64 );
ring_batch/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
ring_claim/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_consume/src/lib.rs:  pub fn sequences( self ) -> impl Iterator< Item = Seq >
ring_mpsc/src/lib.rs:  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< S >
ring_mpsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_registry/src/lib.rs:  pub fn names( &self ) -> impl Iterator< Item = &str >
ring_spsc/src/lib.rs:  pub fn iter( &self ) -> impl Iterator< Item = &S >
ring_tls/src/lib.rs:  pub fn drain( &mut self ) -> impl Iterator< Item = T > + '_
[dev-dependencies]
ring_claim = { path = "../ring_claim" }
ring_consume = { path = "../ring_consume" }
ring_barrier = { path = "../ring_barrier" }
ring_gating = { path = "../ring_gating" }

[target.'cfg(loom)'.dev-dependencies]
33
33
0
0
32
```

| | Value |
|--|------:|
| `Claim` methods that are `const` | 7 of 8 |
| …that *could* be, `sequences` excluded | **7** |
| `BatchClaim` methods that are `const` | **7 of 8** — all achievable |
| `Available` methods that are `const` | 5 of 6 — all achievable |
| Characters separating the two `contains` bodies | **`.0`, twice** |
| E0015 errors from adding `const` to `contains` while keeping the `Seq` operators | 2 |
| …from `overlaps`, the same way | 2 |
| Crates away the blamed `derive` lives | **2** (`ring_types/src/id.rs:24`) |
| Iterator-returning methods with a receiver | 7 |
| …carrying `use<…>` | 2 |
| Family-wide `use<…>` occurrences, receiver or not | 3 |
| …exceptions to "`&self` + owned items ⇒ annotated" | **0** |
| Is omitting `use< >` a local compile error | **no** — it fails in the calling crate |
| Crates / `tests/` dirs / integration files | 33 / 33 / 37 |
| Real `#[ cfg( test ) ]` modules in any `src/` | **0** |
| `pub fn new` constructors family-wide | 32 |
| …behind `doc( hidden )`, `pub( crate )`, or a feature | **0** |
| Preconditions a `Claim` on `publish` would enforce | **2 of 4** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL51 | `ring_claim` | n/a — observation | `contains` and `overlaps` were `const` in `ring_batch` and not in `ring_claim`, with comparison logic identical down to the `.0` field access (`ring_claim`'s bodies also carry rationale comments `ring_batch`'s lack); `E0015` blocks the `Seq` operators because the `PartialOrd` impl is derived and non-`const`, and the diagnostic blames `ring_types/src/id.rs:24`, two crates from the code that cannot compile |
| CL52 | family | **latent hazard** | `+ use< >` appears on exactly the 2 of 7 iterator-returning methods that take `&self` and yield owned values, with zero exceptions — the family's only other occurrence is on a free function, outside a census anchored on `self`; omitting it is **not a local error** — the library compiles and the failure is an `E0597` in a downstream crate, noting a signature the caller does not own |
| CL53 | `ring_publish` | n/a — observation | A public constructor makes `Claim` forgeable, so `ring_publish`'s PB35 proposal would enforce **2 of 4** preconditions rather than 4 — worth making, but it hands the crate above a type that looks like proof of a grant and is not, and PB35 argues for the change without pricing it |
| CL54 | family | n/a — observation | 33 of 33 crates keep tests in `tests/` with zero `#[ cfg( test ) ]` modules in any `src/`, so all 32 family constructors are unconditionally public, and none of `doc( hidden )`, `pub( crate )`, or a test-only feature is used anywhere to narrow one |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| `ring_claim` was the only one of the family's three range types leaving any achievable `const` unclaimed — 5 of 7 against `BatchClaim`'s 7 of 7 — and it was the crate where a `const` predicate would be most usable, which is why CL52 was applied rather than recorded | [001](001_two_const_functions_the_compiler_refuses.md) |
| The two escapes are independent and their free combination was the one neither crate took: `self` by value with `.0` comparisons is `const`, needs no `use< >`, and takes no reference to a `Copy` type — `ring_claim` already wrote `self.start.0..self.end().0` three lines above the methods that could use it, and now those methods use it too | [001](001_two_const_functions_the_compiler_refuses.md) |
| Only the third of CL53's four rows is recoverable, by making `new` private, and the `tests/` convention is the sole thing preventing it — a good trade this definition does not argue against, since forcing 37 test files through the public API is worth more than one type's unforgeability | [002](002_the_constructor_that_exists_for_the_test_directory.md) |
| The family's one written consideration of `cfg(test)` is a doc comment in `ring_flush:257` rejecting it for an unrelated purpose, on the correct grounds that it is "invisible to integration tests" — the consequence reasoned out once and drawn nowhere else | [002](002_the_constructor_that_exists_for_the_test_directory.md) |

