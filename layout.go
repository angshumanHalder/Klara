package main

import "image"

type LayoutNode struct {
	Pane          *Pane
	Left          *LayoutNode
	Right         *LayoutNode
	VerticalSplit bool
}

func (n *LayoutNode) FindPaneNode(paneId string) *LayoutNode {
	if n.Pane != nil {
		if n.Pane.ID == paneId {
			return n
		}
		return nil
	}
	if left := n.Left.FindPaneNode(paneId); left != nil {
		return left
	}

	return n.Right.FindPaneNode(paneId)
}

func (n *LayoutNode) CalculateLayouts(rect image.Rectangle, result map[string]image.Rectangle) {
	if n.Pane != nil {
		result[n.Pane.ID] = rect
		return
	}
	var leftRect, rightRect image.Rectangle
	if n.VerticalSplit {
		mid := rect.Min.X + rect.Dx()/2
		leftRect = image.Rect(rect.Min.X, rect.Min.Y, mid, rect.Max.Y)
		rightRect = image.Rect(mid, rect.Min.Y, rect.Max.X, rect.Max.Y)
	} else {
		mid := rect.Min.Y + rect.Dy()/2
		leftRect = image.Rect(rect.Min.X, rect.Min.Y, rect.Max.X, mid)
		rightRect = image.Rect(rect.Min.X, mid, rect.Max.X, rect.Max.Y)
	}
	n.Left.CalculateLayouts(leftRect, result)
	n.Right.CalculateLayouts(rightRect, result)
}

func (n *LayoutNode) Split(vertical bool, newID string, rows, cols int) error {
	newPane, err := NewPane(newID, rows, cols)
	if err != nil {
		return err
	}

	n.Left = &LayoutNode{Pane: n.Pane}
	n.Right = &LayoutNode{Pane: newPane}
	n.Pane = nil
	n.VerticalSplit = vertical
	return nil
}
