//! Database schema and pool initialization.
//!
//! ## E1 exam data-layer invariants (see `exam_idea_implementation.md`)
//! - `exam_templates` holds prompts and `expected_answer_canonical` (internal only).
//! - `exam_assignments` links live `tasks.id` → template → agent (internal only).
//! - `tasks` must never gain `expected_answer`, `is_exam`, or other exam secrets.
//! - Public REST/MCP handlers must not JOIN or expose `exam_*` tables (E1.6).
//! - Validation routing (E2) and dispatch (E4) are out of scope for E1.

pub mod exam;
pub mod models;

use sqlx::MySqlPool;
use sqlx::mysql::MySqlPoolOptions;

pub type DbPool = MySqlPool;

pub async fn init_db(database_url: &str) -> Result<DbPool, sqlx::Error> {
    tracing::info!("Connecting to database at {}...", database_url);

    let pool = MySqlPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;

    tracing::info!("Connected to database. Running migrations/schema setup...");

    // Create tables in correct dependency order
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS agents (
            public_key VARCHAR(128) PRIMARY KEY,
            name VARCHAR(255) NOT NULL,
            description TEXT NULL,
            metadata_uri VARCHAR(255) NULL,
            endpoint_url VARCHAR(255) NULL,
            api_key VARCHAR(255) NULL,
            model VARCHAR(255) NULL,
            active_jobs INT NOT NULL DEFAULT 0,
            status VARCHAR(50) NOT NULL DEFAULT 'active',
            recommended_price_motes BIGINT UNSIGNED NOT NULL DEFAULT 0,
            custom_price_motes BIGINT UNSIGNED NOT NULL DEFAULT 0,
            system_prompt TEXT NULL,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS tasks (
            id VARCHAR(128) PRIMARY KEY,
            creator_public_key VARCHAR(128) NOT NULL,
            assigned_agent_public_key VARCHAR(128) NULL,
            budget_motes BIGINT UNSIGNED NOT NULL,
            status VARCHAR(50) NOT NULL DEFAULT 'Open',
            result_hash VARCHAR(255) NULL,
            result TEXT NULL,
            metadata_uri VARCHAR(255) NULL,
            transaction_hash VARCHAR(128) NOT NULL,
            domain VARCHAR(100) NOT NULL DEFAULT 'defi_analysis',
            skill_id VARCHAR(100) NULL,
            prompt TEXT NOT NULL,
            deadline BIGINT UNSIGNED NOT NULL DEFAULT 0,
            result_signature TEXT NULL,
            validator_audit JSON NULL,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (assigned_agent_public_key) REFERENCES agents(public_key) ON DELETE SET NULL
        )",
    )
    .execute(&pool)
    .await?;

    // Ensure columns exist on already created tables
    let _ = sqlx::query("ALTER TABLE agents ADD COLUMN model VARCHAR(255) NULL")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN deadline BIGINT UNSIGNED NOT NULL DEFAULT 0")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN result_signature TEXT NULL")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN result TEXT NULL")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN skill_id VARCHAR(100) NULL")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN validator_audit JSON NULL")
        .execute(&pool)
        .await;

    let _ = sqlx::query("ALTER TABLE agents ADD COLUMN is_available TINYINT NOT NULL DEFAULT 1")
        .execute(&pool)
        .await;
    let _ = sqlx::query("ALTER TABLE tasks ADD COLUMN parent_task_id VARCHAR(128) NULL")
        .execute(&pool)
        .await;


    sqlx::query(
        "CREATE TABLE IF NOT EXISTS reputations (
            id VARCHAR(255) PRIMARY KEY,
            agent_public_key VARCHAR(128) NOT NULL,
            skill VARCHAR(100) NOT NULL,
            score INT NOT NULL DEFAULT 0,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (agent_public_key) REFERENCES agents(public_key) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS benchmark_runs (
            id INT AUTO_INCREMENT PRIMARY KEY,
            agent_public_key VARCHAR(128) NOT NULL,
            domain VARCHAR(100) NOT NULL,
            score INT NOT NULL,
            result TEXT NOT NULL,
            rubric_scores JSON NOT NULL,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (agent_public_key) REFERENCES agents(public_key) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS spent_payments (
            deploy_hash VARCHAR(128) PRIMARY KEY,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS exam_templates (
            id VARCHAR(128) PRIMARY KEY,
            prompt TEXT NOT NULL,
            expected_answer_canonical VARCHAR(512) NOT NULL,
            domain VARCHAR(100) NOT NULL,
            status VARCHAR(50) NOT NULL DEFAULT 'active',
            source_metadata JSON NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;


    sqlx::query(
        "CREATE TABLE IF NOT EXISTS exam_assignments (
            task_id VARCHAR(128) PRIMARY KEY,
            template_id VARCHAR(128) NOT NULL,
            agent_public_key VARCHAR(128) NOT NULL,
            bucket VARCHAR(50) NOT NULL DEFAULT 'audit',
            status VARCHAR(50) NOT NULL DEFAULT 'pending',
            verdict VARCHAR(50) NULL,
            created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            validated_at TIMESTAMP NULL,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (template_id) REFERENCES exam_templates(id) ON DELETE RESTRICT,
            FOREIGN KEY (agent_public_key) REFERENCES agents(public_key) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS validators (
            public_key VARCHAR(128) PRIMARY KEY,
            stake_motes BIGINT UNSIGNED NOT NULL DEFAULT 0,
            is_active TINYINT NOT NULL DEFAULT 1,
            total_validations INT NOT NULL DEFAULT 0,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
        )",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS validations (
            id INT AUTO_INCREMENT PRIMARY KEY,
            task_id VARCHAR(128) NOT NULL,
            validator_public_key VARCHAR(128) NOT NULL,
            score INT NOT NULL,
            timestamp TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
            FOREIGN KEY (validator_public_key) REFERENCES validators(public_key) ON DELETE CASCADE
        )",
    )
    .execute(&pool)
    .await?;


    tracing::info!("Database schema successfully checked/initialized.");
    Ok(pool)
}
