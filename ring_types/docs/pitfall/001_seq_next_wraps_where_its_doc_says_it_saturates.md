# Pitfall: Seq::next Wrapped Where Its Doc Said It Saturates

### Scope

- **Purpose**: Record that [`Seq::next`](../item/associated_function/007_seq_next.md)'s doc comment stated release-build behaviour that Rust does not have, that the behaviour it actually has is the exact one the same paragraph said the type was designed to avoid, and — since the comment has since been corrected and this document had not been — which of the three mitigations named below actually landed.
- **Responsibility**: Name the trap, the failures, the mitigations, and the state of each.
- **In Scope**: `Seq`'s three arithmetic methods; what `+` does on overflow under each profile; why the error was harmless in practice and load-bearing in the paragraph it appeared in; the correction to `next` and the separate one to `advanced_by`.
- **Filename note**: the file name states the defect in the present tense and no longer does so accurately. It is kept because thirteen documents across three crates link to it by that name and the corpus indexes findings by path; the H1, the trap, and § Disposition carry the current state.
- **Out of Scope**: Whether `u64` is the right width (→ [`non_functional_requirement/001`](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md)); the fold from `Seq` to `SlotIndex` (→ [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md)).

### Trap

**The doc comment on `Seq::next` described three behaviours and got the middle
one wrong.** As found, it read:

```text
  /// Panics on overflow in a debug build and saturates in a release build,
  /// which is the standard `u64` addition behaviour — deliberately not
  /// wrapping, since a wrapped `Seq` would silently violate the monotonicity
  /// every gate in the family relies on.
```

**`u64` addition does not saturate in a release build. It wraps.** The body is
`Self( self.0 + 1 )` — plain `+`, no `saturating_add`, no `checked_add` — so
the release behaviour was the one outcome the sentence's own subordinate clause
ruled out. The clause was right about the consequence and wrong about the fact:
a wrapped `Seq` would violate monotonicity, and a wrapped `Seq` is what a
release build produces.

**That sentence has since been corrected, and this document had not been.**
What the source carries now, beside the paragraph that was missing entirely:

```sh
cd "$(git rev-parse --show-toplevel)"
# the paragraph as it stands, and the one that was missing beside it
command grep -A4 -F '/// Panics on overflow in a debug build' ring_types/src/id.rs
command grep -A5 -F '/// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs

# the three bodies those paragraphs are about
command grep -A3 -F 'pub const fn next( self ) -> Self' ring_types/src/id.rs
command grep -A3 -F 'pub const fn advanced_by( self, n : u64 ) -> Self' ring_types/src/id.rs
command grep -A3 -F 'pub const fn distance_to( self, later : Self ) -> u64' ring_types/src/id.rs

# which profile decides which half of the paragraph applies. the question is not
# whether the workspace declares profiles — it declares nine — but whether any
# touches `release` or `overflow-checks`. counted and listed, because the count
# alone was what this document used to assert, and it has since gone from 0 to 9
# without the conclusion changing
command grep -c '^\[profile' Cargo.toml | sed 's/^/  profile sections declared: /'
command grep -o '^\[profile\.[a-z]*' Cargo.toml | sort -u | sed 's/^/  scope: /'
command grep -c 'overflow-checks' Cargo.toml | sed 's/^/  lines setting overflow-checks: /'
```

Live output:

```
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
  /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
  /// takes `n` from the caller and reaches it in a single call from any
  /// position. A caller deriving `n` from a batch length or a configured
  /// count owns that bound; nothing here checks it.
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  }
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
  profile sections declared: 8
  scope: [profile.dev
  lines setting overflow-checks: 0
```

**The last three lines are a second correction, to this document rather than to
the source.** The original evidence for "release wraps" was
`grep -c '^\[profile' Cargo.toml` with the inline answer `# 0 — release
defaults apply`, under the prose *"this workspace sets no `[profile]` section at
all."* The workspace now declares eight. The conclusion is unchanged and the
argument for it is stronger — every one of the eight is `[profile.dev...]`, none
names `release`, and `overflow-checks` appears zero times anywhere in the
manifest — but the check as written had inverted from 0 to 9 while its own
comment still said 0. The load-bearing fact was never the count; it was which
profile and which key, which is what the recipe now asks.

**Correction (2026-09-28):** the paragraph above itself went stale the same
way it describes — it was written when the recipe printed nine and said so;
the workspace now declares eight `[profile.dev.package.*]` sections, one
fewer. Same conclusion again, for the same reason: none names `release` or
sets `overflow-checks`, so which profile and which key is still the only
fact that matters.

