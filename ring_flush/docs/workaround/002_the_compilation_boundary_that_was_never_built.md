# Workaround: The Compilation Boundary That Was Never Built

### Scope

- **Purpose**: Record that W2's compensation column names a mechanism that does not exist, that the crate's own source says so, and that the readme carries a recipe which would have caught a neighbouring error and was never run.
- **Responsibility**: The gap between W2's stated compensation and the shipped one; the dependency-count error; the fence type that let both survive.
- **In Scope**: `Cargo.toml`'s dependency block; `[features]` across the family; the `flush-log` proposal wherever it is still written down.
- **Out of Scope**: Why the log exists at all (→ [`data_structure/002`](../data_structure/002_the_flush_log.md)); what the log proves (→ [`non_functional_requirement/001`](../non_functional_requirement/001_three_triggers_proven_by_a_flush_log.md)); W1 (→ [`001`](001_the_obligation_that_binds_nobody.md)).

### The Constraint Is Real; the Compensation Is Not the One Recorded

W2 is sound as a statement about Cargo: `cfg( test )` is off when the crate is
compiled as a dependency for `tests/*.rs`, so a structure gated on it is
invisible to exactly the tests that need it. Nothing here disputes that.

What changed is the answer. The pre-implementation design weighed four options —
`cfg( test )` (rejected: the constraint itself), `cfg( debug_assertions )`
(rejected: coarse), a `flush-log` cargo feature (the leading candidate), and an
unconditional structure. Pending 2 then dissolved rather than being ruled: the
log became an opt-in `Option< FlushLog >` the `Flusher` owns, reached by
[`with_log`](../../src/lib.rs), which needs **no compilation boundary at all**.

| | W2's row says | What shipped |
|---|---------------|--------------|
| Mechanism | A cargo feature (`flush-log` or similar) | A field, `log : Option< FlushLog >` |
| Default | Off | Absent — same effect, no build variance |
| Enabled by | The test profile | A method call in the test |
| Cost stated | A gate that forgets the flag asserts nothing and **passes** | None — there is one build |
| Exists? | No | Yes |

**The stated cost is the reason this matters.** W2's cost column describes a
silent-vacuity failure — a suite that runs against a build with no log and passes
— which is a genuinely serious hazard and is precisely what the shipped design
removes. So the row does not merely name the wrong mechanism; it warns about a
risk that no longer exists, in a table whose purpose is to tell a reader what
this crate is currently paying.

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency block, measured rather than described |
| [`src/lib.rs`](../../src/lib.rs) | Line 38's table row, which records the shipped answer correctly |

### Data Structures

| File | Relationship |
|------|-----------------|
| [`../data_structure/002_the_flush_log.md`](../data_structure/002_the_flush_log.md) | The four-option table W2 points at, which still lists the feature as a live candidate |

### Integrations

| File | Relationship |
|------|-----------------|
| [`../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md`](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | Carries the same dependency count in its title |

### Workarounds

| File | Relationship |
|------|-----------------|
| [`001_the_obligation_that_binds_nobody.md`](001_the_obligation_that_binds_nobody.md) | W1, the constraint that is still live |

### Tests

| Test | Relationship |
|------|--------------|
| `the_log_agrees_with_every_outcome_it_recorded` | Exercises the shipped opt-in, with no feature flag anywhere |
| `a_driver_has_no_log_unless_asked` | The default absence W2 expected a cargo feature to produce |
| `an_unobserved_driver_never_acquires_a_log` | The cost side — the opt-in stays unallocated when nobody asks |

### FL51 — The Compensation Column Names a Cargo Feature the Crate Never Grew

Three documents still carry the proposal; the source records the resolution:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the crate source says shipped --'
command grep -n 'no cargo feature' ring_flush/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- where the feature is still named as the answer --'
command grep -n 'flush-log' \
  ring_flush/docs/workaround/readme.md \
  ring_flush/docs/data_structure/002_the_flush_log.md \
  ring_flush/docs/decisions/readme.md \
  | sed -E 's/^(.{0,120}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and whether any manifest in the family declares it --'
printf '    crates with a [features] section: %s\n' \
  "$( command grep -l '\[features\]' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf '    of them declaring flush-log:      %s\n' \
  "$( command grep -l 'flush-log' ring_*/Cargo.toml 2>/dev/null | wc -l )"
```

Live output:

```
  -- what the crate source says shipped --
//! | The flush log's compilation boundary | An opt-in [`FlushLog`] the [`Flusher`] owns — no cargo feature, no `cfg` | `docs/data_structure/002_the_flush_log.md` |
  -- where the feature is still named as the answer --
ring_flush/docs/workaround/readme.md:| W2 | **`cfg( test )` does not reach integration tests.** Cargo compiles t
ring_flush/docs/data_structure/002_the_flush_log.md:| A cargo feature (`flush-log`) | Explicit, benchmarkable b
ring_flush/docs/decisions/readme.md:`cfg(test)` off. That leaves a cargo feature (`flush-log`), `debug_assertion
  -- and whether any manifest in the family declares it --
    crates with a [features] section: 4
    of them declaring flush-log:      0
```

`src/lib.rs`'s own module-level table records the shipped answer in the right
words — "an opt-in `FlushLog` the `Flusher` owns — no cargo feature, no `cfg`" —
so the crate is not confused about what it built. Three documents still describe
the abandoned candidate as current, and one of them is the workaround table,
which is the document a reader consults specifically to learn what this crate is
compensating for today.

**Four crates in the family do carry a `[features]` section**, so the mechanism
was available, familiar, and declined. That is worth distinguishing from "never
considered": W2's compensation is not aspirational, it is *superseded*, and the
supersession is the interesting part — the constraint was dissolved rather than
absorbed, which is the outcome a workaround entry most wants to record and the
one its table has no column for.

The general shape: **a workaround row is written while the compensation is still
a plan, and nothing revisits it when the plan is replaced by something better.**
The row's "Deleted when" column says "Never, structurally" — true of the Cargo
behaviour and false of this row, which was deletable the moment `with_log`
landed.

### FL52 — The Readme's Own Recipe Would Have Caught the Error in the Sentence Introducing It

The dependency count is stated twice and is wrong both times:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_flush
echo '  -- what the manifest declares --'
sed -n '/^\[dependencies\]/,/^$/p' Cargo.toml
echo '  -- old "two workspace siblings"/"two path dependencies" phrasing still there (expect 0 -- corrected below) --'
printf '    hits: %s\n' "$( command grep -c 'two workspace siblings\|two path dependencies' docs/workaround/readme.md )"
echo '  -- every opening fence in that readme, and the first line it carries --'
awk '/^```/ { lang = substr( $0, 4 ); if ( lang == "" ) next
              printf "    %4d  %-4s  ", NR, lang; getline; print }' docs/workaround/readme.md
