# Architecture Decisions

## AD-0001: Agent Framework Is The First Artifact

Date: 2026-05-13

Status: Accepted

Context: The project will be developed through repeated AI-assisted sessions. Work needs durable instructions, current state, decision records, knowledge, checklists, and playbooks before product code exists.

Decision: Create `AGENTS.md` and `.agents/` as the first committed artifact. Do not create Rust workspace, CI, license, product README, or business code until this framework is committed.

Consequences: Future sessions can resume from repository files instead of chat history. Product work starts with clear boundaries, review gates, and reference hygiene rules.

Review Date: 2026-06-13
