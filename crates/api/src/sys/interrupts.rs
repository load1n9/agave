use crate::sys::gdt;
use crate::sys::ioapic;
use core::sync::atomic::AtomicU64;
use core::sync::atomic::AtomicUsize;
use core::sync::atomic::Ordering;
use core::task::Waker;
use futures::task::AtomicWaker;
use lazy_static::lazy_static;
use spin::Mutex;
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

pub static TIME_MS: AtomicU64 = AtomicU64::new(0);

pub const PIT_FREQUENCY: f64 = 3_579_545.0 / 3.0;
const PIT_DIVIDER: usize = 1193;
const PIT_INTERVAL: f64 = (PIT_DIVIDER as f64) / PIT_FREQUENCY;

pub static RANDTHING1: AtomicUsize = AtomicUsize::new(1);

pub static RANDTHING2: AtomicUsize = AtomicUsize::new(1);

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.alignment_check.set_handler_fn(alignment_check);


        idt.invalid_opcode.set_handler_fn(invalid_opcode);
        idt.bound_range_exceeded.set_handler_fn(bound_range_exceeded);
        idt.general_protection_fault.set_handler_fn(general_protection_fault);
        // TODO: Fix double fault handler type mismatch
        // unsafe {
        //     idt.double_fault.set_handler_fn(double_fault_handler)
        //         .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        // }
        unsafe {
            idt.overflow.set_handler_fn(overflow_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX); // new
        }

        idt.invalid_tss.set_handler_fn(invalid_tss_handler);
        idt.segment_not_present.set_handler_fn(segment_not_present_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);


        idt.page_fault.set_handler_fn(page_fault_handler);


        for i in 32..=255{
            idt[i].set_handler_fn(generic_handler);
        }

        idt[48].set_handler_fn(lapic_timer);
        idt[49].set_handler_fn(lapic_timer2);

        idt[50+0].set_handler_fn(ioapic_handler_0);
        idt[50+1].set_handler_fn(ioapic_handler_1);
        idt[50+2].set_handler_fn(ioapic_handler_2);
        idt[50+3].set_handler_fn(ioapic_handler_3);
        idt[50+4].set_handler_fn(ioapic_handler_4);
        idt[50+5].set_handler_fn(ioapic_handler_5);
        idt[50+6].set_handler_fn(ioapic_handler_6);
        idt[50+7].set_handler_fn(ioapic_handler_7);
        idt[50+8].set_handler_fn(ioapic_handler_8);
        idt[50+9].set_handler_fn(ioapic_handler_9);
        idt[50+10].set_handler_fn(ioapic_handler_10);
        idt[50+11].set_handler_fn(ioapic_handler_11);
        idt[50+12].set_handler_fn(ioapic_handler_12);
        idt[50+13].set_handler_fn(ioapic_handler_13);
        idt[50+14].set_handler_fn(ioapic_handler_14);
        idt[50+15].set_handler_fn(ioapic_handler_15);
        idt[50+16].set_handler_fn(ioapic_handler_16);
        idt[50+17].set_handler_fn(ioapic_handler_17);
        idt[50+18].set_handler_fn(ioapic_handler_18);
        idt[50+19].set_handler_fn(ioapic_handler_19);
        idt[50+20].set_handler_fn(ioapic_handler_20);
        idt[50+21].set_handler_fn(ioapic_handler_21);
        idt[50+22].set_handler_fn(ioapic_handler_22);
        idt[50+23].set_handler_fn(ioapic_handler_23);

        idt
    };
}

pub fn time_between_ticks() -> f64 {
    PIT_INTERVAL
}

extern "x86-interrupt" fn lapic_timer(_stack_frame: InterruptStackFrame) {
    unsafe {
        crate::sys::local_apic::LOCAL_APIC.get().unwrap().eoi();
    };
    let _ms = 1 + TIME_MS.fetch_add(1, Ordering::Relaxed);

    let mut arr = WAKERS.lock();
    for w in arr.iter_mut() {
        if let Some(waker) = w {
            waker.wake();
        }
        *w = None;
    }
    WAKER.wake();
}

