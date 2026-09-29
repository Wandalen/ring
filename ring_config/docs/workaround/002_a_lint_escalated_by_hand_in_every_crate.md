# Workaround: A Lint Escalated by Hand in Every Crate

### Scope

**Purpose:** Record the crate's one crate-level attribute as what it is — a
per-file escalation of a workspace lint — and measure whether the central setting
could carry it instead.

**Responsibility:** The single attribute in `ring_config/src/lib.rs`, the same
attribute in fifty-nine other crates, the table entry all but four crates in the
workspace inherit, and a measured check of what changing that entry would cost.

**In Scope:** `ring_config/src/lib.rs:19`;
`ring_config/Cargo.toml:11-12`; the `missing_docs` row of the workspace
`Cargo.toml` lints table — cited by row rather than by line, because the members
list above it grows.

**Out of Scope:** The other workspace lint this crate relies on, and the property
nothing enforces, are
[`non_functional_requirement/001`](../non_functional_requirement/001_a_record_that_allocates_nothing_and_is_read_once.md).
The `const` workaround is
[`workaround/001`](001_the_question_mark_forecloses_const.md).

---

## One Line, Two Families, One Table Entry

The census below is workspace-wide on purpose — the finding is about what the
workspace lints table could centralise, not about the ring family — and that
made it the one recipe in this crate's corpus that moved whenever an unrelated
crate was added anywhere in the workspace. It moved three times before the
recipe was rewritten to stop printing the denominator at all: the tree held 117
crates at the first census, all of them under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, and has more than
doubled since — spreading across five roots in the process.

So the denominator is deliberately absent from the output below. A count that
any concurrent session can change is not evidence about this crate, and quoting
it made the document stale by other people's work rather than by its own subject
changing.

That rule was then applied to only half the recipe, and the half left quoting a
moving number is the half this document reasons from. The escalating total was
read as fixed — the earlier text asserted it "was fifty-five at every census" —
while the recipe beside it printed 55, then 59, then 60. The ring share is what
actually held: 33 of 33, at every census. The non-ring share is what moved, 22 →
26 → 27, and it moved in one direction only. Both are printed below, separately,
because they are two different facts and only one of them is stable.

Then the failure inverted. Four relocations moved most of the workspace out from
under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, and a recipe rooted at `` went on printing a number
without printing an error: the escalating total it recorded fell to 57, and reads
37 today, while the prose above it still said sixty. The usual reading — prose
goes stale, the recorded output is fresh — had it backwards here. The prose was
written when the recipe searched a complete tree; the recording was captured
after the roots had quietly narrowed underneath it, and the prose is what stayed
right. The recipe below names all five roots, and strips them by name rather than
by field position, for the reason its own comment gives.

