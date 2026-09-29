# Integration: Declared Edges and the Three That Were Missing

### Scope

- **Purpose**: Record the six dependency edges originally assigned to this crate, the three that had to be added before a signature could be written, and the Contract gap that made one of the three unavoidable.
- **Responsibility**: State each edge, why it exists, and what breaks without it.
- **In Scope**: The nine manifest lines; the reached closure and its regeneration command; `ring_flush::Flusher::new`'s argument type; the `Display` impl this crate's first compile forced into `ring_flush`.
- **Out of Scope**: The five-name export Contract itself, which is a decision for the family rather than this crate; whether `ring_handle` should expose `try_clone` (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)).

### System Description

`ring_bench` is the family's first consumer of more than one Contract crate at
once, and the last of the 33 to be written. Both facts shape what this instance
records: every crate it reaches is finished rather than a skeleton, and two gaps
that no single-crate consumer could have had a symptom for became compile errors
here.

### Integration Points

Six edges were assigned to this crate originally. All six survive:

| Crate | Reached by | What breaks without it |
|---|---|---|
| `ring_factory` | `Candidate::ContractRing`, `Candidate::OffTheShelf` | Two candidates, and `RingConfig` — the type a `Workload` is described in |
| `ring_tls` | `Candidate::TlsOverRing` | The staging buffer half of the staged candidate |
| `ring_flush` | `Candidate::TlsOverRing` | The publishing half of it |
| `ring_stats` | Every `Outcome` | `ring_stats`'s counters, which is the second feature this crate claims |
| `ring_spsc` | `Candidate::DirectSpsc` | The candidate that prices the dispatch above the backend |
| `ring_mpsc` | `Candidate::DirectMpsc` | The only in-house candidate with no producer ceiling |

**This is the first crate in the family where every declared edge was correct.**
[`ring_factory`](../../../ring_factory/docs/integration/001_declared_edges_and_the_reached_closure.md)
removed two of its five and added two; this one removed none. The reason is
positional rather than virtuous: a crate that consumes finished crates can be
specified accurately, and a crate specified against skeletons cannot.

#### The three that were added

Each because a signature could not be written without naming its types. This is
the **third occurrence in the family** of that exact reason, and the first where
the missing name sits on the export Contract's own argument list.

| Crate | Named in | Why nothing else would do |
|---|---|---|
| `ring_core` | `Flusher::new`'s second argument | `ring_flush::Flusher::new( buffer, producer, policy )` takes a `ring_core::Producer< '_, T >`, and `ring_flush` re-exports neither that type nor a way to obtain one |
| `ring_slot` | `ring_spsc::Ring< S >`, `ring_mpsc::Ring< S >` | Both are generic over `Slot`, and `TypedSlot< T >` is the only implementor this benchmark exercises. The two direct candidates cannot name their own ring type without it |
| `ring_types` | `RingStats::record_drop` | It takes an `OverflowPolicy`, so per-run counters cannot be written without the policy vocabulary |

**`ring_slot` and `ring_types` are ordinary.** They are internal family crates,
this crate reaches below the Contract by design for the direct candidates, and
naming the implementor this benchmark actually exercises is not a finding.

**Correction (2026-09-20):** the `ring_slot` row quoted the manifest as saying
`TypedSlot< T >` is "the only implementor", and this paragraph concluded from
that wording that naming "a generic parameter's only implementor" is not a
finding. Neither was accurate: `ring_slot` declares two implementors,
`TypedSlot< T >` at `ring_slot/src/lib.rs:176` and `BytesSlot< N >` at
`:385`. The manifest has since been corrected to scope the claim to what this
benchmark exercises and to name the second implementor explicitly; the quote and
the conclusion above are updated to match it. The edge itself is unaffected —
`ring_slot` is still named because the two direct candidates cannot write their
own ring type without it — but the coverage gap the old wording concealed is a
finding, recorded as EV34 in
[`ring_event/non_functional_requirement/001`](../../../ring_event/docs/non_functional_requirement/001_the_indirection_that_is_not_there.md).

**`ring_core` is not ordinary, and it is this instance's principal finding.**

#### Two Contract names that do not compose

**`ring_factory` and `ring_flush` are both on the five-name export surface:**

```sh
cd "$(git rev-parse --show-toplevel)"
cat bench_harness/gate/declared/ring/export_surface.txt
```

Live output:

```
# Ring family export surface — the only ring_* crates a consumer outside
# ring_* may name as a dependency.
#
# Ruled by docs/decision/121_workstream_008_contract_gaps_ruled.md § 4, which
# upholds the five-crate Contract set by decision/120 and rules that features
# 171 (SPSC), 172 (MPSC) and 181 (registry) are reached *through* this surface
# rather than by importing ring_spsc, ring_mpsc or ring_registry directly:
# ring_factory constructs them and hands back ring_handle values.
#
# The other 28 crates are internal to the family and freely refactorable.
# Widening this file widens the workstream's contract — do not add a name
# without a decision that says so.

