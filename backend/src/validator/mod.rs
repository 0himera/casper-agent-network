pub mod llm_judge;
pub mod v2_adapter;

// Legacy-эвалюатор: используется только live-путём `api/tasks.rs` до Фазы 9
// (контракт входных данных). Benchmark переведён на v2 и legacy-fallback не использует.
pub use llm_judge::evaluate_task;
pub use v2_adapter::{evaluate_task_v2, V2Outcome};
