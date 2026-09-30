# Workaround: A Crate Name as a Load-Bearing String

### Scope

- **Purpose**: Record that the family's ban on reaching this crate from the tick path is enforced by matching the literal text `"ring_wait"`, name the missing capability that forces it, and measure the gap between what the roster says and what it checks.
- **Responsibility**: Show the three places the string is load-bearing, compare the declared roster against the real dependency closure, and state what the gap does and does not admit.
- **In Scope**: `ring_poll::PARKING_CRATES` and the two tests built on it, plus `ring_handle`'s forbidden-name scan.
- **Out of Scope**: Why this crate blocks at all — see [`workaround/001`](001_a_sleep_where_a_park_belongs.md).

### The Missing Capability

The family needs to say: *no crate on the tick path may transitively reach a
blocking call.* Cargo has no way to express that. There is no
`forbidden-transitive-dependencies` key, no lint attribute that propagates
through a dependency graph, and `cargo deny`'s bans operate on the resolved
lockfile rather than on a per-crate rule the crate itself declares.

So the family substitutes a string, in three places:

| # | Where | The string does |
|---|-------|-----------------|
| 1 | `ring_poll/src/lib.rs:80` | declares the roster: `PARKING_CRATES : [ &str; 3 ]` |
| 2 | `ring_poll/tests/poll_test.rs:102` | scans every sibling `Cargo.toml` for `"ring_wait"` |
| 3 | `ring_handle/tests/handle_test.rs:502-503` | scans `ring_handle`'s own source for 7 parking-shaped names |

None of the three lives in this crate:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r "PARKING_CRATES\|FORBIDDEN" ring_wait/ --include=*.rs \
  || echo '(none of the guard machinery lives here)'
# control: the identical expression over the two crates that do carry it
grep -rc "PARKING_CRATES\|FORBIDDEN" ring_poll/ ring_handle/ --include=*.rs \
  | grep -v ':0$'
```

Live output:

```
(none of the guard machinery lives here)
ring_poll/tests/poll_test.rs:5
ring_poll/src/lib.rs:4
ring_handle/tests/handle_test.rs:3
```

**No match**, against a control — the identical expression over `ring_poll` and
`ring_handle` — that returns eleven hits across three files. The crate that is
the hazard carries none of the machinery that
fences it off — which is correct (a ban a crate enforces on itself is not a ban)
and worth stating, because it means an edit here cannot break the guard, and a
rename here silently can.

### The Roster

```rust
// ring_poll/src/lib.rs:80
pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
```

`pub` on purpose, and the doc comment says why (`:44-45`):

> This roster is part of the surface rather than a constant inside a test, so
> that adding a crate to it is a visible API change rather than a quiet edit

That is a good call and the main thing keeping the workaround honest. The second
is that the roster is checked against the manifests on disk rather than trusted:

```rust
// ring_poll/tests/poll_test.rs:102-105
if fs::read_to_string( &manifest ).unwrap().contains( "ring_wait" )
{
  measured.push( name );
}
```

```rust
// ring_poll/tests/poll_test.rs:107-113
measured.sort();
assert_eq!
(
  measured, PARKING_CRATES,
  "the crates reaching a parking operation drifted from the declared roster"
);
```

So the roster cannot go stale — a new dependency on this crate fails `ring_poll`'s
suite until the roster is updated, which is exactly the visible edit the doc
comment wanted.

### WT10 — Declared Reach and Actual Reach Differ

The test's own heading states what it means to measure
(`poll_test.rs:66-68`):

> the crates from which a parking operation is reachable are exactly the three
> declared

*Reachable* is transitive. `contains( "ring_wait" )` is not — it sees direct
manifest mentions only. Compute the real closure:

```sh
cd "$(git rev-parse --show-toplevel)"
python3 - <<'PY'
import os, re
deps = {}
for d in sorted( os.listdir( '.' ) ):
  m = os.path.join( d, 'Cargo.toml' )
  if not d.startswith( 'ring_' ) or not os.path.exists( m ): continue
  out = set()
  for s in re.split( r'^\[', open( m ).read(), flags = re.M ):
    if s.startswith( 'dependencies]' ):
      for line in s.splitlines()[ 1 : ]:
        mm = re.match( r'(ring_[a-z_]+)\s*[=.]', line )
        if mm: out.add( mm.group( 1 ) )
  deps[ d ] = out

def reach( c, seen = None ):
  seen = seen if seen is not None else set()
  for x in deps.get( c, () ):
    if x not in seen:
      seen.add( x ); reach( x, seen )
  return seen

print( 'closure reaches ring_wait:',
       [ c for c in sorted( deps ) if 'ring_wait' in reach( c ) ] )
for t in [ 'ring_poll', 'ring_handle', 'ring_core' ]:
  print( f'{t}: reaches ring_wait? {"ring_wait" in reach( t )}  (closure of {len( reach( t ) )})' )
