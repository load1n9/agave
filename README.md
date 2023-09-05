# Agave OS

Agave OS is a simple operating system written in Rust with wasi support. It is designed to be lightweight and easy to use. It is also designed to be easy to customize and extend. Started from [Fomos](https://github.com/Ruddle/Fomos) and the second edition of
[Writing an OS in Rust](https://os.phil-opp.com/) by Philipp Oppermann. Also contains code from [Theseus OS](https://github.com/theseus-os/Theseus)

## Features

- Lightweight and fast
- Supports a wide range of hardware platforms
- Easy to use and customize
- Built with Rust and wasi for maximum performance and security

## WASM apps

```rs
use agave_lib::{get_dimensions, set_pixels, Position, RGBA};

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    let dim = get_dimensions();

    set_pixels(
        Position {
            x: mouse_x,
            y: mouse_y,
        },
        Position {
            x: if mouse_x + 5 > dim.width {
                mouse_x
            } else {
                mouse_x + 5
            },
            y: if mouse_y + 5 > dim.height {
                mouse_y
            } else {
                mouse_y + 5
            },
        },
        RGBA {
            r: if mouse_x >= dim.width / 2 { 50 } else { 255 },
            g: 0,
            b: 255,
            a: 255,
        },
    );
}

```

![WASM app](assets/demo.png)

## Maintainers

- Dean Srebnik ([@load1n9](https://github.com/load1n9))
