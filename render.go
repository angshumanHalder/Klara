package main

import (
	"bytes"
	"image/color"
	"unicode"

	headlessterm "github.com/danielgatis/go-headless-term"
	"github.com/hajimehoshi/ebiten/v2/text/v2"
	"golang.org/x/image/font/gofont/gomono"
)

type TextRun struct {
	Text string
	Fg   color.Color
	Bg   color.Color
}

type RenderEngine struct {
	MonoFace   *text.GoTextFace
	EmojiFace  *text.GoTextFace
	CellWidth  float64
	CellHeight float64
}

func colorsEqual(c1, c2 color.Color) bool {
	if c1 == nil || c2 == nil {
		return c1 == c2
	}
	c1R, c1G, c1B, c1A := c1.RGBA()
	c2R, c2G, c2B, c2A := c2.RGBA()
	return c1R == c2R && c1G == c2G && c1B == c2B && c1A == c2A
}

func BuildTextRuns(cells []*headlessterm.Cell) []TextRun {
	var run []TextRun
	var currentRun TextRun
	for _, cell := range cells {
		if cell == nil || cell.Char == 0 {
			continue
		}
		if currentRun.Text != "" && colorsEqual(cell.Fg, currentRun.Fg) && colorsEqual(cell.Bg, currentRun.Bg) {
			currentRun.Text += string(cell.Char)
		} else {
			if currentRun.Text != "" {
				run = append(run, currentRun)
			}
			currentRun = TextRun{Text: string(cell.Char), Fg: cell.Fg, Bg: cell.Bg}
		}
	}
	if currentRun.Text != "" {
		run = append(run, currentRun)
	}
	return run
}

func IsEmoji(r rune) bool {
	return unicode.Is(unicode.Symbol, r) && r > 0x7F
}

func NewRenderEngine(fontsize float64) *RenderEngine {
	ttf, err := text.NewGoTextFaceSource(bytes.NewReader(gomono.TTF))
	if err != nil {
		panic(err)
	}
	textFace := text.GoTextFace{
		Source: ttf,
		Size:   fontsize,
	}
	width, height := text.Measure("M", &textFace, 0)
	return &RenderEngine{
		MonoFace:   &textFace,
		EmojiFace:  &textFace,
		CellWidth:  width,
		CellHeight: height,
	}
}
