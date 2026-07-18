use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("failed to read configuration from {path}")]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },

    #[error("failed to parse configuration from {path}")]
    Parse {
        path: PathBuf,
        #[source]
        source: toml::de::Error,
    },

    #[error("invalid configuration field `{field}`: {reason}")]
    Invalid { field: &'static str, reason: String },
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct FontConfig {
    pub family: String,
    pub size: f32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct WindowConfig {
    pub opacity: f32,
    pub blur: bool,
    pub width: u32,
    pub height: u32,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    pub background: String,
    pub foreground: String,
    pub accent: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub font: FontConfig,
    pub window: WindowConfig,
    pub theme: ThemeConfig,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();

        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;

        let config: Self = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.font.family.trim().is_empty() {
            return Err(ConfigError::Invalid {
                field: "font.family",
                reason: "must not be empty".into(),
            });
        }

        if !self.font.size.is_finite() || self.font.size <= 0.0 {
            return Err(ConfigError::Invalid {
                field: "font.size",
                reason: "must be a finite number greater than  zero".into(),
            });
        }

        if self.window.width == 0 {
            return Err(ConfigError::Invalid {
                field: "window.width",
                reason: "must be greater than  zero".into(),
            });
        }

        if self.window.height == 0 {
            return Err(ConfigError::Invalid {
                field: "window.height",
                reason: "must be greater than zero".into(),
            });
        }

        if !self.window.opacity.is_finite() || !(0.0..=1.0).contains(&self.window.opacity) {
            return Err(ConfigError::Invalid {
                field: "window.opacity",
                reason: "must be between 0.0 and 1.0".into(),
            });
        }

        validate_hex_color("theme.background", &self.theme.background)?;
        validate_hex_color("theme.foreground", &self.theme.foreground)?;
        validate_hex_color("theme.accent", &self.theme.accent)?;

        Ok(())
    }

    pub fn parse_color(&self, hex: &str) -> [f64; 4] {
        let [r, g, b] = parse_hex_rgb(hex).unwrap_or([0, 0, 0]);
        [
            f64::from(r) / 255.0,
            f64::from(g) / 255.0,
            f64::from(b) / 255.0,
            f64::from(self.window.opacity),
        ]
    }
}

impl Default for Config {
    fn default() -> Self {
        let config: Self =
            toml::from_str(include_str!("../config.toml")).expect("embedded config must be valid");

        config
            .validate()
            .expect("embedded config must pass validation");
        config
    }
}

fn validate_hex_color(field: &'static str, value: &str) -> Result<(), ConfigError> {
    parse_hex_rgb(value).ok_or_else(|| ConfigError::Invalid {
        field,
        reason: format!("expected #RRGGBB, received {value:?}"),
    })?;

    Ok(())
}

fn parse_hex_rgb(value: &str) -> Option<[u8; 3]> {
    let value = value.strip_prefix('#').unwrap_or(value);

    if value.len() != 6 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }

    Some([
        u8::from_str_radix(&value[0..2], 16).ok()?,
        u8::from_str_radix(&value[2..4], 16).ok()?,
        u8::from_str_radix(&value[4..6], 16).ok()?,
    ])
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::io::Write;

    fn valid_config() -> Config {
        Config::default()
    }

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
    fn default_config_is_valid() {
        Config::default().validate().unwrap();
    }

    #[test]
    fn missing_file_returns_read_error() {
        let error = Config::load("/nonexistent/path/config.toml").unwrap_err();
        assert!(matches!(error, ConfigError::Read { .. }));
    }

    #[test]
    fn rejects_empty_font_family() {
        let mut config = valid_config();
        config.font.family = "   ".into();

        assert!(matches!(
            config.validate(),
            Err(ConfigError::Invalid {
                field: "font.family",
                ..
            })
        ));
    }

    #[test]
    fn rejects_invalid_font_size() {
        let mut config = valid_config();
        config.font.size = 0.0;

        assert!(matches!(
            config.validate(),
            Err(ConfigError::Invalid {
                field: "font.size",
                ..
            })
        ));
    }

    #[test]
    fn rejects_invalid_opacity() {
        let mut config = valid_config();
        config.window.opacity = 1.5;

        assert!(matches!(
            config.validate(),
            Err(ConfigError::Invalid {
                field: "window.opacity",
                ..
            })
        ));
    }

    #[test]
    fn rejects_zero_window_dimension() {
        let mut config = valid_config();
        config.window.width = 0;

        assert!(matches!(
            config.validate(),
            Err(ConfigError::Invalid {
                field: "window.width",
                ..
            })
        ));
    }

    #[test]
    fn rejects_malformed_colors_without_panicking() {
        for value in ["", "#", "#fff", "#gg0000", "#00000000"] {
            let mut config = valid_config();
            config.theme.background = value.into();

            assert!(config.validate().is_err(), "{value:?} should be rejected");
        }
    }

    #[test]
    fn rejects_unknown_fields() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(
            file,
            r##"
[font]
family = "Fira Code"
size = 16.0
unexpected = true

[window]
opacity = 0.8
blur = true
width = 800
height = 60

[theme]
background = "#000000"
foreground = "#ffffff"
accent = "#ff0000"
"##,
        )
        .unwrap();

        assert!(matches!(
            Config::load(file.path()),
            Err(ConfigError::Parse { .. })
        ));
    }

    #[test]
    fn parse_color_returns_normalized_rgba() {
        let config = valid_config();
        let color = config.parse_color("#ff8000");

        assert_eq!(color[0], 1.0);
        assert_eq!(color[1], 128.0 / 255.0);
        assert_eq!(color[2], 0.0);
        assert_eq!(color[3], f64::from(config.window.opacity));
    }
}
