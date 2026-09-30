# Decision: What This Crate Is For, Given `ring_core` Already Splits

**Status:** open. The four narrowings below are real and are implemented; whether
four narrowings justify a crate on the five-name export surface is not something
this crate's own evidence can settle. The question belongs to `ring_factory`,
which decides what a caller actually constructs.

### Scope

- **Purpose**: State honestly that this crate's specified purpose is already met one crate down, enumerate the four narrowings it actually adds, and file the question of whether four narrowings justify a place on the five-name export surface.
- **Responsibility**: The measured redundancy, the narrowings N1–N4 with what each is worth, the three options, and the evidence that would settle the choice.
- **In Scope**: What `Split< T >` adds over `ring_core::Ring::ends`.
- **Out of Scope**: Why `is_closed` is absent (→ [`002_why_is_closed_is_absent.md`](002_why_is_closed_is_absent.md)); what a caller actually constructs (→ [`ring_factory/docs/decisions/001`](../../../ring_factory/docs/decisions/001_the_owner_is_the_return_value.md), which closes this question).

### The problem, stated plainly

This crate's specification was written before `ring_core` existed in its current
form. It says the split is what the crate provides. Measured against
the code that exists:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'pub fn try_recv\|pub fn try_push' ring_core/src/lib.rs
```

Live output:

```
  pub fn try_push( &mut self, record : T ) -> Result< (), T >
  pub fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
  pub fn try_recv( &mut self ) -> Option< T >
  pub fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
```

`ring_core::Producer` has no drain. `ring_core::Consumer` has no publish. **The
capability partition this crate's specification asks for is already there, one crate down.**

So a crate whose stated purpose is "the ring's two ends as two separate values"
is, as stated, redundant. That is not a reason to delete it — it is a reason to
say precisely what it *does* add, and to be honest that the list is short.

### What it actually adds

| # | Narrowing | Without it |
|---|---|---|
| N1 | The ring is taken **by value** (`Split::new`) | `Ring::ends` borrows, so the caller still holds the ring and can split it again after the first pair drops. This crate's own If Missing clause — "one shared mutable reference to the whole ring" — is only half addressed |
| N2 | `try_clone` is **withheld** | `ring_core::Producer::try_clone` exists and succeeds on an MPSC backend. On an SPSC ring it refuses at runtime; a handle that cannot be cloned at all refuses at compile time |
| N3 | `Consumer::drain` with a call-time bound | `ring_core` has no iterator drain. The bound is what makes a full drain terminate under a live producer |
| N4 | Nothing reaches the backend | `ring_core::Ring` has six public methods; `Split` re-offers only `ends()` and adds its own `new()`. The other four — `new_crossbeam()`, `backend()`, `capacity()`, `overflow()` — are withheld, and `new_crossbeam()` is a constructor, not a read |

N1 and N2 are the substantive ones. N3 is a convenience with a correctness
argument attached. N4 is weak on its own — it withholds four `ring_core`
methods, not three, and one of them (`new_crossbeam`) is a constructor rather
than a read (→ HD14).

### The three options

| Option | What it means | Cost |
|---|---|---|
| **Keep it as the narrowing layer** (today) | This crate is the export-surface handle; `ring_core` is internal | A forwarding layer whose methods are one line each, and two type names for one concept |
| Merge into `ring_core` | Move N1–N4 down; delete this crate | `ring_core` is on the five-name export list already, so nothing is lost externally. But `ring_core` then owns both the backend dispatch and the export contract, and those change for different reasons |
| Move the ends **out** of `ring_core` | `ring_core` keeps `Ring` and the backend dispatch; the handle types live only here | The cleanest separation, and the most disruptive — every crate that names `ring_core::Producer` (today: `ring_shutdown`, `ring_poll`) changes |

### What would settle it

**Whether a caller ever legitimately wants `ring_core`'s ends rather than this
crate's.** If the answer is no, option three is right and the duplication is
pure. If some caller genuinely wants `try_clone` — an MPSC fan-in, say — then
the two surfaces are serving two populations and option one is right.

`ring_factory` is the crate that constructs rings for real
callers, and it is where the first honest answer appears:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- which ends the first real constructor hands back --'
command grep -r 'ring_core::Producer\|ring_core::Consumer\|ring_handle::Split' ring_factory/src | sed 's|^|    |'
echo '  -- crates whose source mentions try_clone at all --'
command grep -rl 'try_clone' ring_*/src | sed 's|/src/.*||;s|.*/|    |' | sort -u
printf '  -- call sites outside ring_core: %s\n' \
  "$( command grep -rn '\.try_clone()' ring_*/src | command grep -cv 'ring_core/' )"
```

Live output:

```
  -- which ends the first real constructor hands back --
    ring_factory/src/lib.rs://! cannot be written.** `ring_handle::Split::ends` borrows `&mut self` and
    ring_factory/src/lib.rs:use ring_handle::Split;
  -- crates whose source mentions try_clone at all --
ring_bench
ring_core
ring_handle
  -- call sites outside ring_core: 0
```

`ring_factory` names `ring_handle::Split` and no `ring_core` end. `try_clone`
is named in three crates and called from none of them outside `ring_core`'s own
doctest: `ring_core` defines it, `ring_handle` lists it as withheld, `ring_bench`
mentions it in prose. **No caller has asked for the second surface.**

