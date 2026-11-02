# Pause every source for 25 minutes, then restore it with Ctrl-C.
# Run from the repo root after `cargo build --release`: vhs docs/demo/pause.tape
Output "docs/assets/demo-pause.gif"

Set Shell "bash"
Set FontFamily "Menlo"
Set FontSize 20
Set LineHeight 1.4
Set Width 1100
Set Height 660
Set Padding 12
Set Margin 20
Set MarginFill "#0d1117"
Set BorderRadius 10
Set WindowBar "Colorful"
Set WindowBarSize 36
Set Framerate 24
Set PlaybackSpeed 1.1
Set TypingSpeed 35ms
Set Theme "GitHub Dark"

Hide
Sleep 1s
Type "export PATH=$PWD/target/release:$PATH; rm -rf /tmp/blk-demo; mkdir -p /tmp/blk-demo; echo '127.0.0.1 localhost' > /tmp/blk-demo/hosts; export BLK_DIR=/tmp/blk-demo/state; blk --yes --hosts /tmp/blk-demo/hosts update blk/social >/dev/null; export PS1='$ '; clear"
Enter
Sleep 2s
Show
Sleep 500ms

Type "blk --hosts /tmp/blk-demo/hosts pause 25"
Enter
Sleep 2400ms
Ctrl+C
Sleep 1200ms
Type "blk --hosts /tmp/blk-demo/hosts status"
Enter
Sleep 2000ms
