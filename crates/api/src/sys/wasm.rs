#[allow(unused_mut)]
use alloc::vec::Vec;
use wasmi::{Caller, Engine, Func, Instance, Linker, Module, Store};

use super::{
    framebuffer::{shapes::Coordinate, FB, RGBA},
    globals::Input,
};

pub struct WasmApp {
    store: Store<*mut FB>,
    instance: Instance,
}

impl WasmApp {
    pub fn new(wasm: Vec<u8>, val: *mut FB) -> Self {
        let engine = Engine::default();
        let module = Module::new(&engine, &wasm[..]).unwrap();

        let mut store = Store::new(&engine, val);

        let mut linker = <Linker<*mut FB>>::new(&engine);

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

        let args_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, _argv: i32, _argv_buf: i32| {
                return 0;
            },
        );

        linker
            .define("wasi_snapshot_preview1", "args_get", args_get)
            .unwrap();

        let args_sizes_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, offset0: i32, offset1: i32| {
                log::info!("args_sizes_get({}, {})", offset0, offset1);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "args_sizes_get", args_sizes_get)
            .unwrap();

        let environ_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, environ: i32, environ_buf: i32| {
                log::info!("environ_get({}, {})", environ, environ_buf);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "environ_get", environ_get)
            .unwrap();

        let environ_sizes_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, offset0: i32, offset1: i32| {
                log::info!("environ_sizes_get({}, {})", offset0, offset1);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "environ_sizes_get",
                environ_sizes_get,
            )
            .unwrap();

