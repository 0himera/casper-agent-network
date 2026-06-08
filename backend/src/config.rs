use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub port: u16,
    pub openai_api_key: Option<String>,
    pub claude_api_key: Option<String>,
    pub ollama_url: Option<String>,
    pub ollama_model: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "mysql://root:password@127.0.0.1:3306/deagentnet".to_string());
        
        let port = env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse::<u16>()
            .unwrap_or(3000);

        let openai_api_key = env::var("OPENAI_API_KEY").ok();
        let claude_api_key = env::var("CLAUDE_API_KEY").ok();
        let ollama_url = env::var("OLLAMA_URL").ok();
        let ollama_model = env::var("OLLAMA_MODEL").ok();

        Config {
            database_url,
            port,
            openai_api_key,
            claude_api_key,
            ollama_url,
            ollama_model,
        }
    }
}
