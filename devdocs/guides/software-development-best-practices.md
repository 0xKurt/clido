# Software Development Best Practices Guide

## Purpose

This guide defines practical rules for building software that is reliable, maintainable, secure, and easy to evolve.

It is intentionally opinionated. The goal is not to maximize speed in the short term. The goal is to produce software that survives real-world change, failure, and growth.

## Core Principles

### Optimize for clarity first

- Write code for the next engineer, not just for the compiler.
- Prefer simple control flow over clever abstractions.
- Use names that explain intent, not just mechanics.
- If a design needs a long verbal defense, it is probably too complex.

### Favor correctness over speed of delivery

- Do not trade away correctness for small short-term gains.
- Avoid shipping behavior you do not understand.
- If a change is risky, reduce scope before increasing confidence.
- Make the safe path the default path.

### Keep the system easy to change

- Minimize coupling between modules.
- Keep interfaces small and stable.
- Prefer composition over inheritance-heavy designs.
- Avoid spreading business rules across many unrelated files.

### Make failures visible

- Errors should be explicit, understandable, and actionable.
- Silent failure is usually worse than loud failure.
- Log important context, but never secrets.
- Build systems so bugs are diagnosable under production conditions.

## Project and Architecture Rules

### Start with a clear problem statement

Before building:
- define the user problem
- define the desired behavior
- define what is out of scope
- define how success will be measured

Do not start implementation if the expected outcome is still vague.

### Prefer small, well-bounded modules

- Each module should have one clear responsibility.
- A file should not mix unrelated concerns.
- Shared utilities should exist only when they reduce duplication without obscuring meaning.
- Delete dead abstractions quickly.

### Design around contracts

- Define stable inputs, outputs, and invariants.
- Validate boundaries between systems.
- Keep parsing, validation, business logic, and side effects separated where practical.
- Make illegal states hard to represent.

### Choose boring technology by default

- Prefer well-understood libraries over fashionable ones.
- Add dependencies only when they clearly reduce cost or risk.
- Avoid introducing frameworks or infrastructure without a real need.
- Every new dependency increases operational and security surface area.

### Optimize architecture for the likely future

- Do not overbuild for hypothetical scale.
- Do not underbuild for known requirements.
- Handle today's workload cleanly while leaving room for tomorrow's obvious growth path.

## Requirements and Planning Rules

### Write down the change before building it

For non-trivial work, capture:
- the problem
- the constraints
- the intended solution
- the main risks
- the rollback or fallback plan

### Break work into small, testable increments

- Prefer a sequence of safe improvements over a single large rewrite.
- Deliver vertical slices when possible.
- Each step should leave the system in a working state.
- Large changes should have checkpoints and validation gates.

### Define non-goals explicitly

- State what the change will not solve.
- Prevent scope creep early.
- Protect the team from accidentally turning one task into five.

### Use milestones and exit criteria

- Each milestone should have a clear definition of done.
- “Implemented” is not enough; define how it will be verified.
- Include functional, operational, and testing expectations.

## Coding Rules

### Prefer readable code over compact code

- Avoid deeply nested logic when guard clauses or helper functions are clearer.
- Keep functions focused.
- Inline only when it improves understanding.
- Avoid clever one-liners that hide intent.

### Keep functions small and purposeful

A function should ideally:
- do one thing
- have a predictable output
- hide unnecessary detail
- be easy to test in isolation

If a function mixes validation, transformation, I/O, retries, and formatting, split it.

### Use meaningful naming

- Name code after business meaning and domain intent.
- Avoid vague names like `data`, `handle`, `manager`, `temp`, or `misc`.
- Booleans should read like facts: `is_ready`, `has_access`, `should_retry`.
- Collections should use plural names.

### Make state transitions obvious

- Mutations should be easy to locate.
- Shared mutable state should be minimized.
- Prefer immutable data flow when it improves predictability.
- When mutation is necessary, keep it local and explicit.

### Write comments sparingly but usefully

Comments should explain:
- why a non-obvious decision exists
- important invariants
- edge cases
- constraints imposed by external systems

Comments should not repeat what the code already says.

### Keep formatting consistent

- Use automatic formatting tools wherever possible.
- Use linters to enforce consistency, not personal preference in review.
- Keep style debates out of code review by standardizing them.

## Error Handling Rules

### Handle errors deliberately

- Never swallow errors without a clear reason.
- Return structured errors where possible.
- Add context at boundaries so failures are diagnosable.
- Fail early when input is invalid.

### Distinguish expected from unexpected failures

- Validation errors, timeouts, missing files, and permission denials are normal and should be modeled clearly.
- Panics, invariant violations, and corrupted state are exceptional and should be surfaced loudly.

### Preserve useful error context

- Include which operation failed.
- Include the relevant identifier, path, or request reference.
- Avoid exposing secrets, credentials, or sensitive payloads in logs or error messages.

