# API: A Result Whose Error Is Not an Error

### Scope

- **Purpose**: Account for `try_publish`'s `Result< Seq, Seq >` — the one signature in the family whose error type is not an error type — and for the crate's complete absence of `RingError`.
- **Responsibility**: Place the signature against all 39 fallible signatures in the family, show where the shape comes from, state what it buys over the alternatives, and record what it costs.
- **In Scope**: `Result< Seq, Seq >`, and the `RingError` this crate does not import.
- **Out of Scope**: The operation's semantics — see [`algorithm/001`](../algorithm/001_the_compare_exchange_that_refuses.md).

### The Signature, Against Every Other Fallible One in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rhE '^\s*pub (const )?fn .*-> *Result<' ring_*/src/*.rs \
  | sed 's/.*-> *//' | sed 's/.*, *//;s/ *>.*//' | sort | uniq -c | sort -rn
```

Live output:

```
     21 RingError
      4 WorkloadError
      3 Violation
      3 T
      3 Anomaly
      2 BuildError
      1 Seq
      1 RunError
      1 Refusal< T
```

Thirty-nine public functions across the 33 crates return a `Result`. Grouped by
error type:

| Error type | Signatures | What it is |
|------------|-----------:|------------|
| `RingError` | 21 | the family's one shared error type |
| A crate-local error — `WorkloadError` 4, `Violation` 3, `Anomaly` 3, `BuildError` 2, `RunError` 1, `Refusal< T >` 1 | 14 | harness, debug and factory crates |
| A generic `T` | 3 | passthroughs |
| **`Seq`** | **1** | **`try_publish`** |

Every other fallible operation in the family answers *what went wrong*. This one
answers *where the frontier is*, which is not a fault report — and the signature
says so before the documentation does.

### PB12 — The Only Crate That Names `RingError` Without Declaring or Importing It

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do n=$( basename "$c" ); f="$c/src/lib.rs"; [ -f "$f" ] || continue
  if grep -q 'RingError' "$f" && ! grep -qE '^use .*RingError' "$f"; then
    echo "-- $n"; grep 'RingError' "$f"
  fi
done
```

Live output:

```
-- ring_publish
    /// earlier claim has not been published yet. Deliberately not a `RingError`:
-- ring_types
//! | `error` | [`RingError`] — the one error type the family returns |
pub use error::RingError;
```

The 33 crates divide four ways on `RingError`:

| | Count | Crates |
|--|------:|--------|
| Declares it | 1 | `ring_types` (`src/lib.rs:35`, re-exported from `error`) |
| Imports and uses it | 18 | including all three Tier 5 siblings |
| Never names it | 13 | `ring_align`, `ring_atomic`, `ring_cursor`, `ring_seqno`, … |
| **Names it, does not import it** | **1** | **`ring_publish`** |

The single mention is `src/lib.rs:144`, inside `try_publish`'s `# Errors`
section, and it is an argument for the absence:

> Deliberately not a `RingError`: this is not a failure, it is
> `compare_exchange`'s "try again", and the value returned is what to try
> against next.

The three Tier 5 siblings all went the other way — `ring_claim::claim ->
Result< Claim, RingError >`, `ring_consume::commit -> Result< Seq, RingError >`,
`ring_barrier` returning `Option` for its two partial answers. This crate is the
one operation in the handshake whose only non-success outcome is *not yet*, and
`RingError` has no variant for that. Its nearest candidates are `Full` and
`Empty`, which `ring_types::error` groups under `is_transient` — and both name a
*ring* condition (no space, no data) rather than a *turn* condition. A
`try_publish` returning `Err( RingError::Full )` when a predecessor is mid-write
would be reporting backpressure that does not exist.

The whole crate imports two items:

```rust
use ring_cursor::{ PaddedCursor, SeqCell, GATING };  // :57
use ring_types::Seq;                                 // :58
```

Four names, two crates, no error type. Against its three Tier 5 siblings:

| Crate | `use` lines | Names | Imports `RingError` |
|-------|:-----------:|:-----:|:-------------------:|
| `ring_claim` | 3 | 6 | ✔ |
| `ring_consume` | 3 | 6 | ✔ |
| `ring_barrier` | 2 | 4 | ✔ |
| **`ring_publish`** | **2** | **4** | **—** |

Tied with `ring_barrier` for the smallest set, and the only one of the four that
does not import the family's error type at all.

### Where the Shape Comes From

