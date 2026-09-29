# Decisions: Ordering Is the Caller's, Except Where It Isn't

### Scope

**Purpose:** Record the crate's one ordering decision — `order` for the advance,
hard-coded `Acquire` for the two gating reads — its nine-line justification, and
the fact that nothing in the toolchain can check it here.

**Responsibility:** The `order` parameter's scope, the two `Ordering::Acquire`
literals, and the `loom` seam this crate does not sit on.

**In Scope:** `ring_batch/src/lib.rs:232-240`, `:321-322`;
`ring_batch/Cargo.toml`.

**Out of Scope:** The error split is
[`decisions/002`](002_two_errors_not_one.md). What goes wrong in the gap between
the two loads and the advance is
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md).

---

## The Decision, and Every Literal It Governs

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the justification --'
command grep -m1 -A8 -F '/// `order` governs the advance only. The two gating reads are always' ring_batch/src/lib.rs
echo '  -- every Ordering literal outside a doctest --'
command grep -E 'Ordering::[A-Za-z]+' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- crates wired for loom --'
command grep -rl 'loom' --include=Cargo.toml . | command grep -v '^ring/Cargo.toml$' | sed 's|ring/||;s|/Cargo.toml||' | sort | tr '\n' ' '; echo
echo "  -- ring_batch among them: $( command grep -c loom ring_batch/Cargo.toml || true ) --"
```

Live output:

```
  -- the justification --
/// `order` governs the advance only. The two gating reads are always
/// `Ordering::Acquire`, chosen here rather than left to the caller, because
/// they are not free choices: the whole point of reading the consumer's
/// position is to establish that its writes happened-before this claim, and a
/// `Relaxed` load would let a producer act on a stale barrier and overwrite a
/// slot the consumer had not finished with. This crate leaves the *advance*
/// ordering open because it genuinely varies with the protocol built on top;
/// the gating loads do not vary, so pretending they were a parameter would
/// offer a caller a choice with exactly one correct answer.
  -- every Ordering literal outside a doctest --
  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  -- crates wired for loom --
ring_atomic ring_mpsc ring_publish ring_spsc ring_testkit 
  -- ring_batch among them: 0 --
```

---

### BA13 — Nine Lines to Justify Two Literals, and They Are the Only Two

The whole crate contains exactly two `Ordering` values outside doctests, both
`Acquire`, both on lines 321–322. Nine lines of doc comment argue for them —
the longest continuous justification anywhere in `ring_batch`, longer than any
function's body.

**Finding.** The decision is a *scope* decision rather than a value decision.
`claim` forwards whatever the caller names; `claim_gated` forwards it for the
advance and refuses to forward it for the two reads. The stated principle is
that a parameter with exactly one correct answer is not a parameter, and the
crate applies it asymmetrically on purpose: the advance ordering "genuinely
varies with the protocol built on top," the gating loads do not.

The principle travelled. `ring_cursor/src/lib.rs:45-51` makes the same split for
`CursorPair` and cites this function by name as the precedent, between two
crates with no dependency edge — see
[`api/002`](../api/002_two_claim_functions_one_caller.md) BA8. This is the one
piece of `ring_batch` with measurable influence on the rest of the family, and
it is a paragraph rather than a function.

---

### BA14 — The Argument Is Unfalsifiable Here, Because the Crate Is Not on the `loom` Seam

Five crates carry `loom` in their manifest: `ring_atomic`, `ring_mpsc`,
`ring_publish`, `ring_spsc`, `ring_testkit`. `ring_batch` is not one of them.

**Finding.** An ordering argument is checkable in exactly one way — model-check
the interleavings and see whether the claimed happens-before actually holds. The
family owns that machinery, wires it through `RUSTFLAGS` rather than a feature,
and points it at the five crates above. The crate that spends nine lines arguing
for `Acquire` is not among them, so its argument is prose all the way down.

Two consequences follow, and they are different in kind. The first is
measurable: copy the crate, downgrade both literals to `Relaxed`, and run
everything.

```
both gating loads downgraded to Relaxed
running 21 tests
test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
running 10 tests
test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

Twenty-one integration tests and ten doctests, all green, with the exact
downgrade the nine lines argue against. The suite observes results and never
orderings, so the justification has no test that could contradict it.

The second consequence is worse: the same absence is why the window recorded in
[`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) is
not a `loom` failure but a statistical measurement. A `loom` run over
`claim_gated` would have found that window on its first exhaustive pass, because
finding exactly that class of interleaving is what `loom` is for.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](002_two_errors_not_one.md) | The crate's other decision, which does have tests |
| [`pitfall/001`](../pitfall/001_the_window_between_the_gate_and_the_advance.md) | The interleaving the missing seam would have caught |
| [`api/002`](../api/002_two_claim_functions_one_caller.md) | Where this reasoning travelled to |
| [`algorithm/002`](../algorithm/002_check_then_advance.md) | The three steps these two loads sit inside |

### Sources

| Fact | Where |
|------|-------|
| The justification | `ring_batch/src/lib.rs:232-240` |
| The two literals | `ring_batch/src/lib.rs:321-322` |
| The five `loom` crates | Census above |
| The `loom` seam's mechanism | `Cargo.toml` workspace lints, `unexpected_cfgs` check-cfg entry |
| The suite passing with both loads `Relaxed` | `cargo test --all-features` on a patched copy, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_gated_claim_succeeds_while_the_ring_has_room` | That the loads happen, by operation count |
| `an_oversized_request_is_refused_before_the_ring_is_even_consulted` | That they do *not* happen on the size-check path |
| *(to create)* | The orderings themselves, which need this crate on the `loom` seam to be observable at all |
