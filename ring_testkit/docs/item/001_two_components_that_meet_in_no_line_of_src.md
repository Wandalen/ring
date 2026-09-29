# Item: Two Components That Meet In No Line Of `src/`

### Scope

- **Purpose**: Catalogue every public declaration as a set, and record the reachability relation between them — which turns out to have two components rather than one.
- **Responsibility**: The declarations that exist, their kinds, which of them reach which, and where `#[ must_use ]` was placed across the set.
- **In Scope**: Eight public declarations, four `impl` blocks, seven public methods, eight `#[ must_use ]` attributes.
- **Out of Scope**: What each signature guarantees (→ [`api/001`](../api/001_the_script_surface.md)); what the crate does not declare (→ [`002`](002_what_the_crate_does_not_declare.md)).

### The declarations

| # | Kind | Name | Component |
|---|---|---|---|
| 1 | `pub enum` | `Step` | scripted |
| 2 | `pub struct` | `Outcome` | scripted |
| 3 | `pub enum` | `Anomaly` | scripted |
| 4 | `pub struct` | `Script` | scripted |
| 5 | `pub fn` | `audit_received` | scripted |
| 6 | `pub fn` | `audit_received_unordered` | scripted |
| 7 | `pub fn` | `leak` | loom bridge |
| 8 | `pub fn` | `leak_ends` | loom bridge |

Four `impl` blocks — `Outcome`, `Script`, `Display for Anomaly`,
`Error for Anomaly` — and seven public methods between the first two:

| Owner | Methods |
|---|---|
| `Outcome` | `vanished`, `audit` |
| `Script` | `new`, `then`, `steps`, `stage_limit`, `run` |

Fifteen public items in one flat namespace. No `pub mod`, no `pub use`, no
`pub type`, no `pub const`, no trait declared here.

### The reachability relation

Inside `src/lib.rs`, the call graph over these eight declarations has **two
components** and no edge between them:

```text
scripted                              loom bridge
────────                              ───────────
Script::new ─→ Script::then ─→ Script::run ─→ Outcome
                                                │
                                                ├─→ Outcome::vanished
                                                └─→ Outcome::audit ─→ audit_received ─→ Anomaly
                                                                                          ▲
                                                              audit_received_unordered ───┘

                                      leak_ends ─→ leak ─→ Box::leak
```

`Script::run` never calls `leak`. `leak_ends` never produces an `Outcome`.
`Step` is constructed only by a caller; `Anomaly` is produced only by the three
audit functions. Two functions both halves could share —
`audit_received` and `audit_received_unordered` — sit at the boundary:
`audit_received` is called from `Outcome::audit` and from nowhere else in `src/`,
and `audit_received_unordered` is called from nowhere in `src/` at all. It stays
inside the scripted component only because it produces `Anomaly`; every one of
its callers is a test.

**Both components are used together only in `tests/`.** Each of the crate's two
test files imports `audit_received` and `leak_ends`: the loom model uses the
second to reach a thread and the first to check what that thread saw, and the
scripted suite exercises the bridge under the default configuration precisely
because the model compiles to nothing there
(→ [`../integration/002`](../integration/002_the_edge_that_only_exists_under_a_cfg.md)).

So the join exists and is tested twice. What has no line anywhere is a join
inside the library: `src/lib.rs` contains no expression in which a value from one
component reaches the other.

The crate's module documentation describes itself as having two halves. The
halves are not a metaphor — they are two disconnected subgraphs of the
public surface, joined only under a cfg.

### Where `#[ must_use ]` was placed

Eight attributes, all on functions and methods, none on a type. The first five
were there before TK26; the last three are what TK26 added, and each carries a
message rather than the bare form:

| Marked | Returns | Message |
|---|---|---|
| `Outcome::vanished` | `usize` | — |
| `Script::new` | `Self` | — |
| `Script::then` | `Self` | — |
| `Script::steps` | `&[ Step ]` | — |
| `Script::stage_limit` | `usize` | — |
| `leak` | `&'static mut Ring< T >` | names the leak and the lost handle |
| `leak_ends` | `( Producer, Consumer )` | names both allocations |
| `Script::run` | `Outcome` | names the discarded measurement |

