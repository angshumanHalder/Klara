package main

import (
	"os"
	"path/filepath"
	"testing"
)

func TestLoadConfig(t *testing.T) {
	tempDir, err := os.MkdirTemp("", "klara-config-test")
	if err != nil {
		t.Fatalf("Failed to create temp dir: %v", err)
	}
	defer os.RemoveAll(tempDir)

	configContent := []byte(`
font:
  family: "Fira Code"
  size: 16.5
window:
  opacity: 0.9
  blur: true
theme:
  background: "#000000"
  foreground: "#ffffff"
  palette:
    - "#45475a" 
    - "#f38ba8"
`)
	configPath := filepath.Join(tempDir, "config.yaml")
	if err := os.WriteFile(configPath, configContent, 0644); err != nil {
		t.Fatalf("Failed to write temp config file: %v", err)
	}

	cfg, err := LoadConfig(configPath)
	if err != nil {
		t.Fatalf("LoadConfig failed: %v", err)
	}

	if cfg.Font.Family != "Fira Code" {
		t.Errorf("Expected font family 'Fira Code', got '%s'", cfg.Font.Family)
	}
	if cfg.Font.Size != 16.5 {
		t.Errorf("Expected font size 16.5, got '%f'", cfg.Font.Size)
	}
	if cfg.Window.Opacity != 0.9 {
		t.Errorf("Expected window opacity 0.9, got '%f'", cfg.Window.Opacity)
	}
	if !cfg.Window.Blur {
		t.Errorf("Expected window blur to be true, got false")
	}
	if cfg.Theme.Background != "#000000" {
		t.Errorf("Expected theme background '#000000', got '%s'", cfg.Theme.Background)
	}
	if cfg.Theme.Foreground != "#ffffff" {
		t.Errorf("Expected theme foreground '#ffffff', got '%s'", cfg.Theme.Foreground)
	}
	if len(cfg.Theme.Palette) != 2 || cfg.Theme.Palette[0] != "#45475a" || cfg.Theme.Palette[1] != "#f38ba8" {
		t.Errorf("Expected palette colors to be '#45475a' and  '#f38ba8' , got '%v'", cfg.Theme.Palette)
	}
}

func TestLoadConfigMissingFile(t *testing.T) {
	cfg, err := LoadConfig("nonexistent.yaml")
	if err == nil {
		t.Error("Expected error for missing file, got nil")
	}
	if cfg != nil {
		t.Error("Expected nil config for missing file")
	}
}
