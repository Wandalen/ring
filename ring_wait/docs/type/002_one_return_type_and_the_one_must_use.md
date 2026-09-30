# Type: One Return Type and the One `#[ must_use ]`

### Scope

- **Purpose**: Account for the three return shapes the crate emits, record which two of `RingError`'s nine variants it can produce, and record that the single `#[ must_use ]` went to the item that needed it least.
- **Responsibility**: Tabulate the shapes, trace the error set to the family's own definition of "retryable", and measure this crate's annotation density against its neighbours.
- **In Scope**: `Result< usize, RingError >`, `Option< WaitKind >`, `bool`, and the one attribute.
- **Out of Scope**: The integers inside them — see [`001`](001_a_usize_budget_and_a_u64_count.md).

### Three Shapes Over Six Functions

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E "^pub (const )?fn|^-> " ring_wait/src/lib.rs
```

Live output:

```
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
-> Result< usize, RingError >
```

| Shape | Items | Which |
|-------|------:|-------|
| `Result< usize, RingError >` | 4 | `wait_until` `:179`, `wait` `:210`, `for_space` `:239`, `for_data` `:265-266` |
| `Option< WaitKind >` | 1 | `escalation_hint` `:83` |
| `bool` | 1 | `pause` `:112` |

`for_data` is the one whose signature wraps — its parameter list fills `:265` and
its return type sits alone on `:266`, which is why the grep asks for both.

The four fallible items are the crate's whole reason to exist and they agree
exactly: same `Ok`, same `Err`, same meaning for each. `Ok( n )` is *attempts
used*, never a duration and never a count of items
([`data_structure/002`](../data_structure/002_the_budget_and_the_attempt_index.md)),
so the four compose without a caller having to reinterpret anything.

The two odd ones are the two that are not waits — a pure lookup and a single
pause. Their shapes differ because their jobs do.

### WT15 — Two of Nine, and Which Two

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs | grep -oE "RingError::[A-Za-z]+" | sort -u
```

Live output:

```
RingError::Empty
RingError::Full
```

`RingError` carries nine variants (`ring_types/src/error.rs:42-80`). This
crate can produce two: `Empty` and `Full`.

Which two is not arbitrary. The family has a predicate for exactly this set:

```rust
// ring_types/src/error.rs:146-159
pub const fn is_transient( self ) -> bool
{
  match self
  {
    Self::Full | Self::Empty => true,
    Self::CapacityZero
    | Self::CapacityNotPowerOfTwo( _ )
    | Self::Closed
    | Self::NameTaken
    | Self::NameUnknown
    | Self::BatchTooLarge { .. }
    | Self::PolicyUnsupported => false,
  }
}
```

`{ Full, Empty }` — the entire error surface of this crate *is* the family's
definition of "retryable", and neither definition was written with the other in
view. That is the structural statement underneath
[`pitfall/001`](../pitfall/001_reading_empty_as_nothing_to_do.md)'s narrower
point: a caller asking *"should I retry?"* gets `true` from anything this crate
can hand it, unconditionally.

### Nobody Matches on It

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "\(RingError\|Self\)::\(Full\|Empty\|CapacityZero\|CapacityNotPowerOfTwo\)[^A-Za-z]*=>" \
  ring_*/ | grep -v "ring_types/src/error.rs"
# control: the identical expression without the exclusion
grep -rc --include=*.rs "\(RingError\|Self\)::\(Full\|Empty\|CapacityZero\|CapacityNotPowerOfTwo\)[^A-Za-z]*=>" \
  ring_types/src/error.rs