Every public function that returns a plain value is now marked.
`Outcome::audit`, `audit_received` and `audit_received_unordered` return
`Result`, which the standard library already marks. → TK26.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'public declarations:     %s\n' "$( command grep -cE '^pub (struct|enum|fn|trait|type|const|mod|use) ' src/lib.rs || true )"
printf 'by kind:                 %s\n' "$( command grep -oE '^pub (struct|enum|fn|trait|type|const|mod|use)' src/lib.rs | sort | uniq -c | tr '\n' ' ' )"
printf 'impl blocks:             %s\n' "$( command grep -cE '^impl' src/lib.rs || true )"
printf 'public methods:          %s\n' "$( command grep -cE '^  pub fn ' src/lib.rs || true )"
printf 'must_use attributes:     %s\n' "$( command grep -c '#\[ must_use' src/lib.rs || true )"
printf 'what they sit above:     %s\n' "$( awk '/must_use/{ a = 1 } a && /pub fn /{ sub( /.*pub fn /, "" ); sub( /[(<].*/, "" ); printf "%s ", $0 ; a = 0 }' src/lib.rs )"
printf 'of those, carrying a msg: %s\n' "$( command grep -c 'must_use = ' src/lib.rs || true )"
printf 'calls to leak in src:    %s\n' "$( command grep -cE '^ +.*[^:]leak\( ' src/lib.rs || true )"
printf 'leak_ends calls leak at: %s\n' "$( awk '/^pub fn leak_ends/{f=1} f && /leak\( ring \)/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'the one shared function: %s\n' "$( command grep -m1 -oE 'audit_received\( &self\.received, self\.minted \)' src/lib.rs )"
printf 'both halves imported by: %s\n' "$( for f in tests/*.rs ; do command grep -q 'audit_received' "$f" && command grep -q 'leak_ends' "$f" && echo "${f#tests/}" ; done | tr '\n' ' ' )"
```

Live output:

```
public declarations:     8
by kind:                       2 pub enum       4 pub fn       2 pub struct 
impl blocks:             4
public methods:          7
must_use attributes:     8
what they sit above:     vanished leak leak_ends new then steps stage_limit run 
of those, carrying a msg: 3
calls to leak in src:    1
leak_ends calls leak at: let ends : &'static mut Ends< 'static, T > = Box::leak( Box::new( leak( ring ).ends() ) );
the one shared function: audit_received( &self.received, self.minted )
both halves imported by: exhaustive_test.rs testkit_test.rs 
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | What each of these signatures promises |
| [../api/002_the_surface_no_crate_has_taken.md](../api/002_the_surface_no_crate_has_taken.md) | Who calls them — nobody yet |

### Items

| File | Relationship |
|------|--------------|
| [002_what_the_crate_does_not_declare.md](002_what_the_crate_does_not_declare.md) | The absences this catalogue cannot list by enumerating |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The cfg the two components meet under |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration catalogued here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/exhaustive_test.rs` | Both components, joined under the cfg |
| `tests/testkit_test.rs` | Both components, joined without it |

### TK25 — the scripted half and the loom bridge share no line of the library

The crate's public surface is eight declarations, and the call graph over them
inside `src/lib.rs` is two disconnected subgraphs. `Script::new`, `then`, `run`,
`Outcome`, `vanished`, `audit`, `audit_received`, `audit_received_unordered`
and `Anomaly` form one. `leak_ends` and `leak` form the other, and the only
call between them is `leak_ends`' own
`Box::leak( Box::new( leak( ring ).ends() ) )`.

Nothing joins them in the library. `Script::run` builds its ring's ends from the
`&mut Ring< u32 >` a caller hands it and never leaks; `leak_ends` returns a pair
of ends and produces no `Outcome` to audit. The join happens twice, and both
times in `tests/`.

This is not an argument for joining them. The two halves answer different
questions — one runs a script many times and gets the same answer, the other
explores every interleaving of a tiny case — and the module documentation says
so plainly.

What it changes is how the crate should be read. The name suggests one fixture
with a bridge attached. The measurement says two fixtures in one crate, sharing a
manifest, a lint table and a `deny( missing_docs )`, and nothing else. A reader
looking for the seam between them will not find it, because there isn't one.

### TK26 — `#[ must_use ]` is on the five results that are cheap and none of the three that are not

Five attributes, on `vanished`, `Script::new`, `then`, `steps` and
`stage_limit` — four accessors and a builder pair, whose results a caller can
recompute for free by calling again.

Three public functions carry no `#[ must_use ]` and no standard-library one on
their return type:

| Function | Returns | Cost of discarding |
|---|---|---|
| `Script::run` | `Outcome` | The entire result of the run, and the ring has already been mutated |
| `leak` | `&'static mut Ring< T >` | The only handle to leaked memory |
| `leak_ends` | `( Producer, Consumer )` | Same, plus the ring behind them |

`script.run( &mut ring );` compiles as a statement. It is also the exact shape
of the mistake `run`'s own doc comment warns about — *"a caller comparing two
runs must supply two rings, not run twice on one"* — because the discarded first
run is what leaves the ring dirty for the second.

`leak( ring );` compiles too, and `leak`'s doc comment is the one place in the
crate that says **"Nothing frees this."** The function whose documented hazard is
that its result is unrecoverable is the function that lets you drop it silently.

`Outcome::audit` and `audit_received` are covered — they return `Result`, which
the standard library marks. The gap was precisely the three that return a plain
value, and they were the three where the value is the whole point.

**Disposition:** applied — to all three, each with a message rather than the bare
attribute, because the bare form says only *"unused return value"* and the whole
content of this finding is *what* is lost. `leak` reads "nothing frees this —
dropping the reference leaks the ring with no way to reach it again"; `leak_ends`
names both allocations; `Script::run` reads "the Outcome is the measurement —
`script.run( &mut ring );` as a statement drives the ring and discards everything
it observed", which is the sentence `run`'s own doc comment was already implying
one paragraph away. The census above needed fixing to see them: `grep -A1` reads
one line past the attribute, and an attribute carrying a message wraps, so the
old form of that line reported seven of eight and would have kept reporting the
five that fit on one line no matter how many were added. What this does not buy:
`let _ = script.run( &mut ring );` still compiles silently, and the discarded-run
mistake in [`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md)
is reachable through it — the attribute catches forgetting, never deciding.
Now prints: `of those, carrying a msg: 3`