PY
```

Live output:

```
closure reaches ring_wait: ['ring_barrier', 'ring_consume', 'ring_shutdown', 'ring_testkit']
ring_poll: reaches ring_wait? False  (closure of 16)
ring_handle: reaches ring_wait? False  (closure of 16)
ring_core: reaches ring_wait? False  (closure of 15)
```

| | Crates |
|--|--------|
| `PARKING_CRATES`, and what the scan measures | `ring_barrier`, `ring_shutdown`, `ring_wait` |
| Runtime closure that actually reaches `ring_wait` | `ring_barrier`, **`ring_consume`**, `ring_shutdown`, **`ring_testkit`** |

`ring_consume` reaches a parking operation through `ring_barrier`; `ring_testkit`
reaches one through `ring_shutdown`. Neither manifest contains the string, so
neither is measured, and neither is on the roster the doc comment calls
exhaustive.

### WT11 — One of the Three Entries Is a Self-Reference

There is a second mismatch, inside the three that *are* listed. Two of them
(`ring_barrier`, `ring_shutdown`) are there because they declare a dependency.
The third is there because its own manifest carries `name = "ring_wait"`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep "ring_wait" ring_wait/Cargo.toml ring_barrier/Cargo.toml ring_shutdown/Cargo.toml
```

Live output:

```
ring_wait/Cargo.toml:name = "ring_wait"
ring_barrier/Cargo.toml:ring_wait = { path = "../ring_wait" }
ring_shutdown/Cargo.toml:ring_wait = { path = "../ring_wait" }
```

| Crate | Why the string is in its manifest |
|-------|-----------------------------------|
| `ring_barrier` | a dependency edge |
| `ring_shutdown` | a dependency edge |
| `ring_wait` | its own `name =` field |

The scan cannot tell the two apart, so the roster's real membership rule is *"the
text appears"* — which happens to produce the right three, because a crate that
*is* a parking operation belongs on a roster of crates that can reach one. It
arrives at the correct answer for a reason unrelated to the question.

That matters for the same edit WT10 does: the rule that admits `ring_wait` by
self-reference would also admit any crate whose manifest merely *mentions* the
name — in a comment, in a `description`, in a `[package.metadata]` block — and
would then demand it be added to a roster of crates that can park.

### What the Gap Does and Does Not Admit

The guarantee that matters still holds, and holds by closure rather than by
substring:

| Crate | Closure size | Reaches `ring_wait` |
|-------|-------------:|---------------------|
| `ring_poll` | 16 | **no** |
| `ring_handle` | 16 | **no** |
| `ring_core` | 15 | **no** |

`ring_core`'s dependencies are `ring_config`, `ring_mpsc`, `ring_overflow`,
`ring_slot`, `ring_spsc`, `ring_types`, and none of those reaches this crate. So
the tick path is genuinely clean today, and the test asserting it
(`poll_test.rs:107-114`) passes for the right reason.

What the gap admits is a future edit exactly one level deep. Adding
`ring_consume` to `ring_core`'s dependencies would put a parking operation on the
tick path, and:

- `measured` would be unchanged — `ring_core`'s manifest still would not contain
  `"ring_wait"`
- the roster assertion would still pass
- the "no tick-path crate is among them" loop would still pass, because
  `ring_core` is not in `measured`
- `ring_handle`'s forbidden-name scan reads only its own `src/lib.rs`, so it
  would not fire either

The fix is one line — replace the substring scan with the transitive closure the
heading already claims — and it is worth doing precisely because the current
check is otherwise good enough to be trusted.

### The Second String, in `ring_handle`

```rust
// ring_handle/tests/handle_test.rs:502-503
const FORBIDDEN : [ &str; 7 ] =
[ "thread::sleep", "yield_now", "::park", "park(", "Condvar", "Duration", "Waker" ];
```

Same substitution, one level lower: instead of banning a crate, ban the names a
blocking call would be spelled with, and grep the crate's own source for them.
The test strips comments first, naively:

```rust
// ring_handle/tests/handle_test.rs:506-510
let code : String = source
  .lines()
  .map( | line | line.split( "//" ).next().unwrap_or( "" ) )
  .collect::< Vec< _ > >()
  .join( "\n" );
```

which also truncates any line containing `//` inside a string literal — a URL, a
path. That only ever makes the check weaker, never noisier, so it cannot produce
a false alarm; it can produce a false clear.

The stripping is there because the un-stripped version failed immediately, and
the test records why (`:478-484`):

> The first version of this test scanned the raw file for the bare substring
> `park` and failed — on the module documentation explaining that no *parking*
> operation may be reachable. A guard that fires on the prose describing it is
> the defect shape `ring_poll`'s manual plan catalogues first, and it arrived
> here within a minute of the guard being written.

Which is the general hazard of enforcing a rule by matching text: the document
that states the rule is itself a match. This crate's own manual plan hits the
same thing from the other side — W4 (`tests/manual/readme.md:88-97`) requires
that `thread::park` *does* appear in `src/lib.rs`, because the comment explaining
its absence is where it appears.

To its credit `ring_handle`'s test states its own limits (`:486-489`):

