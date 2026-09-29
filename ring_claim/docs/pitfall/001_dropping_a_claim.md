# Pitfall: Dropping a Claim

### Scope

- **Purpose**: Record what happens when a granted range is never published, enumerate the four routes there, and show that the crate's one guard against it fires on none of them.
- **Responsibility**: Reproduce the stranded state, measure what the lint actually catches, and trace how the failure surfaces — in a different thread, at a later call, as a different error.
- **In Scope**: Claim-side loss — a `Claim` that is obtained and then not published.
- **Out of Scope**: Publish-side misuse — publishing a range never claimed, twice, or at the wrong width. That is `ring_publish/docs/pitfall/001_publishing_a_range_you_never_claimed.md`, and the two documents are mirrors.

### The Obligation

`claim` advances the producer cursor and hands back the range it advanced past.
Nothing else in the system will ever advance the *published* cursor over that
range — only the producer holding the claim can, and only by calling
`ring_publish::Publisher::publish`. The type says so
(`src/lib.rs:95`):

> a claimed range that is never published strands its slots and stalls every
> consumer

The obligation rests entirely on that lint, because `Claim` has no destructor
and deliberately so ([`decisions/002`](../decisions/002_must_use_without_drop.md)).
The question this file answers is how much of the obligation the lint actually
carries.

### CL43 — The Message Fires on None of the Four Routes, and Through `claim` It Never Prints At All

Four ways a bound `Claim` fails to reach a `publish`, all of them ordinary Rust:

```rust
// 1 — explicitly discarded
let _ = claimer.claim( 4 );

// 2 — an early return between the claim and the write
let claim = claimer.claim( 4 )?;
if something_failed { return Err( RingError::Full ); }

// 3 — an unwind between the claim and the write
let claim = claimer.claim( 4 ).unwrap();
assert!( claim.len() > 99, "forced" );

// 4 — a loop that skips the publish
for i in 0..2
{
  let claim = claimer.claim( 1 ).unwrap();
  if i == 0 { continue; }
}
```

All four are compiled by name — one numbered marker each — in
`none_of_the_four_abandonment_routes_warns`, and the count below is read from
that test rather than from prose:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -c '^  // [1-4] —' ring_claim/tests/claim_test.rs
# `touch` first, deliberately. A warm cache re-emits no diagnostics at all, so a
# build run without it reports zero warnings whether or not anything warns --
# which is exactly how an earlier form of this recipe printed a zero it had
# never measured.
touch ring_claim/tests/claim_test.rs
cargo build --tests -p ring_claim 2>&1 | command grep -cE '^warning|must_use'
cargo test -p ring_claim --test claim_test none_of_the_four_abandonment_routes_warns 2>&1 \
  | command grep -E '^test .* \.\.\. ok$'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
4
0
test none_of_the_four_abandonment_routes_warns ... ok
```

Four routes, **zero warnings.** All four compile clean. The lint is a `#[ must_use ]`, so it
fires on an *unused value* — and every one of the four uses the value. Route 1
uses it by discarding it explicitly, which is exactly the idiom `must_use`
treats as consent; routes 2–4 bind it, which satisfies the lint on the spot,
and then lose it later, when no lint is watching.

The one shape that does warn is a bare discarded temporary, and it is worse than
useless:

| Written | Warns | Note printed |
|---------|:-----:|--------------|
| `claimer.claim( 4 );` | ✔ | *"this `Result` may be an `Err` variant, which should be handled"* |
| `claimer.claim( 4 ).unwrap();` | ✔ | *"a claimed range that is never published strands its slots…"* |
| `Claim::new( Seq( 0 ), 4 );` | ✔ | *"a claimed range that is never published strands its slots…"* |
| all four routes above | **✘** | — |

The first row is the one a caller could plausibly write, and it prints
**`Result`'s** generic message. `claim` returns `Result< Claim, RingError >`;
`Result` carries its own `#[ must_use ]`, it is the outer type, and it wins. The
sentence this crate wrote — the only place in the codebase that explains what
stranding costs — is shadowed by a stock library string about `Err` variants.

The message is reachable, but only down two paths:

- `claim( … ).unwrap();` as a bare statement — collapsing the `Result` exposes
  the `Claim`, and nobody writes this.
- `Claim::new( … );` as a bare statement — the public constructor, which is
  precisely the item `ring_publish/docs/pitfall/001` § PB35 argues should not
  be public at all ([`api/001`](../api/001_seventeen_items_and_nothing_that_drops_silently.md)).