        let clock_res_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, id: i32, offset0: i32| {
                log::info!("clock_res_get({}, {})", id, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "clock_res_get", clock_res_get)
            .unwrap();

        let clock_time_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, id: i32, precision: i64, offset0: i32| {
                log::info!("clock_time_get({}, {}, {})", id, precision, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "clock_time_get", clock_time_get)
            .unwrap();

        let fd_advise = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i64, len: i64, advice: i32| {
                log::info!("fd_advise({}, {}, {}, {})", fd, offset, len, advice);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_advise", fd_advise)
            .unwrap();

        let fd_allocate = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i64, len: i64| {
                log::info!("fd_allocate({}, {}, {})", fd, offset, len);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_allocate", fd_allocate)
            .unwrap();

        let fd_close = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>, fd: i32| {
            log::info!("fd_close({})", fd);
            return 0;
        });
        linker
            .define("wasi_snapshot_preview1", "fd_close", fd_close)
            .unwrap();

        let fd_datasync = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>, fd: i32| {
            log::info!("fd_datasync({})", fd);
            return 0;
        });
        linker
            .define("wasi_snapshot_preview1", "fd_datasync", fd_datasync)
            .unwrap();

        let fd_fdstat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset0: i32| {
                log::info!("fd_fdstat_get({}, {})", fd, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_fdstat_get", fd_fdstat_get)
            .unwrap();

        let fd_fdstat_set_flags = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, flags: i32| {
                log::info!("fd_fdstat_set_flags({}, {})", fd, flags);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "fd_fdstat_set_flags",
                fd_fdstat_set_flags,
            )
            .unwrap();

        let fd_fdstat_set_rights = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             fs_rights_base: i64,
             fs_rights_inheriting: i64| {
                log::info!(
                    "fd_fdstat_set_rights({}, {}, {})",
                    fd,
                    fs_rights_base,
                    fs_rights_inheriting
                );
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "fd_fdstat_set_rights",
                fd_fdstat_set_rights,
            )
            .unwrap();

        let fd_filestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset0: i32| {
                log::info!("fd_filestat_get({}, {})", fd, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_filestat_get", fd_filestat_get)
            .unwrap();

        let fd_filestat_set_size = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, size: i64| {
                log::info!("fd_filestat_set_size({}, {})", fd, size);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "fd_filestat_set_size",
                fd_filestat_set_size,
            )
            .unwrap();

        let fd_filestat_set_times = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, atim: i64, mtim: i64, fst_flags: i32| {
                log::info!(
                    "fd_filestat_set_times({}, {}, {}, {})",
                    fd,
                    atim,
                    mtim,
                    fst_flags
                );
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "fd_filestat_set_times",
                fd_filestat_set_times,
            )
            .unwrap();

        let fd_pread = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             offset: i64,
             offset0: i32| {
                log::info!(
                    "fd_pread({}, {}, {}, {}, {})",
                    fd,
                    iov_buf,
                    iov_buf_len,
                    offset,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pread", fd_pread)
            .unwrap();

        let fd_prestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset0: i32| {
                log::info!("fd_prestat_get({}, {})", fd, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_prestat_get", fd_prestat_get)
            .unwrap();

        let fd_prestat_dir_name = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, path: i32, path_len: i32| {
                log::info!("fd_prestat_dir_name({}, {}, {})", fd, path, path_len);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "fd_prestat_dir_name",
                fd_prestat_dir_name,
            )
            .unwrap();

        let fd_pwrite = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             ciov_buf: i32,
             ciov_buf_len: i32,
             offset: i64,
             offset0: i32| {
                log::info!(
                    "fd_pwrite({}, {}, {}, {}, {})",
                    fd,
                    ciov_buf,
                    ciov_buf_len,
                    offset,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_pwrite", fd_pwrite)
            .unwrap();

        let fd_read = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             offset1: i32| {
                log::info!("fd_read({}, {}, {}, {})", fd, iov_buf, iov_buf_len, offset1);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_read", fd_read)
            .unwrap();

        let fd_readdir = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             buf: i32,
             buf_len: i32,
             cookie: i64,
             offset0: i32| {
                log::info!(
                    "fd_readdir({}, {}, {}, {}, {})",
                    fd,
                    buf,
                    buf_len,
                    cookie,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_readdir", fd_readdir)
            .unwrap();

        let fd_renumber = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, to: i32| {
                log::info!("fd_renumber({}, {})", fd, to);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_renumber", fd_renumber)
            .unwrap();

        let fd_seek = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i64, whence: i32, offset0: i32| {
                log::info!("fd_seek({}, {}, {}, {})", fd, offset, whence, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_seek", fd_seek)
            .unwrap();

        let fd_sync = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>, fd: i32| {
            log::info!("fd_sync({})", fd);
            return 0;
        });
        linker
            .define("wasi_snapshot_preview1", "fd_sync", fd_sync)
            .unwrap();

        let fd_tell = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset0: i32| {
                log::info!("fd_tell({}, {})", fd, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_tell", fd_tell)
            .unwrap();

        let fd_write = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             ciov_buf: i32,
             ciov_buf_len: i32,
             offset0: i32| {
                log::info!(
                    "fd_write({}, {}, {}, {})",
                    fd,
                    ciov_buf,
                    ciov_buf_len,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "fd_write", fd_write)
            .unwrap();

        let path_create_directory = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i32, length: i32| {
                log::info!("path_create_directory({}, {}, {})", fd, offset, length);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "path_create_directory",
                path_create_directory,
            )
            .unwrap();

        let path_filestat_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             flags: i32,
             offset: i32,
             length: i32,
             offset0: i32| {
                log::info!(
                    "path_filestat_get({}, {}, {}, {}, {})",
                    fd,
                    flags,
                    offset,
                    length,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "path_filestat_get",
                path_filestat_get,
            )
            .unwrap();

        let path_filestat_set_times = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             flags: i32,
             offset: i32,
             length: i32,
             atim: i64,
             mtim: i64,
             fst_flags: i32| {
                log::info!(
                    "path_filestat_set_times({}, {}, {}, {}, {}, {}, {})",
                    fd,
                    flags,
                    offset,
                    length,
                    atim,
                    mtim,
                    fst_flags
                );
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "path_filestat_set_times",
                path_filestat_set_times,
            )
            .unwrap();

        let path_link = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             old_fd: i32,
             old_flags: i32,
             old_offset: i32,
             old_length: i32,
             new_fd: i32,
             new_offset: i32,
             new_length: i32| {
                log::info!(
                    "path_link({}, {}, {}, {}, {}, {}, {})",
                    old_fd,
                    old_flags,
                    old_offset,
                    old_length,
                    new_fd,
                    new_offset,
                    new_length
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_link", path_link)
            .unwrap();

        let path_open = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             dirflags: i32,
             offset: i32,
             length: i32,
             oflags: i32,
             fs_rights_base: i64,
             fdflags: i64,
             fs_rights_inheriting: i32,
             offset0: i32| {
                log::info!(
                    "path_open({}, {}, {}, {}, {}, {}, {}, {}, {})",
                    fd,
                    dirflags,
                    offset,
                    length,
                    oflags,
                    fs_rights_base,
                    fdflags,
                    fs_rights_inheriting,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_open", path_open)
            .unwrap();

        let path_readlink = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             offset: i32,
             length: i32,
             buf: i32,
             buf_len: i32,
             offset0: i32| {
                log::info!(
                    "path_readlink({}, {}, {}, {}, {}, {})",
                    fd,
                    offset,
                    length,
                    buf,
                    buf_len,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_readlink", path_readlink)
            .unwrap();

        let path_remove_directory = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i32, length: i32| {
                log::info!("path_remove_directory({}, {}, {})", fd, offset, length);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "path_remove_directory",
                path_remove_directory,
            )
            .unwrap();

        let path_rename = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             old_offset: i32,
             old_length: i32,
             new_fd: i32,
             new_offset: i32,
             new_length: i32| {
                log::info!(
                    "path_rename({}, {}, {}, {}, {}, {})",
                    fd,
                    old_offset,
                    old_length,
                    new_fd,
                    new_offset,
                    new_length
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_rename", path_rename)
            .unwrap();

        let path_symlink = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             old_offset: i32,
             old_length: i32,
             fd: i32,
             new_offset: i32,
             new_length: i32| {
                log::info!(
                    "path_symlink({}, {}, {}, {}, {})",
                    old_offset,
                    old_length,
                    fd,
                    new_offset,
                    new_length
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "path_symlink", path_symlink)
            .unwrap();

        let path_unlink_file = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, offset: i32, length: i32| {
                log::info!("path_unlink_file({}, {}, {})", fd, offset, length);
                return 0;
            },
        );
        linker
            .define(
                "wasi_snapshot_preview1",
                "path_unlink_file",
                path_unlink_file,
            )
            .unwrap();

        let poll_oneoff = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             in_: i32,
             out: i32,
             nsubscriptions: i32,
             offset0: i32| {
                log::info!(
                    "poll_oneoff({}, {}, {}, {})",
                    in_,
                    out,
                    nsubscriptions,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "poll_oneoff", poll_oneoff)
            .unwrap();

        let proc_exit = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>, rval: i32| {
            log::info!("proc_exit({})", rval);
        });
        linker
            .define("wasi_snapshot_preview1", "proc_exit", proc_exit)
            .unwrap();

        let proc_raise = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>, rval: i32| {
            log::info!("proc_raise({})", rval);
            return 0;
        });
        linker
            .define("wasi_snapshot_preview1", "proc_raise", proc_raise)
            .unwrap();

        let sched_yield = Func::wrap(&mut store, |_caller: Caller<'_, *mut FB>| {
            log::info!("sched_yield()");
            return 0;
        });
        linker
            .define("wasi_snapshot_preview1", "sched_yield", sched_yield)
            .unwrap();

        let random_get = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, buf: i32, buf_len: i32| {
                log::info!("random_get({}, {})", buf, buf_len);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "random_get", random_get)
            .unwrap();

        let sock_accept = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, flags: i32, offset0: i32| {
                log::info!("sock_accept({}, {}, {})", fd, flags, offset0);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_accept", sock_accept)
            .unwrap();

        let sock_recv = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             iov_buf: i32,
             iov_buf_len: i32,
             ri_flags: i32,
             offset0: i32,
             offset1: i32| {
                log::info!(
                    "sock_recv({}, {}, {}, {}, {}, {})",
                    fd,
                    iov_buf,
                    iov_buf_len,
                    ri_flags,
                    offset0,
                    offset1
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_recv", sock_recv)
            .unwrap();

        let sock_send = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>,
             fd: i32,
             ciov_buf: i32,
             ciov_buf_len: i32,
             si_flags: i32,
             offset0: i32| {
                log::info!(
                    "sock_send({}, {}, {}, {}, {})",
                    fd,
                    ciov_buf,
                    ciov_buf_len,
                    si_flags,
                    offset0
                );
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_send", sock_send)
            .unwrap();

        let sock_shutdown = Func::wrap(
            &mut store,
            |_caller: Caller<'_, *mut FB>, fd: i32, how: i32| {
                log::info!("sock_shutdown({}, {})", fd, how);
                return 0;
            },
        );
        linker
            .define("wasi_snapshot_preview1", "sock_shutdown", sock_shutdown)
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
            .get_typed_func::<(), ()>(&self.store, "_start");

        match start {
            Ok(start) => {
                start.call(&mut self.store, ()).unwrap();
            }
            Err(_) => {}
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
            Err(_) => {}
        }
    }
}