> **What it does not catch, stated so nobody mistakes it for more:** a busy loop,
> a `Duration` arriving through a type alias, or a blocking call reached through
> a dependency other than `ring_wait`.

That last clause is the same gap as WT10, named from the other side, in the crate
that would be the victim of it.

### The Deletion Condition

Both strings can go when a dependency ban becomes expressible as a rule rather
than as text — a Cargo feature, a workspace lint, or a `cargo deny` policy
checked in CI with the tick-path crates named as roots. Until then the string is
what the family has, and the honest reading is: the roster is a *declaration*
that the manifest scan keeps from drifting, not a proof of unreachability.

**The test that would notice a rename.** None. Renaming this crate breaks nothing
loudly: `PARKING_CRATES` keeps a stale string that matches no directory, the scan
finds two crates instead of three, and the assertion fails with *"the crates
reaching a parking operation drifted from the declared roster"* — which is the
right alarm for the wrong reason, and would be read as a dependency change rather
than a rename. Cheap to improve: assert each roster entry names a directory that
exists.


### WT52 — The Allow-List Lives in the Crate That Enforces It

The parking ban is a fixed-size array of crate names compiled into `ring_poll`.

```sh
cd "$(git rev-parse --show-toplevel)"
# the doc comment wraps, so join it into one line before matching anything in it
DOC=$( awk '/pub const PARKING_CRATES/{ exit } /^\/\/\//{ sub( /^\/\/\/ ?/, "" ); printf "%s ", $0 }' ring_poll/src/lib.rs )
printf 'the array, as declared:  %s\n' "$( command grep -m1 -oE 'pub const PARKING_CRATES.*$' ring_poll/src/lib.rs )"
printf 'the crate that holds it: %s\n' "$( command grep -rl 'pub const PARKING_CRATES' ring_*/src/lib.rs | cut -d/ -f1 | tr '\n' ' ' )"
printf 'the test that enforces:  %s\n' "$( printf '%s' "$DOC" | command grep -m1 -oE 'the_tick_path_cannot_reach_a_parking_operation' )"
printf 'what that test reads:    %s\n' "$( printf '%s' "$DOC" | command grep -m1 -oE 'reads the manifests off disk and compares them against this array' )"
printf 'and what fails on drift: %s\n' "$( printf '%s' "$DOC" | command grep -m1 -oE 'without being listed here fails the suite' )"
```

Live output:

```
the array, as declared:  pub const PARKING_CRATES: [&str; 3] = ["ring_barrier", "ring_shutdown", "ring_wait"];
the crate that holds it: ring_poll 
the test that enforces:  the_tick_path_cannot_reach_a_parking_operation
what that test reads:    reads the manifests off disk and compares them against this array
and what fails on drift: without being listed here fails the suite
```

`PARKING_CRATES : [ &str; 3 ]` names `ring_barrier`, `ring_shutdown`, and
`ring_wait`, and the test reads every manifest in the family and fails for any
crate that takes a `ring_wait` dependency without being on the list.

The direction that puts is worth stating. A crate that legitimately needs to wait
— a new consumer helper, say — cannot declare that for itself. It adds the
dependency, `ring_poll`'s suite fails, and the fix is an edit to `ring_poll`'s
source by whoever owns the new crate. The permission lives with the enforcer, and
the array's length is part of its type, so the edit is two places in one line.

That is the correct trade for a rule Cargo cannot express — the alternative is a
per-crate opt-in marker, which is a rule anyone can grant themselves and
therefore not a rule. It costs one cross-crate edit per legitimate addition, and
it means the roster's three entries (one of which is a self-reference, WT11) are
a list of exceptions maintained by the crate the exceptions are exceptions to.

### Workarounds

| File | Relationship |
|------|--------------|
| [001_a_sleep_where_a_park_belongs.md](001_a_sleep_where_a_park_belongs.md) | The blocking call this ban exists to fence off |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The two dependents and the roster that names them |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_repetition_is_a_counted_for.md](../invariant/001_every_repetition_is_a_counted_for.md) | The property the tick path is being protected from losing |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | Why a parking operation on the tick path is the thing to ban |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_predicate_the_pause_and_the_budget.md](../pattern/001_the_predicate_the_pause_and_the_budget.md) | The three loops `ring_poll` open-codes rather than depend on this crate |

### Sources

| File | Relationship |
|------|--------------|
| `ring_poll/src/lib.rs:40-80` | The roster, and the rationale for making it public |
| `ring_poll/tests/poll_test.rs:83-123` | The manifest scan and the roster assertion |
| `ring_poll/tests/poll_test.rs:125-142` | `ring_poll`'s own manifest, checked at compile time |
| `ring_handle/tests/handle_test.rs:487-522` | The forbidden-name scan, why comments are stripped, and its stated limits |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § W4 | The same text-matching hazard from this side — `thread::park` must appear, in the comment |

Nothing in this crate's own suite touches the ban, which follows from the
`PARKING_CRATES`/`FORBIDDEN` grep above returning nothing here: the guard is
entirely a consumer-side construct, and this crate has no way to break it or to
check it.
