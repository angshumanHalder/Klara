package main

import (
	"image"
	"testing"
)

func TestFineActiveNode(t *testing.T) {
	pane := &Pane{ID: "pane-active"}
	node := &LayoutNode{Pane: pane}
	found := node.FindPaneNode("pane-active")
	if found == nil {
		t.Fatalf("expected non-nil node, got nil")
	}
	if found.Pane.ID != "pane-active" {
		t.Fatalf("expected 'pane-active', got %q", found.Pane.ID)
	}
}

func TestCalculateLayouts(t *testing.T) {
	left := &LayoutNode{Pane: &Pane{ID: "left"}}
	right := &LayoutNode{Pane: &Pane{ID: "right"}}
	root := &LayoutNode{Left: left, Right: right, VerticalSplit: true}
	result := map[string]image.Rectangle{}
	root.CalculateLayouts(image.Rect(0, 0, 100, 50), result)
	if result["left"].Max.X != 50 {
		t.Fatalf("expected left max X 50, got %d", result["left"].Max.X)
	}
	if result["right"].Min.X != 50 {
		t.Fatalf("expected right min X 50, got %d", result["right"].Min.X)
	}
}
