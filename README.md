# keytaps

A lightweight macOS key remapper written in Rust. Uses `CGEventTap` to intercept keyboard events at the HID level.

## Features

- **Right Control tap → Escape**: Tap Right Control quickly to send Escape. Hold it to use as a normal Control modifier.
- **Alt+HJKL → Arrow keys**: Vim-style arrow navigation. Shift/Ctrl/Cmd modifiers are preserved.

## Building

```
cargo build --release
```

## Usage

```
cargo run --release
```

The terminal running keytaps must have **Accessibility** permission (System Settings → Privacy & Security → Accessibility).

## License

MIT
