#![no_std]
#![no_main]
#![feature(try_blocks)]

extern crate agave_os;
extern crate alloc;
extern crate anyhow;
extern crate bootloader;
extern crate x86_64;
use agave_os::println;
use agave_os::sys::task::executor::Executor;
use agave_os::sys::task::executor::Spawner;
use agave_os::sys::task::keyboard;
use bootloader::{entry_point, BootInfo};
use x86_64::VirtAddr;

entry_point!(kernel_main);

static LOGO: &str = r"
________   ________   ________   ___      ___  _______      
|\   __  \ |\   ____\ |\   __  \ |\  \    /  /||\  ___ \     
\ \  \|\  \\ \  \___| \ \  \|\  \\ \  \  /  / /\ \   __/|    
 \ \   __  \\ \  \  ___\ \   __  \\ \  \/  / /  \ \  \_|/__  
  \ \  \ \  \\ \  \|\  \\ \  \ \  \\ \    / /    \ \  \_|\ \ 
   \ \__\ \__\\ \_______\\ \__\ \__\\ \__/ /      \ \_______\
    \|__|\|__| \|_______| \|__|\|__| \|__|/        \|_______|
                                                             
                                                             
                                                                                          
";

fn kernel_main(boot_info: &'static BootInfo) -> ! {
    use agave_os::sys::allocator;
    use agave_os::sys::memory::{self, BootInfoFrameAllocator};

    agave_os::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialization failed");

    agave_os::vga::set_color(
        agave_os::api::vga::Color::LightGreen,
        agave_os::api::vga::Color::Black,
    );
    println!("{}", LOGO);
    agave_os::vga::set_color(
        agave_os::api::vga::Color::LightCyan,
        agave_os::api::vga::Color::Black,
    );

    let _result: anyhow::Result<()> = try {
        let spawner = Spawner::new(100);
        let mut executor = Executor::new(spawner.clone());
        spawner.add(agave_os::sys::wasm::example_exec());
        spawner.add(keyboard::print_keypresses());
        // spawner.add(kernel::task::mouse::process());
        println!("Still running somehow");
        executor.run();
    };
}
