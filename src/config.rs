use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub struct ConfigManager {
    pub config_path: PathBuf,
    pub default_lang: String,
    pub default_dir: String,
    data: HashMap<String, String>,
}

impl ConfigManager {
    pub fn load() -> Self {
        let config_path = Self::config_path();
        let mut data = HashMap::new();

        if config_path.exists() {
            if let Ok(content) = fs::read_to_string(&config_path) {
                for line in content.lines() {
                    if let Some((key, value)) = line.split_once('=') {
                        data.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
        }

        let default_lang = data
            .get("default_lang")
            .cloned()
            .unwrap_or_else(|| "indonesian".to_string());
        let default_dir = data
            .get("default_dir")
            .cloned()
            .unwrap_or_else(|| ".".to_string());

        Self {
            config_path,
            default_lang,
            default_dir,
            data,
        }
    }

    fn config_path() -> PathBuf {
        let home = home::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".sub").join("config")
    }

    pub fn api_key(&self) -> Option<String> {
        if let Ok(key) = std::env::var("SUBSOURCE_API_KEY") {
            if !key.is_empty() {
                return Some(key);
            }
        }
        self.data.get("api_key").cloned()
    }

    pub fn save_api_key(&self, key: &str) {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        let mut lines: Vec<String> = if self.config_path.exists() {
            fs::read_to_string(&self.config_path)
                .ok()
                .map(|c| {
                    c.lines()
                        .filter(|l| !l.starts_with("api_key="))
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        lines.push(format!("api_key={}", key));
        fs::write(&self.config_path, lines.join("\n") + "\n").ok();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&self.config_path) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                fs::set_permissions(&self.config_path, perms).ok();
            }
        }
    }

    pub fn remove_api_key(&self) -> bool {
        if !self.config_path.exists() {
            return false;
        }
        if let Ok(content) = fs::read_to_string(&self.config_path) {
            let lines: Vec<String> = content
                .lines()
                .filter(|l| !l.starts_with("api_key="))
                .map(String::from)
                .collect();
            fs::write(&self.config_path, lines.join("\n") + "\n").ok();
            !lines.is_empty()
        } else {
            false
        }
    }

    pub fn set_config(&self, key: &str, value: &str) {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        match key {
            "lang" => self.set_key("default_lang", value),
            "dir" => {
                fs::create_dir_all(value).ok();
                self.set_key("default_dir", value);
            }
            _ => {
                eprintln!("{}", format!("Unknown config key: {}", key).red());
                std::process::exit(1);
            }
        }
        println!("{}", format!("Config '{}' set to '{}'", key, value).green());
    }

    fn set_key(&self, key: &str, value: &str) {
        let mut lines: Vec<String> = if self.config_path.exists() {
            fs::read_to_string(&self.config_path)
                .ok()
                .map(|c| {
                    c.lines()
                        .filter(|l| !l.starts_with(&format!("{}=", key)))
                        .map(String::from)
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        lines.push(format!("{}={}", key, value));
        fs::write(&self.config_path, lines.join("\n") + "\n").ok();
    }

    pub fn reset(&self) {
        if self.config_path.exists() {
            fs::remove_file(&self.config_path).ok();
        }
        println!("{}", "Config reset to defaults".green());
    }
}
