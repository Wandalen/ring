# Integration: Two Dependencies and the Barrier It Cannot See

### Scope

- **Purpose**: Work out the seams of a crate that declares two dependencies and can reach twenty, and of a policy variant named after a crate that is not among them.
- **Responsibility**: Enumerate the declared edges, the load-bearing transitive ones, the absences, and the failure mode each seam carries.
- **In Scope**: `ring_tls`, `ring_core`; the transitive `ring_batch`; the departed `ring_barrier`; the fourteen unreachable crates; the consumer edge.
- **Out of Scope**: The export boundary (→ [A Decision on the Export Surface](002_a_decision_on_the_export_surface.md)); why the barrier seam is a trap rather than merely a gap (→ [`pitfall/001`](../pitfall/001_on_barrier_cannot_see_the_barrier.md)).

### System Description

**Three declared dependencies. Confirm rather than take it on trust:**

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,$p' ring_flush/Cargo.toml
```

Live output:

```
[dependencies]
ring_tls = { path = "../ring_tls" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

[dev-dependencies]
ring_config = { path = "../ring_config" }

[lints]
workspace = true
```

The dependency and dev-dependency sections, as the manifest's own syntax:

```toml
[dependencies]
ring_tls = { path = "../ring_tls" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

[dev-dependencies]
ring_config = { path = "../ring_config" }
```

That is the whole list — three path dependencies and one dev-dependency, and
`ring_types` is now among them. **The addition of `ring_types` moved this crate
out of the minority it used to belong to**, joining twenty-seven of the
thirty-three that name it directly:

```sh
cd "$(git rev-parse --show-toplevel)"
for m in ring_*/Cargo.toml; do
  sed -n '/^\[dependencies\]/,/^\[/p' "$m" | command grep -q '^ring_types' && echo "$m"
done | wc -l
```

Live output:

```
27
```

This one now depends on it directly rather than reaching it only
transitively through `ring_tls` and `ring_core`. [`FlushPolicy`](../type/001_flush_policy.md) is defined
here, not there — which is a decision worth noting given that the *other*
policy enum, `OverflowPolicy`, ended up living in `ring_types` instead.

**Nineteen crates are reachable.** The gap between three and nineteen is this
instance's subject:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_flush --prefix none --no-dedupe \
  | awk '{print $1}' | command grep '^ring_' | sort -u | wc -l
```

Live output:

```
19
```

**Re-run it rather than trusting the number.** This closure moved during
authoring — it stood at twenty-four, including `ring_barrier` and the whole
consumer-side chain, until `ring_spsc` and `ring_mpsc` were implemented and
dropped their `ring_consume` and `ring_publish` edges. Every count in this
instance ships with the command that regenerates it for that reason.

### Integration Points

| # | Crate | Edge | Load-bearing for |
|---|-------|------|-----------------|
| E1 | [`ring_tls`](../../../ring_tls/readme.md) | **Declared** | As built: `TlsBuffer` itself, `push`, `len`, `is_full`, `capacity`, `drain`. Not `flush_into` |
| E2 | [`ring_core`](../../../ring_core/readme.md) | **Declared** | The ring the drain publishes into; its fullness is `OnFull`'s question |
| E3 | [`ring_batch`](../../../ring_batch/readme.md) | Transitive, via `ring_core` | **The contiguous claim** — what makes a flush cost one fence |
| E4 | [`ring_barrier`](../../../ring_barrier/readme.md) | **None.** Not in the closure at all | **Nothing** — and it would still be nothing if the edge came back. See below |
| E5 | The consumer | Inbound, undeclarable | Every drive call; on `OnBarrier`, the fact itself |
| E6 | [`ring_shutdown`](../../../ring_shutdown/readme.md) | **Unreachable** | `drain_all` vs `drain_final` — an open ownership question |
| E7 | [`ring_overflow`](../../../ring_overflow/readme.md) | Transitive, via `ring_core` | Reachable and unused; owns the rejected-append question this crate leaves open |

**E1 is the seam this crate exists to complete.** `ring_tls` exposes seal, drain
and reset as three separate operations and its own documentation states that the
trigger belongs elsewhere. This crate is that elsewhere — which means the seam is
not a dependency in the ordinary sense but a division of one operation across two
crates, with the failure mode that the halves can drift.

**The seam as built is narrower than that, and `flush_into` is on the wrong side
of it.** `ring_tls` offers two ways to move records out: `flush_into( cursor,
order )`, which fuses claiming and draining against a caller-supplied cursor, and
`drain()`, which yields a `std::vec::Drain` and empties the buffer as that
iterator drops. This crate uses `drain()`.

| | `flush_into` | `drain` |
|---|---|---|
| Publishes through | A cursor the caller supplies | Whatever consumes the iterator |
| Claim decision | Inside `ring_tls` | This crate's, before the call |
| Empties the buffer when | It runs, whether or not records land | The iterator drops |
| Fits this crate | **No** — the claim is the step this crate must be able to fail *before* touching the buffer | Yes |

`flush_into` is the fused operation this instance says would "make this crate
redundant," and it exists. It is not used, because fusing claim and drain is
exactly what puts the rejection path after the seal — the shape
[`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)'s
reconciliation reordered away. **The split `ring_tls` provides is real and this
crate consumes the half of it that keeps the failure early.**

That leaves `flush_into` with no caller in this family, and
`ring_tls`'s own 19 doc instances describing a `seal`/`drain`/`reset` surface
that was not built that way — a documentation debt owed by that crate, recorded
here because this is where the mismatch is visible.

**E3 is declared nowhere and depended on heavily.** The single-contiguous-claim
guarantee shows up in two places — `ring_tls`'s own `flush_into` criterion ("one
`flush_into` moves all `N` into the ring as a single contiguous claim") and
`ring_batch`'s claim/drain criterion ("a claim of 64 slots issues one fence, not
64"). Both guarantees are `ring_batch`'s to provide. This crate consumes
that guarantee through `ring_core` without naming the crate that makes it, so
nothing links a change in claim width to this crate's tests
(→ [`pitfall/002`](../pitfall/002_two_batch_sizes_that_must_not_diverge.md)).

**E4 is a seam that does not exist, and the interesting part is that it briefly
did.** `ring_barrier` is not reachable from here:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo tree -p ring_flush --prefix none --no-dedupe | command grep -c '^ring_barrier'
```

Live output:

```
0
```

It used to be, four edges away, because `ring_spsc` and `ring_mpsc` declared
`ring_consume`, which declares `ring_barrier`. Implementing those two crates
moved their claim and publish logic inline against `ring_cursor` and `ring_slot`
and dropped both edges. The whole consumer-side chain — `ring_consume`,
`ring_publish`, `ring_barrier`, `ring_wait` — left this crate's graph together.

**The current owners of that chain are worth reading off directly**, because
`ring_publish` now has no dependents at all:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order. the dependent list
# is joined with `paste` rather than `tr '\n' ' '`, and the `<-` is printed
# without a following space when the list is empty, so no line ever ends in
# whitespace — a recipe whose output has trailing spaces can never match a
# quoted block, because the markdown file has had its own stripped
for c in ring_consume ring_publish ring_barrier ring_wait; do
  d=$( command grep -l "^$c = " ring_*/Cargo.toml \
    | sed 's|ring/||;s|/Cargo.toml||' | LC_ALL=C sort | paste -sd' ' - )
  printf '%-14s <-%s\n' "$c" "${d:+ $d}"
done
```

Live output:

```
ring_consume   <- ring_publish
ring_publish   <-
ring_barrier   <- ring_consume ring_publish
ring_wait      <- ring_barrier ring_shutdown
```

**Neither the old topology nor the new one gives this crate a barrier it can
use**, which is why E4's "Load-bearing for" cell reads Nothing in both. The
old edge arrived through the *consumer's* side of the ring — the machinery a
reader uses to know what is safe to read — and being able to name that type was
never being able to observe the event
(→ [`pitfall/001`](../pitfall/001_on_barrier_cannot_see_the_barrier.md)).

### The fourteen unreachable crates

```sh
cd "$(git rev-parse --show-toplevel)"
comm -23 <(ls -d ring_*/ | sed 's|ring/||;s|/||' | sort) \
         <(cargo tree -p ring_flush --prefix none --no-dedupe \
           | awk '{print $1}' | command grep '^ring_' | sort -u)
```

Live output:

```
ring_barrier
ring_bench
ring_consume
ring_debug
ring_event
ring_factory
ring_handle
ring_poll
ring_publish
ring_registry
ring_shutdown
ring_testkit
ring_trace
ring_wait
```

| Crate | Why absent | Consequence |
|-------|-----------|-------------|
| `ring_barrier` | **Dropped from the closure** when `ring_spsc`/`ring_mpsc` were implemented | E4. None either way |
| `ring_bench` | Measures, is not measured by | None |
| `ring_consume` | **Dropped with `ring_barrier`** — now reached only from `ring_publish` | None. The read side was never this crate's |
| `ring_debug` | Sibling; inspection | None |
| `ring_event` | Reaches `ring_tls` only as a **dev-dependency** — `tests/tls_test.rs` imports it, and `ring_tls`'s own `src/lib.rs:33` explicitly disclaims slot translation as "`ring_store`'s and `ring_event`'s business" | None — the boundary `ring_tls` draws around its own production code holds one crate further out too |
| `ring_factory` | Constructs this crate's peers | **Correct direction** — the factory depends on the parts |
| `ring_handle` | Sibling; the other half of the export surface | Both are exported; neither knows the other |
| `ring_poll` | Sibling | None |
| `ring_publish` | **Dropped, and now has no dependents at all** | None here — but worth someone's attention at family grain |
| `ring_registry` | Naming | None |
| `ring_shutdown` | Sibling | **E6's open question** |
| `ring_testkit` | The counting-allocator shim | **Blocks the append-path measurement's clean home** |
| `ring_trace` | — | None |
| `ring_wait` | Reached only from `ring_barrier` and `ring_shutdown` | None. The family's no-parking-on-the-tick-path rule forbids it anyway |

**`ring_publish`'s row is the one that is not this crate's business and is
recorded anyway.** A crate with zero dependents is either premature or
orphaned, and neither state is visible from any single crate's documentation.
It is noted here because this instance is the one that ran the query.

**`ring_handle`'s row is the interesting one for this crate.** Two of the five exported names
sit in crates with no edge between them, in the same stage. That is a
correctness property — the surface is five independent decisions rather than one
cluster — and it means neither crate's documentation can describe the other's
role from its own dependencies.

**`ring_shutdown` and `ring_testkit` are the two absences with cost.** The first
leaves an ownership question open ([`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md));
the second means the shared counting-allocator shim is not available, so the
append-path measurement's cleanest home does not exist yet — though a
dev-dependency from `ring_tls` reaches it today
(→ [`non_functional_requirement/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)).

### Error Handling

| Seam | Failure | Detected by | Severity |
|------|---------|-------------|----------|
| E1 | `ring_tls` changes `drain`'s empty-on-drop semantics | `ring_flush`'s own tests — every one that asserts `staged()` after a flush | High — records duplicated or dropped otherwise |
| E1 | Something calls seal/drain/reset directly, bypassing the policy | **Nothing** for an unbound buffer; **impossible** for a bound one, which was moved into the `Flusher` by value | High — the publication-point invariant's P4 |
| E2 | The ring reports full spuriously | `OnFull` fires early; the flush log's `count` shows it | Low |
| E3 | `ring_batch`'s claim width changes | **Nothing here.** No declared edge, no shared test | Medium — `OnBatch( n )` degrades to multi-claim silently |
| E4 | A consumer assumes `OnBarrier` self-detects | **Nothing.** The buffer simply never flushes | **High and silent** |
| E5 | `drive_at_barrier` called when no barrier passed | **Nothing.** Unverifiable from here | Medium |
| E6 | `drain_all` and `drain_final` both exist and disagree | **Cannot happen.** `ring_shutdown` is written and its `drain_all` is consumer-side; the two never touch the same records | Closed |
| E7 | A rejected append has no policy | **Nothing.** The state machine's T10 — T11 is unreachable now that a rejection never strands the buffer | Medium |

**Six of the eight rows read "Nothing"**, which is the honest summary of this
crate's integration position. It is a decision crate: it composes primitives
that remain individually callable, states obligations on callers it cannot
reach, and depends transitively on a guarantee it does not name. Every one of
those is a property of being a policy rather than a mechanism.

**Two of the six moved after implementation, and neither moved by adding a
check.** E6 closed because `ring_shutdown` turned out to be doing something
else entirely, and E1 half-closed because taking the buffer by value made the
bypass unrepresentable for any buffer this crate holds. **Four still read
"Nothing"** — E3, E4, E5, E7 — and all four are about things outside this
crate: a claim width it does not declare, a barrier it cannot see, an
announcement it cannot verify, and an overflow policy it does not own. That
distribution is the finding: the gaps a crate can close by construction are the
ones inside its own ownership boundary, and no amount of care closes the rest.

### Compatibility Requirements

| # | Requirement | Direction |
|---|-------------|-----------|
| Y1 | `ring_tls` keeps seal, drain and reset separately callable | Inbound — a fused `flush()` would make this crate redundant and the invariant unenforceable |
| Y2 | ~~`ring_tls`'s seal transfers ownership atomically w.r.t. the writer~~ **No longer assumed.** `Flusher` takes the buffer by value, so there is no concurrent writer for a seal to be atomic against | Inbound — the buffer state machine's I3, now vacuous |
| Y3 | `ring_core`'s claim is try-only, never blocking | Inbound — the family's no-parking-on-the-tick-path rule |
| Y4 | `ring_batch`'s claim stays contiguous | Inbound, **transitive and unnamed** |
| Y5 | `FlushPolicy` and `FlushOutcome` stay `Copy` and small | Outbound — the export surface's contract |
| Y6 | A consumer drives, and drives again after a rejection | Outbound, **unenforceable** |

**Y1 is worth stating because it is a requirement that a refactor would break
while looking like an improvement.** Collapsing `ring_tls`'s three primitives
into one `flush()` is a smaller API and it deletes this crate's reason to exist,
along with the invariant that a publication point is designed rather than
inherited.

### Integrations

| File | Relationship |
|------|--------------|
| [002_a_decision_on_the_export_surface.md](002_a_decision_on_the_export_surface.md) | The outbound boundary; `ring_handle`'s row above from the other side |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) | E1's second failure row, as a standing restriction |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_consolidation_cycle.md](../lifecycle/001_the_consolidation_cycle.md) | E1 and E2 as phases; its dependency table is this one condensed |
| [../lifecycle/002_from_configuration_to_the_final_drain.md](../lifecycle/002_from_configuration_to_the_final_drain.md) | E6's open question, worked out |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | E4 as a trap — this instance states the topology, that one states the cost |
| [../pitfall/002_two_batch_sizes_that_must_not_diverge.md](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) | E3's consequence |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_state_through_a_flush.md](../lifecycle/003_buffer_state_through_a_flush.md) | E7's T10 and T11 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | E3 — `an_overshooting_batch_publishes_everything_staged_in_one_claim` is that case: eight records under `OnBatch( 4 )` publish as one claim of eight. It reads the width from this side of the seam only; a change inside `ring_tls` that split the claim would fail it, but nothing here proves the claim is atomic in `ring_tls`'s own terms |

### FL17 — The Block Under "Confirm Rather Than Take It on Trust" Is Typed, Not Captured

The instruction is to confirm; the thing beneath it is a transcription:

```sh
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/integration/001_two_dependencies_and_the_barrier_it_cannot_see.md
echo '  -- what the manifest declares today --'
sed -n '/^\[dependencies\]/,$p' ring_flush/Cargo.toml | command grep -E '^\[|^ring_' | sed 's/^/    /'
echo '  -- what this instance displays as that command'\''s output --'
awk '/^```toml/{ i = 1 } i { printf "    %s\n", $0 } i && /^```$/ && ++c == 2 { exit }' "$F" | head -8
echo '  -- fence types in this instance --'
printf '    ```bash blocks (invisible to the gate): %s\n' "$( command grep -c '^```bash' "$F" )"
printf '    ```sh   blocks (executed and compared): %s\n' "$( awk '/^### FL/{ exit } /^```sh/{ n += 1 } END { print n + 0 }' "$F" )"
```

Live output:

```
  -- what the manifest declares today --
    [dependencies]
    ring_tls = { path = "../ring_tls" }
    ring_core = { path = "../ring_core" }
    ring_types = { path = "../ring_types" }
    [dev-dependencies]
    ring_config = { path = "../ring_config" }
    [lints]
  -- what this instance displays as that command's output --
    ```toml
    [dependencies]
    ring_tls = { path = "../ring_tls" }
    ring_core = { path = "../ring_core" }
    ring_types = { path = "../ring_types" }
    
    [dev-dependencies]
    ring_config = { path = "../ring_config" }
  -- fence types in this instance --
    ```bash blocks (invisible to the gate): 0
    ```sh   blocks (executed and compared): 6
```

Three path dependencies and a dev-dependency. The instance shows two, inside a
fence marked ```` ```toml ```` positioned directly under a `sed` command, under
a sentence that says to confirm rather than trust.

**Nothing about the block is marked as a transcript and nothing distinguishes it
from one.** A reader sees a command, then a fenced block containing exactly the
output that command would produce, and concludes the pair was executed together.
It was typed, once, when the answer was two — and it has stayed two through the
addition of `ring_types` and `ring_config`.

**The consequence is not a wrong count; it is a reversed argument.** The
paragraph immediately below reads the absence of `ring_types` as this crate's
distinguishing feature — "the first thing to notice," putting it "in a small
minority," reaching the crate transitively and "needing nothing from it." The
crate now declares `ring_types` directly, which makes it a member of the majority
the paragraph contrasts it against, and the sentence about reaching it
transitively describes a route that is no longer taken.

The instance's own convention names the mechanism precisely: "Every count in this
instance ships with the command that regenerates it." The commands are all
present and all fenced ```` ```bash ````, which the corpus gate does not execute
(→ [`workaround/002`](../workaround/002_the_compilation_boundary_that_was_never_built.md)'s
FL52). **A regeneration command that is never run is a transcript with extra
steps** — and because it is displayed next to its answer, it certifies the answer
rather than checking it.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
F=ring_flush/docs/integration/001_two_dependencies_and_the_barrier_it_cannot_see.md
echo '  -- the block now matches the manifest --'
# The fence lines are stripped rather than printed: this output is itself
# fenced, and a line opening a second fence inside it silently ends the block
# early for anything reading the file structurally -- which hid the FL18
# heading below from the shape checker.
awk '/^```toml/{ i = 1; next } i && /^```$/ { exit } i { print }' "$F" | head -8
echo '  -- the reversed-argument paragraph now reads --'
command grep -m1 'moved this crate' "$F"
```

Live output:

```
  -- the block now matches the manifest --
[dependencies]
ring_tls = { path = "../ring_tls" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

[dev-dependencies]
ring_config = { path = "../ring_config" }
  -- the reversed-argument paragraph now reads --
`ring_types` is now among them. **The addition of `ring_types` moved this crate
```

**Disposition:** applied — the `toml` block under "Confirm rather than take it
on trust" now shows all three path dependencies and the dev-dependency the
manifest actually declares, and the paragraph beneath it states the argument
the finding says is now reversed: `ring_types` moved this crate from the
minority to the majority, and the dependency is direct rather than transitive.
The regeneration command's own comment was re-run rather than copied, so the
count (28, not the stale 21) was genuinely current at the time — it has since
moved again, to 27, per the regenerated block above. The broader point — that
`` ```bash `` blocks are invisible to the corpus gate and so a transcript next
to its answer can drift silently — is `workaround/002`'s `FL52` to fix, not
this finding's; this disposition corrects the one instance FL17 names, not the
mechanism that let it happen. Now prints: `moved this crate`

### FL18 — Two Documents Found the Same Orphan Independently, and Only One of Them Could Do Anything About It

The observation this instance filed as "worth someone's attention" already has an
owner, in a decision this crate does not cite:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the chain, as it stands --'
for c in ring_consume ring_publish ring_barrier ring_wait
do
  printf '    %-14s <- %s\n' "$c" \
    "$( command grep -l "^$c = " ring_*/Cargo.toml 2>/dev/null \
        | sed 's|ring/||; s|/Cargo.toml||' | tr '\n' ' ' )"
done
echo '  -- the decision that caused it, and routed it --'
command grep 'used by no crate in the family' docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md
command grep 'becomes its own decision' docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md
echo '  -- whether the two documents know about each other --'
printf '    decision 124 naming ring_flush:        %s\n' "$( command grep -c 'ring_flush' docs/decision/124_ring_mpsc_publication_stamped_not_cursor.md )"
printf '    ring_flush prose citing decision 124:  %s\n' \
  "$( command grep -rl 'decision/124\|124_ring_mpsc' ring_flush 2>/dev/null \
      | while read -r f
        do awk '/^### FL/{ exit } /decision\/124|124_ring_mpsc/{ print FILENAME; exit }' "$f"
        done | wc -l )"
printf '    ring_flush prose citing decision 121:  %s\n' \
  "$( command grep -rl 'decision/121\|121_workstream' ring_flush 2>/dev/null \
      | while read -r f
        do awk '/^### FL/{ exit } /decision\/121|121_workstream/{ print FILENAME; exit }' "$f"
        done | wc -l )"
echo '  -- and whether the escape condition 124 named has fired --'
printf '    ring_core naming ring_publish:         %s\n' "$( command grep -c '^ring_publish = ' ring_core/Cargo.toml )"
```

Live output:

```
  -- the chain, as it stands --
    ring_consume   <- ring_publish 
    ring_publish   <- 
    ring_barrier   <- ring_consume ring_publish 
    ring_wait      <- ring_barrier ring_shutdown 
  -- the decision that caused it, and routed it --
- `ring_publish` and `ring_consume` are now used by no crate in the family.
  two primitives with no consumer and that becomes its own decision.
  -- whether the two documents know about each other --
    decision 124 naming ring_flush:        0
    ring_flush prose citing decision 124:  0
    ring_flush prose citing decision 121:  9
  -- and whether the escape condition 124 named has fired --
    ring_core naming ring_publish:         0
```

A ruling recorded elsewhere already records the orphan in its own consequences —
"`ring_publish` and `ring_consume` are now used by no crate in the family. That is
a real finding and is **not** resolved here" — and routes it: `ring_core` is
named as the crate that could adopt them, and if it does not, "that becomes its
own decision."

**Two documents reached the same finding from opposite directions.** That ruling
reached it by deciding that `ring_mpsc` would publish with its own per-slot
stamps, and could therefore see what its ruling would strand. This instance
reached it by running `cargo tree` and noticing a crate with no dependents.
Neither names the other: the ruling mentions `ring_flush` nowhere, and no prose
in this crate cites it, even though several of this crate's other documents cite
a separate, earlier ruling. **That ruling is the one that explains the paragraph
directly above this finding**, where the departure of the whole consumer-side
chain is narrated as something that happened when "`ring_spsc` and `ring_mpsc`
were implemented" — which is that ruling's verdict, described as weather.

**The escape condition has not fired.** `ring_core` does not name `ring_publish`,
so by that ruling's own terms the follow-on decision is now due, and nothing has
recorded that it is. Two documents observing one fact, one of them holding the
trigger, and no link between them is exactly the arrangement in which a due
decision stays unnoticed.

The reusable shape: **a graph query and a decision record are two ways of
discovering the same thing, and only one of them can act.** The instance did the
right thing by writing the observation down. What it had no way to do — and what
nothing in the corpus does — is check whether the observation was already
somebody's, which is the difference between a duplicate finding and a second
witness.
