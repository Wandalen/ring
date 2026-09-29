# docs

Design documentation for `ring_handle`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The split that creates every guarantee, and the delegation path that must add nothing |
| `api/` | Two caller surfaces, documented as much by what is absent from them as by their operations |
| `data_structure/` | One field per handle, why only one of its three candidate shapes was ever reachable, and the single struct that is not a newtype |
| `decisions/` | Why this crate exists given `ring_core` already splits, and why `is_closed()` is absent |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | One dependency concealing three backends with different contracts, and an export boundary this crate sits *on* |
| `invariant/` | Two restrictions on one surface, enforced by absence along two independent axes |
| `item/` | The five nouns and twelve verbs the surface consists of, catalogued against what each wraps |
| `lifecycle/` | The pair's arc from split to drop, and ownership and liveness as two orthogonal axes that combine badly in one state |
| `non_functional_requirement/` | This crate's binary Reached condition, split into the two independent properties it bundles |
| `pattern/` | Enforce by withholding rather than by checking, and the three-way rule every method in the crate obeys |
| `pitfall/` | Eight reasonable-looking edits, five of which nothing catches even after the suite was widened, and a pinned diagnostic nobody declared |
| `type/` | Two rights expressed as values, whose absent trait impls are as much a part of them as their fields |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

**This crate's guarantees are absences, and that shapes every directory above.**
`Producer` cannot drain; `Consumer` cannot publish; neither can park, clone, or
reach the backend. There is no runtime check anywhere — the operation a caller
is not entitled to does not exist on the value it holds, which this crate's
own design describes as making "who is allowed to consume a question answered
by ownership rather than by convention."

Twelve of the thirteen definitions sit at three grains. `invariant/` and
`non_functional_requirement/` document the **contract grain** — the two
restrictions and the binary Reached condition
[the acceptance table](../../bench_harness/docs/acceptance/001_feature_reached_tests.md)
grades them by. `algorithm/`, `data_structure/`, `pattern/`, `pitfall/` and
`workaround/` document the **mechanism grain**: how a partition is created once,
preserved by adding nothing, and asserted by a compilation that has to fail on
purpose. `api/`, `item/`, `lifecycle/`, `type/` and
`integration/` document the **surface grain** — what a caller touches, when, and
across which crate boundary. `decisions/` is the thirteenth and belongs to none
of the three: it records what was chosen, not what holds.

**The surface grain is expensive here, and the docs commit accordingly.**
`ring_handle` is one of the family's five exported crates,
so a signature change reaches every consumer rather than stopping two crates
away. That is the exact inverse of
[`ring_spsc`](../../ring_spsc/docs/readme.md), whose `api/` instances leave
three candidate shapes open precisely because it is internal. Same family, same
week, opposite treatment — decided entirely by which side of a five-name list a
crate falls on (→ [`integration/002`](integration/002_on_the_export_surface.md)).

**Two findings run through the whole set and are worth stating up front.**

First, **the acceptance criterion catches the edit nobody makes and misses the
ones that happen.** This crate's own specification mandates two `trybuild` compile-fail cases: a
drain on `Producer`, a publish on `Consumer`. Both are obviously wrong to their
author. The edits that actually arrive — `#[derive(Clone)]`, an `impl Deref`, a
`pub fn inner()`, a `try_push_timeout( Duration )` — all compile and pass every
positive test
(→ [`pitfall/001`](pitfall/001_a_convenience_method_undoes_the_crate.md)).

**Acted on rather than recorded.** The suite here has five cases, not two: the
extra three pin `!Clone` on both handles and `!Sync`, which are the violations
that produce a data race rather than an error someone eventually sees. Writing
a stricter local test needed no amendment to `bench_harness`'s shared table —
a criterion the crate exceeds is not a criterion the crate changed. `Deref` and
an accessor are still uncovered, and for a structural reason: a compile-fail
case names one thing that must not exist, and those two violations are "any
route", which has no name to write down.