**That is a data point, not a settlement.** `ring_factory` is the only
constructor written so far, and the MPSC fan-in the criterion imagines is
exactly the case it has not yet had to serve. What the measurement rules out is
the weaker claim this instance previously rested on.

**The previous reading of this measurement was a tool error mistaken for
evidence.** The recipe as first written used a path relative to the gate's
working directory, so it reported `No such file or directory`, and the prose
recorded that as "that population does not exist yet." `ring_factory/src`
was committed at the time. A recipe that cannot find its target and a recipe
that finds nothing produce different exit codes and the same-looking
conclusion — which is why every recipe in this corpus carries its own absolute
`cd`.

### Why it is filed rather than decided

Deciding now would mean choosing between three options on the strength of taste,
and the switching cost is asymmetric: option one is reversible at any time, and
option three touches every consumer. Filing it with a named owner and a named
measurement costs nothing and keeps the reversible option open.

**What is *not* deferred:** the honesty. The crate's own module documentation and
[`readme.md`](../../readme.md) both open by saying `ring_core` already splits, so
a reader is not left to discover the overlap themselves.

### Related

- [`data_structure/001`](../data_structure/001_two_handles_over_one_backend.md) —
  the D2 shape, which option three would not change
- [`integration/002`](../integration/002_on_the_export_surface.md) — why the
  export-surface question is what makes this worth deciding rather than leaving

### HD13 — The Status Line Says Open and the ADR That Closes It Is Accepted

This instance defers the question to `ring_factory`. Its ADR ruled, and said so:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- this instance status --'
command grep -m1  -A4 -F '# Decision: What This Crate Is For, Given `ring_core` Already Splits' ring_handle/docs/decisions/001_what_this_crate_is_for.md | tail -n 3
echo '  -- and the ADR that names it --'
command grep -m1 -A5 -F '**Status:** accepted, 2026-08-28. Rules Pendings 1, 2, 3 and 4 together, which' ring_factory/docs/decisions/001_the_owner_is_the_return_value.md
```

Live output:

```
  -- this instance status --
**Status:** open. The four narrowings below are real and are implemented; whether
four narrowings justify a crate on the five-name export surface is not something
this crate's own evidence can settle. The question belongs to `ring_factory`,
which decides what a caller actually constructs.
  -- and the ADR that names it --
**Status:** accepted, 2026-08-28. Rules Pendings 1, 2, 3 and 4 together, which
[`readme.md`](readme.md) recommended doing in one ADR in the order 2 → 1 → 3.
Also closes the question
[`ring_handle/docs/decisions/001`](../../../ring_handle/docs/decisions/001_what_this_crate_is_for.md)
left to it — "the question belongs to `ring_factory`, which decides what a
caller actually constructs."
```

`ring_factory/docs/decisions/001` is accepted, dated, and states in its own
status line that it closes the question this instance raised.

**Two instances disagree about whether one question is settled, and the one that
is wrong is the one a reader arrives at first.** A reader following the crate's
own decisions directory finds an open question with three live options; a reader
following `ring_factory`'s finds the same question closed in favour of option
one. Nothing links forward from here — the Out of Scope line names the closing
ADR, and the Status line above it still says "open".

The ruling itself is not in doubt. What is missing is the back-edge: a decision
closed elsewhere leaves no trace at the site that raised it unless someone goes
back, and nothing in the corpus checks that they did.

### HD14 — N4 Withholds Four Methods and Says Three, and One of Them Is Not a Read

The narrowing table calls the withheld `ring_core::Ring` surface "three harmless
reads":

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- everything ring_core::Ring offers --'
sed -n '/^impl< T : Send > Ring< T >/,/^}/p' ring_core/src/lib.rs \
  | command grep -E '^  pub (const )?fn ' | sed 's/ ->.*//'
echo '  -- and everything Split offers --'
sed -n '/^impl< T : Send > Split< T >/,/^}/p' ring_handle/src/lib.rs \
  | command grep -E '^  pub (const )?fn ' | sed 's/ ->.*//'
```

Live output:

```
  -- everything ring_core::Ring offers --
  pub fn new( config : &RingConfig )
  pub fn new_crossbeam( config : &RingConfig )
  pub const fn backend( &self )
  pub fn capacity( &self )
  pub const fn overflow( &self )
  pub fn ends( &mut self )
  -- and everything Split offers --
  pub const fn new( ring : Ring< T > )
  pub fn ends( &mut self )
```

`Ring` has six public methods: `new`, `new_crossbeam`, `backend`, `capacity`,
`overflow`, `ends`. `Split` re-offers `ends` and adds `new`. So four are
withheld, not three, and `new_crossbeam` is a **constructor**, not a read.

**Withholding a constructor is a different act from withholding an accessor**,
and it is the one with a consequence: a `Split` can never be built on the
crossbeam backend through this crate, so the `crossbeam` feature this crate
declares is reachable only via `ring_factory::build_crossbeam`. The feature
exists here, forwards to `ring_core/crossbeam`, and has no constructor of its
own to apply it to.

N4 is described as the weakest of the four narrowings. On the corrected count it
is the one that actually removes a capability rather than a query.

**Disposition:** applied — the N4 row and the summary paragraph above HD13 no
longer say "three" or "harmless reads." Both now name all four withheld
methods and single out `new_crossbeam` as a constructor, matching this
section's own Live output.
Now prints: `pub fn new_crossbeam( config : &RingConfig )`
