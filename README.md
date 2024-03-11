![logo](static/logo.png)
# openapi-tui

[![CI](https://github.com/zaghaghi/openapi-tui/workflows/CI/badge.svg)](https://github.com/zaghaghi/openapi-tui/actions)

Terminal UI to list, browse and run APIs defined with openapi spec.

# Demo
![demo](static/demo.gif)

## Nested Components
![nested-refrences](static/nested-refs.gif)

# Installation
Install from source:
```bash
❯ cargo install openapi-tui
```
Or download pre-built artifact from release page.

# Usage
```bash
❯ openapi-tui --help
This TUI allows you to list and browse APIs described by the openapi specification.

Usage: openapi-tui [OPTIONS]

Options:
  -o, --openapi-path <PATH>  Input file, i.e. json or yaml file with openapi specification [default:
                             openapi.json]
  -h, --help                 Print help
  -V, --version              Print version
```

# Keybindings

| Key | Action|
|:----|:-----|
| `→`, `l`| Move to next pane |
| `←`, `h` | Move to previous pane |
| `↓`, `j` | Move down in lists |
| `↑`, `k` | Move up in lists |
| `1...9` | Move between tabs |
| `g` | Go in nested items in lists|
| `Backspace`, `b` | Get out of nested items in lists|


# Milestones
- [X] Viewer
- [ ] Remote API specification
- [ ] Pane Fullscreen Mode
- [X] Nested Components
- [X] Status Line
- [ ] Command Line
- [ ] Execute 
