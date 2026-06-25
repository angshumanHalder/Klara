package main

import (
	"image/color"
	"testing"

	headlessterm "github.com/danielgatis/go-headless-term"
)

func TestBuildRun(t *testing.T) {
	red := color.RGBA{R: 255, A: 255}
	white := color.RGBA{R: 255, G: 255, B: 255, A: 255}
	cells := []*headlessterm.Cell{
		{
			Char: '!',
			Fg:   red,
		},
		{
			Char: '=',
			Fg:   red,
		},
		{
			Char: '>',
			Fg:   white,
		},
	}
	runs := BuildTextRuns(cells)
	if len(runs) != 2 {
		t.Fatalf("Expected runs to have length 2, got %d", len(runs))
	}

	if runs[0].Text != "!=" {
		t.Fatalf("Expected runs[0] to be '!=', got %s", runs[0].Text)
	}

	if runs[1].Text != ">" {
		t.Fatalf("Expected runs[1] to be '>', got %s", runs[1].Text)
	}
}

func TestBuildRunsSingleRun(t *testing.T) {
	red := color.RGBA{R: 255, A: 255}
	cells := []*headlessterm.Cell{
		{
			Char: '!',
			Fg:   red,
		},
		{
			Char: '=',
			Fg:   red,
		},
		{
			Char: '>',
			Fg:   red,
		},
	}
	runs := BuildTextRuns(cells)
	if len(runs) != 1 {
		t.Fatalf("Expected runs to have length 1, got %d", len(runs))
	}
	if runs[0].Text != "!=>" {
		t.Fatalf("Expected runs to be '!=>', got %s", runs[0].Text)
	}
}
