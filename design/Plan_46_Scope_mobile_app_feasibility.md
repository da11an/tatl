# Plan 46: Mobile App Feasibility (Android / iOS)

## Context (from repo)
- The app is a Rust CLI (tatl) targeting Linux/Windows/macOS.
- Data store is local SQLite (via rusqlite) with a single-user local DB.
- There is no existing mobile UI, mobile SDK, or sync service.

## Feasibility Summary
Adding mobile is feasible, but it is not a “new platform build” of the existing app. It is a new product surface that requires:
- A mobile UI and interaction model for all key workflows.
- A data access strategy for mobile (local DB + sync, or remote API).
- A compatibility contract for CLI and mobile to share data safely.

## Core Decision: Data Architecture
Mobile cannot safely share the same local SQLite file as the desktop CLI across devices, so choose one:

### Option A: Local-first + Sync Service (recommended for cross-device use)
- Keep SQLite locally on each device; add a sync layer (server + conflict resolution).
- Requires defining a sync protocol and migration strategy.
- Highest effort, best UX for multi-device.

### Option B: Remote-first API
- CLI and mobile both use a shared backend API; local storage optional.
- Requires building a backend, auth, and offline strategy.
- Simpler consistency, more infrastructure and ops.

### Option C: Mobile-only local DB (no sync)
- Mobile is standalone and does not sync with desktop.
- Lower effort but limited usefulness unless clearly positioned as a separate workflow.

## Platform Implementation Options
Because the core is a CLI, “reusing” the existing code depends on approach:

### 1) Native mobile app (Swift/SwiftUI, Kotlin/Compose)
- Fastest to align with platform UX and system integrations.
- Core business logic must be ported or reimplemented.

### 2) Rust shared core + native UI
- Extract domain logic into a Rust crate and expose via FFI.
- Higher initial complexity, better long-term parity.

### 3) Cross-platform UI (Flutter/React Native)
- UI layer shared, but Rust core still needs bridging.
- Saves UI work, still significant integration effort.

## Approximate Work (Rough Order of Magnitude)
Assumes a mobile app with feature parity for core flows (tasks, queue, time tracking, filters) and a sync strategy.

### 1) Product + UX Definition
- Define minimal viable workflows for mobile
- Map CLI commands to mobile interactions
Estimated: 2–4 weeks

### 2) Data Architecture
- Pick Option A/B/C
- Define schema compatibility, migrations, conflict rules
Estimated: 4–8 weeks (Option A/B), 1–2 weeks (Option C)

### 3) Mobile App Build (per platform)
- App shell, navigation, core views
- Task creation/editing, queue, sessions
- Search/filter, list views
Estimated: 8–14 weeks per platform

### 4) Sync/Backend (if A or B)
- API design, auth, infra
- Sync engine + conflict resolution
- Observability + backups
Estimated: 8–16 weeks

### 5) QA + Release
- Device testing matrix, beta, store releases
Estimated: 3–5 weeks

### Total Rough Estimates
- **Option C (no sync, single platform):** ~12–20 weeks
- **Option A/B (sync + single platform):** ~22–40+ weeks
- **Both platforms:** add ~60–80% more effort depending on shared layers

## Key Dependencies
- Clear definition of mobile MVP vs parity
- Decision on data architecture (sync vs API vs local-only)
- Auth and user identity model (currently none in CLI)
- Migration strategy for existing CLI users

## Major Risks
- Sync correctness and conflict resolution complexity
- Feature parity pressure with a CLI that supports many power features
- UX mismatch: CLI workflows may not translate well to touch UI
- Schema evolution and backward compatibility with existing CLI data

## Open Questions
- Is mobile intended for the same single-user local workflow, or multi-device sync?
- Do we need full parity or a focused mobile subset?
- Are we willing to run backend infrastructure?
- Should the Rust core be shared, or reimplemented per platform?

## Suggested Next Steps
- Define a 5–7 screen mobile MVP (core flows only)
- Decide on data architecture (Option A/B/C)
- Draft a mobile data model + sync contract if needed
