# Item: Seven Nouns, Thirteen Variants, and One Measured Width

### Scope

- **Purpose**: Catalogue every type this crate declares, against what it wraps, how wide it is, and which traits it carries — so that a claim about any of the three can be checked rather than believed.
- **Responsibility**: The seven nouns, their thirteen variants, the four measured widths, and the derive lists.
- **In Scope**: `pub struct` and `pub enum` declarations in `src/lib.rs`, their fields, variants, widths and trait impls.
- **Out of Scope**: The methods (→ [`002`](002_seventeen_verbs_that_never_touch_the_producer.md)); why `FlushPolicy` is a value rather than a call site (→ [`pattern/001`](../pattern/001_policy_as_a_value.md)); why `FlushOutcome` is not a `Result` (→ [`type/002`](../type/002_flush_outcome.md)).

### The Seven

Four enums and three structs. The split is not decorative: **every enum is a
vocabulary term a consumer matches on, and every struct is state somebody owns.**

| Noun | Kind | Variants / fields | Width | Owns |
|------|------|-------------------|------:|------|
| `FlushPolicy` | enum | 3 variants, one carrying `usize` | 16 | Nothing — pure configuration |
| `FlushCause` | enum | 4 variants, all unit | 1 | Nothing |
| `FlushOutcome` | enum | 4 variants, two carrying `usize` | 16 | Nothing |
| `ConfigError` | enum | 2 variants, one carrying two `usize` | — | Nothing |
| `FlushEntry` | struct | 3 public fields | 40 | A policy, a cause, an outcome — all `Copy` |
| `FlushLog` | struct | 1 private field | — | `Vec< FlushEntry >`, the crate's only allocation |
| `Flusher< 'a, T >` | struct | 4 private fields | — | A `TlsBuffer< T >`, a `Producer< 'a, T >`, a policy, an optional log |

Thirteen variants across the four enums. **Three of the four enums are on the
export surface**, which is what makes the variant count a compatibility
question rather than an implementation detail
(→ [`integration/002`](../integration/002_a_decision_on_the_export_surface.md)'s
X1 and X2).

### The Widths, and Where They Came From

`FlushPolicy`'s sixteen bytes are load-bearing: the type is consulted by value on
a path that must not allocate, and three separate instances assert the figure.
It had never been measured until `tests/manual/readme.md`'s F2, which measured
four types at once and found the prediction held for all three that had one.

`FlushEntry` at 40 bytes was not predicted, because nothing depends on it — it is
allocated once per flush on the cold path, where 40 bytes is noise against a
claim, a copy and a publish.

### Trait Surface

| Noun | Derives | Hand-written |
|------|---------|--------------|
| `FlushPolicy` | `Debug, Clone, Copy, PartialEq, Eq` | — |
| `FlushCause` | `Debug, Clone, Copy, PartialEq, Eq` | — |
| `FlushOutcome` | `Debug, Clone, Copy, PartialEq, Eq` | — |
| `ConfigError` | `Debug, Clone, Copy, PartialEq, Eq` | `Display`, `Error` |
| `FlushEntry` | `Debug, Clone, Copy, PartialEq, Eq` | — |
| `FlushLog` | `Debug, Clone, Default, PartialEq, Eq` | — |
| `Flusher` | `Debug` | — |

**No `Default` on `FlushPolicy`, and it is deliberate** — a default policy is a
policy nobody chose, applied wherever someone wrote `..Default::default()`, which
recreates in one derive exactly the state this crate exists to prevent
(→ [`type/001`](../type/001_flush_policy.md)).

### Data Structures

| File | Relationship |
|------|-----------------|
| [`../data_structure/001_the_policy_enum.md`](../data_structure/001_the_policy_enum.md) | `FlushPolicy`'s shape, worked out as a data structure rather than catalogued |
| [`../data_structure/002_the_flush_log.md`](../data_structure/002_the_flush_log.md) | `FlushLog` and `FlushEntry`, and why the entry carries an outcome rather than a count |

### Items

| File | Relationship |
|------|-----------------|
| [`002_seventeen_verbs_that_never_touch_the_producer.md`](002_seventeen_verbs_that_never_touch_the_producer.md) | The other half of the catalogue |