So the crafted warning surfaces through the constructor a sibling crate wants
removed, and not through either method that actually grants a range.

There is a final turn. On the one call that does warn, `rustc` appends its
standard suggestion:

```
help: use `let _ = ...` to ignore the resulting value
   |
53 |   let _ = claimer.claim( 4 );
```

The compiler's own remedy is route 1 — the first entry in the table above.
Following the suggestion silences the only warning the caller was ever going to
get and strands the claim.

**Disposition:** applied — the four routes were an illustrative `rust` block, and
the recipe under them ran a bare `cargo build` from the checker's own working
directory, compiling none of the four and printing a zero it had read off a warm
cache rather than measured. They are compiled by name now, one numbered marker
each, in `none_of_the_four_abandonment_routes_warns` in `tests/claim_test.rs`:
route 2 returns early through `?`, route 3 unwinds inside `catch_unwind` with
the panic hook silenced, route 4 `continue`s past its publish, and the test
closes by asserting the cursor reached `Seq( 5 )` — five claims granted, nothing
published, and no diagnostic from any of them. Nothing in the crate changed;
what changed is that the claim is measured instead of asserted. Now prints:
`test none_of_the_four_abandonment_routes_warns ... ok`

### CL44 — The Failure Surfaces in Another Thread, One Call Later, as a Different Error

Reproduced against the shipping crates, capacity 8, one stranded sequence. The
reproduction lives in `ring_publish`, not here: `ring_claim` has no dependency,
normal or dev, on `ring_publish`, so its own suite can never observe a
publication that fails to arrive.

```sh
cd "$(git rev-parse --show-toplevel)"
# the four rows below, as the assertions that hold them. Pulled by pattern
# rather than by line address, so an edit above them cannot silently re-target
# this recipe at the wrong function.
command sed -n '/fn one_dropped_claim_pins_the_frontier/,/^  }$/p' \
  ring_publish/tests/handshake_test.rs \
  | command grep -E 'Row [1-4] —|assert_eq!\( claimer\.|Err\( Seq::ZERO \)|RingError::Full'
cargo test -p ring_publish --test handshake_test one_dropped_claim_pins_the_frontier 2>&1 \
  | command grep -E '^test .* \.\.\. ok$'
```

Live output:

```
    // Row 1 — `claim` reports Ok. The ring is already dead.
    // Row 2 — `claimed()` advances, healthily, past exactly that sequence.
    assert_eq!( claimer.claimed(), Seq( 5 ) );
    // Row 3 — `headroom()` reports three slots that can never be freed.
    assert_eq!( claimer.headroom(), 3 );
    // Row 4 — the producer that claimed correctly and wrote correctly is the
      Err( Seq::ZERO ),
    assert_eq!( claimer.headroom(), 0 );
    assert_eq!( claimer.claim( 1 ), Err( RingError::Full ) );
test threaded::one_dropped_claim_pins_the_frontier_and_the_ring_dies_reporting_full ... ok
```

Three things are true at once, and none of them names the fault:

| Signal | Reports | Actually |
|--------|---------|----------|
| `claim` | `Ok` — grants continue normally | the ring is already dead |
| `claimed()` | advancing, `Seq(5)` | advancing past sequences nobody will publish |
| `headroom()` | `3`, a healthy number | 3 slots that can never be freed |
| `try_publish` | `Err( Seq( 0 ) )` | the correct answer, in the wrong thread |

The producer that dropped its claim **is not the one that fails.** It returns
normally, having done nothing observable. The next producer to publish is the
one that blocks, on a range it claimed correctly and wrote correctly, because
the frontier is pinned behind it. With `publish` rather than `try_publish`,
that thread enters a spin with no budget and no exit
([`lifecycle/001`](../lifecycle/001_a_range_from_grant_to_publication.md)) —
holding its core, producing no message, no panic, no exit code.

Then it gets quieter. Once the ring fills behind the strand, the symptom stops
being a hang and becomes an ordinary-looking error:

```
round 1: claimed Seq(1)..Seq(5)   headroom now 3
round 2: Full  <- indistinguishable from ordinary back-pressure
round 3: Full  <- indistinguishable from ordinary back-pressure
```

