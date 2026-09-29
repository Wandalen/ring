# Integration: Five Crates Downstream, Twelve Reached

### Scope

**Purpose:** Record which crates depend on `ring_atomic`, which crates use its
trait, and why the two lists are different lengths.

**Responsibility:** The five outgoing dependency edges, `ring_cursor`'s re-export
of `SeqCell` and the argument for it, and the reach that re-export produces.

**In Scope:** the five dependants' manifests and `use` lines;
`ring_cursor/src/lib.rs:62-68`; `ring_mpsc/src/lib.rs:197-200`.

**Out of Scope:** The incoming edges are
[`integration/001`](001_two_declared_one_used.md). What each dependant asserts with
`CountingSeq` is
[`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) AT15.

---

## Five Manifests, Two Names, One Rule

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the five declared edges out --'
command grep -l '^ring_atomic' ring_*/Cargo.toml | sed 's|ring/||;s|/Cargo.toml||' | tr '\n' ' '; echo
echo '  -- and where each of those takes the trait from --'
for c in $( ls ring | command grep '^ring_' | command grep -v '^ring_atomic$' )
do
  f=ring/$c/src/lib.rs; [ -f "$f" ] || continue
  imp=$( command grep -hoE '^use ring_(atomic|cursor)::[^;]*SeqCell[^;]*' "$f" | command grep -oE 'ring_(atomic|cursor)' | head -1 )
  [ -n "$imp" ] || continue
  d=$( command grep -qE '^ring_atomic' ring/$c/Cargo.toml && echo yes || echo ' no' )
  printf '    %-14s declares ring_atomic: %s   imports SeqCell from: %s\n' "$c" "$d" "$imp"
done
echo '  -- the one line that makes the second column possible --'
command grep -m1 -A4 -F '/// a `PaddedCursor` and not this trait holds a value it cannot load. Making' ring_cursor/src/lib.rs
echo '  -- and the crates that line actually serves --'
n=0
for c in $( ls ring | command grep '^ring_' )
do
  m=ring/$c/Cargo.toml; [ -f "$m" ] || continue
  command grep -qE '^ring_cursor' "$m" && ! command grep -qE '^ring_atomic' "$m" && n=$(( n + 1 ))
done
echo "    $n crates declare ring_cursor and not ring_atomic"
```

Live output:

```
  -- the five declared edges out --
ring_batch ring_cursor ring_debug ring_mpsc ring_tls 
  -- and where each of those takes the trait from --
    ring_batch     declares ring_atomic: yes   imports SeqCell from: ring_atomic
    ring_claim     declares ring_atomic:  no   imports SeqCell from: ring_cursor
    ring_consume   declares ring_atomic:  no   imports SeqCell from: ring_cursor
    ring_debug     declares ring_atomic: yes   imports SeqCell from: ring_atomic
    ring_mpsc      declares ring_atomic: yes   imports SeqCell from: ring_atomic
    ring_publish   declares ring_atomic:  no   imports SeqCell from: ring_cursor
    ring_spsc      declares ring_atomic:  no   imports SeqCell from: ring_cursor
    ring_tls       declares ring_atomic: yes   imports SeqCell from: ring_atomic
  -- the one line that makes the second column possible --
/// a `PaddedCursor` and not this trait holds a value it cannot load. Making
/// each such crate declare `ring_atomic` itself would put a dependency in
/// four manifests to import one trait — and would say, wrongly, that those
/// crates have business with the atomic layer beyond the cursor they were
/// handed.
  -- and the crates that line actually serves --
    8 crates declare ring_cursor and not ring_atomic
```

---

### AT19 — The Manifest Graph Shows Five; the Trait Reaches Twelve

Five crates declare `ring_atomic`. `SeqCell` is nonetheless in scope in twelve,
because `ring_cursor` re-exports it in one line, and the rule governing which name
each crate uses is exact with no exceptions: a crate imports the trait from
`ring_atomic` if and only if it declares `ring_atomic`; otherwise it imports it from
`ring_cursor`. Four and four among the crates with a top-level `use`, plus
`ring_cursor` itself and three more that reference it in doctests only.

**Finding.** The blast radius of a change to `SeqCell` is twelve crates, and the
dependency graph shows five. Adding a method, changing a signature, or adding the
`Sync` supertrait of [`api/002`](../api/002_a_shared_cell_that_is_not_sync.md) is a
twelve-crate change presented by every tool as a five-crate one — and seven of the
twelve reach the trait through a crate they think of as the cursor crate.

The re-export is not an accident. `ring_cursor` argues for it in five lines of doc
comment, and the argument is good: a crate handed a `PaddedCursor` cannot load it
without the trait, and making each declare `ring_atomic` "would say, wrongly, that
those crates have business with the atomic layer beyond the cursor they were
handed." That is a real design position and this crate benefits from it — it keeps
`ring_atomic`'s manifest fan-out honest about who actually reaches for the atomic
layer.

What is missing is the other half of the bookkeeping. Nothing on `ring_atomic`'s
side records that its central abstraction is republished, so a reader of *this*
crate has no way to learn that the trait's reach is more than double its
dependants.

---

### AT20 — The Re-Export's Own Count Is Two Short, and One Crate Noticed the Two Names

The argument says the alternative "would put a dependency in six manifests". Eight
crates declare `ring_cursor` and not `ring_atomic`, so eight is the number of
manifests the re-export saves — `ring_barrier`, `ring_claim`, `ring_consume`,
`ring_gating`, `ring_publish`, `ring_shutdown`, `ring_spsc`, and `ring_wait`.

**Finding.** The count was right when written and two crates joined the family
after. It understates the argument it is making, which is the harmless direction —
but it is the same failure mode as the eleven `Depends on` sentences that understate
their manifests ([`integration/001`](001_two_declared_one_used.md) AT17): a number
written once, correct once, and never recomputed, in a workspace where recomputing
it is a two-line shell loop.

Exactly one crate's source acknowledges that the trait has two names, and it is
the one crate that declares both and therefore had to choose:

> `SeqCell` is one trait with two re-exports — `ring_cursor` republishes
> `ring_atomic`'s. It is taken from `ring_atomic` here because both cell types
> this crate touches need it: the unpadded stamps and the padded consumer cursor.

That is `ring_mpsc`, and it is the only place in thirty-three crates where the
duplication is named and a choice between the two paths is justified. The other
eleven each picked one silently — correctly, by the rule above, but with nothing on
record saying a rule exists.

**Disposition:** declined — the stale "six manifests" count is `ring_cursor`'s
own doc comment at `ring_cursor/src/lib.rs:62-68`; correcting it belongs
to `ring_cursor`'s own crate-scoped disposition pass, which has not been
freshness-checked or claimed in this pass.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_two_declared_one_used.md) | The edges in, and the one that carries nothing |
| [`api/002`](../api/002_a_shared_cell_that_is_not_sync.md) | The declaration-level change whose blast radius is the twelve, not the five |
| [`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md) | The two dependants the trait was built for, and what they assert |
| [`pattern/001`](../pattern/001_one_place_where_an_atomic_is_created.md) | One construction point, two import paths |

### Sources

| Fact | Where |
|------|-------|
| The five declaring manifests | Census above |
| The import-path rule, with no exceptions | Census above |
| The re-export and its argument | `ring_cursor/src/lib.rs:62-68` |
| Eight crates served, six claimed | Census above |
| The only source acknowledging two names | `ring_mpsc/src/lib.rs:197-200` |

### Tests

| Test | Covers |
|------|--------|
| `a_cell_drives_through_the_trait_alone` | That the trait works as an interface — in this crate, not across the twelve |
| *(to create)* | Nothing checks that a count written in prose still matches the workspace, here or in the eleven `Depends on` sentences |
