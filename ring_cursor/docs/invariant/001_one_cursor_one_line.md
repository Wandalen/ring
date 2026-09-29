# Invariant: One Cursor, One Line

### Scope

- **Purpose**: State the three-clause restriction this crate is graded on, show why each clause is satisfiable while the feature fails, and list what the clauses do not reach.
- **Responsibility**: Give each clause with the test that enforces it, and separate the enforced part of the invariant from the part nothing checks.
- **In Scope**: `align_of`, `size_of`, and the two-field gap; the reached-test for this invariant.
- **Out of Scope**: Where the `64` comes from, which is [`invariant/002`](002_the_number_64_never_appears_here.md); whether the separation makes anything faster, which belongs to `ring_bench`.

### The Restriction

Stated in `bench_harness/docs/acceptance/001_feature_reached_tests.md` as
three clauses:

| Clause | Statement | Test |
|:------:|-----------|------|
| 1 | `align_of::< PaddedCursor >() == 64` | `a_padded_cursor_occupies_exactly_one_cache_line` |
| 2 | `size_of::< PaddedCursor >() == 64` | the same test |
| 3 | Two `PaddedCursor` in one struct sit at least 64 bytes apart | `two_cursors_in_one_struct_are_at_least_a_line_apart` |

The whole file runs, so the three clauses are shown in their context rather than
filtered to themselves. Test threads finish in a racing order and cargo's build
line carries a timing, so the names are sorted and the timing dropped:

```sh
cd "$(git rev-parse --show-toplevel)"
out=$( cargo test -p ring_cursor --test cursor_test 2>&1 )
printf '%s\n' "$out" | command grep -E '^test .+\.\.\. ' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test a_capacity_of_one_still_has_room_for_one ... ok
test a_consumer_moving_on_reopens_the_ring ... ok
test a_cursor_holds_the_sequence_it_was_built_with ... ok
test a_cursor_is_shared_by_reference_not_by_copy ... ok
test a_fresh_pair_has_the_whole_ring_free ... ok
test a_padded_cursor_is_its_atomic_and_nothing_else ... ok
test a_padded_cursor_occupies_exactly_one_cache_line ... ok
test a_pair_is_two_lines_plus_its_capacity ... ok
test an_array_of_cursors_gives_each_its_own_line ... ok
test every_cursor_starts_on_a_line_boundary ... ok
test exactly_one_lap_ahead_is_full_and_one_less_is_not ... ok
test free_slots_falls_as_the_producer_advances ... ok
test many_producers_on_one_cursor_lose_nothing ... ok
test may_claim_and_free_slots_never_disagree ... ok
test padding_does_not_change_what_the_cell_does ... ok
test pending_ignores_capacity_and_free_slots_does_not ... ok
test pending_is_the_distance_the_consumer_still_has_to_travel ... ok
test the_gap_survives_the_pair_being_moved ... ok
test the_pair_reads_both_cursors_for_every_reading ... ok
test two_cursors_in_one_struct_are_at_least_a_line_apart ... ok
test two_threads_advancing_two_cursors_do_not_lose_writes ... ok
test writing_one_cursor_leaves_the_other_alone ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

**Three of those twenty-two are the clauses above** —
`a_padded_cursor_occupies_exactly_one_cache_line` carries the first two,
`two_cursors_in_one_struct_are_at_least_a_line_apart` the third.

### Why Three Clauses and Not One

Each of the first two is satisfiable while the feature fails:

| Counter-example | Clause 1 | Clause 2 | What happens |
|-----------------|:--------:|:--------:|--------------|
| size 8, `align_of` 64 | ✅ | ❌ | Two in an array land **8 bytes apart** — alignment says where a value may start, not how much room it occupies |
| size 64, `align_of` 8 | ❌ | ✅ | The value can start at offset 8 and **straddle two lines**, sharing both |

**Only the conjunction says "one per line".** `#[ repr( align( 64 ) ) ]` supplies
both — it rounds the size up as well as constraining the start — so in practice
the two always hold together. They are named separately because a future layout
change could break the second while leaving the first intact, and the first
alone reads like the feature.

### Clause 3 Is a Different Kind of Statement

Clauses 1 and 2 are about a *type*. Clause 3 is about two real fields at two real
addresses, and it is the one that would notice a future `CursorPair` layout that
packed the cursors together despite each still measuring 64 bytes on its own.

`size_of` is a promise about a type; two fields being 64 bytes apart is the fact
the promise was made about.

Five tests carry the invariant between them:

| Test | Adds |
|------|------|
| `a_padded_cursor_occupies_exactly_one_cache_line` | Clauses 1 and 2, plus a third assertion pinning `64` to `CACHE_LINE` rather than to a literal |
| `two_cursors_in_one_struct_are_at_least_a_line_apart` | Clause 3, in both the subtraction and the division form |
| `every_cursor_starts_on_a_line_boundary` | That each address is `≡ 0 (mod 64)` — stronger than "apart" |
| `an_array_of_cursors_gives_each_its_own_line` | The **stride**, which clause 1 alone would not give |
| `the_gap_survives_the_pair_being_moved` | That a heap move preserves it — cannot fail, and says so |

