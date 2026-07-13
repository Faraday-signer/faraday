---
date: 2026-07-14
slug: scroll-world-master
---

# scroll-world-master agent

Added `.claude/agents/scroll-world-master.md` (Opus) ahead of the landing-page work (grant-push milestone: the site needs care, FA-05).

It masters the **scroll-world** technique (`github.com/cth9191/scroll-world` — the hardened fork): landing pages where scroll scrubs a pre-rendered camera flight through AI-generated scenes with frame-identical seams (no cuts). The agent executes from the upstream skill as its playbook (clones it fresh; `SKILL.md` + `references/` are source of truth) and carries the distilled doctrine: actual-frame seam handoffs + SSIM ≥0.95 gate, architecture A (continuous forward take) vs B (dive + connectors), the frame-locking model roster (`seedance_2_0` / `kling3_0` / `seedance_2_0_mini` previz), blob-URL scrub encoding (`-g 8`, posters from encoded clips), and mobile tiering by screen short side.

Faraday-specific grounding baked in: lands in `site/` as a client component around the vanilla engine, dark theme via the engine's `@layer sw` tokens, proposed journey from the real product story (workbench → device → entropy → light channel → hero), copy in the locked disciplined brand voice, and art direction deliberately NOT the default clay diorama.

Hard gate: Higgsfield generations cost real credits — the agent stops and returns an interview + spend estimate unless its invocation carries explicit approval; anchor and previz gates also stop-and-report (subagents can't ask mid-run).

Verified: agent definition only; distilled against the skill source cloned 2026-07-14 (SKILL.md 603 lines + references).
