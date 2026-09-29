# Item: What The Crate Does Not Declare

### Scope

- **Purpose**: Catalogue the load-bearing absences — the declarations, attributes and impls the crate could carry and does not — and say which of them are correct and which are exposures.
- **Responsibility**: Twelve absences, each with the count that establishes it and the reading it deserves.
- **In Scope**: Missing declarations, attributes and trait impls in `src/lib.rs`.
- **Out of Scope**: The declarations that do exist (→ [`001`](001_two_components_that_meet_in_no_line_of_src.md)); the dependency edges the crate was not given (→ [`../integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md)).

### Why absences need their own instance

An absent declaration has no rustdoc page. `#![ deny( missing_docs ) ]` — which
this crate carries — can require prose of everything that exists and nothing of
what does not. Both exposures below are absences, and neither is visible from any
page that lint guarantees.

### The twelve absences

| # | Absence | Count | Reading |
|---|---|---|---|
| A1 | `pub mod` | 0 | Correct — one file, and a module boundary would cut the two components apart without changing what either does |
| A2 | `pub use` | 0 | **Exposed** — the one entry point takes a type the crate does not re-export → TK28 |
| A3 | `pub type`, `pub const` | 0 | Correct — no alias earns a name here, and the one constant a caller supplies is `stage_limit`, an argument |
| A4 | `#[ non_exhaustive ]` | 1 | **Was exposed, now closed** — `Anomaly` is matched by callers and the crate's own docs treated a fourth variant as live; the attribute is on it, and the fourth variant arrived → TK27 |
| A5 | `#[ inline ]` | 0 | Correct — one file family-wide carries the attribute, `ring_trace`, on the disabled-path check its own doc justifies; nothing here has a comparable hot path to argue from |
| A6 | `impl Default` | 0 | Correct — `Script::new` takes a `stage_limit` with no natural default; a zero-slot buffer is a deliberate test case, not a default |
| A7 | `impl Drop` | 0 | Correct — and zero family-wide; `leak` exists precisely so that nothing frees |
| A8 | `impl Iterator` | 0 | Correct — `steps()` returns a slice, which iterates already |
| A9 | `impl From` | 0 | Correct — no conversion the crate owns; `Anomaly` wraps nothing |
| A10 | `fn source` | 0 | Correct — the `Error` impl is empty (`impl core::error::Error for Anomaly {}`) and no `Anomaly` variant carries a cause, so the default `None` is the true answer |
| A11 | `unsafe` | 0 | Correct — the workspace lint table sets `unsafe-code = "deny"`; `leak` reaches `'static` through `Box::leak`, which is safe |
| A12 | serde or any wire format | 0 | Correct — every value here is produced and consumed in one process |

Ten correct, two exposed when this was written. Both exposures are about the
crate's edges: one is what a caller must import to use it, the other is what
breaks for a caller when the crate changes. A4 has since been closed — see its
disposition below; A2 is open, and TK28 states why closing it is a design
question rather than a one-line fix.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
for pat in '^pub mod' '^pub use' '^pub (type|const)' 'non_exhaustive' '#\[ inline' 'impl Default' 'impl Drop' 'impl Iterator' 'impl.*From<' 'fn source' 'unsafe' 'serde' ; do
  printf '%-18s %s\n' "$pat" "$( command grep -cE "$pat" src/lib.rs || true )"
done
printf 'the Error impl, in full:  %s\n' "$( command grep -m1 'impl core::error::Error' src/lib.rs )"
printf 'Anomaly variants:         %s\n' "$( awk '/^pub enum Anomaly/{f=1} f && /^  [A-Z]/{ sub( /^  /, "" ); printf "%s ", $0 } f && /^}$/{exit}' src/lib.rs )"
printf 'run parameter type:       %s\n' "$( command grep -m1 -oE 'ring : &mut Ring< u32 >' src/lib.rs )"
printf 'inline across the family: %s\n' "$( command grep -lE '^ *#\[ inline' ../ring_*/src/*.rs 2>/dev/null | wc -l )"
printf 'non_exhaustive family:    %s\n' \
  "$( for f in ../ring_*/src/*.rs; do command grep -qE '^ *#\[ non_exhaustive' "$f" 2>/dev/null \
        && echo "${f#../}"; done | tr '\n' ' ' )"
printf 'zero-pub-use ring crates: %s\n' "$( for c in ../ring_*/ ; do command grep -qc '^pub use' "$c/src/lib.rs" 2>/dev/null || true ; n=$( command grep -c '^pub use' "$c/src/lib.rs" 2>/dev/null || true ) ; [ "$n" = "0" ] && echo x ; done | wc -l )"
```

Live output:

```
^pub mod           0
^pub use           0
^pub (type|const)  0
non_exhaustive     1
#\[ inline         0
impl Default       0
impl Drop          0
impl Iterator      0
impl.*From<        0
fn source          0
unsafe             0
serde              0
the Error impl, in full:  impl core::error::Error for Anomaly {}
Anomaly variants:         Unaccounted Unminted Overdelivered OutOfOrder 
run parameter type:       ring : &mut Ring< u32 >
inline across the family: 1
non_exhaustive family:    ring_testkit/src/lib.rs ring_types/src/error.rs 
zero-pub-use ring crates: 30
```

### Items

| File | Relationship |
|------|--------------|
| [001_two_components_that_meet_in_no_line_of_src.md](001_two_components_that_meet_in_no_line_of_src.md) | The declarations that do exist |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_no_crate_has_taken.md](../api/002_the_surface_no_crate_has_taken.md) | The zero consumers A2's import cost is currently free of |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Anomaly`'s four variants, and the attribute that let the fourth arrive additively |

### Invariants

| File | Relationship |
|------|--------------|
| [001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The R3 note that states the cost of a new anomaly variant, and declines it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every count above |
| [`Cargo.toml`](../../Cargo.toml) | The lint table A11 rests on |

### TK27 — the one type callers match on is the one type with no room to grow

`Anomaly` had three variants and no `#[ non_exhaustive ]`. It is the only type
this crate hands back for a caller to match: `Outcome::audit`, `audit_received`
and `audit_received_unordered` all return `Result< (), Anomaly >`, and the
natural way to read one is a `match` over the variants.

A fourth variant is therefore a breaking change for every such caller, and the
crate's own documents treat a fourth variant as a live option rather than a
hypothetical. [`../invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)
argues R3 covers duplicates and reorderings together because splitting them
*"would need two anomaly variants for a single observable"* — a cost stated in
variants, weighed, and declined. The thing that makes that cost breaking rather
than additive is the attribute that is not there.

The family knew the attribute: `ring_types::RingError` carried it, so it was
available and used where a type is expected to grow. One of thirty-three was a
deliberate scarcity, not an oversight — and the argument for scarcity was
weakest exactly here, where the type is an error enum returned across a crate
boundary by a crate whose purpose is to be depended on.

The exposure was free: no crate outside this one matches an `Anomaly`, because
no crate outside this one depends on `ring_testkit` at all
(→ [`../api/002`](../api/002_the_surface_no_crate_has_taken.md) TK7). It stops
being free at the first consumer, which is the moment the attribute can no
longer be added without the same breakage it would have prevented.

**Disposition:** applied — while it was still free, and the fourth variant this
entry treated as hypothetical arrived in the same change. `Anomaly` now carries
`#[ non_exhaustive ]` and a `# Not Exhaustive` doc section saying a `match` over
it needs a wildcard arm; the fourth variant is `Overdelivered { accepted, out }`,
which TK11 needed to separate `vanished()`'s two zeros. The order is the whole
point of doing it now: the attribute went on first, so the variant was additive
rather than breaking, and inside this crate the cost was the four `match` sites
in `Display` and the tests. Had the order been reversed — or had either landed
after the first consumer — it would have been a breaking release for every
caller matching on the three, which is exactly the failure this entry
predicted. What this does not buy: the attribute is not retroactive, so anything
that pinned a pre-attribute version and matched exhaustively still breaks on
upgrade; and it costs every future caller a wildcard arm, which silently absorbs
variants that a caller genuinely should have handled. Now prints:
`non_exhaustive     1`

**Correction (2026-09-20):** two counts in this file were wrong from the day
it was written, and both were falsifiable from evidence already on the page.

The first was in TK27's body, which said `ring_spsc` and `ring_types::RingError`
both carried `#[ non_exhaustive ]` — two of thirty-three. `ring_spsc` declares
no public enum and has never carried the attribute; its one `non_exhaustive`
hit is `.finish_non_exhaustive()`, a `Debug` formatter method. The recipe above
was the source: it matched the bare substring on any non-`//` line, so the
formatter call read as an attribute and the Live output named `ring_spsc` as
evidence. The pattern is now anchored to `^ *#\[ non_exhaustive`, which prints
two files rather than three, and the sentence above reads one of thirty-three.

The second was in A5, which read *"zero across all thirty-three `ring_*`
crates"* while the Live output eleven lines below already printed
`inline across the family: 1`. `ring_trace` carries `#[ inline ]` on its
disabled-path check, with a doc comment giving the reason. Here the recipe was
right and the prose ignored it — the exact inverse of A4's failure, in the same
table.

What the pair is worth: a count in prose is only as good as the pattern that
produced it, and a recorded Live output is evidence against the prose above it,
not decoration beneath it. Both checks are cheap, and neither was made.

### TK28 — the fixture's entry point takes a type the fixture does not export

`Script::run( &self, ring : &mut Ring< u32 > ) -> Outcome` is the function the
crate exists for, and `Ring` is a `ring_core` type. `ring_testkit` has **zero**
`pub use` declarations, so calling it requires `ring_core` in the caller's own
manifest — and building the `Ring` to pass requires `ring_config` as well, since
`Ring::new` takes a `&RingConfig`.

Three manifest entries to use a one-function fixture. That is what the crate's
own tests pay: `ring_config` and `ring_types` are dev-dependencies here for
exactly this reason, each with an explanatory comment
(→ [`../integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md)).

Thirty of thirty-three `ring_*` crates have zero `pub use`, so the absence is the
family's norm and not a deviation. What makes it worth recording here rather than
in each of the thirty is the direction of the crate: the other thirty are
depended *on* by a pipeline that already has the whole family in scope. This one
is a fixture, whose value is precisely that a crate outside its own dependency
closure can pick it up — and the shape it currently offers asks that crate to
take two more edges before it can call anything.

A `pub use ring_core::Ring;` would cost one line and remove one of the two. It
is not proposed here — a fixture re-exporting the type it drives is a real design
question, and the crate has no consumer yet to make the tradeoff concrete.