### Types

| File | Relationship |
|------|-----------------|
| [`../type/001_flush_policy.md`](../type/001_flush_policy.md) | The withheld-`Default` argument in full |
| [`../type/002_flush_outcome.md`](../type/002_flush_outcome.md) | Why four variants rather than a `bool` or a `Result` |

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration catalogued here |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | F2, which measured the four widths |

### Tests

| Test | Relationship |
|------|--------------|
| `the_policy_is_a_value` | The one width the suite asserts, written as `2 * size_of::< usize >()` rather than a literal |
| `every_type_can_be_printed` | The `Debug` half of the derive table, exercised rather than assumed |
| `a_binding_refusal_renders_and_chains` | `ConfigError`'s two hand-written impls, the only ones in the crate |

### FL25 — Four Widths Were Measured and One Is Asserted

The manual plan recorded four figures; the suite pins one of them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- widths recorded in the manual plan --'
command grep -E '^\| `(FlushPolicy|FlushOutcome|FlushCause|FlushEntry)` \|' \
  ring_flush/tests/manual/readme.md
echo '  -- size assertions in the suite --'
command grep -r 'size_of' ring_flush/tests/*.rs
```

Live output:

```
  -- widths recorded in the manual plan --
| `FlushPolicy` | 16 | **16** |
| `FlushOutcome` | 16 | **16** |
| `FlushCause` | 1 | **1** |
| `FlushEntry` | not predicted | 40 |
  -- size assertions in the suite --
ring_flush/tests/flush_test.rs:        core::mem::size_of::<FlushPolicy>(),
ring_flush/tests/flush_test.rs:        2 * core::mem::size_of::<usize>(),
```

`FlushOutcome` is sixteen bytes, `FlushCause` one, `FlushEntry` forty — measured
once, on one machine, on one day, and recorded in a document no build reads.
`FlushPolicy` alone is held by an assertion.

**The three unasserted figures are not equally idle.** `FlushOutcome` is
returned by every drive call and is `Copy`; a variant carrying something larger
would widen every return on the driver surface, and nothing would notice.
`FlushCause` at one byte is what makes `FlushEntry` forty rather than
forty-eight, and `FlushEntry` is the one allocated per flush.

Recorded rather than fixed, because the fix is not obviously an assertion. A
width pinned to whatever was measured tests nothing — the crate's own F2 stage
says so about literals — and the honest form is a *relation*, as
`the_policy_is_a_value` uses. `FlushOutcome`'s relation to `FlushPolicy` is the
one worth having: both are a discriminant plus one `usize`, and they are
equal today.

### FL26 — Five Nouns Share One Derive List and the Odd One Out Has No State

The derive column is nearly constant, and the exceptions do not line up with the
hand-written impls:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every derive, in declaration order --'
command grep -E '^#\[ derive' ring_flush/src/lib.rs
echo '  -- and every impl block --'
command grep -E '^impl' ring_flush/src/lib.rs
```

Live output:

```
  -- every derive, in declaration order --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Default, PartialEq, Eq ) ]
#[ derive( Debug ) ]
  -- and every impl block --
impl core::fmt::Display for ConfigError
impl core::error::Error for ConfigError {}
impl FlushEntry
impl FlushLog
impl< 'a, T : Send > Flusher< 'a, T >
```

Five of the seven nouns carry `Debug, Clone, Copy, PartialEq, Eq` verbatim. The
two that differ are the two that own something: `FlushLog` swaps `Copy` for
`Default` because it holds a `Vec`, and `Flusher` drops everything but `Debug`
because it holds a producer.

**The two hand-written impls belong to `ConfigError`, which owns nothing and is
otherwise identical to four other nouns.** They were added late, by the first
crate to consume this one from outside its own tests — `ring_bench` held
`ConfigError`, `RingError` and `BuildError` at once and found this the only one
of the three implementing neither `Display` nor `Error`.

That is the finding: **the trait surface diverged along "what does this consumer
need to render", not along "what does this type own"**, and the divergence was
invisible until a crate held three error types together. Nothing in this crate
could have surfaced it, and nothing here will surface the next one — the
detector is a consumer with a heterogeneous collection, and the family has one.
