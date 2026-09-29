# Decisions — ring_testkit

Open questions and the closed ones worth keeping the reasoning for. Every entry
is either **Closed** with what settled it, or **Pending** with what would settle
it — nothing is left as a preference.

Two of them are recorded at full length as their own records, because each has
since grown a measurement the register paragraph does not carry:

| ID | Name | Rules | Status |
|----|------|-------|--------|
| 001 | [The Record Type Is A `u32`](001_the_record_type_is_a_u32.md) | Whether a script may drive a caller's own records | pending — and TK13 records that half the crate's surface is already generic |
| 002 | [The Model Lives In Tests](002_the_model_lives_in_tests.md) | Where the loom model is placed, and why | closed — and TK15 records that the half it moved has no coverage number |

## Closed

### Closed 1 — Does the crate contain a model checker?

**No.** `loom` already is one — four crates in the
family (`ring_atomic`, `ring_publish`, `ring_spsc`, `ring_mpsc`) depend on it
under `cfg(loom)`. Writing another would be duplication of the most expensive
kind.

What this crate contributes to that half is the **bridge**:
`leak` and `leak_ends`, the two allocations a `loom::thread::spawn` closure
needs to reach a borrow-based ring, and `audit_received`, the assertion worth
making once inside the closure. The model itself is
`tests/exhaustive_test.rs`. → [`integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md).

### Closed 2 — Where does the loom model live, `src/` or `tests/`?

**`tests/`**, and the reason is measurement rather than taste. Under `--cfg
loom`, `ring_atomic` swaps in loom's atomics; an ordinary coverage run does not
set the cfg, and `cfg`-removed lines in `src/` are counted as *uncovered* by
`cargo tarpaulin`. A model in `src/` would put a permanent hole in the crate's
coverage that no test could close.

Keeping `src/` cfg-free is what lets the scripted fixture measure 83/83 on an
ordinary run, with the exhaustive half graded by its own command.

### Closed 3 — Is `vanished` a field or a method?

**A method.** It is `accepted - received.len() - in_ring_at_end`, derived from
three fields that are already stored; a tenth field would be a second copy of
the same number, free to disagree with them if a step updated one and forgot the
others. → [`type/001`](../type/001_outcome_and_anomaly.md).

### Closed 4 — Does `audit` fail on a vanished record?

**No, deliberately.** A record accepted and destroyed by `OverflowPolicy::DropNewest`
is *accounted for* — it is counted in `accepted`, which is all the accounting law
asks. Whether destruction is acceptable is the caller's policy question, and
`vanished` is how they ask it.

Folding the two together would make a legal configuration report as a violation,
so a fixture could not be used to model the policy the family supports.
→ [`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md).

### Closed 5 — Does `Step::Reopen` skip the close when the ring is already open?

**No.** `if !shutdown.is_closed() { .. }` would make the single-threaded no-op
cheaper and the concurrent hazard *worse* — the gap between the check and the
close is another window, in exactly the setting where the window matters. The
mechanism is left alone and documented instead.
→ [`pitfall/002`](../pitfall/002_reopening_closes_first.md).

## Pending

### Pending 1 — Should a `Script` be generic over the record type?

**What is undecided:** `Script::run` takes a `Ring< u32 >`. A caller whose real
records are `MyEvent` cannot drive them through a script; they can drive the
same *shape* of sequence and read the outcome.

**Why it is currently `u32`:** a generic script has to be told how to produce
records — a closure, a seed, an iterator — and every one of those is a way for
two runs to differ. The fixture's whole claim is that the same script produces
the same outcome, and consecutive `u32`s from `0` make that true by construction
rather than by the caller being careful. `Outcome`'s `PartialEq`, which is the
comparison the claim rests on, would also need `T : PartialEq`.

**What would settle it:** a consumer whose record type has behaviour the fixture
needs to exercise — a `Drop` impl whose runs should be counted, say, or a
payload whose bytes should be checked on the way out. Until then the record is a
label, and a `u32` is the cheapest label that is also ordered.

### Pending 2 — Should `ring_shutdown` offer an unconditional `open`?