**`an_array_of_cursors_gives_each_its_own_line` is the load-bearing one.** It is
the test that fails for the "size 8, align 64" counter-example, and the array case
is what `ring_gating::GatingSet` actually holds — a `Vec< PaddedCursor >` read on
every producer claim.

### What the Invariant Does Not Reach

| # | Not covered | Instrument that would |
|---|-------------|-----------------------|
| T1 | That the padding stays *empty* — a field added inside it leaves `size_of` at 64 | `tests/manual/readme.md` M2, a source reading |
| T2 | That the `64` is `ring_align::CACHE_LINE` and not an independent literal | M1 — and see [`invariant/002`](002_the_number_64_never_appears_here.md) |
| T3 | That two cursors in *different allocations* are separated | Nothing, and nothing should — that is allocator placement, not a property of the type |
| T4 | That the separation makes anything faster | `ring_bench` — a measurement under contention, not a unit test |
| T5 | That the cache line on the target machine is actually 64 bytes | Nothing in this crate. `ring_align::CACHE_LINE` owns that question, and its own docs record a 128-byte-line port as the failure case |

**T4 is deliberately excluded and the test file says so.** Asserting a timing
here would produce a test that fails on a loaded CI box for reasons that have
nothing to do with the code. What this suite establishes is that the layout the
measurement will be taken on is actually the layout claimed.

**T5 is the one that would invalidate the whole invariant silently.** On a
machine with 128-byte lines, all five tests pass — they assert against
`CACHE_LINE`, which would still be 64 — and two cursors 64 bytes apart share a
line. The clauses are internally consistent and externally wrong. That failure is
`ring_align`'s to own, and is recorded in
[its `pitfall/001`](../../../ring_align/docs/pitfall/001_a_constant_too_small_buys_nothing.md).

### CU21 — Two Tests in Twenty-Two Carry the Invariant

```
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

The whole-file run is quoted rather than a filtered one so the ratio stays
visible: two of the twenty-two tests carry all three clauses of the invariant.

**Finding.** The property this crate exists for is one-eleventh of what its suite
spends its time on, and the other twenty tests are about arithmetic this crate
delegates. That is not a coverage complaint — the layout claim needs exactly the
assertions it has — but it does mean the suite's size is no evidence for the
invariant's protection.

---

### CU22 — Clause 2 Cannot Tell an Inherited Alignment From a Restated One

| Clause | Survives `#[ repr( align( 64 ) ) ]` written directly? |
|--------|---|
| 1 — `align_of == CACHE_LINE` | yes |
| 2 — `size_of == CACHE_LINE` | yes |
| 3 — two cursors land on distinct lines | yes |

Every clause passes for a `PaddedCursor` that declares its own alignment instead
of inheriting one, because all three measure the outcome rather than the route.

**Finding.** No clause distinguishes the two, which is the honest statement of
what this invariant's tests protect: the *layout*, not the *single source*. The
single source is protected by the grep in
[`002`](002_the_number_64_never_appears_here.md) and by `udeps` at level 4 — both
outside the suite that runs on every change.

**Disposition:** declined — the single-source protection this finding asks
for already exists: `invariant/002`'s own M1 recipe (`command grep -nE
"\b64\b" ring_cursor/src/lib.rs`, filtered to non-doc lines) catches
exactly the `#[ repr( align( 64 ) ) ]` literal a forked declaration would
introduce, backed by `udeps` at verification level 4. Duplicating that check
inside this file's clause-based layout suite would blur the boundary the
corpus already draws on purpose — clauses 1-3 test the *layout*,
`invariant/002` tests the *single source* — which this instance's own
Out-of-Scope line and `invariant/002`'s cross-reference both already assign
correctly; nothing in this crate's `src/` to change for it.

---

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The layout the clauses measure |
| [../data_structure/002_the_cursor_pair.md](../data_structure/002_the_cursor_pair.md) | The structure clause 3 is about, and why field order does not matter |

### Invariants

| File | Relationship |
|------|--------------|
| [002_the_number_64_never_appears_here.md](002_the_number_64_never_appears_here.md) | T2 — the restriction this one cannot see |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_the_layout_claim_is_testable.md](../non_functional_requirement/001_the_layout_claim_is_testable.md) | The verification layers behind these five tests |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_obvious_implementation_forks_the_constant.md](../pitfall/001_the_obvious_implementation_forks_the_constant.md) | The reimplementation that satisfies all three clauses and breaks T2 |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_padded_cursor.md](../type/002_padded_cursor.md) | The same restriction stated as promises P1 and P2 |

### Sources

| File | Relationship |
|------|--------------|
| `bench_harness/docs/acceptance/001_feature_reached_tests.md` | Where the three clauses are stated |
| `ring_cursor/src/lib.rs:23-36` | The crate's own argument for asserting both size and alignment |
| `ring_align/src/lib.rs` | `CACHE_LINE`, which all three clauses are ultimately measured against |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:1-31` | The suite's own statement of why three clauses and what is excluded |
| `tests/cursor_test.rs:54-72` | Clauses 1 and 2 |
| `tests/cursor_test.rs:74-88` | Clause 3 |
| `tests/cursor_test.rs:102-111` | Every cursor on a line boundary |
| `tests/cursor_test.rs:113-126` | The array stride |
| `tests/manual/readme.md` M2 | T1 |
