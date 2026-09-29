# Five Public Ordering Constants

### Scope

- **Purpose**: Record that this crate exports its memory-ordering vocabulary as five public constants, state what each names, and measure who reads them.
- **Responsibility**: The five constants, their values, why they are public, and their reach.
- **In Scope**: `PUBLISH`, `OBSERVE`, `COMMIT`, `OWN`, `UNSTAMPED`.
- **Out of Scope**: The invariant the first four participate in (→ [`../invariant/002`](../invariant/002_publication_ordering.md)); the stamp array they order access to (→ [`../data_structure/002`](../data_structure/002_a_second_array_of_sequence_stamps.md)).

### The Five

| Constant | Value | Names |
|----------|-------|-------|
| `PUBLISH` | `Ordering::Release` | The store that makes a slot's payload visible |
| `OBSERVE` | `Ordering::Acquire` | The load that sees it — the other half of `PUBLISH` |
| `COMMIT` | `Ordering::Release` | The consumer's release of drained slots |
| `OWN` | `Ordering::Relaxed` | The accesses needing no edge at all |
| `UNSTAMPED` | `Seq( u64::MAX )` | The stamp value no valid sequence can equal |

**Four orderings and one sentinel, and they are public for the same reason**:
each is a claim about correctness that a reader should be able to check without
reading the code that uses it. Each carries a doc test asserting its value, so
`PUBLISH == Ordering::Release` is verified by the test suite rather than
asserted in prose.

### Who Reads Them

```sh
cd "$(git rev-parse --show-toplevel)"
for k in PUBLISH OBSERVE COMMIT OWN UNSTAMPED; do
  printf '%-10s outside this crate: %d   inside: %d\n' "$k" \
    "$( grep -rhoE "ring_mpsc::$k" ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null | wc -l )" \
    "$( grep -c "$k" ring_mpsc/src/lib.rs )"
done
```

Live output:

```
PUBLISH    outside this crate: 1   inside: 5
OBSERVE    outside this crate: 1   inside: 8
COMMIT     outside this crate: 1   inside: 4
OWN        outside this crate: 1   inside: 3
UNSTAMPED  outside this crate: 5   inside: 9
```

**No crate outside this one names any of them.** They are read by this crate's
own code, by its own tests, and by the four doc tests that pin their values.

### What That Means

A vocabulary published for readers rather than for callers. That is a
defensible reason to export a constant — `the_orderings_are_the_ones_the_publication_invariant_names`
is a real test and it needs the names to be reachable — but it means the five
are documentation with a compiler check attached, not API.

The distinction matters if this crate is ever pared down: removing an unused
public function costs nothing, and removing one of these costs the ability to
state the invariant in checkable form.

### MP29 — Each Constant Is Pinned by Its Own Doc Test

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -c 'assert_eq!( ring_mpsc::' src/lib.rs
```

Live output:

```
5
```

Five assertions in five doc comments. **This is the mechanism that makes
publishing them worthwhile**: a constant nobody imports is documentation, and a
documented constant with an assertion is documentation that cannot go stale
silently.

It is also the reason `invariant/002`'s value-level check
(→ MP24) has anywhere to stand.

### MP30 — The Constants Are Named for Roles, Not for Orderings

`PUBLISH` and `COMMIT` are both `Ordering::Release`. Naming them separately
means a later change to one does not silently change the other, and it means the
call sites read as intent rather than as mechanism.

**The cost is that the file has two names for one value**, which reads as
redundancy to anyone who has not seen the pairing table in
`invariant/002`.
