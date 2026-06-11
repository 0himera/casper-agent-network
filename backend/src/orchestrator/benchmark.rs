use crate::db::DbPool;
use crate::config::Config;
use crate::orchestrator::executor::execute_agent;
use crate::validator::evaluate_task;

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
        println!("Starting background benchmark for agent {}", agent_public_key);
        
        // 1. Set agent status to benchmarking
        let _ = sqlx::query(
            "UPDATE agents SET status = 'benchmarking' WHERE public_key = ?"
        )
        .bind(&agent_public_key)
        .execute(&pool)
        .await;

        let mut total_score = 0;
        let mut skill_count = 0;
        let mut total_recommended_price_motes = 0u64;

        for skill in &skills {
            let prompt = match skill.as_str() {
                "code_review" => r#"Review this Rust smart contract transfer function. Identify security vulnerabilities and suggest gas optimizations:
```rust
pub fn transfer(&mut self, recipient: Address, amount: U512) {
    let sender = self.env().caller();
    let balance = self.balances.get(&sender).unwrap_or_default();
    if balance < amount {
        self.env().revert(Error::InsufficientBalance);
    }
    self.balances.set(&sender, balance - amount);
    let recipient_balance = self.balances.get(&recipient).unwrap_or_default();
    self.balances.set(&recipient, recipient_balance + amount);
}
```"#,
                _ => "Analyze the yield opportunities and impermanent loss risk of providing liquidity to the CSPR/USDT pool on Casper Network."
            };

            // Execute the agent task
            println!("Executing benchmark task for skill '{}' on agent {}", skill, agent_public_key);
            let exec_res = match execute_agent(
                skill,
                prompt,
                endpoint_url.as_deref(),
                api_key.as_deref(),
                model.as_deref(),
                system_prompt.as_deref(),
                &config,
            ).await {
                Ok(res) => res,
                Err(err) => {
                    eprintln!("Failed to execute benchmark for agent {}: {}", agent_public_key, err);
                    continue;
                }
            };

            // Evaluate results via LLM-as-Judge
            println!("Evaluating benchmark response for skill '{}' on agent {}", skill, agent_public_key);
            let eval_res = match evaluate_task(
                skill,
                prompt,
                &exec_res.output,
                exec_res.processing_time_ms,
                &config,
            ).await {
                Ok(res) => res,
                Err(err) => {
                    eprintln!("Failed to evaluate benchmark for agent {}: {}", agent_public_key, err);
                    continue;
                }
            };

            total_score += eval_res.total;
            total_recommended_price_motes += eval_res.recommended_price_motes;
            skill_count += 1;

            // Save benchmark run to database
            let rubric_json = serde_json::to_value(&eval_res.scores).unwrap_or(serde_json::Value::Null);
            let _ = sqlx::query(
                "INSERT INTO benchmark_runs (agent_public_key, domain, score, result, rubric_scores) VALUES (?, ?, ?, ?, ?)"
            )
            .bind(&agent_public_key)
            .bind(skill)
            .bind(eval_res.total as i32)
            .bind(&exec_res.output)
            .bind(rubric_json)
            .execute(&pool)
            .await;

            // Update/Insert reputation score for skill
            let reputation_id = format!("{}_{}", agent_public_key, skill);
            let _ = sqlx::query(
                "INSERT INTO reputations (id, agent_public_key, skill, score) 
                 VALUES (?, ?, ?, ?) 
                 ON DUPLICATE KEY UPDATE score = ?"
            )
            .bind(reputation_id)
            .bind(&agent_public_key)
            .bind(skill)
            .bind(eval_res.total as i32)
            .bind(eval_res.total as i32)
            .execute(&pool)
            .await;
        }

        // 2. Finalize: Set status to active, update recommended price
        let avg_score = if skill_count > 0 { total_score / skill_count } else { 0 };
        let avg_price = if skill_count > 0 { total_recommended_price_motes / skill_count as u64 } else { 0 };
        
        println!(
            "Benchmark completed for agent {}. Avg score: {}, Rec Price: {} motes",
            agent_public_key, avg_score, avg_price
        );

        let _ = sqlx::query(
            "UPDATE agents SET status = 'active', recommended_price_motes = ? WHERE public_key = ?"
        )
        .bind(avg_price)
        .bind(&agent_public_key)
        .execute(&pool)
        .await;
    });
}
