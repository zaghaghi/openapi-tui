Output static/switch-server.gif
Hide
    Set Shell zsh

    Set FontSize 16
    Set Width 1200
    Set Height 600

    Set Padding 20
    Set FontFamily "JetBrains Mono"

    Set WindowBar Colorful
    Set WindowBarSize 40

    Set Theme "Catppuccin Mocha"
Show
    Type "export OPENAPI_TUI_DEFAULT_SERVER=http://127.0.0.1:8080"
    Enter
    Sleep 5s
    Type "openapi-tui -i examples/petstore"
Hide
    Ctrl+C
    Type "cargo run -q -- -i examples/petstore.json"
    Enter
    Sleep 5s
Show
Sleep 1s
Right@500ms
Right@500ms
Up@1s
Up@1s
Up@1s
Sleep 2s
Enter
Sleep 2s
Up@1s
Up@1s
Up@1s
Sleep 2s
Hide
    Type "q"
    Ctrl+D
Show

