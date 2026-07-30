<!--
Sync Impact Report
- Version change: 1.0.0 -> 1.1.0
- Modified principles: none
- Materially expanded guidance:
  - Development Workflow and Quality Gates: added an 80% test coverage gate
- Added sections: none
- Removed sections: none
- Templates:
  - ✅ updated: .specify/templates/plan-template.md
  - ✅ updated: .specify/templates/spec-template.md
  - ✅ updated: .specify/templates/tasks-template.md
  - ✅ no command templates present: .specify/templates/commands/
- Runtime guidance:
  - ✅ compatible; no change required: README.md
- Deferred items: none
-->

# periScope Constitution

## Core Principles

### I. Dead Simple by Default

Every change MUST use the simplest design that completely solves the current,
demonstrated need. Code paths, APIs, data flow, and control flow MUST be easy to
follow without hidden behavior or unnecessary indirection. Speculative features,
premature abstractions, and infrastructure without an immediate requirement MUST
NOT be added. Any additional complexity requires a documented reason and proof
that a simpler approach is insufficient.

### II. Performance First

Performance is a design constraint, not a final optimization pass. Feature specs
MUST define measurable performance expectations for affected user-visible or hot
paths. Plans MUST identify relevant latency, CPU, memory, startup, redraw, and
idle-cost impacts. Performance-sensitive changes MUST be measured against a
representative baseline, and regressions MUST be fixed or explicitly justified
before merge. Correctness remains non-negotiable; fast incorrect behavior is not
acceptable.

### III. Lightweight Footprint

periScope MUST minimize runtime work, memory use, binary and bundle size,
background activity, and dependency count. Work MUST be event-driven where
practical; polling, continuous animation, and persistent resources require a
measured need. Every new runtime dependency MUST provide clear value that is not
reasonably achievable with the standard library, platform APIs, or existing
dependencies. Unused code, assets, features, and dependencies MUST be removed.

### IV. Modular by Design

Components MUST have one clear responsibility, explicit boundaries, and the
smallest practical public interface. Native overlay rendering, application
lifecycle, settings, persistence, hotkeys, tray behavior, and UI concerns MUST
remain separable unless a documented constraint requires coupling. Modules MUST
be independently understandable and testable at their boundary. Modularity MUST
reduce coupling; layers or wrappers that only forward calls are prohibited.

### V. KISS and DRY Review Discipline

Every code review MUST explicitly evaluate KISS and DRY. Reviewers MUST reject
needless cleverness, nesting, indirection, configuration, and abstraction.
Duplicated knowledge or business rules MUST have one authoritative
representation. Small incidental repetition MAY remain when extraction would
increase coupling or obscure intent; DRY MUST NOT be used to justify premature
abstraction. Approved code MUST be cohesive, clearly named, unsurprising, and
express its intent directly—this is the project's standard for elegance.

## Engineering Constraints

- Native Windows APIs and existing project capabilities MUST be preferred when
  they produce a smaller, faster, and simpler solution.
- The settings UI MUST remain framework-free unless a measured requirement
  demonstrates that a framework is necessary.
- Idle operation MUST perform no continuous redraw or polling unless the feature
  cannot be implemented event-first and the cost is measured and accepted.
- Cross-boundary contracts MUST use small, explicit data structures and avoid
  duplicating state.
- Performance budgets and footprint limits MUST be stated in the feature spec
  when a change can materially affect them; `N/A` requires a short rationale.
- Automated tests MUST maintain at least 80% line coverage for each instrumented
  production codebase. Generated code, declarations, build scripts, and
  platform-boundary code that cannot be exercised reliably MAY be excluded only
  when the exclusion and rationale are documented. Coverage is a minimum gate,
  not a substitute for testing critical behavior, error paths, and boundaries.

## Development Workflow and Quality Gates

1. Specifications MUST identify the smallest useful scope, measurable outcomes,
   performance expectations, and relevant footprint constraints.
2. Plans MUST pass the Constitution Check before implementation and again after
   design. Violations MUST be recorded in Complexity Tracking with the rejected
   simpler alternative.
3. Tasks MUST preserve module boundaries and include proportional verification
   for behavior, performance-sensitive paths, resource cleanup, and the 80%
   line-coverage floor.
4. Before merge, the author MUST run applicable automated checks and provide
   coverage results plus measurements for any performance claim or affected
   budget.
5. Reviewers MUST verify simplicity, performance evidence, footprint and
   dependency impact, modularity, meaningful test quality, the coverage gate, and
   KISS/DRY compliance. Unresolved violations block merge.

## Governance

This constitution supersedes conflicting project practices and templates.
Amendments MUST be proposed as a documented change that explains the reason,
affected principles, migration impact, and required template updates. Approval
requires maintainer review and all dependent artifacts MUST be updated in the
same change.

Constitution versions follow semantic versioning: MAJOR for incompatible
governance or principle changes, MINOR for new principles or materially expanded
requirements, and PATCH for non-semantic clarifications. Every plan and code
review MUST verify current constitutional compliance. Exceptions MUST be
specific, temporary, recorded in the plan's Complexity Tracking table, and
approved before implementation.

**Version**: 1.1.0 | **Ratified**: 2026-07-31 | **Last Amended**: 2026-07-31
