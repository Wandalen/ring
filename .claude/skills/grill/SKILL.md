---
name: grill
description: Stress-test a plan, design or idea by interviewing the user in rounds until every decision is settled. Use when asked to grill, poke holes in, or challenge a design.
disable-model-invocation: true
argument-hint: "[plan or design]"
---

# Grill

Interview the user until you share one understanding. Map it as a **design tree**: each
decision branches into the decisions that depend on it.

Work in **rounds**. The **frontier** is every decision whose prerequisites are settled — the
questions answerable now without guessing. Ask the whole frontier at once, numbered, each with
your recommended answer, then wait.

```
❓ **Q1** — **<title>**: <question, options if any>

➡️ <recommended answer and why>

---

❓ **Q2** — …
```

Each round's answers move the frontier. A question that depends on another open question in
the same round belongs to a later round.

Facts are your job: look them up in the code, the crates' `docs/`, the gates — never ask the
user for something you can read. Decisions are the user's: put each one to them.

Done when the frontier is empty and nothing is silently assumed. Do not act on the result until
the user confirms.
