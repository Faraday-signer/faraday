# Update log — `docs/updates/`

**This folder is the knowledge-base update log.** Every meaningful repo change gets recorded here so the docs never lag the code.

## Why one file per entry

Everyone — humans and agents, across many branches — records changes here. Appending to one shared file would make every PR collide at the top of the log. So:

> **One change = one new file.** Two PRs never touch the same file, so update-log conflicts become impossible.

## The convention

- **Filename:** `YYYY-MM-DD-NN-slug.md` — `NN` is a 2-digit sequence within the day (next number above the highest existing for that date). Ascending filename sort = chronological order.
- **Contents:** what changed and why, the files/cards touched, and how it was verified (tests/CI/human check).
- **When:** after any meaningful repo change. A change that isn't recorded didn't happen, as far as the next person or agent knows.