`Result< Seq, Seq >` is not invented here. It is `SeqCell::compare_exchange`'s
own return type, declared once and implemented three times:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Result< Seq, Seq >' */src/*.rs
```

Live output:

```
ring_atomic/src/lib.rs:  -> Result< Seq, Seq >;
ring_atomic/src/lib.rs:  -> Result< Seq, Seq >
ring_atomic/src/lib.rs:  -> Result< Seq, Seq >
ring_cursor/src/lib.rs:  -> Result< Seq, Seq >
ring_publish/src/lib.rs:  pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
```

| Site | Role |
|------|------|
| `ring_atomic:150` | the `SeqCell` trait declaration |
| `ring_atomic:229` | `AtomicSeq`'s implementation |
| `ring_atomic:495` | `CountingSeq`'s implementation |
| `ring_cursor:219` | `PaddedCursor`'s forwarding implementation |
| **`ring_publish:161`** | **the only public API in the family that returns it** |

`ring_claim` calls the same `compare_exchange` twice (`:407`, `:452`) and
destructures the result immediately — `Ok( _ ) => return Ok( Claim… )`,
`Err( actual ) => current = actual` — converting to `RingError` at the boundary.
So the trait's shape reaches the family's public surface exactly once, and it is
here.

That is a deliberate pass-through rather than a leak. `ring_atomic:143-148` states
the trait's own contract for the error arm as *"The sequence actually found, when
it was not `current` — the multi-producer claim's retry input"*, and `try_publish`
is the one operation whose caller-facing semantics are identical to the atomic's:
the exchange failed, here is what was there instead, decide for yourself.

### What the Shape Buys, and What It Costs

**Buys.** `tests/publish_test.rs:75-77` states it:

> The error value is not decoration — it is what B retries against, so a version
> returning `RingError` or `()` would force B to re-read separately and race
> again.

A `Result< Seq, RingError >` or a `bool` would make the caller call `published()`
to learn where the frontier is: a second atomic load, issued after the exchange,
answering a question the exchange already answered — and answering it about a
*later* moment, so the value can be stale in a way the exchange's own failure
value cannot.

**Costs.** Three, all real and none currently biting:

1. **`?` does not work.** `Seq` implements no error trait, so a caller cannot
   propagate. That is correct — a refusal is not propagatable, it is retryable —
   but it means any caller integrating `try_publish` into a `Result< _, RingError >`
   function must map explicitly. No caller exists to have found this awkward
   ([`api/001`](001_six_methods_and_no_caller.md) § PB10).
2. **The two arms have the same type and different meanings.** `Ok( Seq )` is
   *the new frontier, which is yours*; `Err( Seq )` is *the current frontier,
   which is not yours*. A misread `unwrap_or_else( | e | e )` would compile,
   run, and silently treat a refusal as a success. Nothing prevents it.
3. **The `Ok` payload is synthesised.** `compare_exchange` returns the *previous*
   value on success; `:164`'s `.map( | _ | end )` discards it and substitutes
   `end`. The discarded value is `start`, which the caller supplied — so nothing
   is lost, but the two arms are no longer symmetric with the atomic's own, and
   a reader tracing the type through from `ring_atomic` will find the payload
   changed identity at this line.

Cost 2 is the one worth a second look, because it is the failure mode that a
`RingError` would have made impossible. It has not occurred, and the reason it
has not is that the only in-crate caller — `publish:204` — uses `if let Ok(…)`,
which cannot express the confusion.

### The Absent Variant

`ring_types::RingError` is `#[ non_exhaustive ]` and carries nine variants. None
of them fits, and the closest reading of why is that publication has no failure
mode at all:

| Outcome | Reportable as | Actually |
|---------|---------------|----------|
| It is your turn | `Ok` | the frontier advanced |
| A predecessor has not published | *no variant* | try again, unchanged |
| Your start is behind the frontier | *no variant* | a caller bug ([`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md)) |
| The ring is full | — | cannot happen here; that is `ring_claim`'s gate |
| The ring is closed | — | this crate has no lifecycle ([`lifecycle/001`](../lifecycle/001_a_slot_from_claim_to_visibility.md)) |

The third row is the uncomfortable one: a genuine caller error is reported
through the same `Err( Seq )` as ordinary contention, and the two are
distinguishable only by comparing the returned frontier against the caller's own
`start`. Nothing in the crate does that comparison, and `publish` cannot — which
is exactly how the caller bug becomes a hang rather than a diagnostic
([`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md) § PB9).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The operation with this signature, and its three input positions |
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | The caller that discards the error arm |

### APIs

| File | Relationship |
|------|--------------|
| [001_six_methods_and_no_caller.md](001_six_methods_and_no_caller.md) | The surface this signature sits on |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | The two-crate import set this signature is built from |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The caller error reported through the same arm as ordinary contention |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_the_one_cast.md](../type/001_a_seq_a_usize_and_the_one_cast.md) | `Seq`, in both arms and both parameters |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:57-58,138-165` | The two imports, and the signature with its `# Errors` contract |
| `ring_atomic/src/lib.rs:143-150,228-235,494-499` | The trait declaration this shape comes from, and its two implementations |
| `ring_cursor/src/lib.rs:216-219` | The forwarding implementation this crate actually calls |
| `ring_claim/src/lib.rs:411,425-429,465,475-479` | The sibling that destructures the same shape into `RingError` |
| `ring_consume/src/lib.rs:425` | The other Tier 5 `Seq` return, with an error type |
| `ring_types/src/error.rs:42-80,146-159` | The nine variants, `#[ non_exhaustive ]`, and `is_transient` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:72-85` | The error arm's value asserted, with the reason it is a `Seq` |
| `tests/publish_test.rs:87-99` | The caller-error direction, refused through the same arm |
| `tests/handshake_test.rs:357-380` | Out-of-order publication refused rather than advancing past a gap |
