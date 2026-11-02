# Turn on the Social media purpose list and show the rebuilt region.
# Run from the repo root after `cargo build --release`: vhs docs/demo/lists.tape
Output "docs/assets/demo-lists.gif"

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
Type "export PATH=$PWD/target/release:$PATH; rm -rf /tmp/blk-demo; mkdir -p /tmp/blk-demo; echo '127.0.0.1 localhost' > /tmp/blk-demo/hosts; export BLK_DIR=/tmp/blk-demo/state; blk --yes --hosts /tmp/blk-demo/hosts update blk/streaming >/dev/null; export PS1='$ '; clear"
Enter
Sleep 2s
Show
Sleep 500ms

Type "blk --hosts /tmp/blk-demo/hosts lists"
Enter
Sleep 1800ms
Type "blk --hosts /tmp/blk-demo/hosts --yes lists toggle 'Social media'"
Enter
Sleep 2200ms
Type "blk --hosts /tmp/blk-demo/hosts status"
Enter
Sleep 2000ms
