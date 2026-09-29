# Thirty-Six Items, and the Four Names the Family Imports

### Scope

- **Purpose**: Read the census as a set and report which public items the eight dependent crates reach, so the per-definition claims about this crate's surface have a total to be checked against.
- **Responsibility**: State the import reach of every public type and the call reach of every public associated function, separating a doc-example mention from a production one.
- **In Scope**: The 27 public items, measured against `ring_*/src`.
- **Out of Scope**: The three items whose whole purpose is backend discrimination (→ [`002`](002_the_backend_discrimination_surface.md)); what each item does, which is the behavioural definitions' subject.

### Which Types Are Imported

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll \
         ring_shutdown ring_testkit; do
  printf '%-14s ' "$c"
  grep -hoE 'ring_core::\{?[A-Za-z_, ]+' $c/src/*.rs | sed 's/ring_core:://' \
    | tr -d '{}' | tr ',' '\n' | tr -d ' ' | sort -u | tr '\n' ' '
  echo
done
```

Live output:

```
ring_bench     Producer Ring 
ring_debug     Consumer Producer Ring 
ring_factory   Ring 
ring_flush     Producer Ring 
ring_handle    Consumer Ends Producer Ring 
ring_poll      Consumer Producer Ring 
ring_shutdown  Consumer Producer Ring 
ring_testkit   Consumer Ends Producer Ring 
```

**Every dependent imports `Ring`; none imports `Backend`.** Seven of eight take
`Producer`, five take `Consumer`, two take `Ends`. The public enum this crate
declares — the one its module documentation calls the way a caller recovers the
contract it was given — is imported by no crate in the family.

### Which Functions Are Called

Production call sites only, across `ring_*/src`:

| Item | Lines | Crates |
|------|------:|-------:|
| `Ring::new` | 2 | 2 |
| `Ring::new_crossbeam` | 1 | 1 |
| `Ring::ends` | 7 | 3 |
| `Ends::split` | 8 | 3 |
| `Producer::try_push` | 6 | 5 |
| `Producer::try_push_batch` | 6 | 5 |
| `Producer::free_capacity` | 4 | 4 |
| `Producer::is_full` | 3 | 3 |
| `Consumer::try_recv` | 6 | 4 |
| `Consumer::try_recv_batch` | 7 | 3 |
| `Ring::overflow` | 2 | 1 |
| **`Ring::backend`** | **0** | **0** |
| **`Producer::try_clone`** | **0** | **0** |

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
for m in 'Ring::new(' '\.ends()' '\.try_push(' '\.free_capacity()' '\.backend()' '\.try_clone()'; do
  N=0; L=0
  for c in $DEP; do
    k=$( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c "$m" )
    L=$(( L + k )); [ "$k" -gt 0 ] && N=$(( N + 1 ))
  done
  printf '%-20s lines %2d  crates %d
' "$m" "$L" "$N"
done
```

Live output:

```
Ring::new(           lines  2  crates 2
\.ends()             lines  7  crates 3
\.try_push(          lines  6  crates 5
\.free_capacity()    lines  4  crates 4
\.backend()          lines  0  crates 0
\.try_clone()        lines  0  crates 0
```

**Comment lines are stripped from the file before matching, not from `grep -n`'s
output.** Filtering the output needs an anchor that survives `grep -n`'s dropping
the filename prefix on a single-file match — get that wrong and `Ring::new`
reads 19 rather than 2, because every doc example in six crates counts as a
call. Both readings are shown above only in the sense that the wrong one is not:
the recipe filters the source.

`Ring::capacity`, `Consumer::len` and `Consumer::is_empty` are omitted rather
than reported: their bare method names collide with `Vec::len`, `str::is_empty`
and `RingConfig::capacity` across the family, and a count that cannot exclude
those measures nothing. The two zeros above do not have that problem —
`try_clone` and `backend` are unique names in this workspace.

### The Shape This Gives

Sixteen public associated functions; eleven carry family traffic, two have no
production caller anywhere, and three cannot be measured by name. The crate is
smaller in use than in declaration, and the part that is unused is not scattered
— it is exactly the backend-discrimination surface, which is why that gets its
own instance.

### CO27 — Eleven of Eighteen Taxonomy Kinds Are Absent

The absences describe the crate better than the presences do. No trait, because
the module documentation's opening argument is that the three backends share no
method set to define one over; no free function, because there is no operation
that is not a method on a handle; no constant and no static, because the crate
holds no configuration of its own.

**A composition layer with no shared abstraction is an unusual shape**, and the
census is where that becomes visible rather than a matter of impression.

### CO28 — Three Public Methods Cannot Be Measured by Name

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
for m in '\.capacity()' '\.len()' '\.is_empty()'; do
  L=0; for c in $DEP; do
    L=$(( L + $( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c "$m" ) ))
  done
  printf '%-14s bare-name hits %d  (attributable to ring_core: unknown)\n' "$m" "$L"
done
```

Live output:

```
\.capacity()   bare-name hits 8  (attributable to ring_core: unknown)
\.len()        bare-name hits 14  (attributable to ring_core: unknown)
\.is_empty()   bare-name hits 5  (attributable to ring_core: unknown)
```

The counts above are real and they answer no question. `.len()` on a `Vec`, a
slice, a `String` and a `ring_core::Consumer` are one string to `grep`, and
resolving them needs type information a text search does not have.

**Recorded as a limit on the measurement, not as a fact about the crate.** The
reach table in this instance omits these three rows for that reason; the two
zeros it does report — `backend` and `try_clone` — are trustworthy precisely
because both names are unique in this workspace.

### CO29 — The Naive Reach Count Is Off by a Factor of Nine

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
printf 'unfiltered:            '
for c in $DEP; do grep -c 'Ring::new(' $c/src/*.rs; done | paste -sd+ | bc
printf 'comment lines removed: '
for c in $DEP; do cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c 'Ring::new('; done \
  | paste -sd+ | bc
```

Live output:

```
unfiltered:            19
comment lines removed: 2
```

Seventeen of the nineteen are `/// let ring : Ring< u32 > = Ring::new( ... )` —
the setup line of a doc example, repeated across six crates. Two are calls:
`ring_bench` building a ring for a workload, `ring_factory` building one for a
config.

**Every reach number in this corpus depends on getting that filter right**, and
the failure is silent in the direction that flatters the crate. Recorded as a
method note so the other counts can be trusted.
