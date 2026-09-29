# Pattern: Exhaustive Match as a Tripwire

### Scope

**Purpose:** Place this crate's use of an exhaustive `match` as a compile-time
guard beside the family's four other treatments of compile-time claims, and
record where the same idiom is given away by a wildcard arm.

**Responsibility:** The rationale `name()` states, the six places in the family
that make a compile-time claim, how `ring_flush`, `ring_poll`, `ring_overflow`
and `ring_slot` each discharge theirs, and the three wildcard arms in the
family's enum matches.

**In Scope:** `ring_trace/src/lib.rs:98-100`, `:109-116`;
`ring_flush/tests/flush_test.rs:1018-1024`;
`ring_poll/tests/poll_test.rs:21-25`;
`ring_overflow/src/lib.rs:82-87`; `ring_slot/src/lib.rs:160-167`;
`ring_core/src/lib.rs:394`; `ring_flush/src/lib.rs:220-240`, `:587-608`.

**Out of Scope:** That this crate's *test* states the claim wrongly is
[`item/002`](../item/002_traceop_all_and_the_tripwire_that_is_not_one.md). The
runtime pattern is
[`pattern/001`](001_one_accessor_for_five_lock_sites.md).

---

## Four Ways to Make a Compile-Time Claim

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the tripwire, and the rationale the crate gives for it --'
command grep -m1 -A4 -F '  /// This operation'"'"'s name.' ring_trace/src/lib.rs | tail -n 4
echo '  -- who else in the family states a compile-time claim --'
command grep -rn 'fails to compile' --include=*.rs ring_*/src/ ring_*/tests/ \
  | command grep -o '^[^:]*:[0-9]*:' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- ring_flush measured its claim and quoted the compiler --'
command grep -m1 -A6 -F '/// is not expressible on stable Rust. The second was **measured** rather than' ring_flush/tests/flush_test.rs
echo '  -- ring_poll declined to make one, and said why --'
command grep -m1 -A4 -F '//! **No test that a parking call fails to compile.** It would not fail to' ring_poll/tests/poll_test.rs
echo '  -- and where a wildcard arm turns a variant addition into a silent default --'
# Comment lines excluded (`grep -vE '^\s*(//|///|//!)'`, the same guard
# ring_tls's and ring_registry's own census recipes use) -- without it this
# recipe matches the two crates' own Fix()/Root cause comments, which quote
# the old `_ =>` patterns as permanent historical prose, and would keep
# reporting those two lines forever regardless of what the code now does.
for f in ring_core/src/lib.rs ring_flush/src/lib.rs; do
  out=$( command grep -vE '^\s*(//|///|//!)' "$f" | command grep -n '_ =>' )
  if [ -n "$out" ]; then
    echo "$out" | sed "s|^|    $f:|"
  fi
done
printf '    wildcard arms in ring_trace: %s\n' "$( command grep -c '_ =>' ring_trace/src/lib.rs || true )"
```

Live output:

```
  -- the tripwire, and the rationale the crate gives for it --
  ///
  /// Written as an exhaustive `match` rather than a derive so that adding a
  /// discriminant fails to compile here, where a human then has to say what the
  /// new operation is called.
  -- who else in the family states a compile-time claim --
    ring_overflow/src/lib.rs:
    ring_slot/src/lib.rs:
    ring_trace/src/lib.rs:
    ring_flush/tests/flush_test.rs:
    ring_poll/tests/poll_test.rs:
    ring_trace/tests/trace_test.rs:
  -- ring_flush measured its claim and quoted the compiler --
/// is not expressible on stable Rust. The second was **measured** rather than
/// assumed: adding `fn assert_sync< T : Sync >(){}` here and calling it with
/// this type fails to compile, and the compiler names the reason —
///
/// ```text
/// note: required because it appears within the type `Producer<'_, u32>`
/// note: required because it appears within the type `Flusher<'_, u32>`
  -- ring_poll declined to make one, and said why --
//! **No test that a parking call fails to compile.** It would not fail to
//! compile; it would fail to *resolve*, because the crate is not a dependency —
//! and a test cannot name a crate it cannot see. The manifest scan below is the
//! honest form of that assertion, and `tests/manual/readme.md` P1 records what
//! the compiler actually says when the dependency is added back.
  -- and where a wildcard arm turns a variant addition into a silent default --
    wildcard arms in ring_trace: 0
