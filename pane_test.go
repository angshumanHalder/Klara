package main

import (
	"testing"

	headlessterm "github.com/danielgatis/go-headless-term"
)

func TestGetCursor(t *testing.T) {
	term := headlessterm.New(headlessterm.WithSize(24, 80))
	_, _ = term.Write([]byte("\x1b[6 q"))
	p := &Pane{Term: term}
	if got := p.GetCursorStyle(); got != "beam" {
		t.Fatalf("expected 'beam', got %q", got)
	}
}
