# Decisions: Three Pending Questions and the One Consumer

### Scope

**Purpose:** Follow all three Pending entries to the evidence each says would
settle it — a consumer — find the one consumer that exists, and record that it
has already produced the answer one of the three is waiting for.

**Responsibility:** The three Pending entries and their settling conditions; the
seven registry methods `ring_factory` calls; the sorted `names()` assertion; and
what Pending 3's asymptotic pricing costs when it is measured on the operation
Pending 3 itself names.

**In Scope:** `ring_registry/docs/decisions/readme.md:67`, `:83`, `:100`,
`:105-113`; `ring_factory/tests/factory_test.rs:344-346`.

**Out of Scope:** The four Closed entries are
[`decisions/001`](001_four_closed_questions_and_the_one_measurement_none_took.md).
The same ordered-map claim made in the source rather than here is
[`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md).

---

## Three Questions, One Consumer, Seven Methods

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
d=ring_registry/docs/decisions/readme.md
echo '  -- the three pending questions --'
# Bounded to the first 113 lines: `decisions/readme.md` grew a fourth Pending
# entry after that point, owned by RG8, not this instance's three
head -113 "$d" | command grep -n '^### Pending' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- what each names as the evidence that would settle it --'
head -113 "$d" | command grep -n '^\*\*What would settle it:\*\*' | cut -c1-88 | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and the answer Pending 3 predicts a consumer would reach --'
command grep -m1 -A2 -F 'dump compared between runs, say. The cheaper answer would then be for the caller' "$d" | sed 's/^/    /'
echo '  -- and the correction Pending 3 now states --'
command grep -n '^is the faster of the two on' "$d" | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the only consumer, and every registry method it calls --'
# excludes ./Cargo.toml itself: that workspace manifest lists every
# crate as a member path (including ring_registry), which is not the same
# claim as a manifest declaring ring_registry as a dependency
command grep -rln 'ring_registry' --include=Cargo.toml . | command grep -v '^\./Cargo\.toml$' | sed 's|^\./||' | command grep -v '^ring_registry/' | sed 's/^/    manifest: /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -rhon 'registry\.[a-z_]*(' ring_factory/src ring_factory/tests | sort -t: -k2 -u | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- what that consumer does with the unordered iteration --'
command grep -n -e 'registry.names()' -e 'names.sort_unstable' -e 'assert_eq!( names' ring_factory/tests/factory_test.rs | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the largest registry anything in the repository asserts on --'
command grep -rhoE 'registry\.len\(\), [0-9]+' ring_registry/tests ring_factory | sort -u | tr '\n' ' ' | sed 's/^/    /'
echo
```

Live output:

```
  -- the three pending questions --
    ### Pending 1 — Should a registry hold rings of different record types?
    ### Pending 2 — Should names be validated?
    ### Pending 3 — Should `names()` be ordered?
  -- what each names as the evidence that would settle it --
    **What would settle it:** a consumer that genuinely holds rings of two record
    **What would settle it:** a consumer that formats registry names into output
    **What would settle it:** a consumer that needs a stable listing — a diagnostic
  -- and the answer Pending 3 predicts a consumer would reach --
    dump compared between runs, say. The cheaper answer would then be for the caller
    to sort, and this entry exists so that answer is reached deliberately rather than
    by someone changing the map type.
  -- and the correction Pending 3 now states --
    is the faster of the two on `get_mut` — the choice buys headroom, not speed today.
  -- the only consumer, and every registry method it calls --
    manifest: ring_factory/Cargo.toml
    registry.contains(
    registry.get_mut(
    registry.is_empty(
    registry.len(
    registry.names(
    registry.register(
    registry.remove(
  -- what that consumer does with the unordered iteration --
      let mut names : Vec< &str > = registry.names().collect();
      names.sort_unstable();
      assert_eq!( names, vec![ "events", "telemetry" ] );
  -- the largest registry anything in the repository asserts on --
    registry.len(), 0 registry.len(), 1 registry.len(), 2 registry.len(), 3 registry.len(), 4 
```

## The Operation Pending 3 Names, Measured

`get_mut` against both map types, at the three populations that bracket what a
registry holds. Median of nine paired repetitions, both variants run back to back
inside each repetition, `black_box` on the map and the key, `#[ inline( never ) ]`
on both measured functions:

```rust
// src/bin/btree_get_mut.rs — the measured pair
#[ inline( never ) ]
fn hash_gets( m : &mut HashMap< String, V >, keys : &[ String ], n : usize ) -> u64
{
  let mut acc : u64 = 0;
  for i in 0 .. n
  {
    if let Some( v ) = black_box( &mut *m ).get_mut( black_box( &keys[ i % keys.len() ] ).as_str() )
    { acc = acc.wrapping_add( v[ 0 ] ); }
  }
  acc
}
```

```
=== run 1 ===
     1 names  HashMap  get_mut  median 30.48 ns/call  min 30.25  max 34.46
     1 names  BTreeMap get_mut  median 8.60 ns/call  min 8.55  max 10.39
     1 names  the O(log n) map costs  -21.88 ns/call, -72%
     4 names  HashMap  get_mut  median 30.54 ns/call  min 30.32  max 30.76
     4 names  BTreeMap get_mut  median 16.75 ns/call  min 16.67  max 16.92
     4 names  the O(log n) map costs  -13.79 ns/call, -45%
    16 names  HashMap  get_mut  median 30.44 ns/call  min 30.24  max 36.76
    16 names  BTreeMap get_mut  median 40.51 ns/call  min 40.35  max 41.63
    16 names  the O(log n) map costs  +10.07 ns/call, +33%
=== run 2 ===
     1 names  HashMap  get_mut  median 30.35 ns/call  min 30.28  max 31.09
     1 names  BTreeMap get_mut  median 8.59 ns/call  min 8.54  max 9.19
     1 names  the O(log n) map costs  -21.77 ns/call, -72%
     4 names  HashMap  get_mut  median 31.23 ns/call  min 30.55  max 33.13
     4 names  BTreeMap get_mut  median 16.75 ns/call  min 16.64  max 17.26
     4 names  the O(log n) map costs  -14.48 ns/call, -46%
    16 names  HashMap  get_mut  median 30.87 ns/call  min 30.79  max 31.09
    16 names  BTreeMap get_mut  median 38.99 ns/call  min 38.63  max 42.70
    16 names  the O(log n) map costs  +8.12 ns/call, +26%
```

---

### RG15 — All Three Wait on a Consumer; One Exists, and It Already Answered Pending 3

Each Pending entry names the same class of settling evidence: "a consumer that
genuinely holds rings of two record types", "a consumer that formats registry
names into output where a newline or an empty string actually breaks something",
"a consumer that needs a stable listing". The deferral is well-formed — each
names a falsifiable condition rather than deferring indefinitely, which is
exactly what a Pending entry should do.

One consumer exists. `ring_factory` is the only manifest outside this crate
naming `ring_registry`, and it is not a shallow user: it calls seven of the
registry's eight methods across its source and tests — `contains`, `get_mut`,
`is_empty`, `len`, `names`, `register`, `remove`. Pending 1 has already consulted
it, and cites it correctly as evidence for *not* deciding: "`ring_factory`, the
only consumer, builds one ring at a time."

Pending 3 was not consulted, and the answer is there. Its own text predicts what
a consumer would do — "the cheaper answer would then be for the caller to sort,
and this entry exists so that answer is reached deliberately rather than by
someone changing the map type." At `factory_test.rs:344-346` the consumer
collects `names()` into a `Vec< &str >`, calls `sort_unstable()`, and compares
against a literal. The predicted answer was reached, deliberately, by the one
consumer, and the entry recording the prediction does not know it happened.

**Finding.** Recorded as a stale open question rather than a wrong one. Pending 3
can be closed on evidence that already exists: a consumer needed a stable
listing, sorted at the call site, and the map type did not have to change. The
repair is to move it into Closed with `factory_test.rs:344-346` as what settled
it. Pending 2 is worth a sentence at the same time — the same consumer passes
every name in from its own caller and never formats one into output, so its
settling condition is still genuinely unmet, and saying so is the difference
between an open question and an unexamined one.

---

### RG16 — Pending 3 Prices the Ordered Map Asymptotically, and the Asymptote Does Not Reach

The argument at `:105-108` is that an ordered map "costs `O(log n)` lookups
instead of `O(1)` on the operation that actually matters — retrieval. Paying that
on every `get_mut` to make an enumeration reproducible is the wrong way round."
The reasoning is sound as complexity analysis and names the right operation to
care about. What it does not do is check whether `n` ever gets large enough for
the classes to separate.

Measured on `get_mut` itself, `BTreeMap` is 8.6 ns against `HashMap`'s 30.4 at
one name — 72% faster — and 16.8 against 30.5 at four, 45% faster. The crossover
falls between 4 and 16; by 16 names the ordered map costs 26–33% more and the
entry's direction is correct. Both runs reproduce to within a few hundredths of a
nanosecond at every population.

The largest registry anything in this repository asserts on holds four. The
consumer's own doctest holds one. `HashMap`'s constant does not shrink with `n` —
it is a hash of the key string either way — while a `BTreeMap` at one entry is a
single comparison, which is why the gap is widest exactly where the registries
are.

**Finding.** Recorded as an argument correct in form and inverted at the
population in play — the same inversion
[RG4](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) records
against the source doc's version of the claim, extended here to the operation
this entry singles out and to the sizes the repository actually builds. The
repair is not to switch the map: `HashMap` is flat past the crossover, and a
registry that someday holds sixty names would want it. It is to say so — that the
choice buys headroom rather than present speed, and that below roughly a dozen
names the ordered map is the faster of the two on the very operation named as
deciding. That turns an argument a measurement contradicts into one it supports.

**Disposition:** applied — Pending 3's "Why it is currently unordered" paragraph
at `decisions/readme.md:105-108` keeps the `O(log n)`/`O(1)` complexity claim
verbatim but replaces the "wrong way round" conclusion with the measured
crossover: below roughly a dozen names `BTreeMap` is the faster of the two on
`get_mut`, so the choice buys headroom rather than present speed. `O(log n)` and
`O(1)` stay at line 106 unchanged, so
[RG13](001_four_closed_questions_and_the_one_measurement_none_took.md)'s "two
notations... at line 106" still holds.
Now prints: `is the faster of the two on`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/001`](001_four_closed_questions_and_the_one_measurement_none_took.md) | The four closed questions and their evidence types |
| [`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) | The same claim in the source, measured on `contains_key` |
| [`data_structure/002`](../data_structure/002_the_only_map_in_thirty_three_crates.md) | The `alloc` substitute the ordered map would also buy |
| [`api/001`](../api/001_the_registry_surface.md) | The eight methods, seven of which the consumer calls |
| [`integration/001`](../integration/001_one_declared_edge_of_three.md) | The edge the consumer comes in over |

### Sources

| Fact | Where |
|------|-------|
| The three Pending entries | `ring_registry/docs/decisions/readme.md:67`, `:83`, `:100` |
| Pending 3's asymptotic argument and its prediction | `ring_registry/docs/decisions/readme.md:105-113` |
| The sole consumer and its seven method calls | Census above |
| The consumer sorting `names()` | `ring_factory/tests/factory_test.rs:344-346` |
| `get_mut` at 1, 4 and 16 names | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `names_lists_every_live_name` | The iteration Pending 3 is about |
| `unusual_names_are_ordinary_names` | The behaviour Pending 2 would change |
| `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` | The `get_mut` the measurement is of |
