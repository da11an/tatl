# Plan 51: Mobile App Architecture (Android/iOS)

## Overview

This plan details the architecture, UX design, and implementation approach for bringing TATL to mobile platforms. It builds on the feasibility assessment in Plan 46 with concrete technical decisions and work estimates.

## Executive Summary

**Recommended approach:** Rust shared core + native UI (SwiftUI for iOS, Jetpack Compose for Android) with local-first SQLite and optional cloud sync via a lightweight backend.

**Estimated total effort:** 24-32 weeks for single platform MVP, 36-48 weeks for both platforms with sync.

---

## Part 1: Architecture

### 1.1 Recommended Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      Mobile UI Layer                        │
│  ┌─────────────────────┐     ┌─────────────────────┐       │
│  │   iOS (SwiftUI)     │     │ Android (Compose)   │       │
│  └──────────┬──────────┘     └──────────┬──────────┘       │
│             │                           │                   │
│             ▼                           ▼                   │
│  ┌──────────────────────────────────────────────────┐      │
│  │            UniFFI Bindings (Generated)           │      │
│  └──────────────────────┬───────────────────────────┘      │
└─────────────────────────┼───────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────────┐
│                    Rust Core Library                        │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐ ┌──────────┐ │
│  │   Models   │ │    Repo    │ │   Filter   │ │ Respawn  │ │
│  └────────────┘ └────────────┘ └────────────┘ └──────────┘ │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐              │
│  │    Sync    │ │ Migrations │ │   Utils    │              │
│  └────────────┘ └────────────┘ └────────────┘              │
│                         │                                   │
│                         ▼                                   │
│              ┌─────────────────────┐                       │
│              │   SQLite (rusqlite) │                       │
│              └─────────────────────┘                       │
└─────────────────────────────────────────────────────────────┘
                          │
                          │ (optional sync)
                          ▼
┌─────────────────────────────────────────────────────────────┐
│                    Sync Backend (Optional)                  │
│  ┌────────────┐ ┌────────────┐ ┌────────────┐              │
│  │  Auth API  │ │  Sync API  │ │  Storage   │              │
│  └────────────┘ └────────────┘ └────────────┘              │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 Why This Stack?

| Decision | Rationale |
|----------|-----------|
| **Rust core** | Reuse existing battle-tested logic (models, repos, filters, respawn). Single source of truth for business rules. |
| **UniFFI** | Mozilla's tool generates Swift/Kotlin bindings from Rust. Mature, well-documented, handles async. |
| **Native UI** | Platform-native feel critical for productivity apps. SwiftUI/Compose are modern, declarative, and fast to iterate. |
| **Local-first SQLite** | Matches CLI architecture. Works offline. No mandatory server dependency. |
| **Optional sync** | Users can opt into cloud sync. Not required for basic functionality. |

### 1.3 Core Library Extraction

The existing CLI already has good separation. Extract these modules into `tatl-core`:

```
tatl-core/
├── src/
│   ├── models/          # Task, Session, Project, etc.
│   ├── repo/            # Database access layer
│   ├── filter/          # Parser + evaluator
│   ├── respawn/         # Respawn rule engine
│   ├── db/              # Migrations, connection
│   ├── utils/           # Date parsing, fuzzy matching
│   └── sync/            # NEW: Sync engine
├── Cargo.toml
└── uniffi.toml          # UniFFI configuration
```

**What stays in CLI:** `cli/` module (clap, terminal output, pipe operator)

**What's shared:** Everything else

### 1.4 Data Synchronization Strategy

**Approach: CRDT-inspired last-writer-wins with vector clocks**

Each record gets:
- `uuid`: Immutable unique identifier
- `version`: Incrementing version number
- `modified_ts`: Timestamp of last modification
- `device_id`: Which device made the change

**Conflict resolution rules:**
1. Higher version wins
2. If versions equal, later `modified_ts` wins
3. If timestamps equal, lexicographically higher `device_id` wins (deterministic tiebreaker)

**Sync flow:**
1. Device pushes local changes since last sync
2. Server returns remote changes since last sync
3. Device applies remote changes (conflict resolution as needed)
4. Device confirms receipt

**Soft deletes:** Tasks are never hard-deleted during sync. `status=deleted` propagates. Hard delete only after all devices confirm.

---

## Part 2: Mobile UX Design

### 2.1 Core Screens

**1. Dashboard (Home)**
```
┌────────────────────────────────┐
│ TATL                    [⚙️]   │
├────────────────────────────────┤
│ ▶ Task 5: Fix auth bug         │
│   ⏱ 1h 23m                     │
│   [Stop] [Annotate]            │
├────────────────────────────────┤
│ Queue (3)                      │
│ ┌────────────────────────────┐ │
│ │ 1. Review PR #42           │ │
│ │ 2. Update docs             │ │
│ │ 3. Deploy staging          │ │
│ └────────────────────────────┘ │
├────────────────────────────────┤
│ Today: 4h 15m across 3 tasks   │
└────────────────────────────────┘
│  [Home]  [Tasks]  [+]  [Time]  │
└────────────────────────────────┘
```