```

Live output:

```
ring_mpsc/tests/mpsc_test.rs:                Err( RingError::Full ) =>
ring_mpsc/tests/mpsc_test.rs:                Err( RingError::Full ) =>
ring_shutdown/src/lib.rs:      Self::Full( _ ) => false,
ring_shutdown/src/lib.rs:      Self::Full( _ ) => RingError::Full,
ring_spsc/tests/spsc_test.rs:            Err( RingError::Full ) => std::thread::yield_now(),
4
```

**Four matches**, and the control confirms the pattern also finds the four
arms it excludes — three `Display` arms and `is_transient`'s own. Three of
the four are test code — `ring_mpsc` branches on `Err( RingError::Full )` in
two separate retry loops (`:141`, `:350`) and `ring_spsc` in one — and the
fourth, `ring_shutdown/src/lib.rs:385`, is a conversion rather than a
decision: it maps its own `Refusal::Full` onto `RingError::Full`. The
remaining hit, `ring_shutdown/src/lib.rs:375`, is a fourth coincidence
beside the three `Display` ones: `Refusal<T>::is_closed` asks its own
`Full` variant a question that has nothing to do with `RingError`.

So exactly one non-test site matches on a variant and means `RingError` by
it — the same one as before. Everything else asks a predicate:
`is_configuration()` and `is_transient()` are called across twelve files in
the family, which is the shape the enum's own documentation asks for.

**Correction (2026-09-28):** this section read three matches, three
coincidental arms, and a control of `3`. Three unrelated changes moved
every number by one, two of them identically-shaped fixes.
`ring_types/src/error.rs`'s
`Fix(ring_error_classification_not_exhaustive)` rewrote `is_transient` from
a `matches!()` call — invisible to this pattern — into the exhaustive
`match` already shown above, so the control now also counts its
`Self::Full | Self::Empty => true,` arm: `3` → `4`. `ring_shutdown/src/lib.rs`'s
own `Fix(refusal_is_closed_classification_not_exhaustive)` did the identical
rewrite to `Refusal::is_closed` (`:370-377`), turning its
`Self::Full( _ ) => false,` arm into a fourth coincidence beside the three
`Display` ones. Separately, `ring_mpsc/tests/mpsc_test.rs` grew a second
retry loop, over a multi-word record tracking its own `full_retries`
counter — the fourth real match. The substance is unchanged: one non-test
site matches, and it is a conversion, not a decision.

The earlier spelling of this check could not have found these. It required
`=>` to follow the variant with only spaces between, so `Err( RingError::Full ) =>`
— the form every real site uses — fell outside it, and the empty result read as
"nobody matches on a variant" when it meant "nobody matches on a bare variant".

The reason is one line above the enum:

```rust
// ring_types/src/error.rs:43
#[ non_exhaustive ]
```

the only `#[ non_exhaustive ]` in `ring_types`. A downstream `match` needs a
wildcard arm whatever it does, so the enum is built to be *asked*, not
*destructured* — and the family took the hint.

That places [`pitfall/001`](../pitfall/001_reading_empty_as_nothing_to_do.md)
precisely: the `Empty`-means-`Full` confusion is real and currently unexercised,
because the one call shape that would hit it — branching on which variant — is
the shape the enum discourages. Unexercised for a structural reason is better
than unexercised by luck, and still not the same as prevented.

### WT8 — The Annotation Went to the Wrong Item

```sh
cd "$(git rev-parse --show-toplevel)"
grep "must_use" ring_wait/src/lib.rs
```

Live output:

```
#[ must_use ]
```

**One hit, at `:82`**, on `escalation_hint`:

```rust
// ring_wait/src/lib.rs:82-83
#[ must_use ]
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
```

Correct on its own terms — a pure function whose return is its only effect. It is
also the item with no caller anywhere in the family
([`lifecycle/002`](../lifecycle/002_the_escalation_ladder_nobody_climbs.md)), so
the attribute currently protects nobody.

`pause` has none, and `pause`'s `bool` is the load-bearing one:

| | `escalation_hint` | `pause` |
|--|-------------------|---------|
| Return | `Option< WaitKind >` | `bool` |
| Discarding it means | nothing happened | **a guarantee was ignored** |
| The guarantee | — | `false` from `None` is the tick-path promise ([`invariant/002`](../invariant/002_none_looks_exactly_once.md)) |
| `#[ must_use ]` | yes, `:82` | no |
| Callers in the family | 0 | 1 — `wait_until` `:189`, which does check it |

A `pause( WaitKind::None, 0 );` compiles clean, blocks nothing, and silently
throws away the only signal the non-blocking strategy has. That is not
hypothetical API surface: `pause` is `pub`, documented with its own doctest at
`:107`, and reachable by anyone.

### The Density Is the Evidence

Every immediate neighbour annotates at roughly one per function; this crate does
not:

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_wait ring_cursor ring_seqno ring_types ring_config ring_gating ring_barrier; do
  printf '%-14s %2s pub fn  %2s must_use\n' "$c" \
    "$( grep -hcE '^[[:space:]]*pub (const )?fn' ring/$c/src/*.rs | paste -sd+ | bc )" \
    "$( grep -hc 'must_use' ring/$c/src/*.rs | paste -sd+ | bc )"
done
```

Live output:

```
ring_wait       6 pub fn   1 must_use
ring_cursor    13 pub fn  13 must_use
ring_seqno        5 pub fn   5 must_use
ring_types     12 pub fn  11 must_use
ring_config    12 pub fn  11 must_use
ring_gating    11 pub fn  10 must_use
ring_barrier    9 pub fn   8 must_use
```

| Crate | `pub fn` | `#[ must_use ]` |
|-------|---------:|----------------:|
| `ring_cursor` | 13 | 13 |
| `ring_seqno` | 5 | 5 |
| `ring_types` | 12 | 11 |
| `ring_config` | 12 | 11 |
| `ring_gating` | 11 | 10 |
| `ring_barrier` | 9 | 8 |
| **`ring_wait`** | **6** | **1** |