ring_factory
ring_handle
ring_tls
ring_flush
ring_types
```

**A consumer bound to those five names cannot build the staged write path**,
which is one of the four candidates this crate names. The chain breaks at one
type:

| Step | Type produced | Type required | Reachable? |
|---|---|---|---|
| `Factory.build::< T >( cfg )` | `ring_handle::Split< T >` | — | ✅ on the Contract |
| `split.ends().split()` | `( ring_handle::Producer, ring_handle::Consumer )` | — | ✅ on the Contract |
| `Flusher::new( buf, ?, policy )` | — | **`ring_core::Producer`** | ❌ not on the Contract |

Nothing on the Contract converts a `ring_handle::Producer` into a
`ring_core::Producer`, and `ring_flush` re-exports neither the type nor a
constructor for one. So `Candidate::TlsOverRing` builds its ring by calling
`ring_core::Ring::new` **directly, one level below the door the Contract names**
— which is exactly what
[`ring_factory/docs/invariant/002`](../../../ring_factory/docs/invariant/002_construction_is_the_only_path.md)
enumerates as a leak, now with a fifth entry and, for the first time, a caller
that had no alternative.

**This is a different shape of gap from the ones already recorded.** The four
leaks that invariant catalogues are *convenience* paths — `Ring::new` and
`Ring::with_config` exist and are easier than the factory. This one is a
*necessity* path: following the Contract and building the thing are
incompatible, so the leak is not a discipline failure but the only available
route. **A chokepoint with an unreachable case is not a chokepoint that leaks;
it is a chokepoint with a hole in the Contract.**

Three candidate closures, none of which this crate can rule:

| Option | Cost |
|---|---|
| `ring_flush` re-exports `ring_core::Producer` | One line, and it puts a sixth crate's type on a Contract name's public surface without adding the crate to the Contract |
| `ring_flush::Flusher` accepts a `ring_handle::Producer` | Changes a Contract crate's primary constructor; `ring_flush` would gain a `ring_handle` dependency it does not have |
| `ring_factory` grows a `build_staged` door | Consistent with `build_crossbeam`, and puts the composition where the other doors are |

Filed for the family rather than decided here — this crate is the consumer that
found the gap, not the owner of any of the three surfaces.
→ [`decisions/readme.md`](../decisions/readme.md), Pending 1.

### Error Handling

Every refusal that crosses one of the six edges above is re-rendered by
`RunError` and never re-decided — the wrapping variant names the route, the inner
error keeps its own wording. That only works if each dependency's error type can
actually be printed, and one of them could not.

#### The `Display` impl this crate forced

**`ring_flush::ConfigError` implemented neither `Display` nor `Error` until this
crate's first compile.** `RunError` wraps three dependency refusals —
`BuildError` from `ring_factory`, `RingError` from `ring_types`, and
`ConfigError` from `ring_flush` — and the first two already rendered. The third
did not, so `RunError`'s own `Display` would not compile.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'impl core::fmt::Display for ConfigError' ring_flush/src/lib.rs
command grep 'impl core::error::Error for ConfigError' ring_flush/src/lib.rs
```

Live output:

```
impl core::fmt::Display for ConfigError
impl core::error::Error for ConfigError {}
```

**Three error types on the same Contract, and one of them could not be printed.**
It was invisible while `ring_flush`'s only consumer was its own test suite,
where `Debug` is sufficient and `assert_eq!` never asks for a rendering. It
became a compile error the moment a crate held all three at once and tried to
fold them into a `Box< dyn Error >`.

**The finding is about who can see a gap like this.** No test in `ring_flush`
could have failed for it, no gate greps for it, and the crate's own
documentation was accurate. It takes a *consumer* — and specifically a consumer
of more than one Contract crate — for the omission to have a symptom. This crate
is the family's first, which is why both this gap and the composition gap above
surfaced in the same afternoon.

The fix landed in `ring_flush` with a covering test
(`a_binding_refusal_renders_and_chains`) rather than being worked around here,
because a `Display` impl belongs to the type's own crate and a local
`impl Display for RunError` that formatted the wrapped variant by hand would
have hidden the gap from the next consumer.

### Compatibility Requirements

