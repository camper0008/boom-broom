# boom-broom

minesweeper tui with rust

run with

`<exe> <width> <height> <bombs>`

controls are w/a/s/d+up/left/down/right, space to flag and enter to trip. space and enter also progress if blank or loss

remaining features

- [x] MVP gameplay
- [x] expand 0s automatically
- [x] restart on loss
- [x] restart on win
- [x] highlighting flagged fields & tripped mines
- [x] hud (x mines left, x flags placed, etc)
- [x] time
- [x] reveal neighbours of tile if tile with N bombs and N flags is tripped
- [ ] configuring parameters from tui instead of cli
- [ ] scrollable view if board is larger than frame