```

---

### TR39 — The Family Has a Discipline for Compile-Time Claims and This Crate's Claim Skipped It

Six places in the family assert something about what will or will not compile,
and five of them handle it in a way worth copying.

`ring_flush` states a claim and then discharges it by measurement: `Send` and
`!Sync` together are the property it wants, "the second was **measured** rather
than assumed: adding `fn assert_sync< T : Sync >(){}` here and calling it with
this type fails to compile, and the compiler names the reason" — followed by the
compiler's actual notes, quoted. The claim is not a prediction; it is a recorded
observation with the tool's own words attached.

`ring_poll` states a claim it refuses to make, under a heading for what is
deliberately absent: "**No test that a parking call fails to compile.** It would
not fail to compile; it would fail to *resolve*, because the crate is not a
dependency — and a test cannot name a crate it cannot see." It then names the
honest substitute, a manifest scan, and points at the manual plan for what the
compiler says when the dependency is restored. The claim was examined, found to
be the wrong instrument, and replaced.

`ring_overflow` and `ring_slot` each state a claim about code that already
exists, and each leaves a reader something to run. `ring_overflow`'s is
structural: a fourth `OverflowPolicy` "already fails to compile against the two
`match` blocks below and the length assertion in `ring_types`", and the
asymmetry it goes on to name — that a `match` catches a new variant only where
one is written, and nothing published a count a test could pin for `Resolution`
— is exactly what the constant beside it fixes. `Resolution::ALL.len()` is now
asserted against `3` twice, in the constant's own doctest and in
`overflow_test.rs`. `ring_slot`'s is narrower and names its instrument outright:
replacing the hand-written `Default` with the derive would make
`a_slot_is_default_for_a_payload_that_is_not` fail to compile — and that test
exists, at `slot_test.rs:438`. Neither quotes the compiler the way `ring_flush`
does, but neither asks to be taken on trust either.

`ring_trace` makes two. The one in `src/lib.rs:98` is correct: an exhaustive
`match` rather than a derive, so adding a discriminant "fails to compile here,
where a human then has to say what the new operation is called". The one in
`trace_test.rs:203-208` is not, and
[`item/002`](../item/002_traceop_all_and_the_tripwire_that_is_not_one.md) shows a
compiled counterexample. It is the only compile-time claim in the family that was
neither measured nor examined, and it is the only one that turned out to be
false.

**Finding.** Recorded as a family pattern with an established standard that this
crate's second claim did not meet. The standard is already written down four
times, in four crates, in four forms — quote the compiler, explain why you
cannot, name the structure that already refuses, or name the test that would
stop compiling — and any of them would have caught the defect immediately,
because attempting to produce the compile error is exactly what does not happen
when the error does not exist.
The remedy for the specific claim is in `item/002`; the remedy worth recording
here is the rule: a compile-time assertion in this family is discharged by
running the compiler, not by describing it.

---

### TR40 — One Chosen Wildcard Arm Is Left, and It Converts a Variant Addition Into a Silent Default

`ring_trace` has no `_ =>` arm anywhere, which is what makes its tripwire work.
Two exist elsewhere in the family, and they are not equivalent to each other.

`ring_flush:226` matches a `FlushOutcome` for its count and reads `_ => 0`, so a
new outcome that moved records silently reports moving none. It is over a
family-local enum, it turns what would be a compile error into a
plausible-looking default, and it carries no comment saying the default is
deliberate.

`ring_flush:576` is different and should not be counted with it. Its arms are
guarded — `FlushPolicy::OnFull if self.buffer.is_full()`, and two more — and
guarded arms never contribute to exhaustiveness, so the wildcard is not a choice
the author made but a requirement the language imposed. Removing it would not
compile.

**There were three when this was written, and the third is the reason to keep
reading the census.** `ring_core:394` matched a `Resolution` and read
`_ => Err( record )`, so a new overflow resolution silently became a refusal.
That arm now names `EvictedOldest` and `Refused`; it was corrected under
`ring_core`'s CO1, which found the same line independently from inside the crate
that owns it. Two documents in two crates converging on one arm is the census
earning its cost — this one could see that the family had a wildcard problem and
not which one was expensive, CO1 could see the expense and not the pattern, and
the arm that was in both readings is the one that got fixed first.

**Finding.** Recorded because the idiom this crate relies on is only as good as
its absence of wildcards, and the family is one arm away from losing it in one
place. The distinction between the forced wildcard and the chosen one is the
useful part: `ring_flush:576` needs nothing, while `ring_flush:226` needs either
an enumerated arm per variant or one comment saying which future variants the
default is correct for. A wildcard over a crate-local enum is a decision, and
this one does not record itself as one.

**Disposition:** superseded — both closed independently, from inside
`ring_flush`, the same way `ring_core`'s already had. `count()`'s `_ => 0` is
now three named arms (`NotTriggered`, `TriggeredEmpty`, `Rejected { .. }`),
under `Fix(flush_entry_count_catchall_not_exhaustive)` — the exact remedy this
finding asked for. `trigger`'s guard-based match ending `_ => None` is now
matched on the variant exhaustively first, with each guard moved inside its
own arm, under `Fix(flush_policy_trigger_catchall_not_exhaustive)`.

That second fix corrects this finding's own analysis, not just the code: the
"needs nothing" verdict on `:576` assumed the guarded `_ => None` was a
requirement the language imposed on the match *as structured*, but the match
could be restructured to not need one at all — and `ring_flush`'s own
`Root cause` comment on that fix names exactly the hazard this finding waved
off, from the other direction: "a fourth `FlushPolicy` variant would silently
never trigger... regardless of its own condition," the same silent-default
shape as `:226`, just reached through a guard instead of a bare wildcard.
"Forced by the language" was true of the arm as written, not of the match as
it had to be written.

Both fixes' own `Fix`/`Root cause` comments quote the old `_ => 0` / `_ => None`
patterns as permanent historical prose, which is what the census recipe above
was matching after the code changed underneath it — a comment-exclusion guard
now keeps that recipe honest (see the recipe above). Verified directly against
`ring_flush/src/lib.rs:233-239` and `:602-608`, 2026-09-27 — neither function
contains a wildcard arm today, and the family-wide census this finding started
(`ring_core`, `ring_flush`) now stands at zero chosen or forced wildcards over
a crate-local enum.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](../item/002_traceop_all_and_the_tripwire_that_is_not_one.md) | The claim this crate got wrong |
| [`pattern/001`](001_one_accessor_for_five_lock_sites.md) | The crate's runtime pattern |
| [`type/001`](../type/001_five_discriminants_and_the_array_beside_them.md) | The enum the tripwire guards |
| [`invariant/001`](../invariant/001_disabled_means_zero_forever.md) | The other property with no compile-time guard |

### Sources

| Fact | Where |
|------|-------|
| The crate's stated rationale | `ring_trace/src/lib.rs:98-100` |
| The six compile-time claims in the family | Census above |
| `ring_flush` quoting the compiler | `ring_flush/tests/flush_test.rs:1018-1024` |
| `ring_poll` declining and substituting | `ring_poll/tests/poll_test.rs:21-25` |
| `ring_overflow` naming the structure that refuses | `ring_overflow/src/lib.rs:82-87` |
| `ring_slot` naming the test that would stop compiling | `ring_slot/src/lib.rs:160-167`, `ring_slot/tests/slot_test.rs:438` |
| The chosen wildcard, as found (TR40's `count()` case) | `ring_flush/src/lib.rs:220-229` (now the `Fix` comment; the code itself is `:231-240`) |
| The guarded wildcard, as found (TR40's `trigger()` case) | `ring_flush/src/lib.rs:587-599` (now the `Fix` comment; the code itself is `:600-608`) |
| Both now closed, from inside `ring_flush` | Disposition above |
| The earlier `ring_core` wildcard, closed under CO1 | `ring_core/src/lib.rs:394` |
| No wildcard anywhere in this crate | Census above |

### Tests

| Test | Covers |
|------|--------|
| `every_operation_kind_has_its_own_name` | The match the tripwire lives in |
| `an_operation_prints_as_its_name` | `Display`, which delegates to it |
| `the_operation_kinds_are_exactly_the_five_declared` | The claim examined in `item/002` |
