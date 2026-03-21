# Planner (experimental)

The planner is an experimental feature that decomposes a complex task into a directed acyclic graph (DAG) of subtasks before executing them. This can improve performance on large tasks by giving the agent an explicit execution plan.

::: warning Experimental
The planner is experimental. It is disabled by default and may produce incorrect plans for some tasks. Always review the plan before running it on a critical codebase.
:::

## What the planner does

Without the planner, clido uses a **reactive loop**: the agent calls tools, observes results, and decides its next action one step at a time. This works well for most tasks.

With the planner enabled, clido first asks the LLM to:

1. Analyse the task
2. Break it into a set of named subtasks
3. Describe the dependencies between subtasks (forming a DAG)

The resulting plan is serialised as JSON and used to guide execution. Each node in the DAG corresponds to an agent sub-turn.

## Enabling the planner

Pass `--planner` on the command line:

```bash
clido --planner "migrate all deprecated API calls in this codebase to the new v2 API"
```

Or enable it in a workflow step:

```yaml
steps:
  - id: plan-and-execute
    prompt: "Refactor all database calls to use the new connection pool"
    planner: true
```

## Viewing the current plan in the TUI

When the planner is active, use `/plan` in the TUI to display the current task graph:

```
/plan
```

```
Plan: "Migrate API calls to v2"
  [✓] 1. Identify all deprecated API call sites
  [→] 2. Update src/api/client.rs
  [ ] 3. Update src/handlers/auth.rs
  [ ] 4. Update src/handlers/data.rs
  [ ] 5. Run tests and fix failures
  [ ] 6. Update documentation

Dependencies:
  2,3,4 depend on 1
  5 depends on 2,3,4
  6 depends on 5
```

## Plan format

The plan is a JSON object with a list of nodes and dependency edges:

```json
{
  "task": "Migrate API calls to v2",
  "nodes": [
    { "id": "1", "title": "Identify deprecated call sites", "status": "done" },
    { "id": "2", "title": "Update src/api/client.rs", "status": "in_progress" },
    { "id": "3", "title": "Update src/handlers/auth.rs", "status": "pending" },
    { "id": "4", "title": "Update src/handlers/data.rs", "status": "pending" },
    { "id": "5", "title": "Run tests and fix failures", "status": "pending" },
    { "id": "6", "title": "Update documentation", "status": "pending" }
  ],
  "edges": [
    { "from": "1", "to": "2" },
    { "from": "1", "to": "3" },
    { "from": "1", "to": "4" },
    { "from": "2", "to": "5" },
    { "from": "3", "to": "5" },
    { "from": "4", "to": "5" },
    { "from": "5", "to": "6" }
  ]
}
```

## Fallback behaviour

If the LLM produces an invalid plan (malformed JSON, cyclic dependencies, empty node list), clido falls back to the standard reactive loop without interrupting the task. A warning is printed to stderr:

```
Warning: Planner returned an invalid graph. Falling back to reactive loop.
```

## When to use the planner

The planner is most useful for:

- Tasks involving many files that need to be changed in a specific order
- Tasks where parallel sub-agents can be used effectively (future feature)
- Cases where you want an explicit overview of the execution strategy before it runs

The planner adds one extra LLM call (to generate the plan) and is therefore slightly more expensive per session.

## Limitations

- **Single planning call** — the plan is generated once at the start. If circumstances change during execution (a file does not exist as expected, a test fails in an unexpected way), the plan is not updated.
- **No re-planning** — the current implementation does not support dynamic re-planning. If a subtask fails, the agent continues with the reactive loop for subsequent steps.
- **Quality depends on model** — smaller or less capable models may generate poor plans. Claude 3.5 Sonnet and Claude 3 Opus produce the best results.
- **Single LLM call for planning** — the plan is generated in one pass and not iteratively refined.
