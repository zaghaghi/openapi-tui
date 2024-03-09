# openapi-tui

[![CI](https://github.com/zaghaghi/openapi-tui/workflows/CI/badge.svg)](https://github.com/zaghaghi/openapi-tui/actions)

Terminal UI to list, browse and run APIs defined with openapi spec.

# Demo
![alt text](static/demo.gif)

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

# Milestones
- [ ] Viewer
- [ ] Remote API specification
- [ ] Pane Fullscreen Mode
- [ ] Nested Components
- [ ] Status Line
- [ ] Command Line
- [ ] Execute 