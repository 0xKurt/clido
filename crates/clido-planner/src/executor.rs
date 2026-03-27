//! DAG executor: run tasks in dependency order, batching parallel tasks.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use futures::future::join_all;

use crate::graph::{TaskGraph, TaskNode};

/// Result of a single task execution.
#[derive(Debug, Clone)]
pub struct TaskResult {
    pub task_id: String,
    pub output: String,
    pub success: bool,
    pub duration_ms: u64,
}

/// Result of a full plan execution.
#[derive(Debug, Clone)]
pub struct PlanResult {
    pub task_results: Vec<TaskResult>,
    pub total_duration_ms: u64,
    pub success: bool,
    /// Set when execution used fallback (graph invalid or task failures).
    pub used_fallback: bool,
}

/// Trait for executing a single task (injected by caller so the executor stays provider-agnostic).
#[async_trait]
pub trait TaskRunner: Send + Sync {
    /// Run a single task, given the accumulated context from previously-completed tasks.
    /// The context map keys are prior task IDs, values are their text outputs.
    async fn run_task(&self, task: &TaskNode, context: &HashMap<String, String>) -> TaskResult;
}

/// Execute a TaskGraph in dependency order, passing task outputs as context to later tasks.
pub struct PlanExecutor;

impl PlanExecutor {
    /// Execute a graph using the given runner. Returns a PlanResult with all task results.
    ///
    /// Tasks within the same dependency batch run in parallel (true concurrent execution
    /// via `join_all`). Batches are ordered by dependency: all tasks in batch N complete
    /// before any task in batch N+1 starts.
    ///
    /// If the graph is invalid (cycle, etc.) returns a fallback result immediately.
    pub async fn execute(graph: &TaskGraph, runner: &dyn TaskRunner) -> PlanResult {
        let batches = match graph.parallel_batches() {
            Ok(b) => b,
            Err(_) => {
                return PlanResult {
                    task_results: vec![],
                    total_duration_ms: 0,
                    success: false,
                    used_fallback: true,
                }
            }
        };

        let mut task_results: Vec<TaskResult> = Vec::new();
        let mut context: HashMap<String, String> = HashMap::new();
        let start = std::time::Instant::now();
        let mut all_success = true;

        // Wrap runner in Arc so it can be shared across parallel futures.
        let runner = Arc::new(RunnerRef(runner));

        for batch in batches {
            if batch.is_empty() {
                continue;
            }
            let batch_ctx = context.clone();
            // Run all tasks in this batch concurrently.
            let futures: Vec<_> = batch
                .iter()
                .map(|task| {
                    let r = runner.clone();
                    let ctx = batch_ctx.clone();
                    // Clone the node so it can be moved into the async closure.
                    let task: TaskNode = (*task).clone();
                    async move { r.0.run_task(&task, &ctx).await }
                })
                .collect();

            let batch_results = join_all(futures).await;

            for result in batch_results {
                if !result.success {
                    all_success = false;
                }
                context.insert(result.task_id.clone(), result.output.clone());
                task_results.push(result);
            }
        }

        PlanResult {
            task_results,
            total_duration_ms: start.elapsed().as_millis() as u64,
            success: all_success,
            used_fallback: false,
        }
    }
}

/// Wrapper so we can put `&dyn TaskRunner` inside `Arc` without lifetime issues.
struct RunnerRef<'a>(&'a dyn TaskRunner);

