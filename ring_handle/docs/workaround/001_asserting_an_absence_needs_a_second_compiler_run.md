# Workaround: Asserting an Absence Needs a Second Compiler Run

### Scope

- **Purpose**: Give W1 — Rust's inability to express "this method must not exist" — its own instance, with the compensation's full cost and its deletion condition.
- **Responsibility**: State the constraint, the mechanism that stands in for it, what the mechanism costs, and what would retire it.
- **In Scope**: The `trybuild` suite; the pinned `.stderr` files; the regeneration command; the dev-dependency it forces.
- **Out of Scope**: What the pinned files happen to *contain* (→ [`002_the_expected_output_names_crates_this_one_never_sees.md`](002_the_expected_output_names_crates_this_one_never_sees.md)); the design being asserted (→ [`pattern/001`](../pattern/001_enforce_by_withholding.md)).

### The Constraint

**Rust has no negative trait bound and no first-class "this must not compile"
assertion.** The crate's entire guarantee — that no code path yields a second
`Producer` or a second `Consumer` over one ring — is a statement about what is
*absent*. A test can call a method that exists; nothing in the test harness can
call a method that does not.

The nearest first-class expressions all fail:

| Attempt | Why it does not work |
|---------|----------------------|
| `where Self : !Clone` | Negative bounds are unstable and have never been on a stabilization path |
| A runtime assertion | There is nothing to assert against — the method does not exist to be probed |
| A doc comment saying so | Documents the intent, detects nothing |
| Deleting the method and trusting review | The next contributor re-adds it in good faith; nothing objects |

### The Compensation

A `trybuild` suite: seven `.rs` files that must fail to compile, each paired
with a `.stderr` file holding the exact expected message, driven from
`tests/ui_test.rs` as an ordinary `#[ test ]`.

The mechanism is a **second full compilation**, launched from inside the test
binary, whose *failure* is the pass condition.

### The Cost

| Cost | Detail |
|------|--------|
| A published dev-dependency | `trybuild = "1.0"` — the crate's only non-sibling dependency of any kind |
| Toolchain-coupled expectations | The `.stderr` files pin compiler diagnostics, which are explicitly not a stability surface |
| A regeneration escape hatch | `TRYBUILD=overwrite` accepts whatever the compiler now says — including a message that changed because the guarantee broke |
| Compile time | Seven extra compilations, each of which must reach the type checker to produce its error |

**The third row is the one that matters.** The regeneration command is both the
routine maintenance path after a toolchain upgrade *and* the exact command that
would silently accept a real regression. Nothing distinguishes the two cases at
the moment the command is run.

### Deletion Condition

Negative bounds, or any stable mechanism for asserting that a given expression
does not type-check, landing in the language. **Not on a roadmap this crate can
name**, so this workaround is expected to be permanent.

If that changes, the deletion is mechanical: the seven `.rs`/`.stderr` pairs and
the `trybuild` dependency all go, and the bounds move onto the impls.

### Workarounds

| File | Relationship |
|------|--------------|
| [002_the_expected_output_names_crates_this_one_never_sees.md](002_the_expected_output_names_crates_this_one_never_sees.md) | What the compensation's pinned files turned out to contain |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | The design this workaround exists to assert |
| [../pattern/002_forward_narrow_or_add.md](../pattern/002_forward_narrow_or_add.md) | The narrow category, whose enforcement is this suite |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | The acceptance criterion this mechanism satisfies |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md](../pitfall/002_a_pinned_diagnostic_is_a_dependency_nobody_declared.md) | The trap the pinned files set for a maintainer |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | Where `trybuild` is declared |
| [`tests/ui_test.rs`](../../tests/ui_test.rs) | The driver |

### Tests

| Test | Relationship |
|------|--------------|
| `no_parking_shaped_name_appears_in_the_source` | A different absence, asserted by grep rather than by compilation — the comparison that shows how narrow `trybuild`'s reach is |

### HD45 — The Readme Says "No Published Crate" and the Manifest Names One

The directory readme's own overview and its W1 row disagree:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the manifest declares --'
sed -n '/^\[dependencies\]/,$p' ring_handle/Cargo.toml | command grep -vE '^\s*$|^\[lints\]|^workspace'
echo '  -- what the workaround readme says about that --'
# the readme's own findings table is excluded: this finding is recorded there
# too, and a scan that counted its own row would be quoting itself
command grep 'published crate\|test dependency' ring_handle/docs/workaround/readme.md \
  | command grep -v '^| HD'
