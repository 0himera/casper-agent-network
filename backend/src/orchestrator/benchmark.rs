use crate::config::Config;
use crate::db::DbPool;
use crate::orchestrator::executor::execute_agent;
use crate::validator::{
    build_benchmark_llm_config, evaluate_benchmark_skill_stage, warn_serpapi_if_needed,
};

/// Normalize input to the current benchmark domain contract.
pub(crate) fn normalize_benchmark_domain(input: &str) -> Option<&'static str> {
    match input {
        "defi" => Some("defi"),
        "rwa" => Some("rwa"),
        "other" => Some("other"),
        _ => None,
    }
}

fn benchmark_prompt(domain: &str) -> Option<&'static str> {
    match domain {
        "defi" => Some(
            "Analyze a DeFi opportunity on Casper. Recommend an allocation strategy, explain expected yield, identify protocol and liquidity risks, and give concrete risk mitigation steps.",
        ),
        "rwa" => Some(
            "Evaluate a real-world asset oracle update. Assess source quality, identify outliers or compliance risks, explain the valuation logic, and recommend any collateral-factor adjustment.",
        ),
        "other" => Some(
            "Answer the user's analytical task clearly and safely. State assumptions, provide actionable reasoning, identify material risks, and avoid unsupported claims.",
        ),
        _ => None,
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
            "Starting background benchmark for agent {} (stage pipeline)",
            agent_public_key
        );

        warn_serpapi_if_needed(&build_benchmark_llm_config(&config));

        // 1. Set agent status to benchmarking
        let _ = sqlx::query("UPDATE agents SET status = 'benchmarking' WHERE public_key = ?")
            .bind(&agent_public_key)
            .execute(&pool)
            .await;

        let mut total_score = 0;
        let mut skill_count = 0;
        let mut total_recommended_price_motes = 0u64;

        for skill in &skills {
            let Some(domain) = normalize_benchmark_domain(skill) else {
                eprintln!(
                    "domain '{}' is not supported by benchmark, skipping",
                    skill
                );
                continue;
            };
            let Some(prompt) = benchmark_prompt(domain) else {
                eprintln!("domain '{}' has no benchmark prompt, skipping", domain);
                continue;
            };

            println!(
                "Executing benchmark task for domain '{}' on agent {}",
                domain, agent_public_key
            );
            let exec_res = match execute_agent(
                domain,
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

            println!(
                "Evaluating benchmark response for domain '{}' on agent {} (stage pipeline)",
                domain, agent_public_key
            );
            let Some(eval) = evaluate_benchmark_skill_stage(
                domain,
                prompt,
                &exec_res.output,
                exec_res.processing_time_ms,
                &config,
            )
            .await
            else {
                continue;
            };

            total_score += eval.score;
            total_recommended_price_motes += eval.recommended_price_motes;
            skill_count += 1;

            let _ = sqlx::query(
                "INSERT INTO benchmark_runs (agent_public_key, domain, score, result, rubric_scores) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&agent_public_key)
            .bind(domain)
            .bind(eval.score as i32)
            .bind(&exec_res.output)
            .bind(eval.rubric_json)
            .execute(&pool)
            .await;

            let reputation_id = format!("{}_{}", agent_public_key, domain);
            let _ = sqlx::query(
                "INSERT INTO reputations (id, agent_public_key, skill, score) 
                 VALUES (?, ?, ?, ?) 
                 ON DUPLICATE KEY UPDATE score = ?",
            )
            .bind(reputation_id)
            .bind(&agent_public_key)
            .bind(domain)
            .bind(eval.score as i32)
            .bind(eval.score as i32)
            .execute(&pool)
            .await;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_benchmark_domain_accepts_only_current_domains() {
        assert_eq!(normalize_benchmark_domain("defi"), Some("defi"));
        assert_eq!(normalize_benchmark_domain("rwa"), Some("rwa"));
        assert_eq!(normalize_benchmark_domain("other"), Some("other"));
        assert_eq!(normalize_benchmark_domain("legacy_skill"), None);
        assert_eq!(normalize_benchmark_domain("unsupported"), None);
    }
}
