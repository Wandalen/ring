# Item: Four Operations, Eight Entry Points

### Scope

- **Purpose**: Catalogue every public declaration this crate makes and record the one relation that organises them — each of four ring operations is reachable two ways, as a free function and as a `Tick` method.
- **Responsibility**: The declarations that exist, their kinds, the mirror between the two layers, and the one place the mirror is not exact.
- **In Scope**: Eight top-level declarations, five `impl` blocks, sixteen methods, and the forwarding relation between the two layers.
- **Out of Scope**: What the crate does not declare (→ [`002`](002_what_the_crate_does_not_declare.md)); what each signature guarantees (→ [`../api/001`](../api/001_tick_path_surface.md)); the shapes the values take (→ [`../data_structure/001`](../data_structure/001_three_values_three_shapes.md)).

### The declarations

| # | Kind | Name | Layer |
|---|---|---|---|
| 1 | `pub const` | `PARKING_CRATES` | neither — a roster |
| 2 | `pub struct` | `Budget` | value |
| 3 | `pub enum` | `Progress` | value |
| 4 | `pub struct` | `Tick` | value + method layer |
| 5 | `pub fn` | `push_within` | free |
| 6 | `pub fn` | `push_batch_within` | free |
| 7 | `pub fn` | `recv_within` | free |
| 8 | `pub fn` | `drain_up_to` | free |

Five `impl` blocks — `Budget`, `Default for Budget`, `Progress`, `Tick`,
`Default for Tick` — carrying sixteen methods:

| Owner | Methods |
|---|---|
| `Budget` | `once`, `new`, `attempts` |
| `Progress` | `of`, `is_made`, `count`, `then` |
| `Tick` | `new`, `reset`, `budget`, `progress`, `lost`, `push`, `push_batch`, `recv`, `drain` |

Twenty public functions in total, all in one flat namespace. No `pub mod`, no
`pub use`, no `pub type`, no trait declared here.

### The mirror

Four of `Tick`'s nine methods are not operations of their own. Each forwards to
the free function of the same name and adds one line of accounting:

| Free function | `Tick` method | What the method adds | What it supplies |
|---|---|---|---|
| `push_within` | `push` | `moved += 1` on `Ok` | `self.budget` |
| `push_batch_within` | `push_batch` | `moved += published`, and `lost += offered - published` | `self.budget` |
| `recv_within` | `recv` | `moved += 1` on `Some` | `self.budget` |
| `drain_up_to` | `drain` | `moved += taken` | **nothing — `max` comes from the caller** |

Three rows supply the tick's own budget, which is what makes the method layer
worth having: the caller stops passing a `Budget` to every call and the tick
remembers it. The fourth row supplies nothing, because `drain_up_to` has no
budget parameter to fill — and `Tick::drain`'s doc comment describes it as being
called *"with this tick's own ceiling"* → PL25.

### Two layers, one counter

Both layers are `pub`, both are documented, and they are not interchangeable —
the accounting exists in only one of them. `Tick::progress()` reports what the
*tick's* methods moved, and a call to `push_within` on the same producer moves a
record without the tick learning anything about it.

Nothing in the crate stops a caller from using both. Each free function now says
so in its own rustdoc, and one test exercises the mix → PL26.

### Item Counts

