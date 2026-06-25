package main

import (
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	Font struct {
		Family string  `yaml:"family"`
		Size   float64 `yaml:"size"`
	} `yaml:"font"`
	Window struct {
		Opacity float64 `yaml:"opacity"`
		Blur    bool    `yaml:"blur"`
	} `yaml:"window"`
	Theme struct {
		Background          string   `yaml:"background"`
		Foreground          string   `yaml:"foreground"`
		Cursor              string   `yaml:"cursor"`
		SelectionBackground string   `yaml:"selection_background"`
		SelectionForeground string   `yaml:"selection_foreground"`
		Palette             []string `yaml:"palette"`
	} `yaml:"theme"`
}

func LoadConfig(path string) (*Config, error) {
	file, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}
	var config Config
	if err := yaml.Unmarshal(file, &config); err != nil {
		return nil, err
	}
	return &config, nil
}
