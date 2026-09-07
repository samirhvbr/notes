# Decisions (ADR log)

> **Status:** `ACTIVE` · The single, chronological record of decisions taken in
> this repository. Format: Architecture Decision Record.

An ADR records a decision **and the reason it was taken**, so that the next
session does not re-litigate it. A how-to does not argue direction — it links
the ADR.

Numbering is sequential and never reused. A superseded ADR is not deleted: its
status changes to `SUPERSEDED` and it names the ADR that replaced it.

The template for an entry:

---

## ADR-001 — <the decision, as a sentence>

**Status:** `ACCEPTED` · <DD/MM/YYYY>

**Context.** What was true before, and what forced a choice. Facts and
measurements, not opinions — if a number decided it, put the number here.

**Decision.** What was decided, in the imperative. One decision per ADR.

**Consequences.** What this makes easy, what it makes hard, and what has to
change elsewhere because of it. Name the cost — an ADR that lists only benefits
is a decision that has not been thought through.

---

<!-- Delete the example above once ADR-001 is real. Candidates for the first
     ADRs of a new project: the stack and why; the branch and release regime;
     what the version slots mean here; where secrets live; what the agent is and
     is not allowed to run. -->
