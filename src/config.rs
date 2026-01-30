use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    pub library_path: String,
    pub theme: String,
    pub margin: u16,
    pub line_spacing: u16,
    pub auto_resume: bool,
    pub daily_goal_words: usize,
    pub pomodoro_work_minutes: u64,
    pub pomodoro_break_minutes: u64,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            library_path: dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .to_string_lossy()
                .to_string(),
            theme: "Default".to_string(),
            margin: 2,
            line_spacing: 0,
            auto_resume: true,
            daily_goal_words: 1500,
            pomodoro_work_minutes: 25,
            pomodoro_break_minutes: 5,
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self, ConfigError> {
        let s = Config::builder()
            .add_source(File::with_name("tbook").required(false))
            .build()?;
        s.try_deserialize()
    }

    #[allow(dead_code)]
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let toml = toml::to_string(self)?;
        std::fs::write("tbook.toml", toml)?;
        Ok(())
    }
}