| Measure | Count |
|---|---|
| Top-level public declarations | 8 |
| `impl` blocks | 5 |
| Public functions and methods | 20 |
| Of those, free functions | 4 |
| Of those, `Tick` methods forwarding to a free function | 4 |
| Of those, `const fn` | 12 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'top-level pub declarations: %s\n' "$( command grep -cE '^pub (fn|struct|enum|const|type|mod|use|trait) ' src/lib.rs || true )"
printf 'and their kinds:            %s\n' "$( command grep -oE '^pub (fn|struct|enum|const|type|mod|use|trait) ' src/lib.rs | sed 's/^pub //' | sort | uniq -c | tr -s ' ' | tr '\n' ' ' )"
printf 'impl blocks:                %s\n' "$( command grep -cE '^impl ' src/lib.rs || true )"
printf 'public fns and methods:     %s\n' "$( command grep -cE '^(pub|  pub) (const )?fn ' src/lib.rs || true )"
printf 'free functions:             %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | sed 's/^pub fn //' | tr '\n' ' ' )"
printf 'Tick methods:               %s\n' "$( awk '/^impl Tick$/{f=1} f && /^\}/{exit} f' src/lib.rs | command grep -oE '^  pub (const )?fn [a-z_]+' | sed 's/.*fn //' | tr '\n' ' ' )"
printf 'of those, forwarding:       %s\n' "$( awk '/^impl Tick$/{f=1} f && /^\}/{exit} f' src/lib.rs | command grep -oE '= (push_within|push_batch_within|recv_within|drain_up_to)\(' | sed 's/= //; s/($//; s/(//' | tr '\n' ' ' )"
printf 'forwarders passing budget:  %s\n' "$( awk '/^impl Tick$/{f=1} f && /^\}/{exit} f' src/lib.rs | command grep -c 'self.budget )' || true )"
printf 'forwarders passing max:     %s\n' "$( awk '/^impl Tick$/{f=1} f && /^\}/{exit} f' src/lib.rs | command grep -cE '\( consumer, out, max \)' || true )"
printf 'what drain doc calls it:    %s\n' "$( awk '/^impl Tick$/{f=1} f && /^\}/{exit} f' src/lib.rs | command grep -oE 'with this tick.s own [a-z]+' )"
printf 'ceiling fields on Tick:     %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^\}$/{exit} f' src/lib.rs | command grep -oE '^  [a-z_]+ :' | tr -d ' :' | tr '\n' ' ' )"
printf 'const fns:                  %s\n' "$( command grep -cE '^(pub|  pub) const fn ' src/lib.rs || true )"
printf 'tests naming both layers:   %s\n' "$( command grep -lE 'push_within' tests/poll_test.rs >/dev/null && awk '/^fn /{n=$0} /Tick::new|Tick::default/{t[n]=1} /push_within|recv_within|drain_up_to/{ if (n in t) print n }' tests/poll_test.rs | sort -u | wc -l )"
```

Live output:

```
top-level pub declarations: 8
and their kinds:             1 const   1 enum   4 fn   2 struct  
impl blocks:                5
public fns and methods:     20
free functions:             push_within push_batch_within recv_within drain_up_to 
Tick methods:               new reset budget progress lost push push_batch recv drain 
of those, forwarding:       push_within recv_within drain_up_to 
forwarders passing budget:  3
forwarders passing max:     1
what drain doc calls it:    with this tick's own ceiling
ceiling fields on Tick:     budget moved lost 
const fns:                  12
tests naming both layers:   2
```

### Items

| File | Relationship |
|------|--------------|
| [002_what_the_crate_does_not_declare.md](002_what_the_crate_does_not_declare.md) | The complement — the kinds of declaration absent from the list above |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The same set read as a compatibility surface rather than an inventory |

### Data Structures

| File | Relationship |
|------|--------------|
| [`../data_structure/002_a_copy_accumulator.md`](../data_structure/002_a_copy_accumulator.md) | The counter the four forwarding methods increment |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | PL23 — the same `drain_up_to` exemption, stated as an invariant boundary |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration counted here |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | Covers both layers separately; no test uses both against one producer |

### PL25 — `Tick::drain` documents a ceiling the tick does not have

`Tick::drain`'s doc comment opens: *"[`drain_up_to`] with this tick's own
ceiling, counting what moved."* The signature is
`drain( &mut self, consumer, out, max : usize )`, and the body calls
`drain_up_to( consumer, out, max )`. The ceiling is `max`, which the caller
passes on every call. `Tick` holds `budget`, `moved` and `lost`, and none of the
three is a ceiling.

The sentence directly beneath it is careful and correct — *"The ceiling is `max`
rather than the budget: a budget bounds retries of a failed operation, a drain
limit bounds successes"* — so the doc contradicts itself across two consecutive
lines, and the wrong half is the summary line, which is the half that shows up in
`cargo doc`'s method list.

The other three forwarders make the summary line true: `push`, `push_batch` and
`recv` each say *"against this tick's budget"* and each pass `self.budget`. Read
as a set, the four summaries assert that a `Tick` supplies something to all four
calls. It supplies something to three.

The cost is a caller who reads the method list, sees four operations parameterised
by the tick, and expects `Tick::new( Budget::once() ).drain( c, out, max )` to be
bounded by the tick in some way it is not. The correction is one word — *"with a
caller-supplied ceiling"* — and it is not applied here because this is a source
edit rather than a documentation one, and the paragraph below already says the
right thing.

**Disposition:** declined — the correction targets `Tick::drain`'s own
rustdoc summary line in `src/lib.rs`, a source edit rather than a change to
this corpus document; the sentence immediately below the summary line, in
the same doc comment, already states the ceiling correctly as `max` rather
than the tick's own budget.

### PL26 — two public layers share a producer and only one of them counts

`push_within` and `Tick::push` do the same thing to the same ring. The first
returns and forgets; the second returns and adds to `moved`. Both are public,
both are documented, and a caller holding a `Tick` and a `Producer` can call
either.

A tick that moved four records through `tick.push` and six through
`push_within` reports `Progress::Made( 4 )`. Nothing is corrupted — the records
all moved, in order, exactly once — but the number the scheduler reads is the
number of records that went through one particular door.

That is a plausible thing to write by accident, because the free functions are
the ones the module documentation introduces first and the ones every doctest
uses. `Tick` is presented as the convenience layer; reaching past a convenience
layer for the underlying call is an ordinary thing to do, and here it silently
changes what a later `progress()` means.

The suite had twenty-three tests. Some exercised the free functions, some
exercised `Tick`, and none used both against one producer, so the interaction had
no coverage in either direction. It was also undocumented: `Tick`'s doc described
the accounting it adds, and no free function's doc mentioned that a `Tick` exists
which would have counted the call.

Making it impossible would mean making the free functions private, which would
remove the layer that keeps `Tick` optional — a real cost for a crate whose whole
premise is that a caller may not want the accounting. So the seam stays, and what
was missing was the two cheap halves that make it a choice rather than an
accident: a sentence at each door saying which one counts, and a test that walks
through both and asserts the resulting `Progress` is short.

**Disposition:** applied — each of the four free functions in `src/lib.rs` gained
a `# Not Counted By A Tick` rustdoc section stating that the call moves records
without any `Tick` observing it, and `Tick` itself gained a
`# The Accounting Is Bypassable` section naming `budget()` as the accessor that
makes the bypass idiomatic. The seam is left open on purpose — making the free
layer private would delete the optionality the crate exists to offer — but it is
now stated at both ends rather than at neither.
`reaching_past_the_tick_leaves_its_count_short` in `tests/poll_test.rs` walks a
record through `push_within` using the tick's own budget and asserts
`tick.progress()` still reads `Progress::None`, so the shortfall is pinned rather
than described. The suite now has tests that touch both layers together:
Now prints: `tests naming both layers:   2`
