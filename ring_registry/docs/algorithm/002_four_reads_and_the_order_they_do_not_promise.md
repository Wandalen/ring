# Algorithm: Four Reads and the Order They Do Not Promise

### Scope

**Purpose:** Measure the four immutable reads and test the two sentences
`names()` uses to dismiss an ordered map — one about when the order changes, one
about what determinism would cost.

**Responsibility:** `contains`, `len`, `is_empty` and `names`; the receiver split
across all eight public methods; the observed iteration order across registries
built identically in one process; and the measured price of `BTreeMap` against
`HashMap` at registry-sized populations.

**In Scope:** `ring_registry/src/lib.rs:204-235`, particularly `:225` and
`:227-231`.

**Out of Scope:** The write path is
[`algorithm/001`](001_two_branches_and_what_the_refusal_costs.md). Why there is no
immutable `get` is [`api/002`](../api/002_the_receiver_split_and_the_sweep_it_forbids.md).
The choice of `HashMap` as a family outlier is
[`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md).

---

## The Four Reads, and What the Crate Promises About Them

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the four reads, whole --'
command grep -m1 -A31 -F '  /// Whether a name is live.' ring_registry/src/lib.rs
echo '  -- how the eight public methods divide by receiver --'
# the inherent impl only, and its declarations only: `&mut self` also appears in
# get_mut's prose and `( &self` in Display's fmt, neither a method of Registry
decl=$( command grep -m1 -A135 -F 'impl< T > Registry< T >' ring_registry/src/lib.rs | command grep -v '///' )
printf '    pub fn: %s   &mut self: %s   &self: %s   no receiver: %s\n' \
  "$( printf '%s\n' "$decl" | command grep -c '^  pub fn' || true )" \
  "$( printf '%s\n' "$decl" | command grep -c '&mut self' || true )" \
  "$( printf '%s\n' "$decl" | command grep -c '( &self' || true )" \
  "$( printf '%s\n' "$decl" | command grep -c 'pub fn new()' || true )"
echo '  -- and the one ordering promise the crate makes --'
command grep 'in no particular order\|varies between one map\|not because it costs more' ring_registry/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  -- the four reads, whole --
  /// Whether a name is live.
  #[ must_use ]
  pub fn contains( &self, name : &str ) -> bool
  {
    self.rings.contains_key( name )
  }

  /// How many rings are registered.
  #[ must_use ]
  pub fn len( &self ) -> usize
  {
    self.rings.len()
  }

  /// Whether nothing is registered.
  #[ must_use ]
  pub fn is_empty( &self ) -> bool
  {
    self.rings.is_empty()
  }

  /// Every live name, in no particular order.
  ///
  /// `HashMap` iteration order is unspecified and varies between one map and
  /// the next, even within a single run — so a caller comparing this against
  /// an expected list must sort or collect into a set first. An ordered map
  /// would remove that hazard; it is not used because the population is not
  /// known to stay small, not because it costs more today.
  pub fn names( &self ) -> impl Iterator< Item = &str >
  {
    self.rings.keys().map( String::as_str )
  }
  -- how the eight public methods divide by receiver --
    pub fn: 8   &mut self: 3   &self: 4   no receiver: 1
  -- and the one ordering promise the crate makes --
      /// Every live name, in no particular order.
      /// `HashMap` iteration order is unspecified and varies between one map and
      /// known to stay small, not because it costs more today.
```

Each of the four is a one-line delegation. None of them allocates, and the four
together with a single-name `names()` walk cost nothing on the heap:

```
    Registry::new()                      0 allocations, 0 bytes
    register into a free name            2 allocations, 1810 bytes
    register into a taken name (refused) 2 allocations, 12 bytes
    contains + get_mut + len + names(1) 0 allocations, 0 bytes
```

```
=== run 1 ===
    contains( "events" )   median 24.03 ns/call  min 23.92  max 26.53
    get_mut( "events" )    median 28.42 ns/call  min 28.29  max 28.52
    register into a taken name  median 113.28 ns/call  min 112.21  max 115.90
=== run 2 ===
    contains( "events" )   median 24.34 ns/call  min 24.19  max 25.30
    get_mut( "events" )    median 28.64 ns/call  min 28.47  max 28.98
    register into a taken name  median 114.36 ns/call  min 112.51  max 160.51
```

## What `names()` Actually Returns, Five Times Over

*This probe, and the `BTreeMap`-vs-`HashMap` timings further down, both ran
once from scratch binaries under `-rg_probe/` and were swept afterward with
the rest of that gitignored directory, per this project's convention for
temporary files — neither can be re-run to reconfirm. `names()` and the
ordering language at `src/lib.rs:225-231` still read exactly as both probes
assume, so nothing here is known to have drifted, but the numbers below stand
as a recorded measurement rather than a live guarantee.*

Five registries, each built in the same process by the same function, each given
the same five names in the same order:

```rust
// -rg_probe/src/bin/names_order.rs
fn order() -> String
{
  let mut registry = Registry::new();
  for name in [ "a", "b", "c", "d", "e" ]
  {
    registry.register( name, ring() ).unwrap();
  }
  registry.names().collect::< Vec< _ > >().join( "" )
}
```

```
=== run 1 ===
    five registries built identically in one process: decab dcabe daebc ebcad dceba
    distinct orders among them: 5
=== run 2 ===
    five registries built identically in one process: bcead cebad deabc bceda acbed
    distinct orders among them: 5
```

## The Cost of the Alternative the Doc Rejects

`contains_key` against a `BTreeMap< String, [ u64; 48 ] >` and a
`HashMap< String, [ u64; 48 ] >` holding the same names, at three populations,
median of nine paired repetitions:

```rust
// -rg_probe/src/bin/btree_vs_hash.rs
if black_box( m ).contains_key( black_box( &keys[ i % keys.len() ] ).as_str() ) { hits += 1; }
```

```
=== run 1 ===
     4 names  HashMap  contains_key  median 29.76 ns/call  min 29.71  max 30.02
     4 names  BTreeMap contains_key  median 15.36 ns/call  min 15.33  max 19.91
     4 names  the ordered map costs  -14.40 ns/call, -48%
    16 names  HashMap  contains_key  median 30.00 ns/call  min 29.94  max 30.24
    16 names  BTreeMap contains_key  median 37.24 ns/call  min 37.18  max 37.47
    16 names  the ordered map costs  +7.24 ns/call, +24%
    64 names  HashMap  contains_key  median 30.05 ns/call  min 29.90  max 31.96
    64 names  BTreeMap contains_key  median 60.72 ns/call  min 60.30  max 67.41
    64 names  the ordered map costs  +30.68 ns/call, +102%
=== run 2 ===
     4 names  HashMap  contains_key  median 29.84 ns/call  min 29.73  max 30.40
     4 names  BTreeMap contains_key  median 15.42 ns/call  min 15.35  max 15.56
     4 names  the ordered map costs  -14.43 ns/call, -48%
    16 names  HashMap  contains_key  median 29.86 ns/call  min 29.77  max 30.31
    16 names  BTreeMap contains_key  median 37.99 ns/call  min 37.41  max 46.27
    16 names  the ordered map costs  +8.13 ns/call, +27%
    64 names  HashMap  contains_key  median 30.35 ns/call  min 29.96  max 39.59
    64 names  BTreeMap contains_key  median 61.01 ns/call  min 60.53  max 66.79
    64 names  the ordered map costs  +30.65 ns/call, +101%
```

---

### RG3 — The Order Varies Between Registries in One Process, Not Between Runs

`names()`'s doc says "`HashMap` iteration order is unspecified and varies between
runs". Both halves are true and the second one is weaker than the truth, in a way
that changes what a reader thinks is safe.

"Varies between runs" describes a failure a caller meets at the process boundary:
write down an expected order today, and a different execution — a different seed,
a different `std` — reorders it. That reading makes a *within*-run comparison look
sound, and it is the natural reading, because it is the guarantee `HashMap`'s own
documentation is usually paraphrased into.

Measured, five registries built by the same function in the same process, given
the same five names in the same order, produce five different orderings — five
distinct out of five, twice over. `RandomState` reseeds per `HashMap`, not per
process. So a caller that builds two registries and compares their `names()`
fails on the first attempt, on this machine, in this run, with no version change
and no second execution involved.

**Finding.** Recorded as an understated hazard rather than a false statement. The
advice the sentence gives — sort, or collect into a set — is exactly right and
covers the real case as well as the stated one, so a caller who follows it is
safe either way. What the sentence costs is the caller who reads the *reason* and
concludes the hazard is remote: they will not follow advice whose stated trigger
they believe cannot fire in their program. One word fixes it — "varies between
registries", or "varies between one `HashMap` and the next, including within a
single run" — and the fix is worth making precisely because everything else in
the sentence is already correct.

**Disposition:** applied — `src/lib.rs:227` now reads "varies between one map
and the next, even within a single run", replacing "varies between runs" with
the stronger and accurate trigger the measurement above establishes. Now
prints: `varies between one map and`

---

### RG4 — The Ordered Map the Doc Rejects as "A Real Cost" Is Faster at Registry Size

The same doc comment closes by pricing the alternative: an ordered map, "to make
the iteration deterministic — would be a real cost paid for a property nothing
needs". The argument has two halves and the measurement contradicts the first.

At **4 names** — a plausible population for a registry of rings, which the crate
itself calls a setup-time structure — `BTreeMap::contains_key` runs at 15.4 ns
against `HashMap`'s 29.8 ns. The ordered map is **48% faster**, reproduced to
0.03 ns across two runs. Comparing four short strings that differ in their first
bytes beats hashing one. The crossover is between 4 and 16 names, where the
ordered map turns 24–27% more expensive, and by 64 names it is 2× slower and the
doc's characterisation is correct.

So the sentence is right about large maps and backwards about the size this
registry runs at. And the property it dismisses as unneeded is the same property
[RG3](#rg3--the-order-varies-between-registries-in-one-process-not-between-runs)
shows the crate warning callers about two sentences earlier: an ordered map would
have removed that warning, not merely satisfied a taste for determinism.

**Finding.** Recorded as a cost argued from a general intuition about hash maps
and never checked against the population in question. Two honest repairs, and
they point in different directions. The cheap one is to correct the sentence —
say the ordered map is dismissed because the family standardises on `HashMap`
elsewhere, or because the population is not known to be small, rather than
because it costs more. The larger one is to take the measurement seriously:
`BTreeMap` is a drop-in for every one of the eight methods, is faster below the
crossover, needs no `RandomState`, deletes RG3's hazard, and — as
[`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md)
records — is the only one of the two that exists in `alloc`, which matters to a
family whose other 32 crates are `no_std`-eligible.

**Disposition:** applied — took the cheap repair: `src/lib.rs:227-231` no
longer calls the ordered map "a real cost paid for a property nothing needs";
it now says the population is "not known to stay small, not because it costs
more today", which is what the measurement above actually shows at 4 names.
Now prints: `not because it costs more`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`algorithm/001`](001_two_branches_and_what_the_refusal_costs.md) | The write path these four are not |
| [`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md) | The `HashMap` these reads delegate to, and its `alloc` substitute |
| [`api/002`](../api/002_the_receiver_split_and_the_sweep_it_forbids.md) | The receiver split, and what the four reads cannot compose into |
| [`invariant/002`](../invariant/002_what_survives_a_refusal_and_what_does_not.md) | What the reads report after a refused registration |
| [`pitfall/002`](../pitfall/002_the_remove_that_drops_a_ring_without_a_word.md) | The read-shaped method that is not a read |

### Sources

| Fact | Where |
|------|-------|
| The four reads, whole | `ring_registry/src/lib.rs:204-235` |
| "in no particular order" | `ring_registry/src/lib.rs:225` |
| "varies between runs" and the ordered-map dismissal | `ring_registry/src/lib.rs:227-231` |
| 3 `&mut self`, 4 `&self`, 1 free, over 8 public methods | Census above |
| Five distinct orders from five identical registries | Probe above, two runs |
| `BTreeMap` 48% faster at 4 names, 2× slower at 64 | Probe above, two runs |
| Zero allocations across the four reads | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `names_lists_every_live_name` | `names()`, sorted before comparison |
| `an_empty_registry_is_empty` | `len` and `is_empty` agreeing at zero |
| `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` | `contains` on both answers |
