# Workaround: Reimplementing the Pause Hint

### Scope

- **Purpose**: Record that this crate emits its own `core::hint::spin_loop()` at three sites rather than calling `ring_wait::pause`, what that costs, and the condition under which it could be deleted.
- **Responsibility**: The constraint being absorbed, the shape of the local replacement, the behavioural difference from the thing it replaces, and where the reason is written down.
- **In Scope**: The three hint sites, the helper one crate over, and the backoff policy the two do not share.
- **Out of Scope**: Why the dependency is refused at all (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)); whether the roster enforcing that refusal is correct (→ [`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md)).

### Constraint

`ring_wait` offers `pause( kind : WaitKind, attempt : usize ) -> bool`, and two of
its four `WaitKind` variants are tick-safe. `ring_poll` still does not depend on
it, for a reason argued in
[`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md):
taking the dependency to reach the safe variants puts the parking ones one
autocomplete away, and autocomplete is the failure being defended against.

The constraint is therefore self-imposed rather than external, which is why it
sat outside this directory for so long. It still meets the definition this
directory uses — a cost absorbed here, with a shape and a deletion condition —
and the cost is paid by every reader of this crate's retry loops.

### Replacement

Three sites emit the hint directly:

```rust
attempt += 1;
if attempt < budget.attempts()
{
  core::hint::spin_loop();
}
```

One hint, unconditionally, on every attempt but the last.

### Cost

The readme that used to dismiss this priced it at *"three lines"* — the cost of
duplicating a helper that already exists one crate over. Both halves of that are
wrong, and the second is the expensive one.

**It is not the same behaviour.** `ring_wait::pause` under `WaitKind::Spin` runs
`for _ in 0..=( attempt % 8 ) { core::hint::spin_loop(); }` — an escalating
backoff, one to eight hints depending on how many attempts have already failed.
This crate emits exactly one, always. Adopting the helper would not remove three
lines, it would change the retry policy from flat to escalating, on the one path
where the cost of a retry is the thing the whole crate is about → PL49.

**It is not a shared helper being declined.** No crate in the family calls
`ring_wait::pause` — the only file naming it is `ring_wait`'s own. There is no
convergence being broken here, because there is no convergence.

So the honest cost is not duplication. It is that `ring_poll` has a backoff
policy — a flat one — that nobody wrote down as a choice, arrived at by not
importing the crate that has the other one.

### Deletion condition

This can be deleted when either the roster's enforcement moves somewhere that
can see transitive reachability (→ [`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md)
PL38), making a `ring_wait` dependency safe to hold because the dangerous call
would be caught rather than merely undepended-on; or when a measurement shows
flat and escalating backoff are indistinguishable at this crate's attempt
counts, which `ring_bench` is the place to run.

Neither has happened, and no version, ticket or threshold is attached to either,
so the practical answer today is that it is permanent.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'hint sites in ring_poll:      %s\n' "$( command grep -c 'core::hint::spin_loop' ring_poll/src/lib.rs || true )"
printf 'of those carrying the reason: %s\n' "$( command grep -c 'A pause hint, and nothing more' ring_poll/src/lib.rs || true )"
printf 'ring_poll, hints per pause:   %s\n' "$( awk '/A pause hint, and nothing more/{f=1} f&&/^        \}$/{exit} f' ring_poll/src/lib.rs | command grep -c 'spin_loop' || true )"
printf 'ring_wait, hints per pause:   %s\n' "$( awk '/WaitKind::Spin =>/{f=1} f&&/^    \}$/{exit} f' ring_wait/src/lib.rs | command grep -oE 'for _ in [^{]*' )"
printf 'the helper one crate over:    %s\n' "$( command grep -oE 'pub fn pause\( kind : WaitKind, attempt : usize \) -> bool' ring_wait/src/lib.rs )"
printf 'compiled call sites for it:   %s\n' "$( command grep -rhn 'ring_wait::pause' --include='*.rs' ring_* 2>/dev/null | command grep -vc '///' || true )"
printf 'where the string appears:     %s\n' "$( command grep -rl 'ring_wait::pause' ring_*/src ring_*/tests 2>/dev/null | tr '\n' ' ' )"
printf 'crates depending on ring_wait:%s\n' "$( for f in ring_*/Cargo.toml; do awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' "$f" | command grep -q 'ring_wait' && printf ' %s' "${f%/Cargo.toml}"; done )"
printf 'ring_poll among them:         %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_poll/Cargo.toml | command grep -c 'ring_wait' || true )"
printf 'WaitKind variants:            %s\n' "$( awk '/^pub enum WaitKind/{f=1} f&&/^\}$/{exit} f' ring_types/src/policy.rs | command grep -oE '^  [A-Z][a-z]+,' | tr -d ' ,' | tr '\n' ' ' )"
```

Live output:

```
hint sites in ring_poll:      3
of those carrying the reason: 1
ring_poll, hints per pause:   1
ring_wait, hints per pause:   for _ in 0..=( attempt % 8 )
the helper one crate over:    pub fn pause( kind : WaitKind, attempt : usize ) -> bool
compiled call sites for it:   0
where the string appears:     ring_wait/src/lib.rs ring_poll/tests/manual/readme.md 
crates depending on ring_wait: ring_barrier ring_shutdown
ring_poll among them:         0
WaitKind variants:            Spin Yield Park None 
```

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The guarantee this workaround pays for |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | Part 1 of that pattern — *do not take the dependency* — is what makes this local copy necessary |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_bounded_retry.md`](../algorithm/001_bounded_retry.md) | Where the hint sits in the retry loop, and the pause it is not |

### Workarounds

| File | Relationship |
|------|--------------|
| [`002_a_counter_bounded_while.md`](002_a_counter_bounded_while.md) | The other one, and the contrast: that constraint is external, this one is chosen |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The three `core::hint::spin_loop()` sites, and the one comment among them |

### PL49 — the workaround was priced as duplication, and it is not duplication

The readme dismissed this as not-a-workaround on the grounds that the cost is
*"reimplementing a `spin_loop()` hint that already exists one crate over — three
lines"*. Three lines of duplication is a real but trivial cost, and pricing it
that way is what kept the item out of this directory.

The helper one crate over does something else. `ring_wait::pause` under
`WaitKind::Spin` is `for _ in 0..=( attempt % 8 ) { core::hint::spin_loop(); }`:
the number of hints grows with the attempt count, one through eight, resetting
every eight. `ring_poll` emits exactly one hint per attempt regardless. These are
two different backoff policies, and importing the helper would change this
crate's retry behaviour rather than deduplicate it.

The second half fails too. `ring_wait::pause` has no compiled call site
anywhere in the family. The string appears exactly twice: once in `pause`'s own
doctest, and once in this crate's manual probe P1 — inside a file written to
*fail* to compile, whose whole result is `E0432: unresolved import`. There is no
shared helper being declined and no convergence being broken; the escalating
policy has one implementation and zero consumers.

What that leaves is the finding: `ring_poll` has a flat backoff, the family's
only other spin implementation has an escalating one, and nothing anywhere
records that as a decision. The invariant document argues about *parking*, which
is a `Yield`/`Park` question and correctly rules those out; it does not reach the
`Spin` variant at all, and `Spin` is the one whose policy differs. So the
difference arrived as a side effect of a dependency refusal made for an unrelated
and good reason — which is precisely the shape a workaround is supposed to make
visible, and the reason this file now exists.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F "the backoff policy that differs as a result" ring_poll/docs/workaround/readme.md
```

Live output:

```
| 001 | [Reimplementing the Pause Hint](001_reimplementing_the_pause_hint.md) | A hint emitted locally rather than imported, and the backoff policy that differs as a result | 🔄 |
```

**Disposition:** applied — `workaround/readme.md`'s own Overview Table entry
for this instance no longer prices it as three duplicated lines; its Purpose
column now names the actual cost, the differing backoff policy, matching what
this finding measured. Now prints: `the backoff policy that differs as a result`

### PL50 — the reason is written at one of the three copies

Three sites emit `core::hint::spin_loop()`. One of them, in `push_within`,
carries the explanation: *"A pause hint, and nothing more. Yielding here would be
the parking this crate exists to keep off the tick path — see
`docs/invariant/001`."* The other two, in `push_batch_within` and `recv_within`,
are bare.

That is the ordinary decay of a duplicated fragment, and it matters more than
usual here because of what the comment does. It is not describing the line; it is
warding off the edit that would break the crate's central guarantee — swapping
the hint for a `yield_now()`, which is the natural thing to reach for when a spin
looks too hot. A maintainer profiling `recv_within` sees three lines with no
argument attached to them.

The fix is not three copies of the comment. Both this and PL49 point the same
way: a single private `fn pause_hint( attempt : usize, budget : Budget )` would
give the policy one home, the comment one site, and the flat-versus-escalating
choice somewhere to be written down — at the cost of one more item in a crate
whose whole style is free functions, and of a name that would need to not sound
like `ring_wait::pause`. Recorded rather than applied, because it changes the
crate's shape to fix a documentation problem, and PL49's policy question should
be settled first.
