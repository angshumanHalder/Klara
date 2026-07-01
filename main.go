package main

import (
	"fmt"
	"image"
	"image/color"
	"log"

	headlessterm "github.com/danielgatis/go-headless-term"
	"github.com/hajimehoshi/ebiten/v2"
	"github.com/hajimehoshi/ebiten/v2/text/v2"
)

type Klara struct {
	Cfg          *Config
	Width        int
	Height       int
	RenderEngine *RenderEngine
}

func NewKlara() *Klara {
	cfg, err := LoadConfig("./config.yaml")
	if err != nil {
		log.Printf("could not load config %v", err)
		cfg = &Config{}
		cfg.Font.Family = "JetBrains Mono"
		cfg.Font.Size = 14.0
		cfg.Window.Opacity = 0.95
		cfg.Window.Blur = false
		cfg.Theme.Background = "#1e1e2e"
		cfg.Theme.Foreground = "#cdd6f4"
	}
	re := NewRenderEngine(cfg.Font.Size)
	return &Klara{Cfg: cfg, RenderEngine: re}
}

func (k *Klara) Layout(outsideWidth, outsideHeight int) (int, int) {
	k.Width = outsideWidth
	k.Height = outsideHeight
	return outsideWidth, outsideHeight
}

func (k *Klara) Update() error {
	return nil
}

func (k *Klara) Draw(screen *ebiten.Image) {
	screen.Fill(parseHexColor(k.Cfg.Theme.Background, uint8(k.Cfg.Window.Opacity*255)))
}

func (k *Klara) drawPane(screen *ebiten.Image, pane *Pane, rect image.Rectangle) {
	pane.Mutex.RLock()
	defer pane.Mutex.RUnlock()
	for row := 0; row < pane.Rows; row++ {
		var cells []*headlessterm.Cell
		for col := 0; col < pane.Cols; col++ {
			cells = append(cells, pane.Term.Cell(row, col))
		}
		runs := BuildTextRuns(cells)
		x := float64(rect.Min.X)
		y := float64(rect.Min.Y) + float64(row)*k.RenderEngine.CellHeight
		for _, run := range runs {
			fg := headlessterm.ResolveDefaultColor(run.Fg, true)
			op := &text.DrawOptions{}
			op.GeoM.Translate(x, y)
			op.ColorScale.ScaleWithColor(fg)
			text.Draw(screen, run.Text, k.RenderEngine.MonoFace, op)
			w, _ := text.Measure(run.Text, k.RenderEngine.MonoFace, 0)
			x += w
		}
	}
}

func (k *Klara) drawPaneBorders(screen *ebiten.Image, rect image.Rectangle, isActive bool) {
	borderColor := color.RGBA{R: 0x45, G: 0x47, B: 0x5a, A: 0xff}
	if isActive {
		borderColor = color.RGBA{R: 0xfe, G: 0x64, B: 0x0d, A: 0xff}
	}

	drawRect(screen, rect.Min.X, rect.Min.Y, rect.Dx(), 1, borderColor)
	drawRect(screen, rect.Min.X, rect.Max.Y-1, rect.Dx(), 1, borderColor)
	drawRect(screen, rect.Min.X, rect.Min.Y, 1, rect.Dy(), borderColor)
	drawRect(screen, rect.Max.X-1, rect.Min.Y, 1, rect.Dy(), borderColor)
}

func main() {
	app := NewKlara()
	ebiten.SetWindowSize(1024, 768)
	ebiten.SetWindowTitle("Klara")
	ebiten.SetWindowResizingMode(ebiten.WindowResizingModeEnabled)
	if err := ebiten.RunGameWithOptions(app, &ebiten.RunGameOptions{
		ScreenTransparent: true,
	}); err != nil {
		log.Fatal(err)
	}
}

func parseHexColor(hex string, alpha uint8) color.Color {
	var r, g, b uint8
	if len(hex) == 7 && hex[0] == '#' {
		_, _ = fmt.Sscanf(hex[1:], "%02x%02x%02x", &r, &g, &b)
		return color.RGBA{R: r, G: g, B: b, A: alpha}
	}
	return color.RGBA{R: 0x1e, G: 0x1e, B: 0x2e, A: alpha}
}

func drawRect(screen *ebiten.Image, x, y, w, h int, c color.Color) {
	screen.SubImage(image.Rect(x, y, x+w, y+h)).(*ebiten.Image).Fill(c)
}