```

Live output:

```
  -- what the manifest declares --
[dependencies]
ring_core = { path = "../ring_core" }
[dev-dependencies]
ring_config = { path = "../ring_config" }
ring_types = { path = "../ring_types" }
trybuild = "1.0"
  -- what the workaround readme says about that --
`ring_core` — and on no published crate; W1's own compensation adds one
| W1 | **Rust cannot express "this method must not exist" as a bound.** There is no negative trait bound, no `where Self: !Drain`, and no way to assert an absence in the type system itself | A `trybuild` compile-fail suite — a second compilation, driven from a test, comparing against pinned stderr | A test dependency; brittle expected-output files that break on toolchain upgrades; a regeneration command (`TRYBUILD=overwrite`) that is also how a genuine regression gets accepted | Negative bounds or a stable "assert this does not compile" mechanism lands in the language. Not on any roadmap this crate can name |
```

`trybuild = "1.0"` is a published crate, declared in this crate's own manifest,
pulled from crates.io, and named in W1's own Cost column as "a test dependency".
Nine lines above that row the readme states the crate depends "on no published
crate", and its Sources table repeats the claim at the bottom of the file.

**Both sentences were true of different things and the readme does not say
which.** The overview means *runtime* dependencies — `[dependencies]` really is
one path entry — and W1's cost column means dev-dependencies. A reader auditing
supply-chain surface reads the overview, stops, and gets the wrong answer, which
is exactly the audience a workaround directory is written for.

The fix is one qualifying word in the overview. Recorded here rather than
silently patched because the same elision is likely to recur wherever a
"dependencies" claim is made without naming the section it came from.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'runtime path dependency' ring_handle/docs/workaround/readme.md
```

Live output:

```
| `Cargo.toml` | The dependency surface examined for this finding — one runtime path dependency, no published runtime crates, and one published `dev-dependency` (`trybuild`) |
```

**Disposition:** applied — the readme's Overview now scopes the "no published
crate" claim to the runtime surface and names `trybuild` as W1's own
published `dev-dependency` in the same sentence; the Sources row makes the
same runtime/dev split instead of repeating "no published crates" unqualified.
Now prints: `one runtime path dependency, no published runtime crates`

### HD46 — The Escape Hatch and the Maintenance Path Are the Same Command

`TRYBUILD=overwrite` is how the pinned files are regenerated and how a
regression is accepted:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- pinned expectation files --'
ls ring_handle/tests/ui/*.stderr | wc -l
echo '  -- and the guard that would notice the difference --'
printf '  test files naming TRYBUILD:             %s\n' \
  "$( command grep -rlc 'TRYBUILD' ring_handle/tests/ 2>/dev/null | wc -l )"
printf '  checked-in hashes of the expected text: %s\n' \
  "$( ls ring_handle/tests/ui/*.sha* 2>/dev/null | wc -l )"
```

Live output:

```
  -- pinned expectation files --
7
  -- and the guard that would notice the difference --
  test files naming TRYBUILD:             1
  checked-in hashes of the expected text: 0
```

Seven pinned files, no hash, no second record of what the message used to say
beyond the files themselves.

**So the only artefact distinguishing "the compiler rephrased this" from "the
guarantee broke" is the diff a reviewer reads.** That is a real check and it is
also the weakest link in the chain — a toolchain upgrade produces seven
simultaneous stderr diffs, all expected, and the one that is not expected
arrives in the same commit as the six that are.

Nothing here is fixable inside this crate; it is inherent to the mechanism W1
forces. It is recorded because the crate's confidence rests on this suite, and
a reader should know the suite's failure mode is *quiet acceptance* rather than
a false alarm.

**Disposition:** declined — verified rather than taken on trust: the only
crate-local artefact that could add a second signal beyond "the diff a
reviewer reads" would be a checked-in hash or line count of each `.stderr`
file, and that adds nothing `git diff` does not already give a reviewer for
free — it would not distinguish a toolchain rephrasing from a real regression
any better than the diff itself does, since both change the hash identically.
The shared-command property is inherent to `trybuild`'s pinned-diagnostic
design (this file's own Deletion Condition: only a first-class "assert this
does not compile" mechanism retires it), not a ring_handle-specific gap this
crate's own `src/` or `tests/` can close.
