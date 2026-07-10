use std::fs;

use anyhow::Result;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct WindowConfig {
    pub opacity: f32,
    pub blur: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub font: FontConfig,
    pub window: WindowConfig,
    pub theme: ThemeConfig,
}

impl Config {
    pub fn load(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)?;
        let config = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn default() -> Self {
        toml::from_str(include_str!("../config.toml")).expect("default config invalid")
    }

    pub fn parse_color(&self, hex: &str) -> [f64; 4] {
        let hex = hex.trim_start_matches("#");
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0) as f64 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f64 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f64 / 255.0;
        [r, g, b, self.window.opacity as f64]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_load_config_parses_toml() {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(
            f,
            r##"
[font]
family = "Fira Code"
size = 16.0

[window]
opacity = 0.8
blur = true
width = 800
height = 60

[theme]
background = "#000000"
foreground = "#ffffff"
accent = "#ff0000"
"##
        )
        .unwrap();

        let cfg = Config::load(f.path().to_str().unwrap()).unwrap();
        assert_eq!(cfg.font.family, "Fira Code");
        assert_eq!(cfg.font.size, 16.0);
        assert_eq!(cfg.window.opacity, 0.8);
        assert_eq!(cfg.theme.background, "#000000");
    }

    #[test]
    fn test_default_config_loads() {
        let cfg = Config::default();
        assert_eq!(cfg.font.family, "JetBrains Mono");
        assert!(cfg.window.opacity > 0.0);
    }

    #[test]
    fn test_load_config_missing_file() {
        assert!(Config::load("/nonexistent/path/config.toml").is_err())
    }
}
