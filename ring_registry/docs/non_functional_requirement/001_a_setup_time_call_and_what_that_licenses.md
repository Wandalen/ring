# Non-Functional Requirement: A Setup-Time Call and What That Licenses

### Scope

**Purpose:** Identify the crate's single load-bearing performance premise — that
`register` is called once per ring and never in a loop — show that three separate
cost conclusions rest on it, and record that nothing in the repository states it
where a caller would look, tests it, or benchmarks it.

**Responsibility:** The premise at `src/lib.rs:140-141`; the three conclusions it
licenses; the family's benchmark coverage; and the measured numbers that exist
nowhere in the tree.

**In Scope:** `ring_registry/src/lib.rs:123`, `:140-141`;
`ring_registry/docs/type/001_registry_error.md:50`;
`ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md:58`;
`ring_bench/Cargo.toml`.

**Out of Scope:** The allocation profile is
[`non_functional_requirement/002`](002_no_allocation_on_any_read_unstated.md).
The individual measurements are
[`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) and
[`algorithm/002`](../algorithm/002_four_reads_and_the_order_they_do_not_promise.md).

---

## One Premise, Three Conclusions, No Bench

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the premise, and the heading it lives under --'
command grep '# The .result_large_err. allow\|setup-time call' ring_registry/src/lib.rs | sed 's/^/    /'
echo '  -- the three cost conclusions that rest on it --'
command grep 'the width is paid where it does not matter' ring_registry/src/lib.rs | sed 's|ring_registry/||' | sed 's/^/    /'
command grep 'not worth' ring_registry/docs/type/001_registry_error.md | sed 's/^/    type\/001:/'
command grep 'two hashes for one decision' ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md | sed 's/^/    pitfall\/001:/'
echo '  -- and which crates the family benchmarks --'
command grep -o 'ring_[a-z_]*' ring_bench/Cargo.toml | sort -u | tr '\n' ' ' | sed 's/^/    /'
printf '\n    ring_registry named in any bench source: %s\n' \
  "$( command grep -rc 'Registry' --include=*.rs ring_bench/ 2>/dev/null | awk -F: '{ s += $2 } END { print s + 0 }' )"
```

Live output:

```
  -- the premise, and the heading it lives under --
        /// # The `result_large_err` allow
        /// wide, including on the `Ok` path. `register` is a setup-time call — once
  -- the three cost conclusions that rest on it --
        /// per ring, never in a loop — so the width is paid where it does not matter.
    pitfall/001:the silent replace. It also performs two hashes for one decision on the path
  -- and which crates the family benchmarks --
    ring_bench ring_core ring_event ring_factory ring_flush ring_mpsc ring_slot ring_spsc ring_stats ring_tls ring_types 
    ring_registry named in any bench source: 0
```

## What the Premise Would Have Cost If It Were False

Every number the crate reasons about, measured — and none of them appears
anywhere in the repository:

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

```
    Registry::new()                      0 allocations, 0 bytes
    register into a free name            2 allocations, 1810 bytes
    register into a taken name (refused) 2 allocations, 12 bytes
    contains + get_mut + len + names(1) 0 allocations, 0 bytes
```

---

### RG33 — The Premise Three Decisions Rest On Is Stated Once, Under a Heading About a Lint

`register` is a setup-time call — once per ring, never in a loop. That single
claim is what makes the crate's three most argued cost decisions correct: a
448-byte `Result` is acceptable because the call is rare, a double allocation on
the refusal path is acceptable because the call is rare, and an `Entry` match
that [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md)
measures as the slower of the two candidate forms on that same path is acceptable
for the same reason. Remove the premise and all three conclusions need
re-deriving.

It is written down once, at `src/lib.rs:141`, inside a doc subsection headed
"`# The result_large_err allow`" — a place a reader arrives at only if they are
already investigating a clippy suppression. `register`'s summary line does not
carry it. The readme's operation table, which gives one row per method, does not
carry it. The crate's `docs/` tree restates the three conclusions in three
separate files and the premise in none of them.

**Finding.** Recorded as a load-bearing requirement filed under a heading about
something else. The repair is placement, not content: the sentence is correct and
well-phrased, and belongs in `register`'s own doc summary and in the readme's
table, where a caller deciding whether to call it in a loop would meet it. As it
stands, the one reader who most needs the constraint — someone registering rings
in a hot path — is the one least likely to be reading about a lint allow.

---

### RG34 — The Family Benchmarks Nine Crates and This Is Not One of Them

`ring_bench` declares nine `ring_*` dependencies and names `Registry` zero times
in any of its sources. So the crate that argues hardest about cost in the family
— twenty lines on a `Result`'s width, a paragraph on a clone, a paragraph on a
hash count — is outside the only measurement apparatus the family has.

Nothing about that is unreasonable on its own: a setup-time call is a poor
benchmark subject, which is exactly what RG33's premise says. The consequence is
that the premise is unfalsifiable in place. There is no artefact in the repository
that would notice if `register` became expensive, if the `Entry` form's margin
grew, or if someone did start calling it in a loop; the only thing standing
between the crate and a wrong cost model is that nobody has changed the code.

Measured here for the first time, the numbers are undramatic and support the
premise's *conclusion* while contradicting one of its supporting arguments:
`register` refuses in 113 ns, allocates 1810 bytes the first time it succeeds and
12 on a refusal, and the reads are 24–29 ns and allocation-free. Nothing in that
set needs the "once per ring" constraint to be acceptable — a thousand
registrations in a loop would cost about 113 microseconds.

**Finding.** Recorded as a measurement gap, not a performance problem. The
premise turns out to be more conservative than it needs to be, which is the
better direction to be wrong in, and the cheap repair is to write the four
numbers into the crate's own docs so the argument stops being purely deductive. A
`ring_bench` case would be better still and is harder to justify, since what it
would guard against is a change nobody plans to make — worth deferring, worth
naming as deferred rather than leaving as an unremarked absence.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_no_allocation_on_any_read_unstated.md) | The other unstated profile, and the family idiom for stating it |
| [`algorithm/001`](../algorithm/001_two_branches_and_what_the_refusal_costs.md) | The two conclusions measured, one of them backwards |
| [`data_structure/001`](../data_structure/001_forty_eight_bytes_that_do_not_move.md) | The 448 bytes the premise licenses |
| [`item/001`](../item/001_two_lints_one_allow_and_the_reason_beside_it.md) | The lint whose heading the premise lives under |
| [`decisions/001`](../decisions/001_four_closed_questions_and_the_one_measurement_none_took.md) | The four closed questions, all decided without a number |

### Sources

| Fact | Where |
|------|-------|
| The heading the premise lives under | `ring_registry/src/lib.rs:123` |
| "a setup-time call — once per ring, never in a loop" | `ring_registry/src/lib.rs:140-141` |
| The clone conclusion | `ring_registry/docs/type/001_registry_error.md:50` |
| The hash-count conclusion | `ring_registry/docs/pitfall/001_insert_would_have_replaced_silently.md:58` |
| Nine benchmarked crates, `Registry` named zero times | Census above |
| 113 ns per refusal, 24–29 ns per read | Probe above, two runs |

### Tests

| Test | Covers |
|------|--------|
| `a_second_registration_under_a_live_name_is_refused` | The path the cost argument is about |
| `two_names_hold_two_distinct_rings` | More than one registration, still not a loop |
| `names_lists_every_live_name` | The read whose cost nothing states |
