# API: Two Claim Functions, One Caller

### Scope

**Purpose:** Record which of the twelve public items anything in the family
actually reaches, and what that says about the feature the crate delivers.

**Responsibility:** `ring_batch`'s manifest dependents and every import of its
items family-wide.

**In Scope:** `ring_tls/src/lib.rs:44`;
`ring_tls/tests/tls_test.rs:20`; `ring_cursor/src/lib.rs:46`.

**Out of Scope:** The surface itself is
[`api/001`](001_twelve_items_seven_must_use.md). Why the ungated and gated forms
are two functions is
[`pattern/002`](../pattern/002_two_functions_where_one_would_have_hidden_it.md).

---

## Everything That Reaches In

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- who names ring_batch in a manifest --'
command grep -rl '^ring_batch = ' --include=Cargo.toml .
echo '  -- what the one dependent imports --'
command grep 'use ring_batch' ring_tls/src/lib.rs ring_tls/tests/tls_test.rs
echo '  -- every mention of the other two functions, family-wide, outside this crate --'
command grep -r 'claim_gated\|drain_order' --include=*.rs . | command grep -v '^ring_batch/'
```

Live output:

```
  -- who names ring_batch in a manifest --
ring_tls/Cargo.toml
  -- what the one dependent imports --
ring_tls/src/lib.rs:use ring_batch::{ claim, BatchClaim };
ring_tls/tests/tls_test.rs:use ring_batch::BatchClaim;
  -- every mention of the other two functions, family-wide, outside this crate --
ring_cursor/src/lib.rs://! rather than take a parameter — the same choice `ring_batch::claim_gated`
```

One manifest edge, one import line, and one comment.

---

### BA7 — The Crate Has One Dependent and It Uses a Third of the Surface

`ring_tls` is the only crate in the family whose manifest names `ring_batch`.
It imports `claim` and `BatchClaim`, and its use of `claim` is a single call at
`src/lib.rs:252` — `claim( cursor, self.items.len(), order )` — with the result
stored in a `BatchClaim` field.

**Finding.** Of twelve public items, four are reached from outside the crate:
`BatchClaim`, `claim`, and (through `ring_tls`'s own accessor) `len` and
`start`. The other eight — including both of the crate's other free functions —
have no caller anywhere in the 33 crates.

That is not the usual shape of an unused function. `claim_gated` and
`drain_order` are not conveniences left in place against future need; they are
half the crate's stated deliverable. This crate's own contract is titled "Batch Claim And
Batch Drain," and its acceptance criterion has three clauses of which the third
is "a batch drain reads them in issue order." That clause is implemented, tested
here, and called by nothing.

---

### BA8 — The Only Cross-Crate Mention of `claim_gated` Cites It as a Precedent, Not as a Function

`ring_cursor/src/lib.rs:46` is the sole appearance of `claim_gated` outside this
crate:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A6 -F '//! [`CursorPair`]'"'"'s gating readings are the opposite case, and fix `Acquire`' ring_cursor/src/lib.rs
```

Live output:

```
//! [`CursorPair`]'s gating readings are the opposite case, and fix `Acquire`
//! rather than take a parameter — the same choice `ring_batch::claim_gated`
//! makes, for the same reason. A caller asking "may I claim?" is about to
//! overwrite a slot on the answer; a `Relaxed` load there would let it act on a
//! stale barrier and overwrite a slot the consumer had not finished with. There
//! is no legitimate second option to offer, so offering one would only be a way
//! to get it wrong.
```

**Finding.** `ring_cursor` cites this crate for a *design decision* it copied,
not for behaviour it depends on — and the two crates have no dependency edge in
either direction. It even restates `claim_gated`'s own justification almost
word for word: `ring_batch/src/lib.rs:236-237` says a `Relaxed` load "would let
a producer act on a stale barrier and overwrite a slot the consumer had not
finished with," and the paragraph above is that sentence rewritten. The
reasoning propagated across the family by being read, which is the only
mechanism available — nothing in a manifest records that one crate's ordering
choice was justified by another's.

So the gated function's measurable influence on the family is entirely
documentary. It has shaped one other crate's public API and has never been
executed outside its own test suite — which is also why the window recorded in
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md)
has never cost anything.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_twelve_items_seven_must_use.md) | The surface these four items are drawn from |
| [`integration/002`](../integration/002_the_feature_is_planned_its_problems_are_addressed.md) | The dependent in full, and the feature's status |
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | The hazard that has never been reached because of BA7 |
| [`decisions/001`](../decisions/001_ordering_is_the_callers_except_where_it_is_not.md) | The reasoning `ring_cursor` copied |

### Sources

| Fact | Where |
|------|-------|
| The only manifest dependent | `ring_tls/Cargo.toml` |
| The import and the call | `ring_tls/src/lib.rs:44`, `:280` |
| The precedent citation | `ring_cursor/src/lib.rs:45-51` |

### Tests

| Test | Covers |
|------|--------|
| `a_batch_drain_reads_in_issue_order` | The uncalled third clause, asserted here |
| `drain_order_folds_through_ring_index_and_not_a_second_implementation` | The uncalled function's arithmetic |
| *(to create)* | Nothing tests reach; the census above is the only check that exists |