**2. Task List**
```
┌────────────────────────────────┐
│ Tasks              [Filter] 🔍 │
├────────────────────────────────┤
│ ┌────────────────────────────┐ │
│ │ ● Fix auth bug       work  │ │
│ │   Due: Today    ▶ 1h 23m   │ │
│ ├────────────────────────────┤ │
│ │ ○ Review PR #42      work  │ │
│ │   Due: Tomorrow            │ │
│ ├────────────────────────────┤ │
│ │ ○ Update docs        docs  │ │
│ │   Planned                  │ │
│ └────────────────────────────┘ │
│                                │
│ [Group: Project ▼]  [Sort ▼]   │
└────────────────────────────────┘
```

**3. Task Detail**
```
┌────────────────────────────────┐
│ ← Task #5                [Edit]│
├────────────────────────────────┤
│ Fix the authentication bug     │
│                                │
│ Project: work                  │
│ Tags: +urgent +backend         │
│ Due: 2026-02-05                │
│ Stage: active                  │
│ Timer: 1h 23m / 2h (69%)       │
│ ████████████░░░░░░             │
├────────────────────────────────┤
│ Annotations                    │
│ • Found issue in middleware    │
│   Feb 3, 14:23                 │
│ • Token refresh not called     │
│   Feb 3, 15:45                 │
├────────────────────────────────┤
│ Sessions (3 today)             │
│ • 09:00-10:30 (1h 30m)         │
│ • 11:00-11:53 (53m)            │
├────────────────────────────────┤
│ Children                       │
│ • #12 Write tests              │
│ • #13 Update docs              │
├────────────────────────────────┤
│ [Enqueue] [Start] [Close]      │
└────────────────────────────────┘
```

**4. Quick Add (Sheet)**
```
┌────────────────────────────────┐
│ New Task              [Cancel] │
├────────────────────────────────┤
│ ┌────────────────────────────┐ │
│ │ Description                │ │
│ └────────────────────────────┘ │
│                                │
│ Project: [work        ▼]       │
│ Tags:    [+urgent] [+tag] [+]  │
│ Due:     [None      ▼]         │
│ Parent:  [None      ▼]         │
│                                │
│ ☐ Add to queue                 │
│ ☐ Start timing immediately     │
│                                │
│ [Create Task]                  │
└────────────────────────────────┘
```

**5. Time View**
```
┌────────────────────────────────┐
│ Time                   [Week ▼]│
├────────────────────────────────┤
│        Mon Tue Wed Thu Fri     │
│ work   2h  3h  1h  4h  2h      │
│ docs   1h  0   2h  0   1h      │
│ ──────────────────────────     │
│ Total  3h  3h  3h  4h  3h  16h │
├────────────────────────────────┤
│ Today's Sessions               │
│ • 09:00-10:30 Fix auth    work │
│ • 11:00-11:53 Fix auth    work │
│ • 14:00-now   Review PR   work │
└────────────────────────────────┘
```

**6. Settings**
- Account & Sync
- Default project
- Notification preferences
- Theme (light/dark/system)
- Data export

### 2.2 Gestures & Interactions

| Action | Gesture |
|--------|---------|
| Start timing | Tap task → "Start" button, or long-press → quick action |
| Stop timing | Tap active task banner, or swipe down on timer |
| Add to queue | Swipe right on task |
| Remove from queue | Swipe left on queued task |
| Close task | Swipe left → "Close" action |
| Reorder queue | Drag handle in queue view |
| Quick add | FAB (+) button → sheet |
| Filter | Search bar with smart parsing (`project:work +urgent`) |

### 2.3 Notifications

- **Timer running reminder:** Optional ping after N hours
- **Task due soon:** Configurable (1h, 1d before)
- **Daily summary:** Optional morning/evening digest

### 2.4 Widgets

**iOS Widgets:**
- Small: Current task + timer
- Medium: Queue (top 3) + active timer
- Large: Today's time breakdown

**Android Widgets:**
- Similar, plus quick-action buttons (start/stop)

---

## Part 3: Implementation Phases

### Phase 1: Core Library Extraction (4-6 weeks)

**Goals:**
- Extract `tatl-core` crate from CLI
- Add UniFFI annotations
- Generate and test Swift/Kotlin bindings
- CLI continues to work unchanged

**Tasks:**
1. Create `tatl-core` workspace member
2. Move models, repo, filter, respawn, db, utils
3. Add `#[uniffi::export]` to public API
4. Build UniFFI scaffolding
5. Write integration tests for bindings
6. Update CLI to depend on `tatl-core`

**Deliverable:** `tatl-core` crate with working UniFFI bindings

### Phase 2: iOS MVP (8-10 weeks)

**Goals:**
- Native iOS app with core functionality
- Local SQLite (no sync yet)
- App Store ready

**Tasks:**
1. Xcode project setup with Swift Package Manager
2. Integrate `tatl-core` via XCFramework
3. Build UI screens (Dashboard, Task List, Task Detail, Quick Add, Time)
4. Implement state management (SwiftUI + ObservableObject)
5. Add Spotlight search integration
6. Add widgets (small, medium)
7. iOS-specific polish (haptics, animations)
8. TestFlight beta
9. App Store submission