| Requirement | Consequence if it stops holding |
|---|---|
| A candidate's construction must go through the door its `Candidate` variant names | The `Build`/`Ring` split stops being evidence about the Contract, and a test changes rather than a paragraph going stale |
| Reaching below the export Contract stays confined to the direct candidates and the staged one | This crate is not an example of Contract-bound consumption and must not be read as one; gate G5 permits the reach only because it skips `ring_*` manifests |
| A dependency's error type must implement `Display` and `Error` before it can be wrapped | `RunError`'s own `Display` stops compiling — which is exactly how the `ring_flush` gap surfaced |
| Every count in this document is regenerated, never quoted | See below |

#### The reached closure

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_bench --all-features -e normal --prefix none \
  | command grep -oE '^ring_[a-z_]+' | sort -u | wc -l
```

Live output:

```
23
```

Regenerate rather than trust the number in any prose: `ring_factory`'s own
integration instance records its closure moving twice during authoring, and the
lesson taken from it is that a count in a document is a snapshot with no
mechanism to notice it has expired. **This crate is the terminal case for that
hazard** — it is the last of the 33, so every crate below it carries a real
implementation and no claim here is a prediction:

```sh
cd "$(git rev-parse --show-toplevel)"
( for c in ring_*/; do
  [ "$( cat "$c"src/*.rs | command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' )" = 0 ] \
    && basename "$c"
done ) || true
```

Live output:

```
```

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_five_candidates_for_four_named_paths.md](../decisions/001_five_candidates_for_four_named_paths.md) | Why the direct candidates exist, which is what makes `ring_slot` and the two backend crates reachable |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | The other Contract finding — the producer ceiling `ring_handle` imposes |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_run_error.md](../type/002_run_error.md) | The four-variant refusal, three variants of which relay a dependency's own error |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | The five names, and gate G5's enforcement of them |
| [`ring_flush/src/lib.rs`](../../../ring_flush/src/lib.rs) | `Flusher::new`'s signature, and the `Display`/`Error` impls added for this crate |
| [`ring_factory/docs/invariant/002`](../../../ring_factory/docs/invariant/002_construction_is_the_only_path.md) | The four leaks this instance adds a fifth to, and the first that is a necessity rather than a convenience |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `a_policy_refusal_names_the_crate_that_refused` asserts that the staged candidate's refusal arrives as `RunError::Ring` rather than `RunError::Build` — which is the composition gap visible as a behavioural difference rather than as a paragraph |
| [`ring_flush/tests/flush_test.rs`](../../../ring_flush/tests/flush_test.rs) | `a_binding_refusal_renders_and_chains` covers the `Display` impl this crate forced |

### BN17 — The Only Cross-Crate Test Citation in This Crate Is Written in the Schema No Checker Reads

The last row of this document's own `### Tests` table cites a test in another
crate — `ring_flush/tests/flush_test.rs`, the covering test for the `Display`
gap above. It has to be written the way it is, and the way it is written is the
way `citations.py` cannot see:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the checker matches only the first cell, and only as a bare name --'
command grep -m1 -F 'TEST_ROW = re.compile( r'"'"'^\| `?([a-z_][a-z0-9_]*)`? \|'"'"' )' bench_harness/gate/corpus/citations.py | sed 's/^/    /'
echo '  -- rows inside a "### Tests" section in this crate, by whether that matches --'
python3 -c "
import pathlib, re
R = re.compile( r'^\| \`?([a-z_][a-z0-9_]*)\`? \|' )
hit = mis = 0
for p in sorted( pathlib.Path( 'ring_bench/docs' ).rglob( '*.md' ) ):
  inside = False
  for ln in p.read_text().splitlines():
    if ln.startswith( '### Tests' ) : inside = True; continue
    if inside and ln.startswith( '###' ) : inside = False
    if not inside or not ln.startswith( '| ' ) : continue
    if ln.startswith( ( '|--', '| File ', '| Test ' ) ) : continue
    if R.match( ln ) : hit += 1
    else             : mis += 1
print( f'    cross-checked against tests/ : {hit}' )
print( f'    never cross-checked          : {mis}' )
"
echo '  -- the row in question, and the test it names --'
command grep -nE '^\| \[.*flush_test\.rs' ring_bench/docs/integration/001_declared_edges_and_the_three_that_were_missing.md \
  | sed -E 's/^(.{0,104}).*/\1/' | sed 's/^/    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -n 'fn a_binding_refusal_renders_and_chains' ring_flush/tests/flush_test.rs \
  | sed 's|^|    ring_flush/tests/flush_test.rs:|' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- the checker matches only the first cell, and only as a bare name --
    TEST_ROW = re.compile( r'^\| `?([a-z_][a-z0-9_]*)`? \|' )
  -- rows inside a "### Tests" section in this crate, by whether that matches --
    cross-checked against tests/ : 17
    never cross-checked          : 21
  -- the row in question, and the test it names --
    | [`ring_flush/tests/flush_test.rs`](../../../ring_flush/tests/flush_test.rs) | `a_binding_refusal_r
    ring_flush/tests/flush_test.rs:fn a_binding_refusal_renders_and_chains()
```

