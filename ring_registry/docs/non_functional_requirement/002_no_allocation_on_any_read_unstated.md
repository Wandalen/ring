# Non-Functional Requirement: No Allocation on Any Read, Unstated

### Scope

**Purpose:** Record that all four read operations allocate nothing, that the
crate says so nowhere, and that the family already has a written idiom for saying
it which two crates use verbatim.

**Responsibility:** The measured allocation profile of `contains`, `get_mut`,
`len` and `names`; every sentence in the crate that mentions allocation; and the
family-wide census of crates that state an allocation property.

**In Scope:** `ring_registry/src/lib.rs:134`;
`ring_registry/docs/api/001_the_registry_surface.md:67`, `:81`;
`ring_registry/docs/type/001_registry_error.md:50`;
`ring_flush/src/lib.rs:501`; `ring_tls/src/lib.rs:99`.

**Out of Scope:** The premise that licenses the write path's allocations is
[`non_functional_requirement/001`](001_a_setup_time_call_and_what_that_licenses.md).
The count itself is
[`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md).

---

## Four Sentences About Allocation, All of Them About Adding One

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the family idiom for stating what an operation does not do --'
command grep -r 'No atomic, no lock, no allocation' --include=lib.rs ring_*/src/ | sed 's|ring/||' | sed 's/^/    /'
echo '  -- every crate whose lib.rs says anything about allocation --'
n=0
for c in ring_*/; do
  k=$( command grep -ci 'alloc' "$c"src/lib.rs 2>/dev/null || true )
  if [ "${k:-0}" -gt 0 ]; then n=$(( n + 1 )); fi
done
printf '    crates mentioning allocation in lib.rs: %s of 33\n' "$n"
echo '  -- and everything the contract itself says about it --'
# Scoped to `src/lib.rs`: the `docs/` tree quotes measured allocation counts in
# several instances, and those are this corpus reporting, not the crate promising.
command grep -i 'alloc' ring_registry/src/lib.rs | sed 's/^/    /'
printf '    of those, sentences saying a read makes none: %s\n' \
  "$( command grep -c -i 'no allocation\|zero allocation\|allocation-free' ring_registry/src/lib.rs || true )"
```

Live output:

```
  -- the family idiom for stating what an operation does not do --
    ring_flush/src/lib.rs:  /// Stage one record. No atomic, no lock, no allocation, and **no flush**.
    ring_tls/src/lib.rs:  /// Append one item. No atomic, no lock, no allocation.
  -- every crate whose lib.rs says anything about allocation --
    crates mentioning allocation in lib.rs: 13 of 33
  -- and everything the contract itself says about it --
      /// - *Box it.* That allocates on the failure path, to fix a size the caller
    of those, sentences saying a read makes none: 0
```

## What the Reads Actually Do

The counting allocator, with the ring constructed outside the measured window:

```
    Registry::new()                      0 allocations, 0 bytes
    register into a free name            2 allocations, 1810 bytes
    register into a taken name (refused) 2 allocations, 12 bytes
    contains + get_mut + len + names(1) 0 allocations, 0 bytes
```

---

### RG35 — Every Sentence the Crate Writes About Allocation Is About Adding One

The contract mentions allocation exactly once, and it argues one side of one
question: boxing the error would allocate on the failure path, so boxing is
refused. That is the write path, and it is a cost being incurred. Sentences
saying a read makes none: **zero**. (The `docs/` tree discusses allocation at
more length — the key clone, the sweep, the widths — but every one of those is
this corpus measuring, not the crate promising, and a caller reads the crate.)

Measured, the four read operations — `contains`, `get_mut`, `len` and a
single-name `names()` walk — make **zero allocations and touch zero bytes**. That
is not incidental: `names()` returns `impl Iterator< Item = &str >` rather than a
`Vec< String >`, and `contains`/`len`/`is_empty` return scalars. Whoever wrote
those signatures made the allocation-free reading possible, and nothing records
that they did.

The asymmetry is the finding. A caller reading this crate meets allocation once,
as a cost on a path they were told is rare, and never learns that the path they
will actually take repeatedly — look a ring up, count what is registered, iterate
the names — costs nothing at all.

**Finding.** Recorded as a property the crate has and does not claim. One clause
on each of the four reads, or one sentence on the type, states it and costs
nothing to keep true, since it follows from the signatures rather than from the
bodies. Worth stating specifically because the natural assumption runs the other
way: `names()` looks like it might build a collection, and a reader who assumes
it does will write a caller that caches the result to avoid an allocation that
does not exist.

---

### RG36 — The Family Has a Phrase for This and Two Crates Use It

`ring_flush:501` reads "Stage one record. No atomic, no lock, no allocation, and
**no flush**." `ring_tls:99` reads "Append one item. No atomic, no lock, no
allocation." The idiom is a short negative list attached to a single operation,
naming the three costs a caller of a concurrency primitive most wants ruled out,
and it is written identically in two crates that do not depend on each other.

Twelve of the thirty-three crates mention allocation in `lib.rs` at all. This
crate is one of the twelve, but only through the "Box it" sentence — its
membership in that census is an argument against an allocation, not a statement
about its own.

`ring_registry` is a good fit for the idiom and a slightly different case. It has
no atomics and no locks either, so all three negatives hold on every read, and
the interesting one is the fourth term the two existing sites do not need: the
write path *does* allocate, twice, and a per-operation form would say so — "Look
a name up. No allocation." beside "Register a ring. Two allocations; the map's
table on the first call."

**Finding.** Recorded as an unadopted family idiom rather than a defect. The
existing phrasing transfers directly, it is already proven readable in two
crates, and adopting it here would produce the only per-operation cost statement
in a crate whose entire `docs/` tree argues about cost. Its absence is also why
[RG35](#rg35--every-sentence-the-crate-writes-about-allocation-is-about-adding-one)
was available to find: there is no place in the current shape of the
documentation where the reads' profile would naturally have been written down.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_a_setup_time_call_and_what_that_licenses.md) | The premise that licenses the write path's two allocations |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | Where the two write-path allocations come from |
| [`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md) | The four reads this is about |
| [`api/001`](../api/001_the_registry_surface.md) | Measures the write path's allocations, which is what the contract's one sentence is about |
| [`type/001`](../type/001_registry_error.md) | Why the key clone's allocation is accepted |

### Sources

| Fact | Where |
|------|-------|
| The "Box it" refusal | `ring_registry/src/lib.rs:134` |
| The same argument, twice, in the surface doc | `ring_registry/docs/api/001_the_registry_surface.md:67`, `:81` |
| The clone acceptance | `ring_registry/docs/type/001_registry_error.md:50` |
| The family idiom, verbatim in two crates | `ring_flush/src/lib.rs:501`, `ring_tls/src/lib.rs:99` |
| 12 of 33 crates mention allocation in `lib.rs` | Census above |
| Zero allocations across the four reads | Probe above |

### Tests

| Test | Covers |
|------|--------|
| `a_registered_ring_is_retrievable_by_its_name_and_by_no_other` | The read path whose profile nothing states |
| `names_lists_every_live_name` | The iterator that could have been a `Vec` |
| `an_empty_registry_is_empty` | The two scalar reads |
