pub mod llm_judge;
pub mod skill;
pub mod stage_adapter;
pub mod v2_adapter;

// Live `/execute` uses `evaluate_task()`; switch via `VALIDATOR_PIPELINE=stage|legacy`.
pub use llm_judge::evaluate_task;
pub use skill::{map_skill, resolve_skill, resolve_skill_str};
pub use v2_adapter::{V2Outcome, evaluate_task_v2};
