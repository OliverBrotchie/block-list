# Domain docs

How the engineering skills read this repo's domain documentation before exploring the code.

## Read before exploring

At the repo root, read `GLOSSARY.md`. If `GLOSSARY-MAP.md` exists instead, it points at one `GLOSSARY.md` per context, and each context relevant to the topic gets read.

Read the ADRs in `docs/adr/` that touch the area about to change. In a multi-context repo, also check `src/<context>/docs/adr/`.

If any of these files are missing, continue without comment. Do not flag the absence and do not suggest creating them. `/domain-modeling` creates them lazily when a term or a decision is actually resolved.

## File structure

This repo is single-context:

```
/
├── GLOSSARY.md
├── docs/adr/
│   ├── 0001-example-decision.md
│   └── 0002-another-decision.md
└── src/
```

A multi-context repo adds `GLOSSARY-MAP.md` at the root, a `GLOSSARY.md` plus `docs/adr/` per context under `src/`, and keeps system-wide decisions in `docs/adr/`.

## Use the glossary's vocabulary

When output names a domain concept, in an issue title, a refactor proposal, a hypothesis, or a test name, use the term as `GLOSSARY.md` defines it. Do not drift to a synonym the glossary avoids.

A concept missing from the glossary means one of two things. Either the project does not use that language, so reconsider it, or there is a real gap, which is noted for `/domain-modeling`.

## Flag ADR conflicts

When output contradicts an existing ADR, say so instead of silently overriding it:

> This contradicts ADR-0007 (event-sourced orders), but it is worth reopening because...
