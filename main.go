package main

import (
	"fmt"
	"image/color"
	"log"

	"github.com/hajimehoshi/ebiten/v2"
)

type Klara struct {
	Cfg *Config
}

func (k *Klara) Update() error {
	return nil
}

func (k *Klara) Draw(screen *ebiten.Image) {
	screen.Fill(parseHexColor(k.Cfg.Theme.Background, uint8(k.Cfg.Window.Opacity*255)))
}

func (k *Klara) Layout(outsideWidth, outsideHeight int) (int, int) {
	return outsideWidth, outsideHeight
}

func parseHexColor(hex string, alpha uint8) color.Color {
	var r, g, b uint8
	if len(hex) == 7 && hex[0] == '#' {
		fmt.Sscanf(hex[1:], "%02x%02x%02x", &r, &g, &b)
		return color.RGBA{R: r, G: g, B: b, A: alpha}
	}
	return color.RGBA{R: 0x1e, G: 0x1e, B: 0x2e, A: alpha}
}

func main() {
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

	app := &Klara{Cfg: cfg}
	ebiten.SetWindowSize(1024, 768)
	ebiten.SetWindowTitle("Klara")
	ebiten.SetWindowResizingMode(ebiten.WindowResizingModeEnabled)
	if err := ebiten.RunGameWithOptions(app, &ebiten.RunGameOptions{
		ScreenTransparent: true,
	}); err != nil {
		log.Fatal(err)
	}
}