pub fn global_time_ms() -> u64 {
    let _ = RANDTHING1.fetch_add(2, Ordering::Relaxed);
    TIME_MS.load(Ordering::Relaxed)
}

pub fn wait_block(ms: u64) {
    let current = TIME_MS.load(Ordering::Relaxed);
    loop {
        if TIME_MS.load(Ordering::Relaxed) > current + ms {
            break;
        } else {
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
            #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
            x86_64::instructions::nop();
        }
    }
}

pub async fn a_sleep(ms: u64) {
    let timer = Timer::new(ms);
    timer.await;
}

pub struct Timer {
    stop: u64,
}

impl Timer {
    pub fn new(ms: u64) -> Self {
        Timer {
            stop: global_time_ms() + ms,
        }
    }
}

impl futures::future::Future for Timer {
    type Output = ();
    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        if global_time_ms() >= self.stop {
            return core::task::Poll::Ready(());
        }

        let _id = add_waker(&cx.waker());
        // WAKER.register(&cx.waker());

        if global_time_ms() >= self.stop {
            core::task::Poll::Ready(())
        } else {
            core::task::Poll::Pending
        }
    }
}

type WakersT = [Option<AtomicWaker>; 128];
static WAKER: AtomicWaker = AtomicWaker::new();

lazy_static! {
    pub static ref WAKERS: Mutex<WakersT> = Mutex::new([(); 128].map(|_| None));
}

pub fn add_waker(waker: &Waker) -> u64 {
    let _ = RANDTHING2.fetch_add(2, Ordering::Relaxed);
    let mut arr = WAKERS.lock();
    for (index, aw) in arr.iter_mut().enumerate() {
        if aw.is_none() {
            let w = AtomicWaker::new();
            w.register(waker);
            *aw = Some(w);
            return index as u64;
        }
    }

    0
}

extern "x86-interrupt" fn lapic_timer2(_stack_frame: InterruptStackFrame) {
    // log::info!("timer2");

    // unsafe {
    //     self::local_apic::LocalApic.get().unwrap().eoi();
    // };
}

pub fn init_idt() {
    IDT.load();
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    log::error!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn overflow_handler(stack_frame: InterruptStackFrame) {
    log::error!("EXCEPTION: overflow_handler\n{:#?}", stack_frame);
    panic!("");
}

extern "x86-interrupt" fn invalid_tss_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    log::error!("EXCEPTION: invalid_tss {}\n{:#?}", error_code, stack_frame);
    panic!("");
}

extern "x86-interrupt" fn segment_not_present_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    log::error!(
        "EXCEPTION: segment_not_present {}\n{:#?}",
        error_code,
        stack_frame
    );
    panic!("");
}

extern "x86-interrupt" fn stack_segment_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    log::error!(
        "EXCEPTION: stack_segment_fault {}\n{:#?}",
        error_code,
        stack_frame
    );
    panic!("");
}

extern "x86-interrupt" fn alignment_check_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    log::error!(
        "EXCEPTION: alignment_check {}\n{:#?}",
        error_code,
        stack_frame
    );
    panic!("");
}

#[allow(dead_code)]
fn double_fault_handler_impl(stack_frame: InterruptStackFrame, error_code: u64) -> ! {
    panic!(
        "EXCEPTION: DOUBLE FAULT\n{:#?}\nError Code: {}",
        stack_frame, error_code
    );
}

#[allow(dead_code)]
extern "x86-interrupt" fn double_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    panic!(
        "EXCEPTION: DOUBLE FAULT\n{:#?}\nError Code: {}",
        stack_frame, error_code
    );
    #[allow(unreachable_code)]
    loop {}
}

extern "x86-interrupt" fn alignment_check(stack_frame: InterruptStackFrame, _error_code: u64) {
    panic!("EXCEPTION: alignment_check{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: invalid_opcode{:#?}", stack_frame);
}

