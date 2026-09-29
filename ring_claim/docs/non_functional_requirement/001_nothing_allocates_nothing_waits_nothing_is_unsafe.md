# Non Functional Requirement: Nothing Allocates, Nothing Waits, Nothing Is Unsafe

### Scope

- **Purpose**: Record the three negative properties this crate holds — no allocation, no blocking, no `unsafe` — establish how each is actually enforced, and record the one that is held in substance but never declared.
- **Responsibility**: Distinguish properties enforced by the compiler from properties held by discipline, and check the whole dependency chain rather than this crate alone.
- **In Scope**: Allocation, blocking, `unsafe`, and `no_std` across `ring_claim` and its six transitive dependencies.
- **Out of Scope**: What the retry loop costs when it does run — see [`non_functional_requirement/002`](002_what_contention_costs.md).

### The Three Properties

| Property | Enforced by | Strength |
|----------|-------------|----------|
| No `unsafe` | `unsafe-code = "deny"` in `[workspace.lints.rust]` | **compiler — cannot be violated** |
| Never blocks | nothing — no `loop`, no `park`, no `spin_loop` in the source | structural; a reviewer must notice |
| No allocation | `tests/allocation_test.rs`, six call shapes with a control arm — was **nothing**, and was false one crate away: see [CL55](#cl55--claim-allocated-once-per-call-two-crates-down) | **test — a regression fails the suite** |

The first is enforced by the compiler and the third by a test. The second is
true and guarded by nothing. The third was not true at all at the scope this
document covers until commit `b7e075ca`, and the reason it read as true for so
long — a single-file grep standing in for a six-crate claim — is in CL55, along
with why the correction needed a test rather than another measurement.

```sh
cd "$(git rev-parse --show-toplevel)"

# this crate's own source
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -E 'unsafe|Vec<|Box<|String|alloc::|\bloop\b|park|spin_loop|yield_now'
# → no output

# and the six transitive dependencies this document's Scope also covers
for c in ring_types ring_cursor ring_gating ring_seqno ring_align ring_atomic; do
  printf '%-12s %s\n' "$c" \
    "$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs \
        | grep -cE '\bVec<|\bVec::|alloc::|Box<|String' )"
done
# → ring_cursor 1, ring_gating 2, every other crate 0

# which sites, exactly
grep -vE "^[[:space:]]*//" ring_cursor/src/lib.rs ring_gating/src/lib.rs \
  | grep -E '\bVec<|\bVec::'
```

Live output:

```
ring_types   0
ring_cursor  0
ring_gating  2
ring_seqno     0
ring_align   0
ring_atomic  0
ring_gating/src/lib.rs:  cursors : Vec< PaddedCursor >,
ring_gating/src/lib.rs:    let mut cursors = Vec::with_capacity( consumers );
```

The comment filter is not optional here: the word "loop" appears three times in
the source, every one of them inside a `//` comment explaining that the gate is
the loop condition. The family's standard `grep -vE "^[[:space:]]*//"` prefix
drops `///`, `//!` and `//` alike, which is why every check in this corpus
starts with it.

The "never blocks" property is the one the module documentation argues for
directly (`:25-35`):

> Every function here returns immediately — `Ok` with a range, or `Err` saying
> why not. That is not an incidental design choice; it is what lets the same
> primitive serve a spinning producer, a parking producer, and the tick path
> that must not block at all. A claim that waited internally would force the
> wait strategy into this crate and make `WaitKind::None` unimplementable above
> it.

Worth being precise, because "never blocks" and "always terminates" are not the
same claim and this crate makes only the first. `claim`'s retry loop is
unbounded: under sustained contention a producer can lose its exchange
arbitrarily many times. It never *waits* — it never parks, never sleeps, never
calls a wait strategy — but it is not wait-free in the technical sense, only
lock-free. Each iteration is another producer's success, so the system makes
progress; an individual caller has no bound
([`algorithm/001`](../algorithm/001_the_gate_inside_the_retry.md)).

### CL35 — `no_std` in Everything but the Attribute

```sh
cd "$(git rev-parse --show-toplevel)"

# std usage across ring_claim and its whole transitive chain
for c in ring_claim ring_types ring_cursor ring_gating ring_atomic ring_align ring_seqno; do
  printf '%-12s %s\n' "$c" \
    "$( grep -vE "^[[:space:]]*//" ring/$c/src/*.rs | grep -cE '\bstd::|extern crate std' )"
done

# crates in the family declaring the attribute
grep -rl 'no_std' ring_*/src/lib.rs | wc -l
```

Live output:

```
ring_claim   0
ring_types   0
ring_cursor  0
ring_gating  0
ring_atomic  0
ring_align   0
ring_seqno     0
3
```

| Crate | `std::` paths | `#![ no_std ]` |
|-------|--------------:|:--------------:|
| `ring_claim` | 0 | ✘ |
| `ring_types` | 0 | ✘ |
| `ring_cursor` | 0 | ✘ |
| `ring_gating` | 0 | ✘ |
| `ring_atomic` | 0 | ✘ |
| `ring_align` | 0 | ✘ |
| `ring_seqno` | 0 | ✘ |
| **All 33 `ring_*` crates** | — | **0 declare it** |

Every `use` statement in the seven-crate chain resolves to `core::` or to
another family crate:

```
ring_claim   use ring_cursor::{ PaddedCursor, SeqCell, GATING };
             use ring_gating::GatingSet;
             use ring_types::{ RingError, Seq };
ring_types   use core::fmt;
ring_cursor  use core::sync::atomic::Ordering;   + ring_align, ring_atomic, ring_types
ring_atomic  use core::sync::atomic::{ AtomicU64, AtomicUsize };   ( or loom, under --cfg loom )
ring_gating  use ring_cursor::PaddedCursor;      + ring_types
ring_align   ( none )
ring_seqno     use ring_types::{ Capacity, Seq };
```

Not one `std::` import in the whole path from Tier 0 to Tier 5. The crate is
`no_std`-clean in substance and `std`-linking in fact, and nothing records that
this is a property anyone is holding.

That matters because the property is cheap to keep and expensive to recover. It
survives today by accident of what nobody has needed yet; the first
`std::collections::HashMap` in `ring_gating`, or the first `std::time::Instant`
in a diagnostic, ends it silently — no test fails, no lint fires, and the
information that it was ever true is nowhere.

Two options, and the first turns out to be free. Measured, not assumed: a copy
of this crate with `#![ no_std ]` prepended to `src/lib.rs` and nothing else
changed compiles, and its whole suite passes.

```sh
cd "$(git rev-parse --show-toplevel)"

# a scratch copy outside the workspace, standalone manifest, edition 2024
mkdir -p ./-nostd_probe/src ./-nostd_probe/tests
cp ring_claim/src/lib.rs ./-nostd_probe/src/lib.rs
cp ring_claim/tests/claim_test.rs ./-nostd_probe/tests/
cat > ./-nostd_probe/Cargo.toml <<'TOML'
[workspace]

[package]
name = "ring_claim"
version = "0.1.0"
edition = "2024"

[dependencies]
ring_cursor = { path = "../ring_cursor" }
ring_gating = { path = "../ring_gating" }
ring_types  = { path = "../ring_types" }
TOML

# the one and only change to the source
sed -i '1i #![ no_std ]' ./-nostd_probe/src/lib.rs
# durations are normalised away: the claim is that the suite passes under
# `no_std`, and a wall-clock figure that moves between runs would make this
# recipe report a failure every time the machine was busy
cargo test --quiet --manifest-path ./-nostd_probe/Cargo.toml 2>&1 \
  | sed -E 's/[0-9]+\.[0-9]+s/<elapsed>/g'
rm -rf -- ./-nostd_probe
```

Live output:

```
warning: unexpected `cfg` condition name: `loom`
  --> tests/claim_test.rs:39:15
   |
39 | #![ cfg( not( loom ) ) ]
   |               ^^^^
   |
   = help: expected names are: `docsrs`, `feature`, and `test` and 32 more
   = help: consider using a Cargo feature instead
   = help: or consider adding in `Cargo.toml` the `check-cfg` lint config for the lint:
            [lints.rust]
            unexpected_cfgs = { level = "warn", check-cfg = ['cfg(loom)'] }
   = help: or consider adding `println!("cargo::rustc-check-cfg=cfg(loom)");` to the top of the `build.rs`
   = note: see <https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html> for more information about checking conditional configuration
   = note: `#[warn(unexpected_cfgs)]` on by default


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in <elapsed>


running 27 tests
...........................
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in <elapsed>


running 17 tests
.................
test result: ok. 17 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in <elapsed>

all doctests ran in <elapsed>; merged doctests compilation took <elapsed>
```

```
test result: ok. 27 passed; 0 failed    # tests/claim_test.rs
test result: ok. 17 passed; 0 failed    # doctests
```

The obvious objection — that the suite uses `std::thread::scope` throughout —
does not apply. `#![ no_std ]` is a property of *this* crate, and integration
tests under `tests/` are separate crates that link `std` on their own; doctests
compile as separate crates too. Neither needs `extern crate std;`, and no
`[dev-dependencies]` entry is required.

| Option | Cost | What it buys |
|--------|------|--------------|
| add `#![ no_std ]` | **one line, measured — 27 + 17 tests still pass** | the compiler enforces it forever |
| write it down as a non-requirement | a sentence | records that it is currently true and deliberately unenforced |

So the choice is not between a cheap option and a careful one. The enforcing
option costs a single line per crate and is verifiable in one command, and it is
the one nobody has taken across 33 crates.

### CL55 — `claim` Allocated Once Per Call, Two Crates Down

The no-allocation property was false, and it was false on the hot path. Every
gate read charged one heap allocation, sized one `Seq` per consumer, and
`claim`'s retry loop evaluates the gate once per iteration — so a contended
claim allocated once per attempt.

The site was `ring_cursor::slowest`, two crates down:

```rust
// ring_cursor/src/lib.rs, as it stood before commit b7e075ca
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  let positions : Vec< Seq > = cursors.iter().map( | c | c.load( GATING ) ).collect();
  ring_seqno::slowest( &positions )
}
```

The `Vec` existed only to change `&[ PaddedCursor ]` into the `&[ Seq ]` that
`ring_seqno::slowest` takes — a type adapter between two crates, materialised on
the heap, on every call. `ring_seqno::slowest` is `cursors.iter().copied().min()`,
so the whole thing could be `cursors.iter().map( | c | c.load( GATING ) ).min()`
with no intermediate collection and no allocation at all. Nothing about the
design required it; the allocation was an artifact of the split.

That is what shipped. The fold is now taken in place, and this crate asserts
what that costs rather than describing it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the fold every row below reaches, one crate down --'
awk '/^pub fn slowest\( cursors : &\[ PaddedCursor \] \)/{ f = 1 } f { print } f && /^\}$/{ exit }' ring_cursor/src/lib.rs
echo '  -- every call shape this crate asserts allocation-free, from the test itself --'
# joined into one line first: one of these assertions is wrapped across
# several source lines, and a line-oriented match would silently skip it
tr '\n' ' ' < ring_claim/tests/allocation_test.rs | tr -s ' ' \
  | command grep -oE '\( 0, 0 \), "[^"]+"' | sed 's/( 0, 0 ), /    /'
# `wc -l` and not `grep -c`: the file is one line by this point, so a line
# count would report 1 no matter how many call shapes the test asserts
printf '    call shapes asserted allocation-free: %s\n' \
  "$( tr '\n' ' ' < ring_claim/tests/allocation_test.rs | tr -s ' ' \
      | command grep -oE '\( 0, 0 \), "[^"]+"' | wc -l )"
echo '  -- and the control arm, without which every zero above is unfalsifiable --'
command grep -c 'control_calls >= 1' ring_claim/tests/allocation_test.rs \
  | sed 's/^/    assertions that a deliberate allocation is seen: /'
```

Live output:

```
  -- the fold every row below reaches, one crate down --
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
{
  cursors.iter().map( | c | c.load( GATING ) ).min()
}
  -- every call shape this crate asserts allocation-free, from the test itself --
    "headroom() ×1000"
    "claim( 1 ) ×1000, all granted"
    "claim( 1 ) ×1000, all refused"
    "claim_up_to( 8 )"
    "claim( 1 ) ×1000 on an ungated set — green before the fix too, and therefore no evidence"
    "Claim::sequences() drained"
    call shapes asserted allocation-free: 6
  -- and the control arm, without which every zero above is unfalsifiable --
    assertions that a deliberate allocation is seen: 1
```

| Call | Allocations per 1000, before | Allocations per 1000, now |
|------|-----------------------------:|--------------------------:|
| `Claimer::headroom()` | **1000** | 0 |
| `Claimer::claim( 1 )`, granted | **1000** | 0 |
| `Claimer::claim( 1 )`, refused | **1000** | 0 |
| `Claimer::claim( 1 )`, ungated set | 0 | 0 |

The before column is not quoted from memory. Restoring the `collect()` body
above and re-running `no_claim_and_no_gate_read_allocates` fails on its first
measured row with `left: (1000, 8000)` — a thousand allocations of eight bytes
each, one `Seq` per consumer per gate read, which is the finding this section
originally recorded.

**The last row is why this went unnoticed, and it is kept on purpose.** An empty
`GatingSet` has an empty `cursors` slice, and a zero-length `Vec` never touches
the allocator. Any test using an ungated claimer measured zero and was telling
the truth about a case no real ring is in. It reads zero before the fix and zero
after, so it can distinguish nothing — the test keeps it labelled with exactly
that, so the next reader does not mistake a green row for evidence.

**Why the corpus said otherwise.** This document's Scope covers "`ring_claim`
and its six transitive dependencies," but the command originally offered as
evidence grepped `ring_claim/src/lib.rs` — one file. Its output was
correct and its scope was one crate narrower than the sentence it was
supporting.

**And why the correction nearly went unnoticed too.** The measurement that found
this was a scratch binary written into this document, run once, and deleted by
its own last line. It was right and it left nothing behind, so when the
allocation was removed the numbers here became false in the other direction with
nothing to catch it. The replacement is a test in the suite, not a probe in a
document.

`ring_gating`'s two `Vec` sites are the benign kind by contrast: a `GatingSet`
owns its cursors, so the field and the `Vec::with_capacity` in `new` are one
allocation per ring at construction, not per operation. That construction cost
is real and is deliberately excluded from every row above — the test builds
every set before its first measurement.

**Disposition:** applied — `ring_cursor::slowest` no longer collects cursor
positions into an intermediate `Vec`; it takes the minimum directly over the
mapped iterator, exactly the fix this finding named as available and required by
nothing in the design. The scratch probe that measured it once is replaced by
`ring_claim/tests/allocation_test.rs`, which asserts every call shape in
the table above at zero, separates the compare-exchange success and refusal
paths the probe conflated, and carries a control arm that fails when the counter
is not watching. Now prints: `call shapes asserted allocation-free: 6`

### CL36 — The Only Property With Teeth Is the One Nobody Would Have Broken

`unsafe-code = "deny"` sits in `[workspace.lints.rust]`, alongside
`undocumented_unsafe_blocks = "deny"` in the clippy table — two lints guarding
the same thing, in a crate whose entire implementation is three integer
comparisons and a compare-exchange.

Meanwhile the two properties a plausible change *would* break — allocation and
non-blocking — had nothing at all. One of them did not survive first contact
with a measurement (CL55), and nothing anywhere reported it. Only the allocation
row has since been closed:

| Change someone might actually make | Breaks | Caught by |
|-----------------------------------|--------|-----------|
| collect cursor positions into a `Vec` to adapt a slice type | no-allocation | ✔ `tests/allocation_test.rs` — it was already there, and this is the test that would have said so |
| collect retry statistics into a `Vec` | no-allocation | ✔ same test, if the collection happens inside a measured call |
| add a `spin_loop()` hint to the retry | non-blocking (arguably) | **nothing** |
| add a `std::thread::yield_now()` after N failures | non-blocking, `no_std` | **nothing** |
| add a bounded retry with a `Duration` | non-blocking, `no_std` | **nothing** |
| write `unsafe { … }` | no-unsafe | ✔ two lints |

The third and fourth are not hypothetical shapes — a retry loop that spins
forever is exactly the thing a reviewer asks about, and `yield_now` is the
standard answer. `ring_publish` faced the same question and answered it in
prose (`:42-53`), which is the family's established practice; this crate does
not discuss it at all.

The observation is not that the `unsafe` lints are wrong — they are inherited
workspace-wide and cost nothing. It is that **enforcement in this crate is
concentrated entirely on its least fragile property**, and the two that a
well-meaning change would break are held by nobody noticing.

The allocation half of that gap is now closed, and closed in the way CL55
argues for — by measuring the behaviour through this crate's own API, so the
whole dependency chain is inside the check, rather than by grepping a file.
A single-file grep is exactly what was already being run, and exactly what
missed an allocation on every `claim`.

The non-blocking half is still held by nobody noticing. `tests/manual/readme.md`
is where it would close cheaply: `§ C2` already greps this source for
`fetch_add` and `store`, and one more line in the same check — no `loop`, no
`yield_now`, no `std::` — would give it the same weight, at the cost of a single
`grep`. That is a weaker instrument than a test, and it is the right one here:
"never blocks" is a property of the source's shape, not of a call's observable
cost, so there is nothing to measure at runtime the way an allocation can be
counted.

### What Is Deliberately Not Required

| Not required | Why |
|--------------|-----|
| wait-freedom | the retry loop is lock-free only, and bounding it would need a give-up path with no correct behaviour |
| a latency bound | there is none to state — contention is the caller's workload, not this crate's |
| `Send`/`Sync` declarations | both are auto-derived; `Claimer` is `Sync` because `PaddedCursor` is |
| a memory ordering weaker than `AcqRel` on success | argued and refused at `:70-75` |

The last row is the one place this crate does hold its own. `CLAIM_SUCCESS`
(`:70-76`) is not stated and left bare — it carries the reason, and the reason
names a concrete reordering:

> `AcqRel` rather than `Release`: the success case is both a release of the
> cursor advance to other producers and an acquire of whatever the producer
> whose value we replaced had done. A bare `Release` would let this producer's
> slot writes be reordered before it observed the previous producer's claim.

`ring_publish` does the same for its own constant in the same shape (`:60-67`),
and names the same class of consequence — "a ring that works on x86, where the
hardware supplies the ordering the code failed to ask for, and races on
aarch64." Both crates define one ordering constant at module scope, argue it in
about six lines against the weaker alternative, and name what breaks. That is a
consistent family practice and it is the strongest documentation either crate
has.

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [002_what_contention_costs.md](002_what_contention_costs.md) | What the loop costs when it runs, and what measures it |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | The unbounded retry, and why it needs no budget |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | Why a lock is refused — the non-blocking requirement, as a decision |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | The dependency chain checked here |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:25-35` | The non-blocking requirement, argued |
| `ring_claim/src/lib.rs:70-76` | `CLAIM_SUCCESS`, argued against the weaker alternative |
| `Cargo.toml` `[workspace.lints.rust]` | `unsafe-code = "deny"`, and the rest of the inherited set |
| `ring_publish/src/lib.rs:42-53` | The family's precedent for arguing a spin in prose |
| `ring_publish/src/lib.rs:60-67` | `PUBLISH`, argued in the same shape and naming aarch64 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § C2` | The structural grep that could carry the other two properties |
| `tests/manual/readme.md § C5` | The dependency check, which already walks this chain |