## Testing Rules

### Test behavior, not implementation trivia

- Focus tests on externally meaningful behavior.
- Avoid tests that merely mirror the code structure.
- Prefer outcomes and invariants over incidental internal details.

### Use a testing pyramid

- Unit tests for pure logic and edge cases.
- Integration tests for module boundaries and tool interactions.
- End-to-end tests for critical user workflows.

### Test failure paths

- Validate retries, timeouts, malformed input, and unavailable dependencies.
- Test what happens when the happy path breaks.
- Make sure failure modes remain understandable.

### Keep tests deterministic

- Control time, randomness, and network dependencies where possible.
- Use fixtures and mocks carefully.
- Flaky tests are defects and should be treated seriously.

### Add regression tests for real bugs

- Every important bug should leave behind a test.
- The test should fail before the fix and pass after it.
- This is one of the cheapest ways to prevent repeat failures.

## Review and Collaboration Rules

### Review for risk, not style theater

Code review should prioritize:
- correctness
- regressions
- security issues
- missing tests
- maintainability

Do not waste review cycles on formatting if tooling can handle it.

### Keep pull requests focused

- One PR should solve one coherent problem.
- Mixed refactors and feature changes are hard to review safely.
- Separate mechanical renames from behavioral changes.

### Explain why, not only what

A good PR description should include:
- the problem
- the chosen approach
- important trade-offs
- how it was tested

### Accept feedback without ego

- Defend decisions with evidence, not ownership.
- Prefer learning over winning arguments.
- If reviewers are confused, the code or explanation probably needs improvement.

## Security Rules

### Treat all external input as untrusted

- Validate and sanitize at system boundaries.
- Use allowlists where practical.
- Reject malformed input early.

### Follow least privilege

- Grant only the access a component actually needs.
- Minimize filesystem, network, and secret exposure.
- Separate read-only and state-changing operations where possible.

### Protect secrets aggressively

- Never hardcode secrets.
- Never log secrets.
- Never commit secrets to the repository.
- Use secret managers or environment-based injection where appropriate.

### Design for safe failure

- Dangerous actions should require explicit intent.
- Destructive operations should be auditable.
- Systems should default to deny, not allow, when trust is uncertain.

## Performance Rules

### Measure before optimizing

- Use profiling and benchmarks to locate actual bottlenecks.
- Avoid cargo-cult optimizations.
- Record the before and after state for important changes.

### Optimize where it matters

- Improve hot paths, startup time, and user-visible latency first.
- Do not sacrifice maintainability for tiny wins in cold paths.
- Cache only when invalidation rules are clear.

### Protect efficiency at scale

- Bound concurrency.
- Set timeouts.
- Limit retries.
- Avoid unbounded memory growth and recursive fan-out.

## Operational Rules

### Build observability in from the start

- Log important events and state transitions.
- Track durations, failures, and retry behavior.
- Include correlation identifiers where they help trace behavior across components.

### Make systems recoverable

- Persist enough information to resume interrupted work.
- Prefer append-only or checkpointed records for important workflows.
- Document recovery procedures for operators and developers.

### Respect deployment reality

- The local environment, CI environment, and production environment are different.
- Minimize environmental assumptions.
- Validate configuration early and fail clearly when it is missing or invalid.

## Documentation Rules

**Everything must be documented. Always. This is not optional.**

Documentation is not a finishing step. It is part of the work. A feature, change, or decision that is not documented is incomplete, regardless of whether the code compiles.

There are two distinct audiences for documentation in this project. Both are required.

---

### Human-Readable Documentation

Written for developers, operators, and contributors who need to understand, use, or maintain the system.

**What to document:**

- **System architecture** — how the major components relate and why they exist.
- **Important workflows** — how to run, configure, test, deploy, or operate the system.
- **Configuration and environment** — every configuration key, its type, default, valid values, and effect. No config key ships undocumented.
- **CLI flags and subcommands** — every flag, its purpose, and an example.
- **Error messages** — for every user-visible error, document what caused it and how to fix it.
- **Design decisions and trade-offs** — for every non-obvious choice, explain why that path was taken and what the alternatives were.
- **Invariants** — constraints the codebase assumes that are not enforced by the type system.
- **Migration and breaking changes** — when behavior or schema changes, document what changed, what breaks, and how to migrate.

**Standards:**

- Write documentation when writing the code, not after. Updating docs in the same commit as the behavior change is mandatory.
- Short and accurate beats long and outdated. Cut dead content aggressively.
- Every example must be correct and, where possible, runnable.
- Use plain language. Assume the reader is competent but unfamiliar with this specific system.
- Do not document what the code obviously does. Document why it does it, what it assumes, and what can go wrong.

**Where it lives:**

