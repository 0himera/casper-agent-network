pub mod benchmark_adapter;
pub mod llm_judge;
pub mod stage_adapter;

// Live `/execute` uses `evaluate_task()`; switch via `VALIDATOR_PIPELINE=stage|legacy`.
pub use benchmark_adapter::{
    build_benchmark_llm_config, evaluate_benchmark_skill_stage, warn_serpapi_if_needed,
    BenchmarkSkillEval,
};
pub use llm_judge::evaluate_task;
