# Pitfall: `OnBarrier` Cannot See the Barrier

### Scope

- **Purpose**: Record that `OnBarrier` is named after a crate this one cannot reach and could not use if it could — the barrier must be *announced* by the caller, never observed from here.
- **Responsibility**: Name the trap, the failure it produces, and the mitigations, including which ones do not work.
- **In Scope**: `OnBarrier`'s trigger; the gap between naming a thing and being wired to it.
- **Out of Scope**: What a barrier *is* (→ [`ring_barrier`](../../../ring_barrier/readme.md)); the driver API shape this forces (→ [The Driver Surface](../api/002_the_driver_surface.md)).

### Trap

**`ring_flush` cannot reach `ring_barrier` at all.** Three declared dependencies,
nineteen reachable crates, and the crate `OnBarrier` is named after is not among
them:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[/p' ring_flush/Cargo.toml   # ring_tls, ring_core, ring_types — three
cargo tree -p ring_flush --prefix none --no-dedupe \
  | awk '{print $1}' | command grep '^ring_' | sort -u | wc -l            # 19
cargo tree -p ring_flush --prefix none --no-dedupe \
  | command grep -c '^ring_barrier'                                       # 0
```

Live output:

```
[dependencies]
ring_tls = { path = "../ring_tls" }
ring_core = { path = "../ring_core" }
ring_types = { path = "../ring_types" }

[dev-dependencies]
19
0
```

**Run these before trusting the numbers.** The closure is not a stable fact
while the family is being implemented: at one point during authoring it stood at
twenty-four and did include `ring_barrier`, four edges away through
`ring_core` → `ring_spsc`/`ring_mpsc` → `ring_consume`. Implementing `ring_spsc`
and `ring_mpsc` dropped their `ring_consume` and `ring_publish` edges — the
claim and publish logic moved inline against `ring_cursor` and `ring_slot` — and
the whole consumer-side chain left this crate's graph with it.

**That volatility is itself the lesson, and it generalises past this crate:** a
finding that rests on a dependency closure is perishable, so it ships the
command that regenerates it rather than the number alone. A closure count
written down without its recipe is a fact with no expiry date printed on it.

**The trap survives the topology change intact, because it was never really
about reachability.** Whether `ring_barrier` is four edges away or absent, the
same two things are missing: the *instance* the consumer is actually gated on,
and a *notification* when that instance advances. A type in scope lets you name
a barrier in a signature. `OnBarrier` needs to know that a specific one just
advanced, and no arrangement of dependency edges delivers that.

**What made the trap convincing before is worth keeping in view**, because it
will recur the moment anything re-introduces the edge: a reader checking whether
the barrier variant is wired found that it compiled, that the crate was present,
and that the type was in scope. All three were true and none meant what they
appeared to.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| F1 | `OnBarrier` is configured and never fires | **Identical to a correctly-idle buffer.** No error, no log entry, no assertion — records accumulate and the ring stays empty |
| F2 | An implementer, seeing `ring_barrier` in scope, constructs a barrier here to poll | Two barriers now exist: the one the consumer waits on and the one this crate reads. They never agree, and the disagreement is invisible |
| F3 | `OnBarrier` is implemented as "flush on every drive call" because the barrier was unavailable | Silently becomes `OnEveryTick` — a fourth policy nobody configured, which the benchmark records under `OnBarrier`'s name |
| F4 | The barrier signal is wired but arrives from a thread other than the buffer's owner | **As built: a compile error, not a runtime hazard.** `Flusher` owns its buffer and its `Producer`, so driving it from another thread means moving it there, and it is `Send` only where `T: Send` — the announcement must reach the owning thread as data, not as a call |
| F5 | The test scripts the barrier by calling the driver directly | Passes. Proves the sequencing, proves nothing about barrier observation — the thing the variant is named for is stubbed out in the only test that covers it |

**F1 is the primary failure and it is silent in the worst way** — a buffer that
never flushes and a buffer with nothing to flush are indistinguishable from
outside. The consumer sees an empty ring in both cases.
[`FlushOutcome`](../type/002_flush_outcome.md) exists partly to make these two
states different, which is why it must distinguish "policy did not fire" from
"policy fired, nothing to move."

**F3 is what actually happens under deadline pressure,** and it corrupts the
measurement rather than the program. The benchmark exists to compare these
policies; a variant that silently becomes a different variant produces a
verdict about a policy that was never run.
[Trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s
V4 is this same defect approached from the invariant side.

**F5 is the one that survives review,** because the test passes and the
sequencing it covers is genuinely correct. What it does not cover is the only
thing that distinguishes `OnBarrier` from `OnDemand`.

**F4 was closed by a decision made for other reasons, and the closure is
borrowed.** `Flusher` is `Send` and not `Sync` — measured, with the compiler's
own reason recorded in
`a_driver_is_movable_between_threads_and_never_shared`. A driver can be handed
to another thread; it cannot be shared with one, so the cross-thread seal F4
describes has no expression. But the `!Sync` comes from `ring_core::Producer`
being `!Sync`, not from anything decided here. **If `ring_core` ever made
`Producer` `Sync`, F4 would reopen and this crate's tests would stay green** —
the same shape as E3's export-boundary hole and O6's parking constraint, and
the third instance in this crate of a property it depends on and does not own.

### Mitigation

**What does not work, listed first because each is the obvious first idea:**

| Attempt | Why it fails |
|---------|--------------|
| Depend on `ring_barrier` directly | Buys a type in scope and nothing else. The missing thing is an instance and a notification, and a `[dependencies]` line supplies neither — this is the mitigation the topology change makes *available* and no more correct than before |
| Poll a barrier from this crate | Requires knowing *which* barrier, which is the consumer's and is not passed here. F2 |
| Have `ring_tls` signal the barrier | It has no more visibility than this crate; its own docs defer the trigger here (→ [Consolidation Cycle](../../../ring_tls/docs/lifecycle/002_consolidation_cycle.md)) |
| Read the consumer's cursor through `ring_core` | Couples the flush policy to the ring's internals and makes the trigger a function of consumption progress rather than of the tick — a different policy wearing this one's name |

**What works:**

1. **The barrier is announced, not observed.** The driver takes the barrier
   crossing as an argument or as an explicit call
   (→ [The Driver Surface](../api/002_the_driver_surface.md)). This crate
   decides; the caller supplies the fact.
2. **`FlushOutcome` distinguishes "did not fire" from "fired, moved nothing."**
   Turns F1 from silent into inspectable.
3. **The scripted test asserts the negative case explicitly** — drive without
   announcing a barrier, assert no flush occurred — so F3's degeneration to
   flush-on-every-drive fails a test rather than passing one.
4. **This instance is cited from the crate root**, because the trap is a
   reasonable inference from `cargo tree` and the only defence against a
   reasonable inference is writing down why it is wrong.

**None of the four mitigations closes F2 or F4.** Both require the consumer to
wire the announcement to the right barrier from the right thread, and no
mechanism here can check that it did. That is the honest limit: this crate can
make the barrier variant *drivable* and cannot make it *correct*.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_driver_surface.md](../api/002_the_driver_surface.md) | Mitigation 1 — the API shape this pitfall forces |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | The two-declared/twenty-reachable gap, worked out as a dependency seam |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | F3 is its V4 seen from the trap side |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_driven_not_self_firing.md](../pattern/002_driven_not_self_firing.md) | The practice this pitfall justifies — the crate decides and never acts unprompted |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | Mitigation 2 — the type that makes F1 inspectable |

### Sources

| File | Relationship |
|------|--------------|
| [`../type/001_flush_policy.md`](../type/001_flush_policy.md) | Names `OnBarrier` as one of three variants, without stating who observes the barrier |
| `Cargo.toml` | Three declared dependencies; the nineteen-crate closure, and the fact that it moved during authoring |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | Mitigation 3 — `on_barrier_never_fires_without_an_announcement`. That it genuinely catches F3 is not assumed: the degeneration was injected and the suite went red (`tests/manual/readme.md`'s F1). Three other tests also caught it, incidentally; this is the only one that would still catch it after an unrelated refactor. F4 — `a_driver_is_movable_between_threads_and_never_shared` asserts `Send` and records the measured `!Sync`, along with the fact that the latter is `ring_core`'s to keep |

### FL41 — The Instance That Names Closure Counts Perishable Ships Two Perished Ones, Behind the Fence That Guarantees Nobody Reruns Them

The lesson, the numbers it was written to protect, and the fence it put them behind:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the numbers as written --'
awk '/^### FL/{ exit } /twenty reachable|Two declared dependencies|no expiry date/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md | sed -E 's/^(.{0,120}).*/\1/'
echo '  -- the same numbers, measured now --'
printf '    declared path dependencies: %s\n' \
  "$( awk '/^\[dependencies\]/{d=1;next} /^\[/{d=0} d && /path *=/' ring_flush/Cargo.toml | wc -l )"
python3 - <<'PY'
import os, re
def deps( c ):
  p = os.path.join( 'ring', c, 'Cargo.toml' )
  if not os.path.exists( p ): return []
  out, sec = [], None
  for ln in open( p ):
    if ln.startswith( '[' ): sec = ln.strip()
    elif sec and 'dependencies' in sec and 'dev' not in sec and 'path' in ln:
      m = re.match( r'\s*([A-Za-z0-9_]+)\s*=', ln )
      if m: out.append( m.group( 1 ) )
  return out
seen, frontier = set(), [ 'ring_flush' ]
while frontier:
  for d in deps( frontier.pop() ):
    if d not in seen: seen.add( d ); frontier.append( d )
print( '    reachable crates:           %d' % len( seen ) )
print( '    ring_barrier among them:    %s' % ( 'ring_barrier' in seen ) )
PY
echo '  -- and the fence the regeneration recipe sits behind --'
printf '    sh blocks in this instance before its findings: %s\n' \
  "$( awk '/^### FL/{ exit } /^```sh$/' ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md | wc -l )"
printf '    bash blocks:                                    %s\n' \
  "$( awk '/^### FL/{ exit } /^```bash$/' ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md | wc -l )"
```

Live output:

```
  -- the numbers as written --
    49: written down without its recipe is a fact with no expiry date printed on it.
  -- the same numbers, measured now --
    declared path dependencies: 3
    reachable crates:           18
    ring_barrier among them:    False
  -- and the fence the regeneration recipe sits behind --
    sh blocks in this instance before its findings: 1
    bash blocks:                                    0
```

This instance states the principle better than anywhere else in the corpus: "a
finding that rests on a dependency closure is perishable, so it ships the command
that regenerates it rather than the number alone. A closure count written down
without its recipe is a fact with no expiry date printed on it."

It then shipped the recipe in a ```` ```bash ```` fence. The corpus gate reads
fences marked ```` ```sh ```` and nothing else, so the three commands have never
run since the day they were typed, and both numbers they guard have moved: two declared
dependencies are now three, twenty reachable crates are now eighteen. The
conclusion — `ring_barrier` is not among them — survived, which is why nothing
looked.

**The instance anticipated its numbers decaying and did not anticipate the
decay going unnoticed.** It says "Run these before trusting the numbers," which
puts the obligation on the reader; the fence type meant the machinery that runs
recipes on every gate pass never took it on. Printing an expiry date is not the
same as having anything check it, and this is the crate's third instance of the
same mechanism (→ [FL35](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md),
[FL52](../workaround/002_the_compilation_boundary_that_was_never_built.md)).

**Only the conclusion was load-bearing, so nothing downstream is wrong.** That is
worth saying plainly rather than dressing the finding up: the trap is real, the
mitigations are unaffected, and what decayed is the evidence rather than the
claim. The interesting part is that this document *predicted* exactly this and
still could not prevent it, because prevention was never in the document's hands.

**Disposition:** applied — the block is now fenced ```` ```sh ```` with a
genuine `Live output:`, so the corpus gate runs it on every pass instead of
never. The two stale claims this finding names have been corrected where they
appear in prose: the Trap paragraph now reads "Three declared dependencies,
nineteen reachable crates," and the Sources row states the same two corrected
numbers. The main block's `cargo tree`-based count (19 — self-inclusive,
dev-dependencies included) and this finding's own Python BFS (18 —
self-exclusive, dev-dependencies excluded) are two different methodologies
measuring related but not identical sets, not a contradiction; each now states
its own number accurately by its own method. `ring_barrier` remains absent
from both. The broader mechanism this finding named — a ```` ```bash ````
fence hiding a recipe from the gate — is FL35's and FL52's to fix at corpus
grain; this disposition corrects the one instance FL41 names.

### FL42 — Mitigation 4 Says This Instance Is Cited From the Crate Root, and the Crate Root Cites Four Others

The mitigation listed under "What works," and what `src/lib.rs` actually points at:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the mitigation --'
awk '/^### FL/{ exit } /cited from the crate root/{ printf "    %d: %s\n", NR, $0 }' \
  ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md
echo '  -- every doc instance the crate root cites --'
command grep -oE 'docs/[a-z_]+/[0-9]{3}_[a-z_]+\.md' ring_flush/src/lib.rs | sort -u | sed 's/^/    /'
echo '  -- how many are this crate own instances, and how many are pitfalls --'
printf '    own doc instances cited: %s\n' \
  "$( command grep -oE 'docs/[a-z_]+/[0-9]{3}_[a-z_]+\.md' ring_flush/src/lib.rs \
       | command grep -v 'docs/workstream/' | sort -u | wc -l )"
printf '    pitfall instances cited: %s\n' "$( command grep -c 'docs/pitfall/' ring_flush/src/lib.rs )"
printf '    instances this crate has: %s\n' \
  "$( find ring_flush/docs -name '[0-9][0-9][0-9]_*.md' | wc -l )"
```