The third output line is stable and always has been: every crate the root
workspace contains inherits the table, escalating or not, and that has read "all
of them" at every census. The four it excludes are not opting out — they declare
their own `[workspace]`, so they are separate workspaces with no root table to
inherit from in the first place.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the workspace lints table says about it --'
command grep 'missing_docs' Cargo.toml   # unanchored: the members list above it grows
echo '  -- crates escalating it to deny by hand, split by family --'
# split three ways rather than two: the ring share is the one that has held at
# 33 of 33 across every census, and the growth is all in the second family
# the crate name is taken by stripping a named root prefix, never by cutting a
# fixed field.  A positional `cut -d/ -f2` reads `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/orbital/orbital_soi`
# as the family `orbital`, collapsing nineteen crates into one token that then
# fails a `^orbital_` match — so the family reports zero rather than nineteen,
# with no error anywhere to say a column moved.
esc=$( command grep -rl 'deny( missing_docs' --include=*.rs */src 2>/dev/null \
  | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##' | cut -d/ -f1 | sort -u )
printf '    escalating, total:  %s\n' "$( printf '%s\n' "$esc" | wc -l )"
printf '    of those, ring_*:   %s\n' "$( printf '%s\n' "$esc" | command grep -c '^ring_' )"
printf '    of those, orbital_*:%s\n' "$( printf '%s\n' "$esc" | command grep -c '^orbital_' )"
printf '    everything else:    %s\n' "$( printf '%s\n' "$esc" | command grep -vc '^ring_\|^orbital_' )"
echo '  -- and does every crate in the workspace inherit the table? --'
# the denominator is deliberately not printed.  It moves whenever any session
# adds a crate anywhere in the workspace, so quoting it would make this document
# stale by unrelated work; what the finding rests on is that the answer is "all",
# and that the escalating count does not move when the denominator does.
# the same glob list is written out three times on purpose.  Pairing an `ls` of
# `` with a recursive `grep -r ` does not compare
# like with like: the recursive side descends into `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/` and counts
# crates the `ls` side cannot see, so the two disagree by construction and the
# check reports a failure of its own making.
n=$(   ls                                 */Cargo.toml 2>/dev/null | wc -l )
own=$( command grep -l '^\[workspace\]'   */Cargo.toml 2>/dev/null | wc -l )
i=$(   command grep -l 'workspace = true' */Cargo.toml 2>/dev/null | wc -l )
[ "$(( n - own ))" = "$i" ] \
  && printf '    all of them, less %s declaring their own [workspace]\n' "$own" \
  || printf '    NO — only %s of %s\n' "$i" "$(( n - own ))"
echo '  -- and the flag order rustc receives, table first then the command line --'
touch ring_config/src/lib.rs
RUSTFLAGS=-Dmissing_docs cargo check -p ring_config -v 2>&1 | command grep 'Running.*ring_config' | command grep -o '\-\-[a-z]*=\?missing_docs\|-Dmissing_docs'
```

Live output:

```
  -- what the workspace lints table says about it --
missing_docs = "warn"
  -- crates escalating it to deny by hand, split by family --
    escalating, total:  60
    of those, ring_*:   33
    of those, orbital_*:19
    everything else:    8
  -- and does every crate in the workspace inherit the table? --
    all of them, less 4 declaring their own [workspace]
  -- and the flag order rustc receives, table first then the command line --
--warn=missing_docs
-Dmissing_docs
```

---

## What the Workspace Does With the Lint Denied

The flag order above is what makes the following check meaningful: the table's
`--warn=missing_docs` is passed first and the command line's `-Dmissing_docs`
second, so the deny is in force for every crate, including every one that does
not escalate on its own.

*The recording below is left exactly as captured on 2026-08-31, not re-run. Its
one quoted crate path predates the relocations — `rhi_vulkan` now sits at
`/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/render/rhi_vulkan` — and the absolute prefix is an artifact of the
machine it ran on rather than part of the finding. What it establishes is the
exit status, which depends on neither.*

```
RUSTFLAGS=-Dmissing_docs cargo check --workspace --all-features
```

```
    Checking rhi_vulkan v0.1.0 (/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox//home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/rhi_vulkan)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1m 02s
──── exit 0 · pid 1104119 · 2026-08-31 · 01:55:05 · elapsed 63s ────────────────
```

---

### RC51 — The Escalation Is Already Free, Measured, and Written Sixty Times Anyway

`#![ deny( missing_docs ) ]` is the only crate-level attribute `ring_config`
declares. It is not a local decision: sixty crates across the workspace carry the
same line — thirty-three of them the whole ring family, nineteen of them the
whole orbital family, eight of them everything else — and every crate the root
workspace contains, escalating or not, inherits `missing_docs = "warn"` from the
workspace lints table with `[lints] workspace = true`.

The obvious question is what one word in the table would cost. Denying the lint
workspace-wide would hold every non-escalating crate to a standard it never
opted into, which is the reason to expect breakage. The check
says there is none: the whole workspace compiles clean under `-Dmissing_docs`,
all features, in about a minute.

**Finding.** So the change is free today, and the sixty lines are currently
buying nothing the table could not buy in one place. That is worth recording
precisely because the answer was not obvious in advance — the non-escalating
crates could have contained a single undocumented public item between them, and
did not.

The re-measurement changes the shape of it, and in the direction opposite to the
one this document previously recorded. The earlier reading was that the
escalating count was fixed at fifty-five while the workspace grew, and therefore
that the hand-written line was a convention new work does not adopt. The census
above refutes that from the same recipe: the count has gone 55 → 59 → 60, and
the growth is not scattered. Two families are at 100% — 33 of 33 ring crates and
19 of 19 orbital crates — and everything else amounts to eight, out of a
remainder in the low hundreds this document does not pin, for the same reason it
stopped printing the denominator.

So the line is adopted, but by the family rather than by the crate. A family
either writes it in every one of its crates or in none of them, which is what a
copied crate template produces and is not what a workspace-wide convention looks
like. Both readings agree the single table word would delete every one of the
sixty lines; they disagree about what the lines mean, and the second reading is
the one the numbers support.

The measurement has a shelf life, which is the interesting part. It is true of the
workspace as it stands and nothing preserves it: a new crate added tomorrow
without the hand-written line gets `warn`, ships an undocumented public item, and
the table entry that would have caught it is still the wrong word. The
sixty-line workaround does not protect that new crate either, since the
protection is exactly the line the new crate did not write — and on the family
reading, whether it gets that protection is decided by which template it was
copied from rather than by anything about the crate.

---

### RC52 — Two Documentation Policies With Nothing Recording Which Crate Has Which

Sixty crates are held to `deny` and every other crate the root workspace
contains to `warn`, and the split is recorded nowhere but in the presence or
absence of one line at the top of each `src/lib.rs`.

Nothing says the split is intentional. All thirty-three ring crates escalate,
which looks like a family convention, and the ring family is where this corpus
lives — and RC51's census makes the same true of all nineteen orbital crates,
which is the stronger version of the same evidence: two families at 100% and no
third family anywhere near it. But the convention is not written in any
rulebook, readme or task file that the crates cite, so the evidence for it is
the sixty lines themselves.

**Finding.** There is a real argument for the per-crate line, and it is not made
anywhere. A line at the top of the file is visible to whoever is editing that
file, in a way a table entry three directory levels up is not, and it survives a
crate being lifted out of the workspace into its own repository — where
`[lints] workspace = true` has nothing to resolve against. Either reason would
justify keeping all sixty and changing nothing.

What cannot be justified is the current state, in which the reason is unrecorded
and the two policies are indistinguishable from an accident of who copied which
template. A reader of `ring_config/src/lib.rs:19` cannot tell whether the line is
a deliberate local escalation, a family convention, or boilerplate — and the
measurement above shows that, whichever it is, it is not currently doing any work
the table is not already positioned to do.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](001_the_question_mark_forecloses_const.md) | The crate's other workaround, and the one with an expected end date |
| [`non_functional_requirement/001`](../non_functional_requirement/001_a_record_that_allocates_nothing_and_is_read_once.md) | The workspace lint that is enforced centrally, and the property that is not enforced at all |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The documented surface this lint guards |

### Sources

| Fact | Where |
|------|-------|
| The crate's one crate-level attribute | `ring_config/src/lib.rs:19` |
| Inheriting the workspace lints table | `ring_config/Cargo.toml:11-12` |
| `missing_docs = "warn"` centrally | `Cargo.toml`, the `missing_docs` row of `[workspace.lints.rust]` — by row, per § In Scope |
| Every crate the root workspace contains inheriting the table, less the four declaring their own; 60 escalating by hand — 33 ring, 19 orbital, 8 other | Census above |
| Table flag first, command-line flag second | Census above, `cargo check -v` |
| The workspace compiling clean under `-Dmissing_docs` | Check above, exit 0 |

### Tests

| Test | Covers |
|------|--------|
| — | The attribute is a build-time lint level; no test observes it |
