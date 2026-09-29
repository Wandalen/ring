# Pitfall: The Window Between the Gate and the Advance

### Scope

**Purpose:** Record that `claim_gated` checks for room and then advances the
cursor in a second, separate operation, that several producers can therefore be
granted the same room at once, how often that happens, and what the family's
higher tier did about it.

**Responsibility:** The three-statement body of `claim_gated` — two `Acquire`
loads, a comparison, and a delegated `fetch_add` — under contention.

**In Scope:** `ring_batch/src/lib.rs:198-199`, `:321-328`;
`ring_claim/src/lib.rs:432-445`.

**Out of Scope:** That the two cells may be the same cell is
[`type/001`](../type/001_the_ring_that_can_gate_against_itself.md). The
arithmetic overflow in `end()` is
[`pitfall/002`](002_the_addition_with_no_panics_section.md). Disjointness — which
this race does *not* break — is
[`invariant/001`](../invariant/001_disjointness_is_free.md).

---

## Check, Then Advance

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gate, and the advance it does not hold a lock across --'
command grep -m1 -A7 -F '  let at = producer.load( Ordering::Acquire );' ring_batch/src/lib.rs
echo '  -- and what the other entry point says about outrunning the ring --'
command grep -m1 -A1 -F '/// Performs **no gating**. A claim taken without consulting a consumer barrier' ring_batch/src/lib.rs
```

Live output:

```
  -- the gate, and the advance it does not hold a lock across --
  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  if ( free_slots( at, behind, capacity ) as usize ) < count
  {
    return Err( RingError::Full );
  }

  Ok( claim( producer, count, order ) )
  -- and what the other entry point says about outrunning the ring --
/// Performs **no gating**. A claim taken without consulting a consumer barrier
/// can outrun the ring; [`claim_gated`] checks first — exactly for a single
```

Two loads compute how much room there is. A separate `fetch_add`, some
instructions later, takes it. Nothing links the two: between the comparison and
the advance, any number of other producers can run the same check against the
same unchanged cursor values, reach the same conclusion, and each take the room
that only one of them can have.

---

### BA42 — Every Producer That Passes the Gate Is Told About the Same Free Slots

Measured. Sixteen producers hitting the gate simultaneously on a ring whose
consumer never moves, so the legal ceiling is exactly the capacity:

```
--- the gate checks, then a separate operation advances ---
--- consumer never moves, so the ring never has room beyond its capacity ---

  threads   per claim   capacity   rounds overrun of 20000   worst overrun
       16          64        512                    105              64
       16           1          1                    554               2
        8         128        512                    261             256
        4           8         64                      0               0
        2           8         64                      0               0
```

**Finding.** The cursor advances past the ring's capacity in roughly one round in
two hundred at sixteen contending producers, and when it does, the excess is a
whole batch — 64 sequences at the 64-item configuration, 256 at the 128-item one.
Those are sequences granted to a producer that will write into slots the consumer
has not finished reading. The four- and two-thread rows are clean zeros across
20,000 rounds each: the window is narrow enough that the race needs real
contention to open, which is exactly what makes it the kind of bug that survives
a test suite.

The rates vary between runs — this is a timing race, not a deterministic fault —
but the worst-case magnitude is structural. With `t` producers each asking for
`n`, all `t` can pass a gate that had room for one, so the overrun is bounded by
`( t - 1 ) × n` and nothing in the function bounds it lower.

Two things make this worth writing down rather than filing as a defect. The
first is that the crate says the opposite: `claim`'s doc offers `claim_gated` as
"the form that checks first," which a reader will take as the form that does not
outrun the ring. It does check first. Checking first is not the property that
prevents the overrun.

The second is that nothing is currently exposed to it —
[`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md)
BA40 records that `claim_gated` has no caller outside this crate's own tests, and
that the only real dependant, `ring_tls`, imports the ungated half. The hazard is
latent in the precise sense: real, reachable through the public API, and reached
by nobody yet.