**The same gap covered [`advanced_by`](../item/associated_function/008_seq_advanced_by.md)**,
whose body is `Self( self.0 + n )` and which carried no overflow paragraph at
all — so the incorrect claim was stated once on `next` and inherited silently by
the method that can overflow in one call rather than one increment at a time.
That is the half the correction to `next` did not reach, and it is now closed
separately (§ Disposition).
[`distance_to`](../item/associated_function/009_seq_distance_to.md) is the only
one of the three that is genuinely saturating, and it is the only one that said
so accurately throughout, because it actually calls `saturating_sub`.

**Why this is a documentation defect and not a bug.** The type's other claim —
that the wrap point is unreachable — holds with enormous margin:

```text
u64::MAX / 1e9 publications per second / 86400 / 365.25 ≈ 584 years
```

No realistic workload runs for 584 years, so no `Seq` reaches the
overflow. The defect is that a reader checking whether the family is safe
against a wrapped sequence finds a paragraph asserting it cannot happen for a
reason that is false, rather than the reason that is true.

**The trap generalises past this crate.** `ring_types` is tier 0 — 31 of the
33 crates depend on it — and its doc comments are the family's only prose on
what a `Seq` guarantees. A wrong guarantee here is inherited by every crate that
reasons from it rather than from the source.

### Failure

| # | Failure | How it presents | State |
|---|---------|-----------------|-------|
| S1 | A reader trusts "saturates" and writes a gate that treats a non-advancing `Seq` as a stalled producer | Correct today, because neither case occurs. Wrong in the same direction as the doc if the premise is ever tested | closed by M1 — there is no longer a "saturates" to trust |
| S2 | Someone reduces `Seq` to `u32` on the strength of "it saturates anyway" | 10⁹/s exhausts a `u32` in **4.3 seconds**. The wrap is then reachable, and the release build wraps rather than saturating | closed by M1 — and the 584-year figure the argument rests on now sits in the same paragraph |
| S3 | A debug-build test asserts the panic and is read as covering the release case | The two profiles genuinely differ here; a debug assertion says nothing about release | **open** — no test exercises either method's overflow under either profile (→ TY49) |
| S4 | `advanced_by( n )` is called with an attacker- or config-supplied `n` | Overflows in one call rather than after 2⁶⁴ increments, and does so silently in release | closed by M2 as a *documented* contract; the arithmetic is unchanged and the caller still owns the bound |
| S5 | The doc is corrected to "saturates" by changing the *code* to `saturating_add` | Monotonicity is preserved but progress stops silently — a saturated `Seq` reports the same position forever, which is the stall S1 describes | **open** as a standing hazard — the correction that landed went the other way, to the comment, which is what this row argues for |

**S2 is the one with a plausible trigger.** `Seq` is 8 bytes
(→ [`data_structure/001`](../data_structure/001_two_position_types_and_the_fold_between_them.md)),
and halving it is exactly the kind of change a cache-line argument motivates —
`ring_align` and `ring_cursor` both exist to make positions pack well. The
584-year figure is what makes `u64` non-negotiable, and it is stated one
sentence away from the claim that is wrong.

**S4 was the one the text did not cover at all, and it outlived the fix to
`next`.** `advanced_by` takes `n : u64` and carried no overflow paragraph, so a
reader looking specifically for the overflow contract of the method they were
calling found nothing and inferred it from `next`'s — which was the incorrect
one. When `next`'s paragraph was corrected, that inference became correct by
accident rather than by statement, and `advanced_by` still said nothing. It now
carries its own paragraph (§ Disposition), which is the one place the two
methods genuinely differ: `next` needs 2⁶⁴ increments to reach the wrap, and
`advanced_by` reaches it in one call from any position.

`ring_trace`'s [`workaround/001`](../../../ring_trace/docs/workaround/001_an_addition_the_types_crate_already_offers.md)
TR49/TR50 is what makes S4 more than hypothetical: it prescribes `advanced_by`
to `ring_batch` as a *remedy* for open-coded addition. A remedy function whose
overflow contract was unstated is the same trap one level up, and
`ring_consume`'s [`workaround/001`](../../../ring_consume/docs/workaround/001_five_functions_none_const.md)
CN55 records what became of that prescription.

**S5 is why the fix is to the comment and not to the code.** Wrapping and
saturating are both wrong for a monotonic counter; the design's actual position
is that the wrap point is unreachable, which makes the arithmetic mode
irrelevant rather than correct. Changing the code to enforce a mode would
implement a guarantee the family does not need and cannot test.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Assert the saturation in a test | It does not saturate. The test either fails or is written in a debug profile where the panic masks the question |
| `#[ cfg( debug_assertions ) ]` on the claim | The claim is about release. Gating it on debug documents the half that was already correct |
| Add `checked_add` and return `Result` | Puts a `?` on the hottest path in the family for a condition 584 years away, and every caller would `unwrap` it |
| Leave it — the wrap is unreachable | The wrap being unreachable is the correct argument. It is not the argument on the page |

