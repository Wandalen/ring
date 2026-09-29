# non_functional_requirement

The crate has the non-functional profile its position in the family demands. It
allocates nothing, names no `std` path, contains no `unsafe`, is `const`
throughout, and is read four times per ring built and never inside a loop. Every
one of those is true and only one of them — the absence of `unsafe` — is checked
by anything.

The second instance takes the other kind of requirement: not what the crate must
avoid, but what its shape is justified by. The crate's own module comment names
all five fields and says the manifest language "eventually" describes exactly
this record. That language is a forty-one-line skeleton that has never heard of
the type, and one of the three symptoms that same argument says the record cures
is still only partly delivered.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_record_that_allocates_nothing_and_is_read_once.md) | A Record That Allocates Nothing and Is Read Once | The allocation and `std` census, the four reads per ring, and what the build actually enforces |
| [002](002_a_record_sized_for_a_design_not_yet_built.md) | A Record Sized for a Design Not Yet Built | The module comment's own justification, its unbuilt consumer, and its one undelivered promise |

## Properties Held by Habit

Twenty-five of the thirty-three `ring_*` crates never name `std::` or `alloc::` in
`src/`, and three of the thirty-three declare `#![ no_std ]` — not `ring_config`,
though `ring_types` in its closure does. `ring_config` is one
of the twenty-five and additionally names no `Vec`, `String` or `Box` — it would
compile without an operating system, and nothing would notice if that stopped
being true.

`unsafe` is the counter-example, and it is what makes the gap legible.
`unsafe-code = "deny"` sits in the workspace lints table, `ring_config` inherits
it, and exactly two crates in the family opt out with a visible
`#![ allow( unsafe_code ) ]`. One property is stated centrally, enforced by the
compiler, and has its exceptions written down. The other is a habit twenty-five
crates happen to share.

## Cost That Never Arrives

The record is thirty-two bytes and `Copy`, and a by-value hand-off measured 3.7×
to 4.0× a reference. Neither number matters, because the whole construction path
reads a `RingConfig` four times — `overflow`, `is_multi_producer`, `overflow`
again, then `capacity` in whichever backend was chosen — and nothing anywhere
reads one inside a publish, a claim or a drain.

The crate reads like one written for a hot path, and is not on one. Nothing
records that, so a sixth field consulted per publish would move it onto one with
no comment, test or lint to notice.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- lines naming an allocation, a std path or unsafe in this crate --'
command grep -c 'std::\|alloc::\|Vec<\|String\|Box<\|unsafe' ring_config/src/lib.rs || true
echo '  -- ring_* crates whose src/ never names std:: or alloc::, then how many declare no_std --'
for c in ring_*/; do n=$( command grep -rc 'std::\|alloc::' "$c"src/ 2>/dev/null | command grep -v ':0$' | wc -l ); [ "$n" = "0" ] && basename "$c"; done | wc -l
command grep -rl 'no_std' ring_*/src/lib.rs | wc -l
echo '  -- read sites of the record on the whole construction path --'
command grep -rnE '(config|cfg)\.(capacity|overflow|is_multi_producer)\(\)' --include=*.rs ring_core/src/lib.rs ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | command grep -v '///\|//!' | wc -l
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC33 | `ring_config` | n/a — observation | `ring_factory` reads no field at all and the entire construction path reads a `RingConfig` four times per ring — `overflow` at `ring_core:152`, `is_multi_producer` at `:161`, `overflow` again at `:167`, `capacity` in the chosen backend — with nothing reading one inside a publish, claim, drain or any loop in the family, so the 3.7×–4.0× by-value cost measured elsewhere is unmeasurable here, the requirement is met by traffic volume rather than by design, and no comment, test or lint would notice a sixth field that moved the record onto a hot path |
| RC34 | `ring_config` | n/a — unenforced | The crate names no `std::` or `alloc::` path, no `Vec`, `String` or `Box` and no `unsafe`, and twenty-five of the thirty-three `ring_*` crates share the first property while three of the thirty-three declare `#![ no_std ]`, `ring_config` not among them though `ring_types` in its closure is — against `unsafe-code = "deny"` in the workspace lints table, which `ring_config` inherits and exactly two crates opt out of visibly, so one non-functional property is centrally stated, compiler-checked and has written-down exceptions while the other is a habit that would be lost silently |
| RC35 | `ring_config` | n/a — observation | The module comment justifies the record's five-field shape by a manifest language that "eventually" describes exactly it, and that consumer — `lang_channel` — is a forty-one-line `src/lib.rs` naming `RingConfig` zero times, so the three fields with no reader are a specified, deliberately-held bet rather than an oversight |
| RC36 | `ring_config` | n/a — doc gap | Without this record, constructors would sprout everywhere with no single place to add a knob or **to read what a given ring was built with** — and the last half of that survives the implementation, since a built ring reports only `capacity()`, `overflow()` and a `backend()` tag while `wait` and `batch` are recoverable from nothing; nothing in the crate records that this payoff landed partially, and closing it is a field swap on a `Copy` record plus one accessor |
