// use core::fmt::Write;

use alloc::{string::String, vec::Vec};
use wasmi::{core::Trap, Caller, Engine, Extern, Func, Instance, Linker, Module, Store};

// use crate::api::wasi::ctx::WasiCtx;

pub struct WasmApp<T> {
    store: Store<T>,
    instance: Instance,
}

impl<T> WasmApp<T>
where
    T: core::fmt::Display + core::fmt::Debug,
{
    pub fn new(wasm: Vec<u8>, val: T) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &wasm[..]).unwrap();
        // let wasi = WasiCtxBuilder::new()
        //     .inherit_stdio()
        //     .inherit_args()
        //     .unwrap()
        //     .build();
        // let mut store = Store::new(&engine, wasi);

        let mut store = Store::new(&engine, val);

        let mut linker = <Linker<T>>::new(&engine);

        let host_hello = Func::wrap(&mut store, |mut caller: Caller<'_, T>, param: i32| {
            log::info!("Received {} from WebAssembly", param);
            let _result = async {
                let memory = match caller.get_export("memory") {
                    Some(Extern::Memory(m)) => m,
                    _ => {
                        return Err(Trap::new(String::from(
                            "missing required WASI memory export",
                        )))
                    }
                };

                let (memory, ctx) = memory.data_and_store_mut(&mut caller);
                // let memory = Memory::new(ctx, memory);
                // let memory = memory.unwrap();
                log::info!("{:?}", ctx);
                Ok(memory)
            };
            log::info!("host state: {}", caller.data());
        });

        // let proc_exit = Func::wrap(&mut store, |mut caller: Caller<'_, T>, param: i32| {
        //     let _result = async {
        //         let memory = match caller.get_export("memory") {
        //             Some(Extern::Memory(m)) => m,
        //             _ => {
        //                 return Err(Trap::new(String::from(
        //                     "missing required WASI memory export",
        //                 )))
        //             }
        //         };

        //         let (memory, ctx) = memory.data_and_store_mut(&mut caller);
        //         // let memory = Memory::new(ctx, memory);
        //         // let memory = memory.unwrap();
        //         println!("{:?}", ctx);
        //         Ok(memory)
        //     };

        //     crate::api::wasi::syscalls::proc_exit(param as u32);
        // });

        linker.define("temp", "hello", host_hello).unwrap();
        // linker.defined("temp", "set_pixel", set_pixel).unwrap();
        // linker
        //     .define("wasi_unstable", "proc_exit", proc_exit)
        //     .unwrap();

        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();

        Self { store, instance }
    }

    pub fn call(&mut self) {
        let start = self
            .instance
            .get_typed_func::<(), ()>(&self.store, "_start")
            .unwrap();
        start.call(&mut self.store, ()).unwrap();
    }
}