So the family's convention is unambiguous and this crate is the outlier — which
makes the single annotation look less like a deliberate choice than like the one
that got written.

Nothing in the build catches it:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[workspace.lints.clippy\]/,/^\[/p' Cargo.toml
```

Live output:

```
[workspace.lints.clippy]
# Unsafe: one operation per block, each with its own `// SAFETY:`.
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
cast_ptr_alignment = "warn"
ptr_as_ptr = "warn"
ptr_cast_constness = "warn"
mem_forget = "warn"
# Concurrency: locks are what this family exists to avoid; refcount bumps stay visible.
mutex_atomic = "warn"
mutex_integer = "warn"
rc_mutex = "warn"
clone_on_ref_ptr = "warn"
# Determinism: hash iteration order would leak into the delivery order.
iter_over_hash_type = "warn"
# Sequence and index arithmetic.
cast_sign_loss = "warn"
precedence_bits = "warn"
# Hygiene.
dbg_macro = "warn"
exit = "warn"
infinite_loop = "warn"
large_stack_frames = "warn"
todo = "warn"
unimplemented = "warn"
unused_result_ok = "warn"

[workspace.lints.rustdoc]
```

The workspace lint tables name neither `must_use_candidate` nor
`unused_results`. The gap is invisible to `cargo clippy` and to every test.

Two annotations, both one line, would close it: `#[ must_use ]` on `pause`, and
— less urgently — on the four `Result`s, which `#[ must_use ]` already covers by
`Result`'s own attribute. Only `pause` is actually unprotected.


### WT50 — One Generic, Monomorphised Once Per Predicate

The crate has one type parameter, and it is the mechanism by which its thirteen-line
loop becomes a fresh copy at every call site.

```sh
cd "$(git rev-parse --show-toplevel)"
grep '< F >' ring_wait/src/lib.rs
grep 'F : FnMut() -> bool' ring_wait/src/lib.rs
# every distinct closure handed to it, in the family's src/
grep -r 'wait_until( kind, spins, ||\|wait_until( WaitKind' --include=*.rs . --exclude-dir=docs \
  | grep -v '^ring_wait/tests' | head
```

Live output:

```
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
  F : FnMut() -> bool,
  F : FnMut() -> bool,
ring_wait/src/lib.rs:/// let outcome = wait_until( WaitKind::Spin, 16, ||
ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.may_claim() ).map_err( | _ | RingError::Full )
ring_wait/src/lib.rs:  wait_until( kind, spins, || pair.pending() >= count )
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

`wait_until` and `wait` are generic over `F : FnMut() -> bool`, and every closure
is a distinct type, so each call site compiles its own copy of the loop with the
predicate inlined into it.

That is the intended trade and it is why the crate can be a shared loop at all: a
`&mut dyn FnMut()` would put an indirect call inside a spin loop whose whole
purpose is to re-read a cursor as tightly as possible, and
[`nfr/001`](../non_functional_requirement/001_two_atomic_loads_for_every_look.md)
measures that loop at two `Acquire` loads per look — a call through a vtable would
be a significant fraction of it.

The consequence for this instance's subject is that the crate has one return
*type* and no single return *instance*: `Result< usize, RingError >` is spelled
once in the source and instantiated per predicate, alongside a copy of the body
that produces it. The crate's compiled footprint is a function of how many
distinct closures the family writes, which is three in `src/` and one per test.

### Types

| File | Relationship |
|------|--------------|
| [001_a_usize_budget_and_a_u64_count.md](001_a_usize_budget_and_a_u64_count.md) | The integers these shapes carry |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | WT1 — the annotated item is the uncalled one |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_crate_with_no_type_of_its_own.md](../data_structure/001_a_crate_with_no_type_of_its_own.md) | WT14 — every type here is borrowed |
| [../data_structure/002_the_budget_and_the_attempt_index.md](../data_structure/002_the_budget_and_the_attempt_index.md) | What `Ok( n )` counts |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The guarantee the discardable `bool` carries |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | `pause` and its four answers |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | Why the annotated function has no caller |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_reading_empty_as_nothing_to_do.md](../pitfall/001_reading_empty_as_nothing_to_do.md) | The confusion the two variants invite |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/error.rs:43,42-80,146-159` | `#[ non_exhaustive ]`, the nine variants, and `is_transient`'s set |
| `Cargo.toml` § `[workspace.lints.clippy]` | The one lint enabled, and the two that are not |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:255-268` | The two variants, produced and distinguished |
| `tests/wait_test.rs:270-278` | Neither is a configuration error |
| `tests/manual/readme.md` § W2 | Exactly two `RingError::` mentions in the source |
