use alloc::vec::Vec;
use wasmi::{Caller, Engine, Func, Instance, Linker, Module, Store};

use crate::api::wasi::ctx::WasiCtx;

pub struct WasmInstance {
    store: Store<u32>,
    instance: Instance,
}

impl WasmInstance {
    pub fn new(wasm: Vec<u8>) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &wasm[..]).unwrap();
        // let wasi = WasiCtxBuilder::new()
        //     .inherit_stdio()
        //     .inherit_args()
        //     .unwrap()
        //     .build();
        // let mut store = Store::new(&engine, wasi);
        type HostState = u32;

        let mut store = Store::new(&engine, 42);

        let mut linker = <Linker<WasiCtx>>::new(&engine);

        let host_hello = Func::wrap(&mut store, |caller: Caller<'_, HostState>, param: i32| {
            println!("Received {} from WebAssembly", param);
            println!("host state: {}", caller.data());
        });

        let proc_exit = Func::wrap(&mut store, |_caller: Caller<'_, HostState>, param: i32| {
            crate::api::wasi::syscalls::proc_exit((param as usize).into());
        });

        // linker
        //     .define("wasi_unstable", "args_get", args_get)
        //     .unwrap();

        linker.define("host", "hello", host_hello).unwrap();
        linker
            .define("wasi_unstable", "proc_exit", proc_exit)
            .unwrap();

        let instance = linker
            .instantiate(&mut store, &module)
            .unwrap()
            .start(&mut store)
            .unwrap();

        Self { store, instance }
    }

    pub fn start(&mut self) {
        // let start = self
        //     .instance
        //     .get_typed_func::<(), ()>(&self.store, "_start")
        //     .unwrap();
        // start.call(&mut self.store, ()).unwrap();
        let hello = self
            .instance
            .get_typed_func::<(), ()>(&self.store, "hello")
            .unwrap();

        hello.call(&mut self.store, ()).unwrap();
    }
}