Live output:

```
  -- the mitigation --
    125: 4. **This instance is cited from the crate root**, because the trap is a
  -- every doc instance the crate root cites --
    docs/algorithm/002_sequencing_seal_drain_reset.md
    docs/api/001_the_policy_surface.md
    docs/data_structure/002_the_flush_log.md
    docs/type/001_flush_policy.md
  -- how many are this crate own instances, and how many are pitfalls --
    own doc instances cited: 4
    pitfall instances cited: 0
    instances this crate has: 28
```

Mitigation 4 is not a description of a design property like the other three; it
is a claim that a specific edit exists, and the edit does not. The crate root
cites four of this crate's doc instances — plus one further citation — and
neither pitfall is among them.

**The reasoning behind the mitigation is the strongest in the list.** The other
three make the barrier variant drivable and its failure inspectable; this one
addresses the reader who has not read the docs at all — who runs `cargo tree`,
sees a plausible graph, and infers that the barrier variant is wired. That
reader's entry point is `src/lib.rs`, and the only defence against a reasonable
inference is meeting it where it is formed.

The pointer is missing at exactly the moment it became most necessary. When this
instance was written, `cargo tree` showed `ring_barrier` four edges away and the
wrong inference was easy to reach; the topology change removed the edge, so today
the graph tells the truth. **The mitigation is now unnecessary for the reason it
was written and necessary for a new one** — the trap moved from "the crate is
present and unusable" to "the variant exists and the fact it needs arrives from
somewhere no dependency shows," which no `cargo tree` reading reveals in either
direction.

Recorded rather than fixed: the edit is one line in a module doc table, and the
question of which of this crate's twenty-eight instances the root should cite is a
crate-wide one rather than this instance's to answer alone.
