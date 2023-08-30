#[allow(unused_mut)]
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
                log::info!("{:?}", ctx);
                Ok(memory)
            };
            log::info!("host state: {}", caller.data());
        });
        linker.define("temp", "hello", host_hello).unwrap();

        let args_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _argv: i32, _argv_buf: i32| {
                return 0;
            },
        );

        linker
            .define("wasi_unstable", "args_get", args_get)
            .unwrap();

        let args_sizes_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _offset0: i32, _offset1: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "args_sizes_get", args_sizes_get)
            .unwrap();

        let environ_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _environ: i32, _environ_buf: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "environ_get", environ_get)
            .unwrap();

        let environ_sizes_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _offset0: i32, _offset1: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "environ_sizes_get", environ_sizes_get)
            .unwrap();

        let clock_res_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _id: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "clock_res_get", clock_res_get)
            .unwrap();

        let clock_time_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _id: i32, _precision: i64, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "clock_time_get", clock_time_get)
            .unwrap();

        let fd_advise = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i64, _len: i64, _advice: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_advise", fd_advise)
            .unwrap();

        let fd_allocate = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i64, _len: i64| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_allocate", fd_allocate)
            .unwrap();

        let fd_close = Func::wrap(&mut store, |_caller: Caller<'_, T>, _fd: i32| {
            return 0;
        });
        linker
            .define("wasi_unstable", "fd_close", fd_close)
            .unwrap();

        let fd_datasync = Func::wrap(&mut store, |_caller: Caller<'_, T>, _fd: i32| {
            return 0;
        });
        linker
            .define("wasi_unstable", "fd_datasync", fd_datasync)
            .unwrap();

        let fd_fdstat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_fdstat_get", fd_fdstat_get)
            .unwrap();

        let fd_fdstat_set_flags = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _flags: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_fdstat_set_flags", fd_fdstat_set_flags)
            .unwrap();

        let fd_fdstat_set_rights = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _fs_rights_base: i64, _fs_rights_inheriting: i64| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "fd_fdstat_set_rights",
                fd_fdstat_set_rights,
            )
            .unwrap();

        let fd_filestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_filestat_get", fd_filestat_get)
            .unwrap();

        let fd_filestat_set_size = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _size: i64| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "fd_filestat_set_size",
                fd_filestat_set_size,
            )
            .unwrap();

        let fd_filestat_set_times = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _atim: i64, _mtim: i64, _fst_flags: i32| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "fd_filestat_set_times",
                fd_filestat_set_times,
            )
            .unwrap();

        let fd_pread = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _iov_buf: i32,
             _iov_buf_len: i32,
             _offset: i64,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_pread", fd_pread)
            .unwrap();

        let fd_prestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_prestat_get", fd_prestat_get)
            .unwrap();

        let fd_prestat_dir_name = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _path: i32, _path_len: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_prestat_dir_name", fd_prestat_dir_name)
            .unwrap();

        let fd_pwrite = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _ciov_buf: i32,
             _ciov_buf_len: i32,
             _offset: i64,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_pwrite", fd_pwrite)
            .unwrap();

        let fd_read = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _iov_buf: i32, _iov_buf_len: i32, _offset1: i32| {
                return 0;
            },
        );
        linker.define("wasi_unstable", "fd_read", fd_read).unwrap();

        let fd_readdir = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _buf: i32,
             _buf_len: i32,
             _cookie: i64,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_readdir", fd_readdir)
            .unwrap();

        let fd_renumber = Func::wrap(&mut store, |_caller: Caller<'_, T>, _fd: i32, _to: i32| {
            return 0;
        });
        linker
            .define("wasi_unstable", "fd_renumber", fd_renumber)
            .unwrap();

        let fd_seek = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i64, _whence: i32, _offset0: i32| {
                return 0;
            },
        );
        linker.define("wasi_unstable", "fd_seek", fd_seek).unwrap();

        let fd_sync = Func::wrap(&mut store, |_caller: Caller<'_, T>, _fd: i32| {
            return 0;
        });
        linker.define("wasi_unstable", "fd_sync", fd_sync).unwrap();

        let fd_tell = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset0: i32| {
                return 0;
            },
        );
        linker.define("wasi_unstable", "fd_tell", fd_tell).unwrap();

        let fd_write = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _ciov_buf: i32,
             _ciov_buf_len: i32,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "fd_write", fd_write)
            .unwrap();

        let path_create_directory = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i32, _length: i32| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "path_create_directory",
                path_create_directory,
            )
            .unwrap();

        let path_filestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _flags: i32,
             _offset: i32,
             _length: i32,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_filestat_get", path_filestat_get)
            .unwrap();

        let path_filestat_set_times = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _flags: i32,
             _offset: i32,
             _length: i32,
             _atim: i64,
             _mtim: i64,
             _fst_flags: i32| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "path_filestat_set_times",
                path_filestat_set_times,
            )
            .unwrap();

        let path_link = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _old_fd: i32,
             _old_flags: i32,
             _old_offset: i32,
             _old_length: i32,
             _new_fd: i32,
             _new_offset: i32,
             _new_length: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_link", path_link)
            .unwrap();

        let path_open = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _dirflags: i32,
             _offset: i32,
             _length: i32,
             _oflags: i32,
             _fs_rights_base: i64,
             _fdflags: i64,
             _fs_rights_inheriting: i32,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_open", path_open)
            .unwrap();

        let path_readlink = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _offset: i32,
             _length: i32,
             _buf: i32,
             _buf_len: i32,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_readlink", path_readlink)
            .unwrap();

        let path_remove_directory = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i32, _length: i32| {
                return 0;
            },
        );
        linker
            .define(
                "wasi_unstable",
                "path_remove_directory",
                path_remove_directory,
            )
            .unwrap();

        let path_rename = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _old_offset: i32,
             _old_length: i32,
             _new_fd: i32,
             _new_offset: i32,
             _new_length: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_rename", path_rename)
            .unwrap();

        let path_symlink = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _old_offset: i32,
             _old_length: i32,
             _fd: i32,
             _new_offset: i32,
             _new_length: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_symlink", path_symlink)
            .unwrap();

        let path_unlink_file = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _offset: i32, _length: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "path_unlink_file", path_unlink_file)
            .unwrap();

        let poll_oneoff = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _in_: i32, _out: i32, _nsubscriptions: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "poll_oneoff", poll_oneoff)
            .unwrap();

        let proc_exit = Func::wrap(&mut store, |_caller: Caller<'_, T>, _rval: i32| {});
        linker
            .define("wasi_unstable", "proc_exit", proc_exit)
            .unwrap();

        let proc_raise = Func::wrap(&mut store, |_caller: Caller<'_, T>, _rval: i32| {
            return 0;
        });
        linker
            .define("wasi_unstable", "proc_raise", proc_raise)
            .unwrap();

        let sched_yield = Func::wrap(&mut store, |_caller: Caller<'_, T>| {
            return 0;
        });
        linker
            .define("wasi_unstable", "sched_yield", sched_yield)
            .unwrap();

        let random_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _buf: i32, _buf_len: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "random_get", random_get)
            .unwrap();

        let sock_accept = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>, _fd: i32, _flags: i32, _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "sock_accept", sock_accept)
            .unwrap();

        let sock_recv = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _iov_buf: i32,
             _iov_buf_len: i32,
             _ri_flags: i32,
             _offset0: i32,
             _offset1: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "sock_recv", sock_recv)
            .unwrap();

        let sock_send = Func::wrap(
            &mut store,
            |_caller: Caller<'_, T>,
             _fd: i32,
             _ciov_buf: i32,
             _ciov_buf_len: i32,
             _si_flags: i32,
             _offset0: i32| {
                return 0;
            },
        );
        linker
            .define("wasi_unstable", "sock_send", sock_send)
            .unwrap();

        let sock_shutdown =
            Func::wrap(&mut store, |_caller: Caller<'_, T>, _fd: i32, _how: i32| {
                return 0;
            });
        linker
            .define("wasi_unstable", "sock_shutdown", sock_shutdown)
            .unwrap();

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