**What is undecided:** there is no way to reopen a `Shutdown` without first
closing it, because `Stopped::reopen` consumes a token only `close` produces.

**Why it is not this crate's decision:** the by-value token is `ring_shutdown`'s
deliberate design — "a drain is only sound while the ring is closed, so the
proof that it is closed must not survive reopening" — and it is a good rule. An
`open( &self )` alongside it would let a caller reopen a ring while another
thread held a `Stopped` and was midway through a drain, which is the exact
unsoundness the token prevents.

**What would settle it:** a caller that genuinely needs to reopen without having
closed, and can say why its drain-safety is guaranteed some other way. This
crate is not that caller — the window is invisible to a single-threaded script.
Recorded here so a concurrent consumer discovering it finds the reasoning rather
than the surprise. → [`pitfall/002`](../pitfall/002_reopening_closes_first.md).

### Pending 3 — Should `ring_core` expose a cursor?

**What is undecided:** `ring_tls::TlsBuffer::flush_into` claims a contiguous run
against a `SeqCell` in one `fetch_add` — the amortisation `flush_into` exists to
provide — and a `ring_core::Ring` has no cursor to hand it. So `flush_into` has no
caller anywhere in the family, including in the one crate that stages and
publishes in the same function.

**Why it is not decided here:** this is the second crate to reach the same wall
from a different side. `ring_debug`'s `check`/`Watch` are unreachable from a
live `ring_core::Ring` for the identical reason. Widening `ring_core`'s public
surface to satisfy two consumers is a decision about `ring_core`, and neither a
diagnostic crate nor a testkit is the right place to make it.

**What would settle it:** a consumer on the family's export Contract — a
`ring_factory` caller, most likely — that stages into a `TlsBuffer` and
publishes into a `ring_core::Ring` in a hot path, where one `fetch_add` per
batch against one per record is a measured difference rather than an argued one.
→ [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md).

### Pending 4 — Is `leak_ends` the right shape, or should the ends be scoped?

**What is undecided:** both leak helpers allocate and never free. That is the
established pattern for loom in this family — `ring_spsc`'s own model has a
`leaked_ring` with the same comment — but it is a pattern, not a proof that
nothing better exists.

**The argument each way:** loom has no scoped threads, so a scoped API would
have to be an owning wrapper that hands out `'static`-looking ends and reclaims
them at the end of the model. That is a real type with real unsafe in it, to
avoid an allocation that a loom execution dwarfs. The leak is honest and one
line; the alternative is subtle and many.

**What would settle it:** a model whose execution count is large enough that the
per-execution leak actually matters, measured. Two slots and one push is not
that model.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/decisions
printf 'records:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'closed / pending entries: %s / %s\n' \
  "$( command grep -c '^### Closed ' readme.md )" "$( command grep -c '^### Pending ' readme.md )"
```

Live output:

```
records:                  2
finding headings inside:  4
rows in the table below:  4
closed / pending entries: 5 / 4
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK13 | Pending 1's scope | n/a — observation | The entry asks whether a `Script` should be generic and argues entirely about minting, while `leak` and `leak_ends` eighty-one lines above `Script::run` are already `< T : Send >` — the crate answered the same question yes for the loom bridge and the entry does not say so. |
| TK14 | Pending 1's stated cost | n/a — doc gap | The settlement condition names `T : PartialEq` as the bound a generic script would need, and `audit_received`'s `value >= minted` pass needs a notion of *mintedness* that no standard trait supplies — a design decision the entry does not name among its costs. |
| TK15 | Closed 2's coverage payoff | n/a — coverage | The `83/83` the decision is credited with is `src/lib.rs`'s, measured by a tarpaulin invocation filtered to `src`; the 163-line file the decision exists to place is behind `#![ cfg( loom ) ]` and is measured by nothing. |
| TK16 | doc examples under the cfg | **latent hazard** | Twenty-four test files in the family carry `#![ cfg( not( loom ) ) ]` because loom's atomics panic outside a model, and three of this crate's four doc examples construct a `Ring` from a comment that no attribute can gate — checked by nothing, since the one stage that sets the cfg runs clippy. |
