use crate::println;

//use wasmi::{Engine, Module};
use wasmi::*;

pub fn read_wasm_string(offset: u32, length: u32, wasm_mem: &[u8]) -> &str {
    ::core::str::from_utf8(&wasm_mem[offset as usize..offset as usize + length as usize])
        .expect("read_wasm_cstring failed to parse invalid utf-8 string")
}

pub async fn example_exec() {
    let engine = Engine::default();
    let wasm = include_bytes!("../../bin/test.wasm");
    let module = Module::new(&engine, &wasm[..]).unwrap();

    type HostState = u32;

    let mut store = Store::new(&engine, 42);
    let host_hello = Func::wrap(&mut store, |caller: Caller<'_, HostState>, param: i32| {
        println!("Received {} from WebAssembly", param);
        println!("host state: {}", caller.data());
    });

    let mut linker = <Linker<HostState>>::new(&engine);

    linker.define("host", "hello", host_hello).unwrap();
    let instance = linker
        .instantiate(&mut store, &module)
        .unwrap()
        .start(&mut store)
        .unwrap();
    let hello = instance.get_typed_func::<(), ()>(&store, "hello").unwrap();

    hello.call(&mut store, ()).unwrap();
}