**What works:**

1. **Replace the mode claim with the reachability claim.** The sentence should
   say that `u64` addition wraps in release and panics in debug, that neither
   is reached because the wrap point is 584 years out, and that this — not a
   saturating mode — is what protects monotonicity. One sentence, and it is the
   argument the type is actually built on.
2. **State the same contract on `advanced_by`**, where a single call can
   overflow. Its `n` comes from a caller — `ring_index`'s `run` derives it from
   a batch length, and it is not the only one.
3. **Pin the reachability figure in the manual plan rather than the suite.**
   `tests/manual/readme.md` already covers the claims a test cannot make about
   itself; "584 years at 10⁹/s" is arithmetic a reader checks once and a test
   cannot.

**Mitigation 1 was the whole fix and it was one sentence.** The reason to
record a pitfall for a one-sentence correction is that the sentence is in tier
0, is inherited by 31 crates, and was wrong in the direction of reassurance —
the failure mode of a comment that overstates a guarantee is that nobody
re-derives it.

**Mitigation 2 turned out to be the larger half, and it is why "one sentence"
undersold this.** The census below counts sixteen `.advanced_by(` call sites
across eight crates. `ring_mpsc` alone has four, `ring_claim` three,
`ring_spsc` three; several pass a length or a count directly —
`self.len as u64`, `granted as u64`, `capacity.get() as u64`. Every one of
those was reading a method whose overflow behaviour was documented nowhere,
inferring it from the paragraph one method up, which was wrong. Correcting
`next` alone made that inference accidentally correct while leaving it an
inference.

**This instance originally declined to change `src/`**, on the grounds that the
crate's implementation was owned elsewhere in the family's current work. That
deferral is what let Mitigation 1 land from another hand while Mitigation 2 sat
unexecuted and this page went on describing both as open.

