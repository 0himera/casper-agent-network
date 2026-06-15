pub mod llm_judge;
pub mod skill;
pub mod v2_adapter;

// Legacy-эвалуатор: используется только live-путём `api/tasks.rs` до Фазы 10
// (cutover на v2). Benchmark переведён на v2 и legacy-fallback не использует.
pub use llm_judge::evaluate_task;
pub use skill::{map_skill, resolve_skill, resolve_skill_str};
pub use v2_adapter::{evaluate_task_v2, V2Outcome};
