# Pause one purpose list from the menu, then restore it with Ctrl-C.
# The hidden setup points a blk shell function at a throwaway hosts file.
# Run from the repo root after `cargo build --release`: vhs docs/demo/pause.tape
Output "docs/assets/demo-pause.gif"

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 20
Set LineHeight 1.4
Set Width 1100
Set Height 560
Set Padding 12
Set Margin 20
Set MarginFill "#1e1e2e"
Set BorderRadius 10
Set WindowBar "Colorful"
Set WindowBarSize 36
Set Framerate 24
Set PlaybackSpeed 1.1
Set TypingSpeed 35ms
Set Theme "Catppuccin Mocha"

Hide
Sleep 1s
Type "export BLKBIN=$PWD/target/release/blk; blk() { $BLKBIN --yes --hosts /tmp/blk-demo/hosts $@; }; export CLICOLOR_FORCE=1; export TERM=xterm-256color; rm -rf /tmp/blk-demo; mkdir -p /tmp/blk-demo; echo '127.0.0.1 localhost' > /tmp/blk-demo/hosts; export BLK_DIR=/tmp/blk-demo/state; blk update blk/streaming blk/social >/dev/null; export PS1='$ '; clear"
Enter
Sleep 2s
Show
Sleep 500ms

Type "blk"
Enter
Sleep 500ms
Hide
Sleep 1200ms
Enter
Sleep 900ms
Show
Sleep 500ms
Down@150ms 2
Enter
Sleep 1200ms
Space
Enter
Sleep 800ms
Type "25m"
Enter
Sleep 2400ms
Ctrl+C
Sleep 1200ms
Hide
Sleep 300ms
Up
Enter
Sleep 900ms
Up
Enter
Sleep 1000ms
