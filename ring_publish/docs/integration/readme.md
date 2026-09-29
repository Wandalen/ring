# integration

Two dependencies out, four dev-dependencies out, zero dependents in — and eleven
crates that discuss this one in prose, ten of them in this family and one outside
it. That combination is the whole subject of this directory: a Tier 5 write-path
primitive everybody has an opinion about and nobody calls.

001 establishes the graph by command. 002 asks why the dependent count is zero
and answers it out of the two declining crates' own module documentation, which
turn out to decline for opposite reasons.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Ten Crates Name It and None Depends On It](001_ten_crates_name_it_and_none_depends_on_it.md) | PB1, PB2, PB3 — every edge in and out, the six crates with no dependents and why this is the only load-bearing one among them, the ten prose references, and the two-command check that keeps the manifest honest |
| 002 | [The Two Crates That Declined](002_the_two_crates_that_declined.md) | PB4, PB5, PB6 — `ring_spsc`'s and `ring_mpsc`'s opposite objections, the ruling this crate's own module doc decided, the escalation condition that has since been met, and the constant the declining crate reproduced anyway |

### The Edges

| Direction | Section | Crates |
|-----------|---------|--------|
| out | `[dependencies]` | `ring_types`, `ring_cursor` |
| out | `[dev-dependencies]` | `ring_claim`, `ring_consume`, `ring_barrier`, `ring_gating` |
| out | `[target.'cfg(loom)'.dev-dependencies]` | `loom = "0.7"` |
| **in** | — | **none** |

The only manifest in the family that ever named this crate is `ring_mpsc`'s, and
what it contains today is a comment recording the removal.

### The Two Refusals

| | `ring_spsc` — one producer | `ring_mpsc` — many producers |
|--|---------------------------|------------------------------|
| Objection | the wait never happens | the wait must never happen |
| So the crate is | unnecessary | unacceptable |
| Ruled by | its own thesis — absence *is* the thesis | [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) |
| Replaced with | a plain store | a per-slot `Seq` stamp array |
| Would have bought | — | *"the whole handshake for free"* |

They are not the same objection. They sit at the two ends of the producer-count
range this crate occupies the middle of, and between them they leave it with
nothing above it. [`decisions/001`](../decisions/001_refused_rather_than_reordered.md)
rejected using `Publisher` anyway on exactly one
property — a producer waiting for a peer — which is the property that same
decision records as chosen deliberately.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# out: both sections
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml | grep -oE '^ring_[a-z_]+'
awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml | grep -oE '^ring_[a-z_]+'

# in: every manifest naming this crate, with the section it appears in. All four
# roots are named explicitly throughout this block: a `` glob reaches
# neither `ring/` nor the substrate crates, and a bare `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` root no longer
# reaches the substrate crates either — they are a repo-root sibling of
# `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, not nested inside it, so `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/` has to be named in its own
# right
for m in $( command grep -rl --include=Cargo.toml 'ring_publish' . | sort ); do
  awk -v f="$m" '/^\[/{s=$0} /ring_publish/{printf "%s  %s  %s\n", f, s, $0}' "$m"
done

# tier and dependent count for every crate in the family
for c in ring_*/; do n=$( basename "$c" )
  t=$( grep -m1 -oE 'Tier [0-9]+' "$c/src/lib.rs" 2>/dev/null )
  d=$( command grep -rl --include=Cargo.toml "^${n} = " . 2>/dev/null | command grep -v "/${n}/" | wc -l )
  printf "%-16s %-8s dependents=%s\n" "$n" "$t" "$d"
done

# prose references, which are not edges. Workspace-wide, unlike `integration/001`'s
# PB2 census, which is deliberately scoped to this family's own `*/src/*.rs`
# — the two agreed at ten until a crate outside the family cited this one
command grep -rl --include=*.rs 'ring_publish' . | command grep '/src/' | command grep -v 'ring_publish/' | sort

# the named next adopter, and every decision that could have resolved it
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_core/Cargo.toml | grep -oE '^ring_[a-z_]+'
# has any ruling after 124 revisited the question it left open?
after=$( for d in docs/decision/[0-9]*.md; do
  n=$( basename "$d" | cut -d_ -f1 )
  [ "$n" -gt 124 ] && grep -l 'ring_publish' "$d"
  true
done )
[ -n "$after" ] && echo "$after" || echo '(no ruling after 124 names ring_publish)'

# the constant, declared twice
command grep -rn --include=*.rs 'const PUBLISH' . | command grep '/src/' | sort
```

| | Value |
|--|------:|
| Dependencies | 2 |
| Dev-dependencies | 4 |
| **Dependents** | **0** |
| Manifests naming this crate | 2 — its own, and one comment recording removal |
| Crates in the family with zero dependents | 6 |
| …that are tooling, harnesses, or optional | 5 |
| …that are a tiered write-path primitive | **1** |
| Dependents of its three Tier 5 siblings | 2, 2, 1 |
| Crates naming it in prose | **11** |
| …in this family, which is what `001` enumerates | 10 |
| …outside it | 1 — `lang_schema`, modelling a loop on PB9 |
| …that changed shape for a test living here | 2 |
| Crates that considered depending on it and declined | 2 |
| …for the same reason | **0** |
| Decisions numbered ≥ 124 | 14 |
| …mentioning `ring_publish` | 1, and it is about an unsafe allowlist |
| `ring_core` dependencies reaching this crate | 0 |
| `const PUBLISH` declarations in the family | 2 |
| …asserted by a doctest | 1 — and it is the other one |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB1 | family | n/a — unadopted | Six crates have no dependents and five are tooling or explicitly optional; this is the only tiered write-path primitive among them, while its three Tier 5 siblings have two, two, and one |
| PB2 | family | n/a — unadopted | Eleven crates name it in prose and none depends on it — ten in this family, which `001` enumerates, and one outside it, `lang_schema`, which models a bare loop's termination argument on PB9; two of the family ten changed shape *for* it — `ring_barrier` opened a signature, `ring_atomic` built a `loom` seam — for a test that lives in a crate neither depends on |
| PB3 | `ring_publish` | n/a — observation | The four dev-dependencies are the reached-test, not the crate; the check that keeps the sections honest is deliberately two commands, because a single-command version reports four false positives on exactly the crate whose distinguishing feature is which section its edges are in |
| PB4 | `ring_publish` | n/a — observation | This crate's own module documentation named `ring_mpsc` before that crate existed, and [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) quotes it back as the reason the mechanism question was never open: *"The dependency answers it against itself"* |
| PB5 | `ring_core` | n/a — drift | [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) set an escalation condition naming `ring_core` as the next possible adopter; `ring_core` has since been implemented and does not adopt, and the decision that ruling said would become necessary is not written |
| PB6 | `ring_mpsc` | n/a — coverage | The declining crate reproduced the constant it declined: same name, same value, near-identical justifying prose including one sentence verbatim — and *its* copy is asserted by a doctest where this crate's is covered only by two hand-run manual checks |