Second, **this crate breaks other crates, never itself.** A `Clone` added here
is a data race in `ring_spsc`; a counter added in a forwarding method fails
`ring_spsc`'s zero-RMW assertion from a crate that does not appear in its
dependency tree at all; a blocking method deadlocks a tick and is caught, if at
all, by `ring_poll`'s test. Adding a method cannot make a positive test fail,
so this crate's own suite goes green through every one of them.

No `format/` directory exists: this crate defines no byte layout. It holds one
reference per handle and forwards — 24 bytes for a `Producer`, 16 for a
`Consumer`, measured rather than estimated
(→ [`data_structure/001`](data_structure/001_two_handles_over_one_backend.md)).

An `item/` directory now exists, and the paragraph that stood here explaining
why one did not was wrong twice over. It said the deferral was family-grain —
"**no implemented crate in the family carries one**" — and that adding one here
alone "would make this crate's docs inconsistent with every sibling's rather
than more complete". Both halves are refuted by the same command it cited:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates in the family:        %s\n' "$( ls -d ring_*/ | wc -l )"
printf 'of them carrying docs/item:  %s\n' "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )"
printf 'public types here:           %s\n' "$( command grep -cE '^\s*pub struct ' ring_handle/src/lib.rs )"
printf 'public methods here:         %s\n' "$( command grep -cE '^\s*pub (const )?fn ' ring_handle/src/lib.rs )"
```

Live output:

```
crates in the family:        33
of them carrying docs/item:  33
public types here:           5
public methods here:         12
```

All thirty-three carry one — and the twenty-seven this paragraph recorded when
it was repaired was itself overtaken the same way, which is the point rather
than an aside. The count moved from zero while a sentence asserting zero sat in
a file that nothing about the move would touch — the same shape recorded in
[`definition/readme.md`](definition/readme.md), and the reason a claim about
*other* crates does not belong in *this* readme at all. The method figure was
independently wrong: the crate declares five types and **twelve** public
methods, not sixteen, which is exactly what
[`item/002`](item/002_twelve_verbs_eight_bare_forwards.md) catalogues.

### Related Crates

One declared dependency, four crates above that drive these handles, two
conspicuous absences, and one crate whose correctness depends on a derive this
crate must never add.

| Crate | Relationship |
|-------|--------------|
| [`ring_core/readme.md`](../../ring_core/readme.md) | The only dependency — the composition point over three backends whose contracts differ in ways one uniform surface cannot express |
| [`ring_factory/readme.md`](../../ring_factory/readme.md) | Constructs the pair and returns it; both crates are on the Contract, which is what lets that signature be written |
| [`ring_poll/readme.md`](../../ring_poll/readme.md) | Claims the row for the non-parking constraint, which **constrains this crate** — the obligation runs opposite to the dependency arrow, so nothing here goes red when it breaks |
| [`ring_flush/readme.md`](../../ring_flush/readme.md) | Decides *when* to publish through a `Producer`; the handle deliberately has no flush method |
| [`ring_shutdown/readme.md`](../../ring_shutdown/readme.md) | Owns `close`/`reset`/`drain_all` and the single authoritative liveness flag both handles read rather than cache |
| [`ring_stats/readme.md`](../../ring_stats/readme.md) | **Conspicuously absent.** This crate is the natural chokepoint for instrumentation, and a counter here is an RMW on the hot path |
| [`ring_batch/readme.md`](../../ring_batch/readme.md) | **Conspicuously absent.** A handle that buffers has a `Drop` that can lose data; the buffer's lifetime belongs where it is explicit |
| [`ring_spsc/docs/readme.md`](../../ring_spsc/docs/readme.md) | The crate that dies if `Producer` becomes `Clone`. Its cardinality invariant is stated there and enforceable only here |
| [`ring_types/readme.md`](../../ring_types/readme.md) | Also on the Contract, and jointly so: a public signature here may mention its discriminants only because it is exported too |
