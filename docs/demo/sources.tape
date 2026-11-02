# Select block lists from the catalog in the TUI, then leave the menu.
# The hidden setup defines a demo sudo shim and points a blk shell function at a throwaway hosts file.
# Run from the repo root after `cargo build --release`: vhs docs/demo/sources.tape
Output "docs/assets/demo-sources.gif"

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 20
Set LineHeight 1.4
Set Width 1100
Set Height 660
Set Padding 12
Set Margin 20
Set MarginFill "#1e1e2e"
Set BorderRadius 10
Set WindowBar "Colorful"
Set WindowBarSize 36
Set Framerate 24
Set PlaybackSpeed 0.9
Set TypingSpeed 50ms
Set Theme "Catppuccin Mocha"

Hide
Sleep 1s
Type "export BLKBIN=$PWD/target/release/blk; blk() { $BLKBIN --yes --hosts /tmp/blk-demo/hosts $@; }; sudo() { $@; }; export CLICOLOR_FORCE=1; export TERM=xterm-256color; rm -rf /tmp/blk-demo; mkdir -p /tmp/blk-demo; echo '127.0.0.1 localhost' > /tmp/blk-demo/hosts; export BLK_DIR=/tmp/blk-demo/state; export PS1='$ '; clear"
Enter
Sleep 2s
Show
Sleep 500ms

Type "sudo blk"
Sleep 500ms
Enter
Hide
Sleep 2500ms
Down@150ms 1
Sleep 400ms
Enter
Sleep 900ms
Show
Sleep 1000ms
Hide
Sleep 300ms
Enter
Sleep 1000ms
Up
Sleep 500ms
Show
Sleep 700ms
Space
Sleep 400ms
Up
Sleep 400ms
Space
Sleep 1200ms
Enter
Sleep 900ms
Enter
Sleep 3000ms
Hide
Sleep 300ms
Enter
Up
Enter
Up
Enter
Sleep 1000ms