extern "x86-interrupt" fn bound_range_exceeded(stack_frame: InterruptStackFrame) {
    panic!("EXCEPTION: bound_range_exceeded{:#?}", stack_frame);
}

extern "x86-interrupt" fn general_protection_fault(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) {
    panic!(
        "EXCEPTION: general_protection_fault {}\n{:#?}",
        _error_code, stack_frame
    );
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    use x86_64::registers::control::Cr2;
    let _ = RANDTHING1.fetch_add(2, Ordering::Relaxed);
    log::error!("EXCEPTION: PAGE FAULT");
    log::error!("Accessed Address: {:?}", Cr2::read());
    log::error!("Error Code: {:?}", error_code);
    // log::error!("{:#?}", stack_frame);

    panic!("EXCEPTION: PAGE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn ioapic_handler_0(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_0_____");
}

extern "x86-interrupt" fn ioapic_handler_1(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_1_____");
    let ioa = ioapic::IO_APIC_0.get().expect("IoApic0");
    let n = ioa.read_redtlb(1);
    let _red = ioapic::RedTbl::new(n);
    // log::info!("{:?}", red);
    // let stored = red.store();

    unsafe {
        crate::sys::local_apic::LOCAL_APIC.get().unwrap().eoi();
    };
}

extern "x86-interrupt" fn ioapic_handler_2(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_2_____");

    let ioa = ioapic::IO_APIC_0.get().expect("IoApic0");
    let n = ioa.read_redtlb(2);
    let red = ioapic::RedTbl::new(n);

    // log::info!("{:?}", red);
    let stored = red.store();
    ioa.write_redtlb(2, stored);
    unsafe {
        crate::sys::local_apic::LOCAL_APIC.get().unwrap().eoi();
    };
}
extern "x86-interrupt" fn ioapic_handler_3(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_3_____");
}
extern "x86-interrupt" fn ioapic_handler_4(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_4_____");
}
extern "x86-interrupt" fn ioapic_handler_5(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_5_____");
}
extern "x86-interrupt" fn ioapic_handler_6(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_6_____");
}
extern "x86-interrupt" fn ioapic_handler_7(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_7_____");
}
extern "x86-interrupt" fn ioapic_handler_8(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_8_____");
}

extern "x86-interrupt" fn ioapic_handler_9(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_9_____");
}

extern "x86-interrupt" fn ioapic_handler_10(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_10_____");
    unsafe {
        crate::sys::local_apic::LOCAL_APIC.get().unwrap().eoi();
    };
}

extern "x86-interrupt" fn ioapic_handler_11(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_11_____");

    unsafe {
        crate::sys::local_apic::LOCAL_APIC.get().unwrap().eoi();
    };
}

extern "x86-interrupt" fn ioapic_handler_12(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_12_____");
}

extern "x86-interrupt" fn ioapic_handler_13(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_13_____");
}

extern "x86-interrupt" fn ioapic_handler_14(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_14_____");
}

extern "x86-interrupt" fn ioapic_handler_15(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_15_____");
}

extern "x86-interrupt" fn ioapic_handler_16(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_16_____");
}

extern "x86-interrupt" fn ioapic_handler_17(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_17_____");
}

extern "x86-interrupt" fn ioapic_handler_18(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_18_____");
}

extern "x86-interrupt" fn ioapic_handler_19(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_19_____");
}

extern "x86-interrupt" fn ioapic_handler_20(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_20_____");
}

extern "x86-interrupt" fn ioapic_handler_21(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_21_____");
}

extern "x86-interrupt" fn ioapic_handler_22(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_22_____");
}

extern "x86-interrupt" fn ioapic_handler_23(_stack_frame: InterruptStackFrame) {
    // log::info!("______ioapic_handler_23_____");
}

extern "x86-interrupt" fn generic_handler(_stack_frame: InterruptStackFrame) {
    // log::info!("______generic_handler_____");
}