`claim_gated`'s own doc comment already carried a `# One producer only`
section stating the race precisely — "Safe for a single producer, racy for
several." The overclaiming this finding names is one clause away from there,
in `claim`'s own doc pointer to it:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A2 -F '/// can outrun the ring; [`claim_gated`] checks first' ring_batch/src/lib.rs
```

Live output:

```
/// can outrun the ring; [`claim_gated`] checks first — exactly for a single
/// producer, advisory under several (see its own `# One producer only`
/// section below). Both exist because the SPSC path knows its own consumer
```

**Disposition:** applied — reworded `claim`'s doc comment in `src/lib.rs` so
"is the form that checks first" no longer stands alone; it now names the
single-producer scope inline, self-contained rather than requiring a reader
to already be inside `claim_gated`'s own doc to find the caveat. The
structural fix (a CAS loop) is deliberately not applied here — that is
BA43's own conclusion, that a `fetch_add`-based claim cannot have one without
becoming `ring_claim`. The crate's 21 unit tests plus 10 doctests
re-verified passing (`cargo test --all-features`, 2026-09-04). Now prints:
`checks first — exactly for a single`

---

### BA43 — The Fix Is Written Down One Tier Up, With This Failure Mode Named

`ring_claim::Claimer::claim` does the same job and does not have this race:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the comment that names it --'
command grep -m1 -A3 -F '    // The gate is the loop condition, and is therefore re-read on every' ring_claim/src/lib.rs
echo '  -- and the loop that closes it --'
command grep -m1 -B1 -A8 -F '    while count <= self.consumers.headroom( current )' ring_claim/src/lib.rs
```

Live output:

```
  -- the comment that names it --
    // The gate is the loop condition, and is therefore re-read on every
    // iteration: on a failed exchange another producer moved the cursor, so
    // the headroom computed against the old value is stale and granting on it
    // would overlap that producer's range.
  -- and the loop that closes it --
    let mut current = self.claimed();
    while count <= self.consumers.headroom( current )
    {
      let next = current.advanced_by( count as u64 );
      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
      {
        Ok( _ ) => return Ok( Claim::new( current, count ) ),
        Err( actual ) => current = actual,
      }
    }
```

**Finding.** The structure is the whole answer: `compare_exchange` fails if
another producer moved the cursor since the gate was read, and the failure
re-enters the loop, which re-reads the gate. The gate and the advance become one
operation because the advance can refuse. `claim_gated`'s `fetch_add` cannot
refuse — it is unconditional by construction, which is precisely why it is fast
and precisely why a check in front of it is advisory.

So the family already contains the correct implementation, three tiers up, with a
four-line comment naming the exact hazard: "the headroom computed against the old
value is stale and granting on it would overlap that producer's range." Neither
crate references the other. A reader of `ring_batch` gets a gate that reads like
a guarantee; a reader of `ring_claim` gets the reason it is not, attached to a
different function in a crate they have no reason to open.

The one-sentence fix in this crate is not a CAS loop — a `fetch_add`-based claim
cannot have one without becoming `ring_claim`. It is `claim_gated` saying what it
is: a check that is exact for a single producer and advisory for several.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](../type/001_the_ring_that_can_gate_against_itself.md) | The second way this signature grants room that is not there |
| [`invariant/001`](../invariant/001_disjointness_is_free.md) | The property this race does *not* break, and the test that proves the wrong thing |
| [`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md) | Why nothing is exposed to this yet |
| [`algorithm/002`](../algorithm/002_check_then_advance.md) | The same three statements, read as a procedure rather than a hazard |

### Sources

| Fact | Where |
|------|-------|
| The gate and the advance | `ring_batch/src/lib.rs:321-328` |
| The claim that gating prevents overrun | `ring_batch/src/lib.rs:198-199` |
| The overrun rates and magnitudes | Release probe, quoted above |
| The CAS loop and its comment | `ring_claim/src/lib.rs:432-445` |

### Tests

| Test | Covers |
|------|--------|
| `a_full_ring_refuses_with_full_and_advances_nothing` | The gate, single-threaded, where it is exact |
| `concurrent_batch_claims_never_overlap` | Disjointness under contention — which holds, and is not this |
| *(to create)* | A contended gate asserting the cursor never passes `consumer + capacity`; nothing in the suite asserts it |
