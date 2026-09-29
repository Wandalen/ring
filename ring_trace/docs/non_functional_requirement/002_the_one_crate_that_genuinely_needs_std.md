# Non-Functional Requirement: The One Crate That Genuinely Needs `std`

### Scope

**Purpose:** Establish that this crate's dependence on `std` is structural rather
than incidental, unlike most of the family's, and record that the family declares
a portability posture on three of its 33 crates and nowhere else, despite 25 of
them touching nothing from `std` at all.

**Responsibility:** The per-crate census of what each `ring_*` crate reaches for
under `std::`, which of those items are `alloc` re-exports, and what the family
declares about `no_std` in source and in manifests.

**In Scope:** `ring_trace/src/lib.rs:277-280`; every `ring_*`
`src/lib.rs` and `Cargo.toml`.

**Out of Scope:** Why the crate holds a lock at all is
[`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md). The
self-imposed cost requirement is
[`non_functional_requirement/001`](001_zero_when_not_is_a_count_not_a_cost.md).

---

## What Eight Crates Reach For

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what each crate that touches std reaches for --'
n=0
for c in ring_*/; do
  # a crate declaring `no_std` cannot be using std, so its only match is the bare
  # `std::` of a comment explaining the attribute — counted below instead, since
  # leaving it here reports a dependency that the declaration rules out
  if command grep -q 'no_std' "$c"src/lib.rs 2>/dev/null; then continue; fi
  hits=$( command grep -o 'std::[a-z_]*::[A-Za-z_]*\|std::[a-z_]*' "$c"src/lib.rs 2>/dev/null | sort -u | tr '\n' ' ' )
  if [ -n "$hits" ]; then n=$(( n + 1 )); printf '    %-15s %s\n' "$( basename "$c" )" "$hits"; fi
done
printf '    crates touching std: %s   touching nothing from it: %s\n' "$n" "$(( 33 - n ))"
echo '  -- and what the family declares about portability --'
d=0; f=0; declarers=''
for c in ring_*/; do
  if command grep -q 'no_std' "$c"src/lib.rs 2>/dev/null; then
    d=$(( d + 1 )); declarers="$declarers $( basename "$c" )"
  fi
  if command grep -q '^std *=\|"std"' "$c"Cargo.toml 2>/dev/null; then f=$(( f + 1 )); fi
done
printf '    ring_* crates declaring no_std in source: %s of 33 —%s\n' "$d" "$declarers"
printf '    ring_* manifests carrying a std feature: %s of 33\n' "$f"
```

Live output:

```
  -- what each crate that touches std reaches for --
    ring_bench      std::collections::VecDeque std::sync::atomic std::sync::Mutex std::sync::PoisonError std::thread::scope std::time::Instant 
    ring_mpsc       std::thread::scope 
    ring_registry   std::collections::hash_map std::collections::HashMap 
    ring_spsc       std::thread::scope std::thread::yield_now 
    ring_testkit    std::thread::spawn 
    ring_tls        std::vec::Drain 
    ring_trace      std::sync::Mutex std::sync::MutexGuard std::sync::PoisonError 
    ring_wait       std::thread::sleep std::thread::yield_now std::time::Duration 
    crates touching std: 8   touching nothing from it: 25
  -- and what the family declares about portability --
    ring_* crates declaring no_std in source: 3 of 33 — ring_overflow ring_stats ring_types
    ring_* manifests carrying a std feature: 0 of 33
```

**Correction (2026-09-20):** the census above used to print `crates touching std:
10   touching nothing from it: 23`, contradicting the 8-and-25 this file states in
six places — including the enumeration in TR35 below, which names the eight and
divides them cleanly. The prose was right and the census over-counted: it listed
`ring_overflow` and `ring_types` with a bare `std::` and no path after it, which is
the comment each of those crates carries *explaining* its own `#![ no_std ]`, not a
dependency a `no_std` crate could have. The loop now skips a crate that declares the
attribute rather than filtering comments, since a comment filter would also drop
`ring_mpsc`, `ring_spsc` and `ring_testkit`, whose only `std::` sits in a doctest.
The second loop gained the three names it had been counting silently.

## Which of Those Are `alloc` in Disguise

*This probe's binary no longer exists to re-run — `-tr_probe/` scratch builds
are swept per this project's convention for temporary files. What it
demonstrates is a stable fact about the standard library
(`alloc::vec::Drain` and `std::vec::Drain` are the same type) plus a census of
this crate's own imports, and both remain true today: `ring_trace` still
reaches only `std::sync::Mutex`, `MutexGuard` and `PoisonError`, none of which
has an `alloc` or `core` path. Read the recording as frozen supporting
evidence for TR35 below, not a live check.*

```rust
// -tr_probe/src/bin/alloc_reach.rs
// If `std::vec::Drain` is `alloc::vec::Drain` re-exported, this compiles with
// no conversion at all — so `ring_tls`'s one std use is not a std dependency.
fn drain_is_the_same_type( d : alloc::vec::Drain< '_, u8 > ) -> std::vec::Drain< '_, u8 >
{
  d
}
```

```
  alloc::vec::Drain and std::vec::Drain are one type: [1, 2, 3]
  Vec needs only alloc: capacity 0
  ring_trace's three std items all live under std::sync:
    Mutex, MutexGuard, PoisonError — no alloc or core path exists for them
```

---

### TR35 — Three Items From `std::sync`, and None of Them Has an `alloc` Path

Eight of the 33 crates reach for anything under `std::`, and the eight divide
cleanly. Four reach for threads — `ring_mpsc`, `ring_spsc`, `ring_testkit` and
`ring_wait` take `scope`, `spawn`, `yield_now` and `sleep`, all of which appear in
concurrency helpers rather than in the data structures themselves.
`ring_registry` takes `HashMap`. `ring_bench` takes everything, which is what a
benchmark harness is for. `ring_tls` takes exactly one item, `std::vec::Drain`,
and the probe shows that is `alloc::vec::Drain` re-exported — the same type, no
conversion needed, so the crate is not tied to `std` at all.

`ring_trace` is the remaining one, and its three items are `Mutex`, `MutexGuard`
and `PoisonError`, all under `std::sync`, none of which exists in `core` or
`alloc`. The log's own `Vec` would be fine — the probe takes one straight from
`alloc` — so the entire `std` dependency is the lock and its poisoning API.

That makes this the family's only library crate whose `std` use is structural.
The crate does not say so. It says a great deal about the lock, in a section
arguing that a `Mutex` is the right cost, and none of it mentions that choosing
the lock is also what fixed the crate's portability floor.

**Finding.** The same decision produced both of the crate's exceptions — the only
lock-without-atomic crate in a lock-free family
([`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md)) and the only
library crate that genuinely cannot be `no_std` — and neither document mentions
the other. One clause in the existing `Mutex` argument closes it: the lock is also
why this crate needs `std`, and there is no `core` or `alloc` substitute for
`Mutex`, so the two consequences are one decision and should be read together.

---

### TR36 — Twenty-Five Crates Touch Nothing From `std` and Three of Them Say So

Twenty-five of the 33 crates reference nothing under `std::` — not a thread, not
a collection, not a clock. `Vec` is available from `alloc`, as the probe shows, so
even the crates that allocate are not thereby tied to `std`. And only three crates
declare `#![ no_std ]` — `ring_types`, `ring_stats` and `ring_overflow` — three of
33 in source, with zero of 33 manifests carrying a `std` feature to gate it
behind.

So the family has a portability posture and has stated it on three crates out of
the 25 that hold it. Three attributes say "this crate is `no_std` and the compiler
will hold it there"; nothing anywhere says what the other 22 are, and no manifest
carries a `std` feature to gate the question behind. The current state is "we do
not care" by default on 22 crates and looks like the first by construction on all
25, which is the worst combination: a reader auditing the family for embedded use
finds 25 crates that would compile under `no_std` today and a signal on three of
them.

**Finding.** Recorded as a partly-adopted capability rather than a defect —
nothing here is broken and no consumer has asked for `no_std`. What makes it worth
writing down is the ratio and the cost. Twenty-two crates are one attribute line
away from the posture three of their siblings already carry; adding it makes the
capability real, because the compiler then holds it, and it turns this crate's
`std` dependence from an invisible fact into a stated exception with a reason.
Nothing enforces the property on those 22 today, so the first of them to reach for
`std::thread` in a data structure will do it silently.

**Correction (2026-09-20):** these two paragraphs read "has never stated it in
either direction", "no signal that any of them is expected to stay that way" and
"Twenty-five crates are one attribute line away", while the paragraph directly
above them — and `definition/readme.md`'s own TR36 row — already said three crates
declare `#![ no_std ]`. The finding is unchanged in substance: the capability is
unadopted on the crates that have not declared it. What changed is the count it
applies to, 22 rather than 25, and that "nothing enforces it" is now true of those
22 specifically rather than of the family.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](../decisions/002_a_mutex_over_a_lock_free_log.md) | The decision that set this floor |
| [`non_functional_requirement/001`](001_zero_when_not_is_a_count_not_a_cost.md) | The other requirement the crate carries |
| [`pattern/001`](../pattern/001_one_accessor_for_five_lock_sites.md) | The API these three `std` items are used through |
| [`data_structure/001`](../data_structure/001_forty_bytes_a_flag_and_a_vector_that_never_allocated.md) | The `Vec` that needs only `alloc` |

### Sources

| Fact | Where |
|------|-------|
| Per-crate `std::` census, 8 of 33 | Census above |
| The crate's three `std::sync` items | `ring_trace/src/lib.rs:277-280` |
| `std::vec::Drain` identical to `alloc::vec::Drain` | Probe above |
| `Vec` available from `alloc` | Probe above |
| Three `no_std` declarations, zero `std` features | Census above |

### Tests

| Test | Covers |
|------|--------|
| `concurrent_recorders_lose_no_entry` | The lock these three items implement |
| `a_disabled_trace_stays_empty_under_contention` | The same, with the lock never taken |
| `an_enabled_trace_records_exactly_one_entry_per_operation` | The `Vec` that needs only `alloc` |
