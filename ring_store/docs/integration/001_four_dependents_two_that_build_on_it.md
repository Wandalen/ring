# Integration: Four Dependents, Two That Build On It

### Scope

**Purpose:** Record who takes this crate and in which manifest section, that the
split is between two rings that build on it and two crates that only test
against it, and how much of the surface the two real consumers actually reach.

**Responsibility:** The crate's outward dependency edges — who declares them,
what kind they are, and which functions they exercise.

**In Scope:** `ring_*/Cargo.toml`; the `use ring_store::` sites;
the twelve-function surface measured against its callers.

**Out of Scope:** *How* the two real consumers reach a slot — through an
`UnsafeCell` and twelve `unsafe` blocks — is
[`integration/002`](002_every_unsafe_block_in_the_family.md). The borrow
discipline that arrangement discards is
[`pattern/002`](../pattern/002_every_write_is_a_borrow.md).

---

### BF1 — Four Manifests Name It, and Two of Them Only for Tests

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: the crate list is sorted first, so the awk output follows it
for c in $( ls -d ring_*/ | sed 's|ring/||;s|/||' | sort ); do
  awk -v c="$c" '/^\[dependencies\]/{s="dependencies"} /^\[dev-dependencies\]/{s="dev-dependencies"} /ring_store =/{printf "  %-10s %s\n", c, s}' ring/$c/Cargo.toml
done
```

Live output:

```
  ring_event dev-dependencies
  ring_mpsc  dependencies
  ring_spsc  dependencies
  ring_tls   dev-dependencies
```

Two ordinary dependencies and two dev-dependencies. The dev-dependency edges are
not vestigial — both crates import the type in a test file:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -r 'use ring_store::' ring_event/tests/*.rs ring_tls/tests/*.rs \
  ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | sed 's|^ring/||' | sort
```

Live output:

```
ring_event/tests/event_test.rs:use ring_store::Buffer;
ring_mpsc/src/lib.rs:use ring_store::Buffer;
ring_spsc/src/lib.rs:use ring_store::Buffer;
ring_tls/tests/tls_test.rs:use ring_store::Buffer;
```

**Finding.** The four edges are two different relationships wearing one name. A
`grep` for `ring_store` across the manifests reports four dependents; the build
graph has two. `ring_mpsc` and `ring_spsc` are built out of this crate —
`Buffer< S >` is a field of each one's `Ring`. `ring_event` and `ring_tls` never
name the type outside `tests/`, and take the dependency so their own tests run
over real storage instead of a hand-rolled array.

That second arrangement is the family's no-mocking rule showing up in the build
graph rather than in a test body, and it is the right shape: `ring_event`'s
translators fill a slot, and a test that filled a stand-in slot would be
asserting against a fixture rather than against the thing. What it costs is that
the crate's apparent reach is double its real one, and neither the crate's
`readme.md` nor its module comment distinguishes the two kinds of dependent.

---

### BF2 — The Two Real Consumers Are Both Tier 6, and Nothing Between Touches It

`ring_store` is Tier 2. Its two ordinary dependents are `ring_mpsc` and
`ring_spsc`, the two Tier 6 rings at the top of the family. Every tier in
between — the cursor at 3, the gating and waiting at 4, the claim/publish/
consume/barrier layer at 5 — is built without reference to storage at all.

That is the shape a storage-only tier is meant to have, and it is worth stating as an achieved
property rather than an intention. The write-path protocol crates reason about
sequences; only the two crates that own an allocation reason about slots.

**Finding.** The storage primitive is used at exactly one place in the family's
depth — the top — and skipped by four intermediate tiers. The consequence is
that a change to `Buffer`'s surface cannot break the protocol layer, because the
protocol layer does not know it exists. The corresponding cost is that
`ring_store` gets no exercise from those four tiers either: the crates that
would most plausibly reveal an awkward storage API never call it.

---

### BF3 — Of Twelve Public Functions, the Real Consumers Call Three

```sh
cd "$(git rev-parse --show-toplevel)"
for m in new clear all_empty capacity len is_empty get get_mut at at_mut iter iter_mut; do
  printf '  %-10s %s\n' "$m" "$( grep -rho "slots\.get() )\.$m(\|Buffer::$m(" ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | wc -l )"
done
```

Live output:

```
  new        2
  clear      0
  all_empty  0
  capacity   0
  len        0
  is_empty   0
  get        0
  get_mut    0
  at         0
  at_mut     0
  iter       0
  iter_mut   0
```

Three functions, each called once per real consumer: construct it, read a slot
by sequence, write a slot by sequence. The other nine have no caller outside
this crate's own tests and doctests.

**Finding.** Three quarters of the surface exists for readers, tests and
possibilities rather than for callers. That is not automatically a defect —
`capacity()` and `len()` are the kind of accessor a container owes its users
whether or not this particular family wants them today, and `clear` has a stated
future consumer ([`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md)
finds that consumer does not depend on this crate). But it changes what the test
suite is for. Nine of twelve functions are covered only by tests written
alongside them, so a regression in `iter_mut` or `all_empty` would be caught by
this crate's suite and by nothing downstream, and a change in what they *should*
do has no caller to argue with.

The three that are used are used identically by both consumers, through the same
`UnsafeCell` indirection, in the same two-function wrapper —
[`integration/002`](002_every_unsafe_block_in_the_family.md) walks it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_every_unsafe_block_in_the_family.md) | How the two real consumers reach the three functions they use |
| [`pattern/002`](../pattern/002_every_write_is_a_borrow.md) | The `&`/`&mut` lattice those consumers bypass |
| [`lifecycle/002`](../lifecycle/002_the_sweep_nothing_calls.md) | `clear`, one of the nine with no caller, and the crate its doc names |
| [`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) | The full surface these three are drawn from |
| [`api/002`](../api/002_six_ways_to_reach_a_slot.md) | The nine functions no consumer calls, one pair at a time |

### Sources

| Fact | Where |
|------|-------|
| Manifest sections | `ring_event/Cargo.toml:13`, `ring_tls/Cargo.toml:14`, `ring_mpsc/Cargo.toml:14`, `ring_spsc/Cargo.toml:9` |
| The four import sites | `ring_{event,mpsc,spsc,tls}` — quoted above |
| The twelve-function surface | `ring_store/src/lib.rs:88-278` |
| Consumer call counts | Census over `ring_mpsc` and `ring_spsc`, quoted above |
| This crate's storage-only scope | `ring_store/docs/readme.md:3-4` |

### Tests

| Test | Covers |
|------|--------|
| `exactly_n_slots_are_allocated_for_capacity_n` | `new`, `len`, `capacity`, `iter` — three of them called nowhere else |
| `a_sequence_addresses_the_slot_ring_index_says_it_does` | `at`/`at_mut`, the two the consumers use |
| `ring_event` — `event_test.rs` | The dev-dependency edge, driving translators over real storage |
| `ring_tls` — `tls_test.rs` | The other dev-dependency edge |
| *(to create)* | A downstream assertion that `all_empty` still folds correctly — today only this crate checks |
