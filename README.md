![logo](static/logo.png)
# openapi-tui

[![CI](https://github.com/zaghaghi/openapi-tui/workflows/CI/badge.svg)](https://github.com/zaghaghi/openapi-tui/actions)

Terminal UI to list, browse and run APIs defined with OpenAPI v3.0 spec.

# Demo
![demo](static/demo.gif)

## Nested Components
![nested-refrences](static/nested-refs.gif)

## Fullscreen
![fullscreen](static/fullscreen.gif)

# Installation
Install from source:
```bash
❯ cargo install openapi-tui
```
Or download pre-built artifact from release page.

## Distro Packages

<details>
  <summary>Packaging status</summary>

[![Packaging status](https://repology.org/badge/vertical-allrepos/openapi-tui.svg)](https://repology.org/project/openapi-tui/versions)

</details>

### Arch Linux

You can install using `pacman` as follows:

```bash
❯ pacman -S openapi-tui
```

### NixOS

You can install the `openapi-tui` package directly with the following command:

```bash
nix profile install github:zaghaghi/openapi-tui
```

You can also install `openapi-tui` by adding it to your `configuration.nix` file.

```nix
# flake.nix

{
  inputs.openapi-tui.url = "github:zaghaghi/openapi-tui";
  # ...

  outputs = {nixpkgs, ...} @ inputs: {
    nixosConfigurations.<your-hostname> = nixpkgs.lib.nixosSystem {
      specialArgs = { inherit inputs; }; # this is the important part
      modules = [
        ./configuration.nix
      ];
    };
  }
}
```

Then, add `openapi-tui` to your `configuration.nix`

```nix
# configuration.nix

{inputs, pkgs, ...}: {
  environment.systemPackages = with pkgs; [
    inputs.openapi-tui.packages.${pkgs.system}.openapi-tui
  ];
}
```


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
| 'f' | Toggle fullscreen pane|
| `g` | Go in nested items in lists|
| `Backspace`, `b` | Get out of nested items in lists|


# Milestones
- [X] Viewer
- [ ] OpenAPI v3.1
- [ ] Remote API specification
- [X] Pane Fullscreen Mode
- [X] Nested Components
- [X] Status Line
- [ ] Command Line
- [ ] Execute 