- User-facing docs: `devdocs/` (plans, guides) and inline `--help` text in the CLI.
- Architecture docs: `devdocs/plans/development-plan.md` and crate-level `README.md` files.
- Operational docs: included with release packages and available via `clido --help` and man pages (Phase 8.5).

---

### Development and Agent-Facing Documentation

Written for automated systems — AI coding agents (including Clido itself), CI pipelines, and future automated reviewers — that need structured, machine-interpretable guidance to work on the codebase correctly.

**What to document:**

- **`CLIDO.md` in each crate or module** — written for Clido and other coding agents working on that code. Include: purpose of the crate, key invariants, which files to read first, common pitfalls, what not to do, and which tests to run to validate changes.
- **API contracts** — function signatures, expected inputs, outputs, and panics in Rust doc comments (`///`). Not restating what the type already says — stating what the function assumes and guarantees.
- **Test intent** — every test must have a comment or a descriptive name that makes its purpose clear to an agent reading it. An agent should be able to read a test and know what behavior it is validating and why.
- **Tool and schema documentation** — every tool defined in `clido-tools/` must have a clear description field in its `ToolSchema` that an agent (or model) can read to understand when and how to use it.
- **Session and storage formats** — JSONL schema, memory DB schema, index format: document the schema in code and in `devdocs/`.
- **Config schema** — the `pricing.toml`, `config.toml`, and `.clido/config.toml` formats must be fully documented with field-by-field descriptions, not just examples.

**Standards:**

- Rust doc comments (`///`) on every public function, struct, enum, and trait. Not boilerplate — substantive descriptions.
- `CLIDO.md` files are first-class artifacts. They must be kept up to date as the crate evolves. Stale agent instructions are worse than no instructions because they actively mislead.
- If a behavior is subtle enough that a human would comment it, it is subtle enough that an agent needs it documented too.

---

### Keep documentation close to the code

- Update docs in the same change when behavior changes.
- Prefer short, accurate documentation over long outdated documentation.
- Examples should be runnable or obviously correct.
- Stale documentation is a bug. Treat it as one.

## Decision-Making Rules

### Prefer reversible decisions

- When uncertain, choose the path that is easier to change later.
- Avoid locking the team into early architectural commitments unless necessary.

### Escalate complexity slowly

- Start with the simplest design that satisfies current needs.
- Introduce advanced systems only after basic approaches prove insufficient.
- Complexity must earn its place.

### Use evidence to settle disputes

- Measure performance.
- Run experiments.
- Compare failure rates.
- Prefer real data over intuition when the stakes are meaningful.

## Rust-Specific Rules

### Make illegal states unrepresentable

Rust's type system is a design tool, not just a compiler requirement.

- Use enums to model state machines precisely.
- Use `Option<T>` intentionally; do not default to `Option` to avoid thinking about initialization.
- Use `NonZeroUsize`, `NonZeroU64`, and similar newtypes where zero is semantically invalid.
- Prefer `&str` for reads, `String` only when ownership is needed.
- Use `Arc<T>` for shared ownership across async tasks; use `Rc<T>` only in single-threaded contexts.

### Handle async correctly

Async Rust introduces specific failure modes that synchronous code does not have.

- Never call blocking I/O inside an async function without `tokio::task::spawn_blocking`.
- Never hold a `Mutex` guard across an `.await` point.
- Prefer `tokio::sync::RwLock` over `std::sync::Mutex` in async code.
- Use `tokio::time::timeout` to bound all network and subprocess calls.
- Set explicit timeouts on all external calls; unbounded futures cause silent hangs.
- Use `tokio::select!` carefully; understand cancellation semantics before using it.

### Use the workspace correctly

Clido is a multi-crate workspace. Keep it tidy.

- Define all shared dependency versions in the root `[workspace.dependencies]` block.
- Child crates reference workspace deps with `{ workspace = true }` — no version duplication.
- Keep crate boundaries aligned with logical responsibilities, not file size.
- Avoid circular dependencies between crates; the dependency graph must be a DAG.
- Add only the features you actually use; avoid `features = ["full"]` for large crates without a reason.

### Error handling in Rust

Rust error handling is explicit. Treat it seriously.

- Use `thiserror` for library/crate errors; use `anyhow` for application-level top-of-stack errors.
- Never use `.unwrap()` in production code paths; use `.expect("reason")` only when a panic would be a programming error, not a runtime condition.
- Propagate errors with `?` rather than panicking.
- Add context to errors at crate boundaries using `.context("...")` from `anyhow`.
- Define `type Result<T> = std::result::Result<T, ClidoError>` per crate to keep signatures readable.

### Write idiomatic Rust

- Use iterators instead of manual index loops where possible.
- Prefer `if let` and `while let` over `match` when only one arm is interesting.
- Use `derive` macros for standard traits (`Debug`, `Clone`, `Serialize`, `Deserialize`) rather than implementing them manually.
- Do not `clone()` more than necessary; understand the borrow checker before reaching for clone.
- Use `#[must_use]` on functions whose return values must not be silently ignored.

