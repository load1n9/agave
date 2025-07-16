#![allow(unused_mut)]
use alloc::vec::Vec;
use wasmi::{Caller, Engine, Func, Instance, Linker, Module, Store, Memory, Extern};

use super::{
    framebuffer::{shapes::Coordinate, FB, RGBA},
    globals::Input,
    wasi,
};

pub struct WasmApp {
    store: Store<*mut FB>,
    instance: Instance,
    memory: Option<Memory>,
}

impl WasmApp {
    pub fn new(wasm: Vec<u8>, val: *mut FB) -> Self {
        log::info!("WASM: Creating new WASM app with {} bytes", wasm.len());
        let engine = Engine::default();
        let module = Module::new(&engine, &wasm[..]).unwrap();

        let mut store = Store::new(&engine, val);

        let mut linker = <Linker<*mut FB>>::new(&engine);

        log::info!("WASM: Setting up function bindings...");

        // Host function to grow memory from WASM
        let grow_memory = Func::wrap(&mut store, |mut caller: Caller<'_, *mut FB>, pages: u64| -> i32 {
            // Try to get the exported memory
            if let Some(Extern::Memory(mem)) = caller.get_export("memory") {
                match mem.grow(&mut caller, pages) {
                    Ok(_) => 1,
                    Err(_) => 0,
                }
            } else {
                0
            }
        });
        linker.define("agave", "grow_memory", grow_memory).unwrap();
        let draw_circle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x: i32,
             y: i32,
             radius: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.draw_circle(
                    Coordinate {
                        x: x as isize,
                        y: y as isize,
                    },
                    radius as usize,
                    RGBA {
                        r: r as u8,
                        g: g as u8,
                        b: b as u8,
                        a: a as u8,
                    },
                );
            },
        );
        linker.define("agave", "draw_circle", draw_circle).unwrap();

        // Add fill_circle function
        let fill_circle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x: i32,
             y: i32,
             radius: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                // Fill circle using Bresenham circle algorithm
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        if dx * dx + dy * dy <= radius * radius {
                            let px = x + dx;
                            let py = y + dy;
                            if px >= 0 && py >= 0 && px < fb.w as i32 && py < fb.h as i32 {
                                if let Some(pixel) =
                                    fb.pixels.get_mut((py * fb.w as i32 + px) as usize)
                                {
                                    pixel.r = r as u8;
                                    pixel.g = g as u8;
                                    pixel.b = b as u8;
                                    pixel.a = a as u8;
                                }
                            }
                        }
                    }
                }
            },
        );

        linker.define("agave", "fill_circle", fill_circle).unwrap();

        // Add fill_gradient function
        let fill_gradient = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x0: i32,
             y0: i32,
             x1: i32,
             y1: i32,
             r1: i32,
             g1: i32,
             b1: i32,
             a1: i32,
             r2: i32,
             g2: i32,
             b2: i32,
             a2: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.fill_gradient(
                    Coordinate {
                        x: x0 as isize,
                        y: y0 as isize,
                    },
                    Coordinate {
                        x: x1 as isize,
                        y: y1 as isize,
                    },
                    RGBA {
                        r: r1 as u8,
                        g: g1 as u8,
                        b: b1 as u8,
                        a: a1 as u8,
                    },
                    RGBA {
                        r: r2 as u8,
                        g: g2 as u8,
                        b: b2 as u8,
                        a: a2 as u8,
                    },
                );
            },
        );
        linker
            .define("agave", "fill_gradient", fill_gradient)
            .unwrap();

        // Add draw_triangle function
        let draw_triangle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x1: i32,
             y1: i32,
             x2: i32,
             y2: i32,
             x3: i32,
             y3: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                let color = RGBA {
                    r: r as u8,
                    g: g as u8,
                    b: b as u8,
                    a: a as u8,
                };

                // Draw triangle edges
                fb.draw_line(
                    Coordinate {
                        x: x1 as isize,
                        y: y1 as isize,
                    },
                    Coordinate {
                        x: x2 as isize,
                        y: y2 as isize,
                    },
                    color,
                );
                fb.draw_line(
                    Coordinate {
                        x: x2 as isize,
                        y: y2 as isize,
                    },
                    Coordinate {
                        x: x3 as isize,
                        y: y3 as isize,
                    },
                    color,
                );
                fb.draw_line(
                    Coordinate {
                        x: x3 as isize,
                        y: y3 as isize,
                    },
                    Coordinate {
                        x: x1 as isize,
                        y: y1 as isize,
                    },
                    color,
                );
            },
        );

        linker
            .define("agave", "draw_triangle", draw_triangle)
            .unwrap();

        let fill_rectangle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x: i32,
             y: i32,
             width: i32,
             height: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.fill_rectangle(
                    Coordinate {
                        x: x as isize,
                        y: y as isize,
                    },
                    width as usize,
                    height as usize,
                    RGBA {
                        r: r as u8,
                        g: g as u8,
                        b: b as u8,
                        a: a as u8,
                    },
                );
            },
        );

        linker
            .define("agave", "fill_rectangle", fill_rectangle)
            .unwrap();

        let draw_rectangle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x: i32,
             y: i32,
             width: i32,
             height: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.draw_rectangle(
                    Coordinate {
                        x: x as isize,
                        y: y as isize,
                    },
                    width as usize,
                    height as usize,
                    RGBA {
                        r: r as u8,
                        g: g as u8,
                        b: b as u8,
                        a: a as u8,
                    },
                );
            },
        );

        linker
            .define("agave", "draw_rectangle", draw_rectangle)
            .unwrap();

        let draw_rounded_rectangle = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x: i32,
             y: i32,
             width: i32,
             height: i32,
             radius: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.draw_rounded_rectangle(
                    Coordinate {
                        x: x as isize,
                        y: y as isize,
                    },
                    width as usize,
                    height as usize,
                    radius as usize,
                    RGBA {
                        r: r as u8,
                        g: g as u8,
                        b: b as u8,
                        a: a as u8,
                    },
                );
            },
        );
        linker
            .define("agave", "draw_rounded_rectangle", draw_rounded_rectangle)
            .unwrap();

        let draw_line = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x0: i32,
             y0: i32,
             x1: i32,
             y1: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.draw_line(
                    Coordinate {
                        x: x0 as isize,
                        y: y0 as isize,
                    },
                    Coordinate {
                        x: x1 as isize,
                        y: y1 as isize,
                    },
                    RGBA {
                        r: r as u8,
                        g: g as u8,
                        b: b as u8,
                        a: a as u8,
                    },
                );
            },
        );

        linker.define("agave", "draw_line", draw_line).unwrap();

        let set_pixel = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>, x: i32, y: i32, r: i32, g: i32, b: i32, a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                fb.pixels
                    .get_mut((y * (fb.w as i32) + x) as usize)
                    .map(|p| {
                        p.r = r as u8;
                        p.g = g as u8;
                        p.b = b as u8;
                        p.a = a as u8;
                    });
            },
        );

        linker.define("agave", "set_pixel", set_pixel).unwrap();

        let set_pixels_from_to = Func::wrap(
            &mut store,
            |caller: Caller<'_, *mut FB>,
             x0: i32,
             y0: i32,
             x1: i32,
             y1: i32,
             r: i32,
             g: i32,
             b: i32,
             a: i32| {
                let fb = unsafe { caller.data().as_mut().unwrap() };
                for y in y0..y1 {
                    for x in x0..x1 {
                        fb.pixels
                            .get_mut((y * (fb.w as i32) + x) as usize)
                            .map(|p| {
                                p.r = r as u8;
                                p.g = g as u8;
                                p.b = b as u8;
                                p.a = a as u8;
                            });
                    }
                }
            },
        );

        linker
            .define("agave", "set_pixels_from_to", set_pixels_from_to)
            .unwrap();

        let get_width = Func::wrap(&mut store, |caller: Caller<'_, *mut FB>| {
            let fb = unsafe { caller.data().as_mut().unwrap() };
            fb.w as i32
        });

        linker.define("agave", "get_width", get_width).unwrap();

        let get_height = Func::wrap(&mut store, |caller: Caller<'_, *mut FB>| {
            let fb = unsafe { caller.data().as_mut().unwrap() };
            fb.h as i32
        });

        linker.define("agave", "get_height", get_height).unwrap();

        let get_time_ms = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>| -> u64 {
            crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed)
        });

        linker.define("agave", "get_time_ms", get_time_ms).unwrap();

        // Keyboard input functions
        let is_key_pressed = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, key_code: i32| -> i32 {
                let input = crate::sys::globals::INPUT.read();
                if key_code >= 0 && (key_code as usize) < input.keys.len() {
                    match input.keys[key_code as usize] {
                        crate::sys::globals::KeyState::OnFromOff => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            },
        );

        linker
            .define("agave", "is_key_pressed", is_key_pressed)
            .unwrap();

        let is_key_down = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, key_code: i32| -> i32 {
                let input = crate::sys::globals::INPUT.read();
                if key_code >= 0 && (key_code as usize) < input.keys.len() {
                    match input.keys[key_code as usize] {
                        crate::sys::globals::KeyState::On
                        | crate::sys::globals::KeyState::OnFromOff
                        | crate::sys::globals::KeyState::OnTransientOff => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            },
        );

        linker.define("agave", "is_key_down", is_key_down).unwrap();

        let is_key_released = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, key_code: i32| -> i32 {
                let input = crate::sys::globals::INPUT.read();
                if key_code >= 0 && (key_code as usize) < input.keys.len() {
                    match input.keys[key_code as usize] {
                        crate::sys::globals::KeyState::OffFromOn => 1,
                        _ => 0,
                    }
                } else {
                    0
                }
            },
        );

        linker
            .define("agave", "is_key_released", is_key_released)
            .unwrap();

        let get_key_history_count = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>| -> i32 {
            let input = crate::sys::globals::INPUT.read();
            // Return the number of events we have, up to the buffer size
            core::cmp::min(input.history_last_index, 64) as i32
        });

        linker
            .define("agave", "get_key_history_count", get_key_history_count)
            .unwrap();

        let get_key_history_event = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, index: i32| -> i64 {
                let input = crate::sys::globals::INPUT.read();
                if index >= 0
                    && (index as usize) < 64
                    && (index as usize) < input.history_last_index
                {
                    let event = input.history_ring[index as usize];
                    // Pack key code in low 32 bits, pressed state in high 32 bits
                    let pressed_bits = if event.trigger { 1i64 << 32 } else { 0 };
                    pressed_bits | (event.key as i64)
                } else {
                    0
                }
            },
        );

        linker
            .define("agave", "get_key_history_event", get_key_history_event)
            .unwrap();

        // Link comprehensive WASI Preview 1 implementation
        wasi::preview1::link_preview1_functions(&mut linker, &mut store).unwrap();

        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();

        // Try to get the exported memory after instantiation
        let memory = instance
            .exports(&store)
            .find_map(|e| match e.into_extern() {
                Extern::Memory(mem) => Some(mem),
                _ => None,
            });

        Self { store, instance, memory }
    }

    /// Grow the WASM memory by the given number of pages (64KiB each). Returns true if successful.
    pub fn grow_memory(&mut self, pages: u64) -> bool {
        if let Some(mem) = &self.memory {
            mem.grow(&mut self.store, pages).is_ok()
        } else {
            false
        }
    }

    pub fn call(&mut self) {
        let start = self
            .instance
            .get_typed_func::<(), ()>(&self.store, "_start");

        match start {
            Ok(start) => {
                start.call(&mut self.store, ()).unwrap();
            }
            Err(e) => {
                log::warn!("WASM: No _start function found: {:?}", e);
            }
        }
    }

    pub fn call_update(&mut self, input: Input) {
        let update = self
            .instance
            .get_typed_func::<(i32, i32), ()>(&self.store, "update");

        match update {
            Ok(update) => {
                update
                    .call(
                        &mut self.store,
                        (input.mouse_x as i32, input.mouse_y as i32),
                    )
                    .unwrap();
            }
            Err(e) => {
                log::trace!("WASM: No update function found: {:?}", e);
            }
        }
    }
}
