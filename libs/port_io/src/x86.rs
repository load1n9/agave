use core::arch::asm;

pub unsafe fn inb(port: u16) -> u8 {
    let result: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") result,
        options(nomem, nostack)
    );

    result
}

pub unsafe fn outb(value: u8, port: u16) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack)
    );
}

pub unsafe fn inw(port: u16) -> u16 {
    let result: u16;
    asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") result,
        options(nomem, nostack)
    );

    result
}

pub unsafe fn outw(value: u16, port: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack)
    );
}

pub unsafe fn inl(port: u16) -> u32 {
    let result: u32;
    asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") result,
        options(nomem, nostack)
    );

    result
}

pub unsafe fn outl(value: u32, port: u16) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack)
    );
}