// SAFETY: TaskRunner requires Send + Sync, so the wrapper is Send + Sync too.
unsafe impl<'a> Send for RunnerRef<'a> {}
unsafe impl<'a> Sync for RunnerRef<'a> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::TaskNode;

    struct MockRunner;

    #[async_trait]
    impl TaskRunner for MockRunner {
        async fn run_task(&self, task: &TaskNode, context: &HashMap<String, String>) -> TaskResult {
            let ctx_summary: String = context
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(";");
            TaskResult {
                task_id: task.id.clone(),
                output: format!("output_of_{}(ctx:{})", task.id, ctx_summary),
                success: true,
                duration_ms: 1,
            }
        }
    }

    struct FailingRunner;

    #[async_trait]
    impl TaskRunner for FailingRunner {
        async fn run_task(
            &self,
            task: &TaskNode,
            _context: &HashMap<String, String>,
        ) -> TaskResult {
            TaskResult {
                task_id: task.id.clone(),
                output: "failed".to_string(),
                success: false,
                duration_ms: 0,
            }
        }
    }

    fn make_node(id: &str, depends_on: Vec<&str>) -> TaskNode {
        TaskNode {
            id: id.to_string(),
            description: format!("Task {}", id),
            depends_on: depends_on.iter().map(|s| s.to_string()).collect(),
            tools: None,
            complexity: crate::graph::Complexity::default(),
            skip: false,
            notes: String::new(),
            status: crate::graph::TaskStatus::default(),
        }
    }

    #[tokio::test]
    async fn execute_linear_chain_all_succeed() {
        let graph = TaskGraph {
            goal: "test".to_string(),
            tasks: vec![
                make_node("t1", vec![]),
                make_node("t2", vec!["t1"]),
                make_node("t3", vec!["t2"]),
            ],
        };
        let runner = MockRunner;
        let result = PlanExecutor::execute(&graph, &runner).await;
        assert!(!result.used_fallback);
        assert!(result.success);
        assert_eq!(result.task_results.len(), 3);
        // t1 runs first with no context
        assert_eq!(result.task_results[0].task_id, "t1");
        // t2 output should contain t1's output in context
        assert!(result.task_results[1].output.contains("t1="));
        // t3 output should contain t2's output in context
        assert!(result.task_results[2].output.contains("t2="));
    }

    #[tokio::test]
    async fn execute_parallel_batch() {
        let graph = TaskGraph {
            goal: "parallel".to_string(),
            tasks: vec![
                make_node("a", vec![]),
                make_node("b", vec![]),
                make_node("c", vec!["a", "b"]),
            ],
        };
        let runner = MockRunner;
        let result = PlanExecutor::execute(&graph, &runner).await;
        assert!(!result.used_fallback);
        assert!(result.success);
        assert_eq!(result.task_results.len(), 3);
        // c must come after a and b
        let ids: Vec<&str> = result
            .task_results
            .iter()
            .map(|r| r.task_id.as_str())
            .collect();
        let c_pos = ids.iter().position(|&x| x == "c").unwrap();
        let a_pos = ids.iter().position(|&x| x == "a").unwrap();
        let b_pos = ids.iter().position(|&x| x == "b").unwrap();
        assert!(c_pos > a_pos);
        assert!(c_pos > b_pos);
    }

    #[tokio::test]
    async fn execute_invalid_graph_returns_fallback() {
        // A cyclic graph: PlanExecutor should return used_fallback = true.
        let graph = TaskGraph {
            goal: "cycle".to_string(),
            tasks: vec![make_node("x", vec!["y"]), make_node("y", vec!["x"])],
        };
        let runner = MockRunner;
        let result = PlanExecutor::execute(&graph, &runner).await;
        assert!(result.used_fallback);
        assert!(!result.success);
    }

    #[tokio::test]
    async fn execute_failing_runner_marks_not_success() {
        let graph = TaskGraph {
            goal: "fail".to_string(),
            tasks: vec![make_node("t1", vec![])],
        };
        let runner = FailingRunner;
        let result = PlanExecutor::execute(&graph, &runner).await;
        assert!(!result.used_fallback);
        assert!(!result.success);
        assert_eq!(result.task_results[0].task_id, "t1");
        assert!(!result.task_results[0].success);
    }

    #[tokio::test]
    async fn execute_empty_graph_succeeds() {
        let graph = TaskGraph {
            goal: "empty".to_string(),
            tasks: vec![],
        };
        let runner = MockRunner;
        let result = PlanExecutor::execute(&graph, &runner).await;
        assert!(!result.used_fallback);
        assert!(result.success);
        assert!(result.task_results.is_empty());
    }
}