```sh
cd "$(git rev-parse --show-toplevel)"
# M1 — the mode claim replaced by the reachability claim
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/  M1  "saturates in a release build" still in id.rs : /'
command grep -c 'wraps to zero in a release' ring_types/src/id.rs \
  | sed 's/^/  M1  "wraps to zero in a release" now in id.rs     : /'

# M2 — advanced_by carrying its own contract, and who calls it. matched on the
# call syntax `.advanced_by(` rather than the bare name, so prose mentions and
# the definition itself do not inflate the count; temporal_substrate is excluded
# because its TimeDomain trait has an unrelated three-argument method of the
# same name
command grep -c 'Overflow behaves exactly as' ring_types/src/id.rs \
  | sed 's/^/  M2  advanced_by overflow paragraph                : /'
command grep -rho '\.advanced_by(' ring_[a-z]*/src/*.rs | wc -l \
  | sed 's/^/  M2  Seq::advanced_by call sites across ring_*     : /'
command grep -rl '\.advanced_by(' ring_[a-z]*/src/*.rs \
  | sed 's|ring/||;s|/src/.*||' | sort -u | paste -sd' ' - \
  | sed 's/^/  M2  crates calling it                             : /'
echo

# M3 — the reachability figure pinned in the manual plan. captured and tested
# for emptiness rather than piped straight into sed: a pipeline's exit status is
# the last command's, so a `|| echo` after `grep | sed` never fires and the
# stage prints nothing at all — which is indistinguishable from a stale pattern
m3="$( command grep -o '584[^,.]*' ring_types/tests/manual/readme.md )"
if [ -n "$m3" ]; then printf '%s\n' "$m3" | sed 's/^/  M3  /'
else echo '  M3  no 584-year figure in the manual plan — open'; fi
```

Live output:

```
  M1  "saturates in a release build" still in id.rs : 0
  M1  "wraps to zero in a release" now in id.rs     : 1
  M2  advanced_by overflow paragraph                : 1
  M2  Seq::advanced_by call sites across ring_*     : 16
  M2  crates calling it                             : ring_claim ring_consume ring_gating ring_index ring_mpsc ring_publish ring_spsc ring_types

  M3  584-year reachability figure holds up under direct arithmetic
  M3  584` years — matching the figure quoted in
  M3  584
  M3  584 years" citation across `src/id
```

**Disposition (2026-09-06): all three mitigations closed.** `tests/manual/readme.md`
now carries an M5 entry pinning the reachability arithmetic (`awk` recomputes
`584.5` years from `u64::MAX`, `1e9`/s, and the days-per-year constant), with a
Run Record row confirming it was actually run rather than only added.

**M1 applied, by another hand and undated here.** `next`'s paragraph now states
that release wraps to zero and that the protection is the reachability
argument, not an arithmetic mode. That is precisely what M1 asked for, down to
the 584-year figure. This document was not updated when it landed, so its trap,
its evidence block, its failure table, and its file name all went on stating a
corrected defect in the present tense — the reason § Trap now leads with what
the source carries rather than with what it carried.

**M2 applied here.** `advanced_by` now carries its own overflow paragraph,
stating the contract by reference to `next` rather than restating it, and
naming the one place the two genuinely differ: `next` needs 2⁶⁴ increments to
reach the wrap, `advanced_by` reaches it in one call from any position, and
nothing checks the `n` a caller supplies. Deliberately not a `# Panics`
section: `next` states the same contract in prose, and the two arithmetic
methods in one `impl` block are read by comparing them. `ring_types`' 19 unit
tests and 20 doctests re-verified passing, and `cargo doc` clean under
`RUSTDOCFLAGS="-D warnings"` for the new intra-doc link
(2026-09-05). Now prints: `M2  advanced_by overflow paragraph : 1`

**M3 still open.** `584` does not appear in `ring_types/tests/manual/readme.md`.
The figure is asserted in two doc comments and computed by
`seq_does_not_wrap_within_any_reachable_workload`, which divides constants and
asserts the quotient exceeds 500 — never calling `next` or `advanced_by` at all
(→ TY49 below). So the claim a reader most needs to check by hand is the one
the manual plan does not carry.

**A second correction, to this page rather than to the source**, is recorded in
§ Trap: the original evidence for "release wraps" counted `[profile]` sections
and asserted zero. There are now nine. None touches `release` and none sets
`overflow-checks`, so the conclusion stands — but the check had inverted while
its own inline answer still read `# 0`.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_validating_a_slot_count_to_a_power_of_two.md](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md) | The other arithmetic contract in this crate, and the one enforced by a type rather than a comment |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | `Seq`'s width, and why S2's halving is tempting |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | The contrast — a guarantee this crate enforces structurally instead of asserting in prose |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_function/007_seq_next.md](../item/associated_function/007_seq_next.md) | The declaration carrying the incorrect paragraph |
| [../item/associated_function/008_seq_advanced_by.md](../item/associated_function/008_seq_advanced_by.md) | S4 — the same arithmetic with no overflow paragraph at all |
| [../item/associated_function/009_seq_distance_to.md](../item/associated_function/009_seq_distance_to.md) | The one of the three that genuinely saturates, and says so correctly |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_errors_and_positions_do_not_allocate.md](../non_functional_requirement/001_errors_and_positions_do_not_allocate.md) | Why `Seq` is a plain `u64` newtype and not a checked wrapper |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_two_name_errors_nothing_constructs.md](002_two_name_errors_nothing_constructs.md) | The other trap — a declaration with no behaviour behind it, rather than a comment with the wrong behaviour behind it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/id.rs`](../../src/id.rs) | `next`'s overflow paragraph and `advanced_by`'s, and the two `+` bodies they describe — anchored by content rather than by line, because the paragraph's own edit moved every number this row used to carry |
| [`Cargo.toml`](../../../../Cargo.toml) | The nine `[profile.dev.package.*]` declarations, none of which names `release` or sets `overflow-checks` |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ⚠️ `seq_does_not_wrap_within_any_reachable_workload` asserts the reachability claim, which is the true one — so the suite already tests the right thing while the comment states the wrong one. No test asserts the saturation, which is why the error survived: **the suite and the comment disagree and only the comment is read by a human** |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | Mitigation 3's home — the plan already exists for claims a test cannot make about itself |

### TY49 — The Test Named for the Property Never Calls the Function

The test that carries the wrap property's name computes a division and asserts
the quotient is over 500:

```rust
// tests/types_test.rs
let years = u64::MAX / per_second / seconds_per_year;
assert!( years > 500, "u64 sequence exhausts in {years} years, too few to assume monotonic" );
```

No `Seq`, no `next`, no `advanced_by`. It is a true and useful statement about
`u64` — it licenses the plain `<` comparisons the family relies on — and it is
not a test of this crate's code.

**So the pitfall this instance documents is unguarded from both sides.** The doc
comment says `next` saturates in release and it wraps; the test named for
non-wrapping does not execute `next`. Between them, the release-build behaviour
of the crate's most-used method is asserted nowhere.