```

Live output:

```
  -- what the manifest declares --
[dependencies]
ring_tls = { path = "../ring_tls" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

  -- old "two workspace siblings"/"two path dependencies" phrasing still there (expect 0 -- corrected below) --
    hits: 0
  -- every opening fence in that readme, and the first line it carries --
      22  sh    cd "$(git rev-parse --show-toplevel)"/ring_flush
      68  sh    cd "$(git rev-parse --show-toplevel)"
     154  sh    cd "$(git rev-parse --show-toplevel)"/ring_flush/docs/workaround
```

Three path dependencies — `ring_tls`, `ring_core`, `ring_types` — plus one
dev-dependency. The readme says two, in the overview and again in its Sources
table, and [`integration/001`](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md)
carries the same count in its filename and title.

**The recipe that settles it is already in the readme, one line below the
claim.** `cargo tree --depth 1` prints the dependency surface; running it once
would have replaced "two workspace siblings" with three. It was never run, and
the reason is mechanical: it is fenced ```` ```bash ````, and the corpus gate
reads only ```` ```sh ```` blocks. A ```` ```bash ```` fence is not a failing
recipe — it is an *invisible* one. The gate does not execute it, does not
compare its output, and does not report it missing.

So the failure is not that nobody checked. Somebody wrote the check, put it in
the right place, and phrased the surrounding sentence as an instruction to run it
("confirm before reading further"). The fence type quietly converted an
executable claim into a decorative one, and the sentence it was there to verify
has been wrong ever since.

**The readme is not a document the gate skips.** The third fence is the
`Regenerate` block the corpus standard requires, and it is ```` ```sh ````, so
the gate runs it on every pass — it counts instances, finding headings and table
rows, and it has never once reported a problem, because there is no problem in
what it counts. The gate is not absent from this file. It is present, green, and
looking three lines away from the false sentence, at the only claim in the
readme nobody was ever in danger of getting wrong.

**That is the sharpest argument in this crate for the ```sh convention being a
rule rather than a style**, and it generalises past this crate: every ```` ```bash ````
block in the corpus is a check somebody thought they had, and a green
`Regenerate` block beside it is not evidence that anything else on the page was
verified.

**Disposition:** applied — `docs/workaround/readme.md` now reads "three
workspace siblings" in its overview and "three path dependencies" in its
Sources table, and its two ```` ```bash ```` blocks (the `cargo tree --depth 1`
recipe this finding names, and a second scratch-workspace recipe demonstrating
the dev-dependency-cycle workaround) are re-fenced ```` ```sh ```` with genuine
`Live output:`, so the corpus gate runs both on every pass instead of neither.
This finding's own recipe confirms it above: all three fences in that readme
are now `sh`, and the grep for the old "two workspace siblings"/"two path
dependencies" phrasing finds nothing. The general argument two paragraphs up —
that a ```` ```bash ```` fence converts an executable claim into a decorative
one — is the reusable lesson and is unaffected by this specific instance being
fixed.
