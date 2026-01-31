# Task Management State Model – Design Overview

## Purpose

This document describes the task-management state model agreed upon in recent design discussions. The goal is to provide:

* **True orthogonality at the data-model level** (independent stored facts)
* **A single, one-dimensional task classification** for user-facing views
* **Clear invariants** that prevent invalid or confusing states
* **Predictable command semantics** (`send`, `recall`, `on`, `off`, etc.)

This document is intended for engineering review and implementation alignment.

---

## Design Principles

1. **Orthogonality first**
   Independent concerns are stored independently. No stored column encodes a concept that can be derived from another table.

2. **Derived classification**
   User-facing task “states” (Proposed, Planned, External, etc.) are *computed*, not stored.
   Note: the derived classification mapping should be user configurable with a good default.

3. **Exclusive external waiting**
   “External” represents an intentional handoff where progress primarily depends on another party.

4. **Queue integrity**
   The queue represents the internal line of work. External tasks do not retain a place in line except while actively worked.

---

## Orthogonal State Groups (Source of Truth)

### 1. Lifecycle State (stored)

Represents whether the task is still alive.

* `open`
* `closed`
* `cancelled`

**Notes**:

* `closed` and `cancelled` are terminal.
* Lifecycle is independent of planning, timing, or externality.
* Task deletion is not a lifecycle state, it is a ledger correction.

---

### 2. Planning / Queue State (stored)

Represents whether the task is in the internal execution line.

* `queue_position IS NULL` → not in queue
* `queue_position IS NOT NULL` → in queue

**Notes**:

* Queue position implies *eligibility* for work, not that work is occurring.
* Queue ordering is meaningful and must be preserved
* A user is can manipulate the queue by enqueueing an already queued task to bump it to the end, but verbage here is
  intentionally sparse -- not a focal point.

---

### 3. Work History (derived)

Represents whether any work has ever occurred.

* **Initiated**: at least one work session exists
* **Pending**: no work sessions exist

Derived from:

* existence of rows in `work_sessions` for the task

---

### 4. Timer / Active Work (derived)

Represents whether the task is actively being worked *right now*.

* **Timer On**: one open work session (`ended_at IS NULL`)
* **Timer Off**: no open session

Derived from:

* `work_sessions` table

**Global invariant**:

* At most one open work session may exist globally (or per user).

---

### 5. External Waiting (stored)

Represents an intentional handoff to another party (review, approval, input, etc.).

Stored in an `external` table:

* `task_id`
* `recipient`
* `note`
* `sent_at`
* `status ∈ {waiting}`

Presence of a row with `status=waiting` means:

* The task is *exclusively waiting on another party*
* The task must remain visible for follow-up

**Non-goals**:

* This does *not* model collaboration or parallel contribution.
* Let's not overcomplicate this.

---

### 6. Scoping (derived)

Represents whether a task has been assigned to a project.

* Scoped if associated with a project
* Independent if unassociated

---

## Key Invariants

The following invariants must be enforced via constraints and/or triggers:

1. **Single active task**
   At most one work session may have `ended_at IS NULL`.

2. **Active work implies queued**
   If timer is on, the task must be in the queue.

3. **External waiting does not keep a queue position**
   If `external_status=waiting` and timer is off, then `queue_position IS NULL`.

4. **External tasks may be worked temporarily**
   While timer is on, an external task is temporarily placed at the front of the queue.

5. **Terminal lifecycle cleanup**
   If lifecycle is `closed` or `cancelled`:

   * task must not be queued
   * task must not be external waiting

---

## One-Dimensional Task Classification (Derived)

A single user-facing classification is computed as a function of the orthogonal facts.

### Classification Precedence (highest wins)

1. `lifecycle = closed` → **Completed**
2. `lifecycle = cancelled` → **Cancelled**
3. `timer_on = true` → **Active**
4. `external_status = waiting` → **External**
5. Internal open states (below)

### Internal Open State Mapping

| Queue | Work History | Classification |
| ----- | ------------ | -------------- |
| No    | No           | Proposed       |
| Yes   | No           | Planned        |
| Yes   | Yes          | In progress    |
| No    | Yes          | Suspended      |

This kind of table should be user configurable. Could optionally extend by mapping in scoping,
allowing the user to split Proposed into Idea/Proposed, etc. The idea is to provide a durable,
orthogonal structure that is safely customizable classification options for the user. While the
minimal Internal Open State Mapping table demonstrates the idea, mixing the 1-5 priority items above
into the mapping table would give the user a large degree of freedom in how their tasks would be
classified and organized.

### Default External Mapping (representable in an extended config table)

* Any open task with `external_status=waiting` and timer off → **External**
* During active follow-up work, classification temporarily becomes **Active**
* When work stops, classification returns to **External** until collected

---

## Command Semantics

### `send task_id recipient [note]`

* Set `external_status=waiting`
* Record recipient and note
* Remove task from queue (`queue_position=NULL`)

### `recall task_id`

* Clear `external_status`
* Insert task into queue (default: bottom unless specified)

### `on [task_id]`

* Start timing specified task (or queue head if omitted)
* Ensure task is queued at position 1
* Allowed even if task is external waiting

### `off`

* Stop the active work session
* If task is external waiting, automatically remove it from the queue

---

## Rationale Summary

* Orthogonal storage enables strong invariants and simple reasoning.
* External is modeled as an *exclusive waiting lane*, not a collaboration tag.
* Queue position is reserved for internal execution priority.
* Classification is deterministic, inspectable, and reversible.

This model is intentionally strict: invalid combinations are structurally prevented rather than tolerated and explained later.

---

## Open Implementation Decisions

* Default queue position on `recall` is top (position 0)
* Whether to store `external.status=done` vs deleting the row on collect doesn't matter (low impact)

These do not affect the core model and can be resolved during implementation.