**Deliverable:** iOS app v1.0 on App Store

### Phase 3: Android MVP (8-10 weeks)

**Goals:**
- Native Android app with feature parity to iOS
- Local SQLite (no sync yet)
- Play Store ready

**Tasks:**
1. Android Studio project setup with Gradle
2. Integrate `tatl-core` via JNI/JNA
3. Build UI screens (Compose)
4. Implement state management (ViewModel + StateFlow)
5. Add app shortcuts
6. Add widgets
7. Android-specific polish (Material 3, predictive back)
8. Internal testing track
9. Play Store submission

**Deliverable:** Android app v1.0 on Play Store

### Phase 4: Sync Backend (6-8 weeks)

**Goals:**
- Optional cloud sync between devices
- User accounts with secure auth
- Conflict resolution

**Tasks:**
1. Design sync API (REST or gRPC)
2. Implement auth (OAuth2 / passkeys)
3. Build sync endpoints (push, pull, ack)
4. Implement conflict resolution
5. Add sync engine to `tatl-core`
6. Update mobile apps with sync UI
7. Deploy backend (serverless or container)
8. Add CLI sync support

**Deliverable:** Working cross-device sync

### Phase 5: Polish & Launch (4-6 weeks)

**Goals:**
- Production-ready quality
- Marketing assets
- Documentation

**Tasks:**
1. Performance optimization
2. Accessibility audit (VoiceOver, TalkBack)
3. Localization (if desired)
4. App Store screenshots and descriptions
5. Landing page
6. User documentation
7. Monitoring and crash reporting

**Deliverable:** Public launch

---

## Part 4: Effort Estimates

### Summary Table

| Phase | Duration | Effort (person-weeks) |
|-------|----------|----------------------|
| 1. Core extraction | 4-6 weeks | 4-6 |
| 2. iOS MVP | 8-10 weeks | 8-10 |
| 3. Android MVP | 8-10 weeks | 8-10 |
| 4. Sync backend | 6-8 weeks | 6-8 |
| 5. Polish & launch | 4-6 weeks | 4-6 |
| **Total (sequential)** | **30-40 weeks** | **30-40** |
| **Total (iOS+Android parallel)** | **22-30 weeks** | **30-40** |

### Breakdown by Category

| Category | Weeks |
|----------|-------|
| Rust/FFI work | 6-8 |
| iOS native development | 10-14 |
| Android native development | 10-14 |
| Backend/infrastructure | 6-8 |
| QA/polish/release | 6-8 |

### Resource Options

**Option A: Solo developer**
- Timeline: 8-10 months
- Cost: Lower
- Risk: Bus factor, slower iteration

**Option B: Small team (2-3)**
- Timeline: 4-6 months
- One person on Rust/backend, others on mobile
- Can parallelize iOS and Android

**Option C: Larger team (4-5)**
- Timeline: 3-4 months
- Dedicated iOS, Android, backend, plus shared Rust
- Faster but coordination overhead

---

## Part 5: Technical Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| UniFFI complexity | Medium | Start with minimal API surface. Expand incrementally. |
| SQLite threading on mobile | Medium | Use WAL mode. Single writer pattern. Async access from UI. |
| Sync conflicts | High | Conservative merge strategy. User-visible conflict UI as fallback. |
| App Store rejection | Medium | Follow guidelines strictly. Plan for iteration. |
| Performance on old devices | Medium | Profile early. Lazy loading. Pagination. |
| Schema drift between CLI/mobile | High | Shared migrations in `tatl-core`. Version compatibility checks. |

---

## Part 6: Open Questions

1. **Sync pricing model?** Free tier with limits? Paid for sync? Self-host option?
2. **Offline-first priority?** How long can mobile operate without sync?
3. **CLI feature parity?** Which power features (respawn rules, complex filters) are mobile-essential?
4. **Tablet/iPad support?** Optimize for larger screens?
5. **Watch apps?** Apple Watch / Wear OS for quick timer control?
6. **Shortcuts/Siri integration?** Voice commands for common actions?

---

## Part 7: Recommended Next Steps

1. **Decide on sync strategy** — Required for MVP, or post-launch?
2. **Create `tatl-core` crate** — Low-risk first step, validates FFI approach
3. **Build iOS prototype** — 2-week spike to validate UniFFI + SwiftUI integration
4. **Define mobile MVP scope** — Which features are essential vs. nice-to-have?
5. **Estimate infrastructure costs** — If sync is in scope, what's the monthly burn?

---

## Appendix: Comparable Apps

| App | Approach | Notes |
|-----|----------|-------|
| Todoist | Native apps + REST API | Full sync, subscription model |
| Things 3 | Native iOS/macOS, no Android | Local + iCloud sync |
| Taskwarrior | CLI-only, community mobile forks | Data sync via taskserver |
| Obsidian | Electron + native mobile | Local-first, optional sync |
| Linear | Web + native mobile | API-first, requires account |

TATL's closest analog is **Things 3** (local-first, native UI, optional sync) or **Taskwarrior** (CLI-first with mobile as secondary).
