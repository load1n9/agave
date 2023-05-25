#![no_std]
#![no_main]

extern crate agave_os;
extern crate alloc;
extern crate bootloader;
extern crate x86_64;
use agave_os::println;
use agave_os::task::executor::Executor;
use agave_os::task::keyboard;
use agave_os::task::Task;
use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;

entry_point!(kernel_main);

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use agave_os::allocator;
    use agave_os::memory::{self, BootInfoFrameAllocator};

    println!("Hello World{}", "!");
    agave_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    let mut executor = Executor::new();
    executor.spawn(Task::new(example_task()));
    executor.spawn(Task::new(keyboard::print_keypresses()));
    executor.run();
}

async fn async_number() -> u32 {
    42
}

async fn example_task() {
    let number = async_number().await;
    println!("async number: {}", number);
}