`citations.py` reads only rows inside a `### Tests` heading, and only matches a
first cell that is a bare or backticked lowercase identifier. A first cell that
is a markdown link to a path — the shape the last row of this document's own
`### Tests` table uses, because the file it cites is in another crate — does not
match, so the row is skipped in silence. There is no `WHERE`, no `TEST`, no
warning that a table row went unread.

**That silence is load-bearing, not incidental.** The checker resolves every
name it *does* match against `docs.parent / 'tests'` — this crate's own test
directory. A row naming `a_binding_refusal_renders_and_chains` in the checked
schema would be reported as citing a test that does not exist, because it lives
in `ring_flush`. The only way to record a true cross-crate citation is to write
it in the schema that is never checked.

So the two schemas are not stylistic alternatives. One is checkable and
crate-local; the other is uncheckable and the only one that can express an edge.
This crate has 38 rows under `### Tests` headings split 17/21 between them, and
the split is invisible from any single document.

The general shape: **a checker scoped to one crate makes cross-crate citations
unrepresentable in its own schema**, so they migrate to whichever schema it
ignores — and the corpus ends up with the citations that matter most recorded in
the format with the least verification.

### BN18 — Every Error on This Path Implements `Error` With an Empty Body, So Nothing Chains

The Compatibility Requirements table above states that a dependency's error type
must implement `Display` and `Error` before `RunError` can wrap it, and records
the `ring_flush` gap that requirement caught. Both impls landed. Neither
implements `source`:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the Error impls on the RunError wrapping path --'
command grep -r 'impl core::error::Error for' ring_bench/src/lib.rs ring_flush/src/lib.rs ring_factory/src/lib.rs ring_types/src/lib.rs | sed 's/^/    /'
echo '  -- family-wide --'
printf '    Error impls in ring_*/src            : %s\n' "$( command grep -rho 'impl core::error::Error for' ring_*/src/ | wc -l )"
printf '    of those, empty-bodied                      : %s\n' "$( command grep -rhoE 'impl core::error::Error for [A-Za-z]+ \{\}' ring_*/src/ | wc -l )"
printf '    "fn source(" anywhere in ring_*/src   : %s\n' "$( command grep -rho 'fn source(' ring_*/src/ | wc -l )"
echo '  -- what the covering test asserts --'
awk '/^fn a_binding_refusal_renders_and_chains/, /^\}/' ring_flush/tests/flush_test.rs | command grep -E 'assert|source|Box' | sed 's/^/    /'
```

Live output:

```
  -- the Error impls on the RunError wrapping path --
    ring_bench/src/lib.rs:impl core::error::Error for WorkloadError {}
    ring_bench/src/lib.rs:impl core::error::Error for RunError {}
    ring_flush/src/lib.rs:impl core::error::Error for ConfigError {}
    ring_factory/src/lib.rs:impl core::error::Error for BuildError {}
  -- family-wide --
    Error impls in ring_*/src            : 8
    of those, empty-bodied                      : 8
    "fn source(" anywhere in ring_*/src   : 0
  -- what the covering test asserts --
      assert_eq!
      assert_eq!
      let boxed : Box< dyn core::error::Error > = Box::new( ConfigError::ZeroBatch );
      assert!( boxed.to_string().contains( "every append" ) );
```

`impl core::error::Error for X {}` takes the default `source`, which returns
`None`. So `RunError::Build( … )` renders its inner `BuildError`'s message
through `Display` and then reports no cause at all. A caller walking the chain —
the standard idiom for finding an error's origin — gets exactly one hop and
stops, at the wrapper, which is the layer that knows least about what went wrong.

**The covering test is named `a_binding_refusal_renders_and_chains` and does not
test chaining.** It boxes a `ConfigError` into a `Box< dyn core::error::Error >`
and asserts the rendering survives. That is coercion, not chaining: the value is
already the leaf, so there is nothing below it to reach and the assertion would
pass identically against an error type with no cause in the world. The name
describes an idiom the code does not implement, and the test that carries the
name is the reason nobody looked.

**Eight out of eight `Error` impls in the family are empty, and `fn source` does
not appear once in any of the 33 crates.** So this is a family-wide property
rather than a `ring_bench` oversight — but this crate is where it costs
something, because it is the only crate that wraps refusals from three other
crates and re-renders them under one type. Everywhere else the error is its own
leaf and the missing chain has no observable consequence.

The general shape, which is the same one the `Display` gap had and one layer
further in: **a trait satisfied by an empty impl is satisfied by the compiler and
unsatisfied by the caller**, and the gap is invisible until some crate is the
first to actually consume the trait's other half.
