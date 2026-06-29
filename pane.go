package main

import (
	"fmt"
	"os"
	"os/exec"
	"sync"

	"github.com/creack/pty"
	headlessterm "github.com/danielgatis/go-headless-term"
)

type Pane struct {
	ID      string
	Term    *headlessterm.Terminal
	PtyFile *os.File
	Cmd     *exec.Cmd
	Mutex   sync.RWMutex
}

func NewPane(id string, rows, cols int) (*Pane, error) {
	shell := os.Getenv("SHELL")
	if shell == "" {
		shell = "/bin/sh"
	}

	cmd := exec.Command(shell)
	ptmx, err := pty.StartWithSize(cmd, &pty.Winsize{
		Rows: uint16(rows),
		Cols: uint16(cols),
	})
	if err != nil {
		return nil, err
	}
	term := headlessterm.New(headlessterm.WithSize(rows, cols))
	p := &Pane{ID: id, Term: term, PtyFile: ptmx, Cmd: cmd}
	go p.readLoop()
	return p, nil
}

func (p *Pane) readLoop() {
	buf := make([]byte, 4096)
	for {
		n, err := p.PtyFile.Read(buf)
		if err != nil {
			return
		}
		p.Mutex.Lock()
		_, _ = p.Term.Write(buf[:n])
		p.Mutex.Unlock()
	}
}

func (p *Pane) GetCursorStyle() string {
	p.Mutex.Lock()
	defer p.Mutex.Unlock()
	switch p.Term.CursorStyle() {
	case headlessterm.CursorStyleBlinkingBar, headlessterm.CursorStyleSteadyBar:
		return "beam"
	case headlessterm.CursorStyleSteadyUnderline, headlessterm.CursorStyleBlinkingUnderline:
		return "underline"
	default:
		return "block"
	}
}

func (p *Pane) SendMouseClick(button, col, row int, isRelease bool) {
	suffix := "M"
	if isRelease {
		suffix = "m"
	}

	seq := fmt.Sprintf("\x1b[<%d;%d;%d;%s", button, row+1, col+1, suffix)
	_, _ = p.PtyFile.Write([]byte(seq))
}

func (p *Pane) WriteInput(data []byte) {
	_, _ = p.PtyFile.Write(data)
}

func (p *Pane) Close() {
	_ = p.PtyFile.Close()
	if p.Cmd.Process != nil {
		_ = p.Cmd.Process.Kill()
	}
}
