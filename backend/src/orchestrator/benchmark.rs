use crate::config::Config;
use crate::db::DbPool;
use crate::orchestrator::executor::execute_agent;
use crate::validator::{V2Outcome, evaluate_task_v2};

fn v2_prompt(skill: &str) -> Option<&'static str> {
    match skill {
        "defi_yield_routing" | "defi_analysis" => Some(
            "Allocate 10,000 CSPR across Casper liquidity pools minimizing impermanent loss risk. Pools data is provided. Show per-pool allocation summing to 10,000 CSPR, fee-adjusted APY math, and impermanent loss reasoning.",
        ),
        "defi_protocol_risk" => Some(
            "Analyze protocol risk based on recent transaction revert patterns. Classify the protocol as Safe or High Risk against the revert-rate threshold and list concrete mitigation steps.",
        ),
        "rwa_appraisal" => Some(
            "Determine a fair gold price for an on-chain oracle update from the provided external sources. Filter outliers, justify source quality, and describe the price-derivation algorithm.",
        ),
        "rwa_compliance" => Some(
            "Assess issuer compliance risk from the provided news items and recommend a collateral-factor adjustment. Separate real threats from FUD and give a remediation plan.",
        ),
        _ => None,
    }
}

/// Оценивает benchmark-skill только через v2-движок (validator-engine).
///
/// Legacy-fallback удалён: неподдержанные/без фикстуры skill пропускаются (`None`),
/// а не оцениваются устаревшим эвалюатором.
async fn evaluate_benchmark_skill(
    skill: &str,
    prompt: &str,
    agent_output: &str,
    processing_time_ms: u64,
    config: &Config,
) -> Option<(u32, u64, serde_json::Value)> {
    match evaluate_task_v2(
        skill,
        prompt,
        agent_output,
        processing_time_ms,
        None,
        config,
    )
    .await
    {
        V2Outcome::Ok(out) => {
            let rubric_json =
                serde_json::to_value(&out.criteria).unwrap_or(serde_json::Value::Null);
            Some((out.total, out.recommended_price_motes, rubric_json))
        }
        V2Outcome::Unsupported => {
            eprintln!(
                "skill '{}' is not supported by the v2 evaluator, skipping",
                skill
            );
            None
        }
        V2Outcome::FixtureInvalid(err) => {
            eprintln!("invalid fixture for skill '{}': {}", skill, err);
            None
        }
        V2Outcome::FixtureMissing(path) => {
            eprintln!("fixture missing for skill '{}' ({}), skipping", skill, path);
            None
        }
        V2Outcome::EngineError(err) => {
            eprintln!("v2 eval failed for skill '{}': {}", skill, err);
            None
        }
    }
}

pub fn start_benchmark_background(
    pool: DbPool,
    agent_public_key: String,
    skills: Vec<String>,
    endpoint_url: Option<String>,
    api_key: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
    config: Config,
) {
    tokio::spawn(async move {
        println!(
            "Starting background benchmark for agent {}",
            agent_public_key
        );

        // 1. Set agent status to benchmarking
        let _ = sqlx::query("UPDATE agents SET status = 'benchmarking' WHERE public_key = ?")
            .bind(&agent_public_key)
            .execute(&pool)
            .await;

        let mut total_score = 0;
        let mut skill_count = 0;
        let mut total_recommended_price_motes = 0u64;

        for skill in &skills {
            // v2-only: skill без v2-промпта (например "code_review") не поддерживается — пропускаем
            // до запуска агента, чтобы не тратить вызов на неоцениваемый skill.
            let Some(prompt) = v2_prompt(skill) else {
                eprintln!(
                    "skill '{}' is not supported by the v2 evaluator, skipping benchmark",
                    skill
                );
                continue;
            };

            // Execute the agent task
            println!(
                "Executing benchmark task for skill '{}' on agent {}",
                skill, agent_public_key
            );
            let exec_res = match execute_agent(
                skill,
                prompt,
                endpoint_url.as_deref(),
                api_key.as_deref(),
                model.as_deref(),
                system_prompt.as_deref(),
                &config,
            )
            .await
            {
                Ok(res) => res,
                Err(err) => {
                    eprintln!(
                        "Failed to execute benchmark for agent {}: {}",
                        agent_public_key, err
                    );
                    continue;
                }
            };

            // Evaluate results via the v2 LLM-as-Judge engine (no legacy fallback)
            println!(
                "Evaluating benchmark response for skill '{}' on agent {}",
                skill, agent_public_key
            );
            let Some((score, recommended_price_motes, rubric_json)) = evaluate_benchmark_skill(
                skill,
                prompt,
                &exec_res.output,
                exec_res.processing_time_ms,
                &config,
            )
            .await
            else {
                continue;
            };

            total_score += score;
            total_recommended_price_motes += recommended_price_motes;
            skill_count += 1;

            // Save benchmark run to database
            let _ = sqlx::query(
                "INSERT INTO benchmark_runs (agent_public_key, domain, score, result, rubric_scores) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&agent_public_key)
            .bind(skill)
            .bind(score as i32)
            .bind(&exec_res.output)
            .bind(rubric_json)
            .execute(&pool)
            .await;

            // Update/Insert reputation score for skill
            let reputation_id = format!("{}_{}", agent_public_key, skill);
            let _ = sqlx::query(
                "INSERT INTO reputations (id, agent_public_key, skill, score) 
                 VALUES (?, ?, ?, ?) 
                 ON DUPLICATE KEY UPDATE score = ?",
            )
            .bind(reputation_id)
            .bind(&agent_public_key)
            .bind(skill)
            .bind(score as i32)
            .bind(score as i32)
            .execute(&pool)
            .await;
        }

        // 2. Finalize: Set status to active, update recommended price
        let avg_score = if skill_count > 0 {
            total_score / skill_count
        } else {
            0
        };
        let avg_price = if skill_count > 0 {
            total_recommended_price_motes / skill_count as u64
        } else {
            0
        };

        println!(
            "Benchmark completed for agent {}. Avg score: {}, Rec Price: {} motes",
            agent_public_key, avg_score, avg_price
        );

        let _ = sqlx::query(
            "UPDATE agents SET status = 'active', recommended_price_motes = ? WHERE public_key = ?",
        )
        .bind(avg_price)
        .bind(&agent_public_key)
        .execute(&pool)
        .await;
    });
}
