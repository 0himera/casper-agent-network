pub mod exam;
mod gates;
pub mod harness;
mod llm;
mod prompts;
pub mod search;
pub mod stage_pipeline;
mod types;

pub use types::{
    CriterionEval, JudgeCascadeMode, JudgeProvider, LlmConfig, ToolResult, ValidatorError,
};

pub use crate::llm::{
    call_judge_raw, judge_call_count, last_judge_provider_used, reset_judge_call_stats,
};
pub use exam::{
    AnswerVerificationMode, ExamAudit, ExamPipelineOutput, ExamVerdict, ExamVerificationPolicy,
    evaluate_exam_pipeline, evaluate_exam_pipeline_mock, evaluate_exam_pipeline_mock_with_config,
    resolve_exam_verification_policy,
};
pub use prompts::{
    build_stage_gibberish_prompts_version, build_stage_refusal_prompts_version,
    build_stage_relevance_prompts_version,
};
pub use stage_pipeline::{
    PipelineRunStats, PipelineVerdict, StageId, StagePipelineOutput, StageResult, StageTiming,
    evaluate_stage_pipeline, evaluate_stage_pipeline_mock, evaluate_stage_pipeline_with_stats,
};
