# Workaround: Four Dev-Dependencies That Look Like a Cycle

### Scope

- **Purpose**: Record why a Tier 5 primitive declares four dev-dependencies on its sibling operations, why that cannot form a cycle, and what the arrangement costs.
- **Responsibility**: State the reason the reached-test lives here, give both dependency closures, prove no reverse edge exists anywhere in the workspace, and place the crate's `ring_seqno` correction among the family's three.
- **In Scope**: `Cargo.toml`'s `[dependencies]` and `[dev-dependencies]` blocks, and the manual check over them.
- **Out of Scope**: The `[target.'cfg(loom)']` block in the same manifest — see [`workaround/001`](001_the_loom_seam_and_its_only_user.md).

### The Manifest, and the Comment That Anticipates the Objection

```toml
[dependencies]
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }

# Tests only. `tests/handshake_test.rs` is feature 170's reached-test, which is
# the whole four-operation handshake — so it needs the other three operations'
# crates. None of them depends on this one, so the graph stays acyclic.
[dev-dependencies]
ring_claim = { path = "../ring_claim" }
ring_consume = { path = "../ring_consume" }
ring_barrier = { path = "../ring_barrier" }
ring_gating = { path = "../ring_gating" }
```

Two real dependencies and four for tests, and the ratio is the point: a crate
whose library needs two crates needs six to *test* itself, because what is being
tested is not the crate.

`tests/handshake_test.rs:11-14` states why the test lives here rather than in one
of the other three:

> It lives in `ring_publish` rather than in any of the other three crates because
> publication is the moment the other three become observable together: before
> it, a claim is invisible; after it, the consumer's whole contract is decided.

That is a real argument and not a convenience one — the alternative placements
each make the test about one participant instead of the handshake. It is also
what makes this a **workaround** rather than a design: the manifest carries four
edges that exist for the reached-test's coverage rather than
for the code.

### PB45 — Not One Manifest in the Workspace Declares `ring_publish`, So the Cycle Cannot Exist

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'ring_publish' */Cargo.toml
```

Live output:

```
ring_mpsc/Cargo.toml:# Eight, not the seven this manifest was scaffolded with. `ring_publish` and
ring_publish/Cargo.toml:name = "ring_publish"
```

Two hits in the entire workspace, and neither is a dependency:

| Hit | What it is |
|-----|-----------|
| `ring_publish/Cargo.toml:2` | this crate's own `name =` line |
| `ring_mpsc/Cargo.toml:8-11` | a comment explaining that `ring_publish` was **removed** |

The second is worth reading in full:

> Eight, not the seven this manifest was scaffolded with. `ring_publish` and
> `ring_consume` are gone and `ring_atomic`, `ring_slot` and `ring_types` are
> new — see decision 124. Publication here is a per-slot stamp this crate owns,
> so there is no published cursor for either removed crate to act on.

So the one manifest that ever named `ring_publish` records its removal and the
reason. The acyclicity claim in this crate's own comment is therefore not a
promise about future edits — it is a checkable present fact with exactly zero
candidates.

The four dev-dependencies' own dependencies confirm it from the other side:

| Crate | Depends on |
|-------|-----------|
| `ring_claim` | `ring_types`, `ring_cursor`, `ring_gating` |
| `ring_consume` | `ring_types`, `ring_cursor`, `ring_barrier`, `ring_seqno` |
| `ring_barrier` | `ring_types`, `ring_cursor`, `ring_wait` |
| `ring_gating` | `ring_types`, `ring_cursor`, `ring_seqno` |

Two of the four are reachable through the other two — `ring_gating` via
`ring_claim`, `ring_barrier` via `ring_consume` — so as a matter of *resolution*
only two declarations are needed. All four are declared because Rust requires a
direct dependency to write `use ring_barrier::Barrier;`, and
`handshake_test.rs:242-245` writes all four.

### The Two Closures

| Build | Crates pulled in | Which |
|-------|-----------------:|-------|
| the library | **5** | `ring_types`, `ring_cursor`, `ring_seqno`, `ring_atomic`, `ring_align` |
| the tests | **10** | the five above, plus `ring_claim`, `ring_consume`, `ring_barrier`, `ring_gating`, `ring_wait` |

The reached-test **doubles** the crate's transitive closure, from 5 of 33 to 10.
That is the arrangement's whole cost, and it is paid only by `cargo test`.

`ring_wait` is the one that arrives without being asked for — it is
`ring_barrier`'s dependency, not this crate's choice — and it is the crate whose
four wait strategies
[`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md)
declines to use. So building this crate's tests compiles the wait-strategy
machinery that `publish`'s bare spin exists in order not to need.

### PB46 — Three Manifests Were Scaffolded Over-Declared, and All Three Corrections Name `ring_seqno`

`src/lib.rs:7-10` records this crate's:

> `ring_seqno` was scaffolded into this crate's manifest before the implementation
> existed and is not among them. Publishing is a cursor advance and a contiguity
> test; it computes no distances, no free slots and no minimum. The same
> over-declaration was found and removed in `ring_claim`.

`ring_claim/src/lib.rs:7-13` records its own, with the sharper version of the
argument:

> every piece of sequence arithmetic claiming needs is either
> `ring_types::Seq`'s own […] or already inside `ring_gating::GatingSet::
> headroom`, which reaches `ring_seqno` on this crate's behalf. Declaring it here
> as well would be a dependency carried for the shape of the forest rather than
> for a call.

And `ring_mpsc/Cargo.toml:8` records a third, in the other direction — a manifest
scaffolded with seven that ended with eight, two removed and three added.

| Crate | Scaffolded with | Corrected to | Concerning |
|-------|-----------------|--------------|------------|
| `ring_publish` | `ring_seqno` among them | 2 deps | `ring_seqno` removed |
| `ring_claim` | `ring_seqno` among them | 3 deps | `ring_seqno` removed |
| `ring_mpsc` | 7 deps | 8 deps | `ring_publish`, `ring_consume` removed; `ring_atomic`, `ring_slot`, `ring_types` added |

The pattern is that **manifests were written before implementations**, so every
one of them encodes a guess about what the crate would need. Two of the three
corrections remove `ring_seqno` specifically, and for the same reason each time:
sequence arithmetic looks like it must need a sequence crate, and turns out to
need `Seq`'s own `Ord` and `advanced_by`.

`ring_seqno` is nonetheless in this crate's closure — `ring_cursor` depends on it
— which is exactly `ring_claim`'s phrase: it is reached on this crate's behalf,
and declaring it directly would be a dependency carried for the shape of the
forest rather than for a call.

### What Catches a Regression

`tests/manual/readme.md § P5` is the check, and it is deliberately two commands
rather than one:

```bash
# The library's own dependencies, against the library.
comm -23 <( … [dependencies] … ) <( … src/lib.rs … )
# The dev-dependencies, against the tests.
comm -23 <( … [dev-dependencies] … ) <( … tests/*.rs … )
```

The plan records why the split was necessary:

> Two sections, two consumers. Sharing one command between them — as the first
> draft of this check did — reports the four `[dev-dependencies]` as unused,
> because the library indeed does not use them and must not.

That failure mode is the finding in miniature: a naive check does not merely miss
the problem, it reports the *correct* arrangement as broken. And the check's
closing note names the drift it is really watching for:

> The second command matters more than it looks: the four dev-dependencies exist
> only because the reached-test is the whole four-operation handshake. If a
> future edit narrows that test, they become the kind of dependency edge that
> makes a family look more tangled than it is — and this is what notices.

**Nine of the family's 33 manual plans carry a check of this shape** —
`ring_barrier`, `ring_claim`, `ring_consume`, `ring_cursor`, `ring_gating`,
`ring_mpsc`, `ring_publish`, `ring_spsc`, `ring_wait` — which makes it the most
replicated check in the family and still a minority of crates. All 33 have a
`tests/manual/readme.md`; 24 have no dependency-usage check in it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'every declared dependency' ring_*/tests/manual/readme.md | wc -l   # 9
ls -d ring_*/tests/manual/readme.md | wc -l                                    # 33
```

Live output:

```
9
33
```

### What Would Make This Not a Workaround

| Change | Effect | Cost |
|--------|--------|------|
| Move the reached-test to a separate `ring_handshake_test` crate | this manifest drops to 2 deps and 0 dev-deps | a crate that is only tests; the family has no such precedent |
| Narrow the test to publication alone | the dev-dependencies go | the reached-test's own criterion names all four operations — the test would stop being the reached-test |
| Leave it | 4 dev-dependencies, 10-crate test closure, one manual check watching them | what is done |

The first is the only one that removes the edges without weakening the
verification, and it trades them for a crate whose entire content is one test
file. The current arrangement is chosen, documented at the manifest, and guarded
by P5 — which is the most a workaround can be.

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | `ring_wait`, compiled into the test closure and declined by the code |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_ten_crates_name_it_and_none_depends_on_it.md](../integration/001_ten_crates_name_it_and_none_depends_on_it.md) | The same zero-reverse-edge fact, from the naming side |
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | Decision 124, and what replaced this crate in `ring_mpsc` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_four_operation_handshake.md](../lifecycle/002_the_four_operation_handshake.md) | The test these four edges exist for |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The `Claim` type these edges make reachable from the tests but not the library |

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_loom_seam_and_its_only_user.md](001_the_loom_seam_and_its_only_user.md) | The other block the same test file adds to this manifest |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/Cargo.toml:8-19` | Two dependencies, four dev-dependencies, and the acyclicity comment |
| `ring_publish/src/lib.rs:7-10` | The `ring_seqno` over-declaration and its removal |
| `ring_claim/src/lib.rs:5-13` | The same correction, with the transitive-reach argument |
| `ring_mpsc/Cargo.toml:8-11` | The only manifest that ever named this crate, recording its removal |
| `ring_barrier/Cargo.toml` | Where `ring_wait` enters the test closure |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:11-14` | Why the test lives here and not in one of the four |
| `tests/handshake_test.rs:242-245` | All four dev-dependencies named directly |
| `tests/manual/readme.md § P5` | Two commands, and the draft that reported the right answer as wrong |
