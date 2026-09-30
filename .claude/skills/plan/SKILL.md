---
name: plan
description: Write an implementation plan for a ring change, or a self-contained prompt to hand the work to another agent.
disable-model-invocation: true
argument-hint: "[task] [--prompt]"
---

# Plan

Default output is a plan for a human. With `--prompt`, write the agent-prompt form instead.

## Before writing

ELIMINATE → SIMPLIFY → REUSE → CREATE: can the need go away, can it be solved with less, does a
crate or verb already do it? Only then plan something new. Read the touched crates' `readme.md`
and `docs/readme.md`; check `docs/capstone/` for topic ownership.

## Plan

- **Goal** — one sentence; in scope / out of scope.
- **Context** — crates and tiers involved, the invariants at stake (`docs/invariant/`), the
  capstone topic if any.
- **Current state** — files, entry points, behaviour that will change. Verified, with paths.
- **Steps** — ordered, each a concrete action and its observable outcome.
- **Evidence** — the test or loom model that fails before and passes after; the verb level to
  run (`verify` skill); doc-corpus instances to update.
- **Done** — `./verb/test level::3` green (`level::5` if gates, lints, `unsafe` or docs moved);
  a new check shown red on a planted violation.
- **Risks / open questions** — what would change the plan.

## Agent prompt

- **Role and context** — the repository, `AGENTS.md` rules that apply, the crates in scope.
- **Task** — what to change and what not to touch.
- **Inputs** — files, docs, commands.
- **Output** — the diff, the verification output, a short report of what was skipped.
- **Quality bar** — the checks that must pass and the evidence to paste.