## Version Control and Commit Discipline

### Keep commits small and meaningful

- Each commit should represent one coherent change.
- Do not mix whitespace fixes, refactors, and feature changes in one commit.
- Commits that mix concerns are hard to revert, bisect, and review.

### Write commit messages that explain intent

A good commit message should answer: "Why does this change exist?"

Format:
- First line: short imperative summary, under 72 characters
- Blank line if a body is needed
- Body: explain the context, the problem, and the trade-offs if non-obvious

Avoid:
- "fix bug"
- "wip"
- "changes"
- "misc"

### Keep branches short-lived

- Prefer branches that live for hours or a few days, not weeks.
- Long-lived branches accumulate conflicts and make integration painful.
- Use feature flags or incremental steps to merge partial work safely.

### Never force-push to protected branches

- `main` and release branches are permanent history.
- Force-pushing rewrites history that others may have based work on.
- Fix mistakes with a new commit, not a rewrite.

### Tag releases and keep them immutable

- Every release gets a version tag.
- Tags are not moved after creation.
- Version numbers follow semantic versioning.

## Agent-Specific Development Rules

Building a coding agent introduces concerns that ordinary software does not have.

### Treat prompt content as code

- System prompts, tool descriptions, and guidance blocks are logic, not prose.
- They should be version-controlled, reviewed, and tested.
- Changes to system prompts can change behavior in unexpected ways.
- When a prompt changes, verify that existing workflows still behave correctly.

### Design tool schemas precisely

- Tool input schemas define the interface between the model and the runtime.
- Overly permissive schemas lead to poorly-specified inputs.
- Overly restrictive schemas cause the model to fail on valid use cases.
- Validate inputs against the schema before execution; this catches malformed model output early.

### Never trust model output uncritically

- Treat tool inputs from the model as untrusted external input.
- Validate all paths, patterns, and content before acting on them.
- Apply the same boundary validation you would apply to any external API consumer.

### Make context assembly a first-class concern

- The context sent to the model determines the quality of its behavior.
- Bloated context wastes tokens and degrades performance.
- Stale context causes the model to reason from outdated state.
- Test context assembly the same way you test business logic.

### Design for observable agent behavior

- Agent behavior should be diagnosable from logs and session files.
- Tool calls, results, costs, and turn counts should be recorded.
- Silent success is fine; silent failure is not acceptable.
- When a task goes wrong, it should be possible to reconstruct what happened from the session record.

### Test agent correctness at the behavior level

- Unit tests on individual tools and providers are necessary but not sufficient.
- End-to-end scenarios on real or realistic fixture repositories are the primary quality signal.
- A feature is not done until a representative workflow involving that feature passes reliably.

## Anti-Patterns to Avoid

- building abstractions before their use cases are clear
- rewriting large working systems without a migration strategy
- hiding important side effects behind innocent-looking helpers
- relying on tribal knowledge instead of documentation
- mixing refactors, feature work, and infrastructure changes in one review
- assuming tests are optional for “small” changes
- introducing silent fallback behavior that masks real failures
- adding tools, services, or dependencies without long-term ownership
- using `.unwrap()` in production code paths as a shortcut
- blocking on I/O inside an async function without `spawn_blocking`
- holding a mutex guard across an `.await` point
- cloning data unnecessarily instead of designing for correct ownership
- ignoring or suppressing compiler warnings
- writing system prompts as throw-away text rather than versioned, tested logic
- treating model tool-call inputs as safe without boundary validation
- shipping new agent capabilities without at least one representative end-to-end test

## Definition of Professional-Quality Software

Professional-quality software is not code that merely works today.

It is software that:
- behaves correctly under normal conditions
- fails safely under abnormal conditions
- can be understood by someone new to the project
- can be changed without fear
- can be debugged under pressure
- can be operated responsibly in real environments

## Development Checklist

Before merging significant work, verify:
- the problem and scope are clearly defined
- the design is simple enough to explain quickly
- the code follows project conventions
- errors are handled explicitly
- tests cover both success and failure paths
- logs and diagnostics are sufficient
- security implications were considered
- performance implications were considered
- **human-readable documentation was updated** — every new flag, config key, behavior, error message, and design decision is documented
- **agent-readable documentation was updated** — `///` doc comments on public API, `CLIDO.md` updated if the crate's purpose or invariants changed, tool schema descriptions updated if tool behavior changed
- the change can be rolled back or recovered from if needed

## Final Rule

Write software in a way that reduces future confusion.

A good developer does not only deliver features. A good developer leaves behind systems that other people can trust, operate, and improve.

Documentation is not separate from that. An undocumented system is one that can only be understood by the person who built it. That is not good enough.
