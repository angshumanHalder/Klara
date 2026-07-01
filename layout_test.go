package main

import "testing"

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
