# NES Emulator

This is a cross-platform NES emulator written in Rust. If you're just looking to
quickly test out the project, check out the [web demo](https://nes.purmple.com).

## Features

- Savestate support
- Game Genie support
- Audio support
- Basic recording/movie playback
- Mappers
  - NROM (used by Super Mario Bros. 1, Donkey Kong, Micro Mages)
  - MMC1 (used by The Legend of Zelda, Tetris)
  - MMC3 (used by Super Mario Bros. 2-3, Kirby's Adventure, [Bad Apple](https://littlelimit.net/bad_apple_2_5.htm))

## Disclaimer

This project is still in heavy development and is not intended to be a viable
alternative to other NES emulators. There are still some issues that have to be
fixed before it's ready for general use. If you're looking for a no-fuss
emulator to play all your favorite games on, this project is not for you.

## Desktop controls

- General controls
  - Start/pause emulation: P
  - Frame step (while paused): Space
  - Reset button: R
  - Quit: Esc
  - Toggle audio channels: 1-5
- Player 1
  - D-Pad: Arrow keys
  - B/A: Z/X
  - Start/Select: Enter/Right shift
- Player 2
  - D-Pad: WASD
  - B/A: K/L

## Building

To build this project, you need to have the Rust compiler installed. The
recommended way to do so is by installing through `rustup` from
<https://www.rust-lang.org/tools/install>.

### Desktop

Building for desktop requires SDL2's development libraries.

If you're on Linux, you can install them through your package manager:

```sh
# Ubuntu/Debian
apt-get install libsdl2-dev
# Fedora
dnf install SDL2-devel
# Arch
pacman -S sdl2
```

The process is a bit more involved if you're on Windows. Follow the guide on
[rust-sdl2's GitHub](https://github.com/Rust-SDL2/rust-sdl2#windows-msvc).

Finally, you can build using:

```sh
cargo build --bin desktop --release --features desktop
```

You can then run:

```sh
./target/release/desktop /path/to/rom.nes
```

If you want to play a recording/movie file, run:

```sh
./target/release/desktop /path/to/rom.nes /path/to/movie.fm2
```

### Web

Compiling to WebAssembly requires
[wasm-pack](https://github.com/rustwasm/wasm-pack).

```sh
wasm-pack build --target web --release -- --features wasm
```

The build files will then be available in `./pkg/`.

## Known issues

- If you're using a 60 Hz monitor, the framerate can appear choppy due to the
  NES's ~60.1 Hz video output.
- TAS recordings will usually desync in the first couple of seconds, most
  commonly during loading transitions.

## Compatibility

| ROM                 | Compatibility | Notes                                 | Workarounds       |
| ------------------- | ------------- | ------------------------------------- | ----------------- |
| Bad Apple           | 🟢 Good       | Gray box covers the top of the screen | Press Start twice |
| Castlevania         | ⚫ None       | Unsupported mapper                    |                   |
| Donkey Kong         | 🔵 Great      |                                       |                   |
| Ice Climber         | 🔵 Great      |                                       |                   |
| Kirby's Adventure   | 🔵 Great      |                                       |                   |
| Mega Man 1          | ⚫ None       | Unsupported mapper                    |                   |
| Mega Man 2          | 🔴 Bad        | Game hangs on startup                 |                   |
| Metroid             | 🔴 Bad        | Game hangs on startup                 |                   |
| Micro Mages         | 🔵 Great      |                                       |                   |
| Super Mario Bros. 1 | 🔵 Great      |                                       |                   |
| Super Mario Bros. 2 | 🔵 Great      |                                       |                   |
| Super Mario Bros. 3 | 🔵 Great      |                                       |                   |
| Tetris              | 🔵 Great      |                                       |                   |
| The Legend of Zelda | 🔵 Great      |                                       |                   |
