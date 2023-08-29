#![no_std]
#![no_main]

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

#[repr(C)]
pub struct Context<'a> {
    pub version: u8,
    start_time: u64,
    log: extern "C" fn(s: *const u8, l: u32),
    pid: u64,
    fb: FB<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[repr(C)]
pub struct FB<'a> {
    pub pixels: &'a mut [RGBA],
    pub w: usize,
    pub h: usize,
}

#[no_mangle]
pub extern "C" fn _start(ctx: &mut Context) -> i32 {
    ctx.version += 1;
    let mut i = 0;
    for px in ctx.fb.pixels.iter_mut() {
        i += 1;
        if i % 50 == 0 {
            px.r = 255;
        } else {
            px.b = 255;
        }
        // px.r = 255;
    }

    return 0;
}