`RingError::Full` is documented as *"back-pressure, so a retry loop should keep
going"* (`src/lib.rs:369-370`). That advice is correct for every cause of `Full`
except this one, where the retry loop is now the failure mode: it will spin
forever waiting for a consumer that is itself waiting for a publication that
will never come. And because `claim`'s retry is internal, the `Full` returned
here is byte-identical to the one returned under healthy contention
([`pattern/001`](../pattern/001_retrying_against_a_moving_target.md) § CL40).

One dropped claim of width one, and the terminal state of the whole ring is a
plausible error code recommending the exact action that hangs.

**Disposition:** applied — this finding's evidence was a hand-written trace
sitting below a recipe that contained no command at all, a shape G15 accepts
because an empty command legitimately produces empty output. The reproduction is
real now, and it had to be filed in a different crate:
`one_dropped_claim_pins_the_frontier_and_the_ring_dies_reporting_full` in
`ring_publish/tests/handshake_test.rs` asserts the table above row for row —
`claim` still `Ok`, `claimed()` at `Seq( 5 )`, `headroom()` reporting `3`,
`try_publish` refused with `Err( Seq::ZERO )` while `published()` stays at
`Seq::ZERO` — and then fills the ring behind the strand to reach the terminal
`Err( RingError::Full )` that is byte-identical to healthy back-pressure.
Nothing in either crate changed. Now prints:
`test threaded::one_dropped_claim_pins_the_frontier_and_the_ring_dies_reporting_full ... ok`

### What Would Catch It

| Mechanism | Catches | Cost |
|-----------|--------:|------|
| the guard shape (`Drop` publishes) | routes 2, 3, 4 | needs a ring to borrow — unavailable at Tier 5 ([`decisions/002`](../decisions/002_must_use_without_drop.md)) |
| `publish( claim : Claim )` taking the value | nothing here, but all four publish-side modes | a `ring_publish` → `ring_claim` dependency that does not exist ([`integration/001`](../integration/001_two_dependents_that_split_one_feature.md)) |
| a debug-only outstanding-claim counter | all four, at drop time | `Claim` gains a destructor and stops being `Copy` |
| a stall detector — frontier unchanged while claimed advances | all four, after the fact | a diagnostic crate's job; `ring_debug` exists and does not do this |

The last row is the cheap one and the only one that does not change a type. The
condition is two loads and a comparison — `claimed()` moving while `published()`
does not, for longer than any producer's write can take — and it is exactly the
signal every table above shows going unreported.

Nothing in the crate is wrong here. The guard shape is genuinely unavailable at
this tier, and taking `Claim` by value in `publish` is genuinely blocked by the
dependency direction. What is missing is any record that the lint covers one
unreachable case out of four, and that the crate's own healthy-looking
diagnostics are the ones that stay healthy-looking after the ring has died.

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_claiming_zero.md](002_claiming_zero.md) | The other undocumented behaviour a caller meets first |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_must_use_without_drop.md](../decisions/002_must_use_without_drop.md) | Why the lint is the only mechanism available |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_range_from_grant_to_publication.md](../lifecycle/001_a_range_from_grant_to_publication.md) | The spin the next producer enters |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_retrying_against_a_moving_target.md](../pattern/001_retrying_against_a_moving_target.md) | Why the terminal `Full` is indistinguishable from contention |
| [../pattern/002_the_half_open_range_as_a_value.md](../pattern/002_the_half_open_range_as_a_value.md) | The annotation rule this crate follows and `ring_batch` does not |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | The public constructor the message reaches through |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_a_refused_claim_moves_nothing.md](../invariant/002_a_refused_claim_moves_nothing.md) | The guarantee that holds even here — a refusal still moves nothing |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:94-96` | The annotation, and the sentence that rarely prints |
| `ring_claim/src/lib.rs:394-403` | `Full` documented as retryable back-pressure |
| `ring_publish/src/lib.rs:183-210` | The spin the next producer enters, documented `# Panics: Never` |
| `ring_publish/docs/pitfall/001_publishing_a_range_you_never_claimed.md` | The publish-side mirror, and PB35 on the public constructor |

### Tests

| File | Relationship |
|------|--------------|
| — | **No test covers any of the four routes, and none can.** Reproducing the strand requires observing a publication that never arrives, and `ring_claim` has no dependency — normal or dev — on `ring_publish` (`Cargo.toml`: `ring_types`, `ring_cursor`, `ring_gating`). The suite never publishes because it cannot |
| `tests/claim_test.rs:4-6` | The header says so directly: the handshake's *"full reached-test lives in `ring_publish/tests/handshake_test.rs`, because the handshake is only observable once publishing exists"* |
