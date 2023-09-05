#![allow(dead_code)]

use crate::sys::{
    create_identity_virt_from_phys_n,
    framebuffer::{FB, RGBA},
    interrupts::global_time_ms,
    task::executor::{yield_once, Spawner},
    virtio::{Desc, Virtio},
};
use alloc::{sync::Arc, vec::Vec};
use core::{
    ptr::read_volatile,
    sync::atomic::{AtomicU64, Ordering},
};
use futures::task::AtomicWaker;
use lazy_static::lazy_static;
use spin::Mutex;

static LAST_FLUSH_MS: AtomicU64 = AtomicU64::new(0);

lazy_static! {
    pub static ref WAKERS: Mutex<[IdWaker; 256]> = Mutex::new([(); 256].map(|_| IdWaker::None));
}

#[derive(Debug)]
pub enum IdWaker {
    None,
    Done,
    Waker(AtomicWaker),
}

impl IdWaker {
    pub fn wake(&mut self) {
        let old = core::mem::replace(self, Self::Done);
        if let IdWaker::Waker(e) = old {
            e.wake();
        } else {
            log::error!("Nothing to wake");
        }
    }
}

pub async fn drive(mut virtio: Virtio, spawner: Spawner, fb: *mut FB) {
    unsafe {
        let q = 0;
        virtio.queue_select(q);
        let _queue = read_volatile(virtio.common.cap);

        let virtio = Arc::new(Mutex::new(virtio));

        let virtio_2 = Arc::clone(&virtio);
        spawner.run(async move {
            loop {
                'checkall: loop {
                    let next = { virtio_2.lock().next_used() };
                    if let Some(used) = next {
                        WAKERS.lock()[used.id as usize].wake();
                    } else {
                        break 'checkall;
                    }
                }
                yield_once().await;
            }
        });

        //Make a few free desc
        {
            let mut virtio = virtio.lock();
            for _ in 0..10 {
                if let Some(desc_id) = virtio.get_free_desc_id() {
                    virtio.set_writable_available(desc_id);
                }
            }
            yield_once().await;
        }

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCtrlHdr {
                type_: VirtioGpuCtrlType::VirtioGpuCmdGetDisplayInfo,
                ..Default::default()
            },
        )
        .await;
        let mut display_info =
            (response_desc.addr as *const VirtioGpuRespOkDisplayInfo).read_volatile();
        // log::info!("{:?}", display_info);

        {
            #[repr(C)]
            #[derive(Clone, Debug)]
            struct VirtioGpuConfig {
                events_read: u32,
                events_clear: u32,
                num_scanouts: u32,
                num_capsets: u32,
            }
            let conf_ptr: *mut VirtioGpuConfig =
                core::intrinsics::transmute((virtio.lock().device.cap) as *const ());
            let rconf = conf_ptr.read_volatile();

            for i in 0..rconf.num_capsets {
                let response_desc = request(
                    Arc::clone(&virtio),
                    VirtioGpuCmdGetCapsetInfo {
                        header: VirtioGpuCtrlHdr {
                            type_: VirtioGpuCtrlType::VirtioGpuCmdGetCapsetInfo,
                            ..Default::default()
                        },
                        capset_index: i as u32,
                        padding: 0,
                    },
                )
                .await;
                let _capsetinfo =
                    (response_desc.addr as *const VirtioGpuRespCapsetInfo).read_volatile();
                // log::info!("CAP {}, {:?}", i, capsetinfo);
            }
            yield_once().await;
        }

        // for capn in 0..display_info.pmodes.

        display_info.pmodes.rect.w = 1600;
        display_info.pmodes.rect.h = 900;

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCtrlHdr {
                type_: VirtioGpuCtrlType::VirtioGpuCmdGetEdid,
                ..Default::default()
            },
        )
        .await;
        let _edid = (response_desc.addr as *const VirtioGpuRespEdid).read_volatile();
        // log::info!("{:?}", edid);

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdResourceCreate2d {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdResourceCreate2d,
                    ..Default::default()
                },
                resource_id: 1,
                format: VirtioGpuFormats::VirtioGpuFormatR8g8b8a8Unorm,
                width: display_info.pmodes.rect.w,
                height: display_info.pmodes.rect.h,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("{:?}", nodata.type_);

        let capacity = (display_info.pmodes.rect.w * display_info.pmodes.rect.h) as usize;

        let framebuffer_bytes = capacity * 4;
        let pages_needed = 1 + framebuffer_bytes / 4096;
        let pages = create_identity_virt_from_phys_n(pages_needed).unwrap();
        let addr = pages.start_address().as_u64();
        // let framebuffer_ptr =
        //     ALLOCATOR.alloc(Layout::from_size_align_unchecked(capacity * 4, 4096));
        let mut framebuffer: Vec<RGBA> = Vec::from_raw_parts(addr as *mut RGBA, capacity, capacity);
        // log::info!("(*fb).update {:?}", addr as *mut RGBA);
        (*fb).update(
            addr as *mut RGBA,
            display_info.pmodes.rect.w as usize,
            display_info.pmodes.rect.h as usize,
        );

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdResourceAttachBacking {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdResourceAttachBacking,
                    ..Default::default()
                },
                resource_id: 1,
                nr_entries: 1,
                //mem
                addr,
                length: (core::mem::size_of::<RGBA>() * framebuffer.len()) as u32,
                padding: 0,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("{:?}", nodata.type_);

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdSetScanout {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdSetScanout,
                    ..Default::default()
                },
                r: display_info.pmodes.rect,
                resource_id: 1,
                scanout_id: 0,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("{:?}", nodata.type_);

        for i in 0..capacity {
            framebuffer[i] = RGBA {
                r: 24,
                g: 27,
                b: 36,
                a: 125,
            };
        }

        //FIRST TRANSFER AND FLUSH
        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdTransferToHost2d {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdTransferToHost2d,
                    ..Default::default()
                },
                r: display_info.pmodes.rect,
                resource_id: 1,
                padding: 0,
                offset: 0,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("{:?}", nodata.type_);

        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdResourceFlush {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdResourceFlush,
                    ..Default::default()
                },
                r: display_info.pmodes.rect,
                resource_id: 1,
                padding: 0,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("{:?}", nodata.type_);

        let mut debug_name: [char; 64] = ['1'; 64];
        let name = "Debug\0";
        for (index, e) in name.chars().enumerate() {
            debug_name[index] = e;
        }
        let response_desc = request(
            Arc::clone(&virtio),
            VirtioGpuCmdCtxCreate {
                header: VirtioGpuCtrlHdr {
                    type_: VirtioGpuCtrlType::VirtioGpuCmdCtxCreate,
                    ctx_id: 1,
                    ..Default::default()
                },
                nlen: name.len() as u32,
                debug_name,
                context_init: 0,
            },
        )
        .await;
        let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
        // log::info!("VirtioGpuCmdCtxCreate {:?}", nodata.type_);

        //FIRST 3D SUBMIT
        {
            let mut buffer: Vec<u32> = Vec::with_capacity(512);

            fn cmd_clear(
                buffer: &mut Vec<u32>,
                buffers: u32,
                rgba: [u8; 4],
                _depth: f64,
                _stencil: u32,
            ) {
                let len = 8;
                buffer.push((len << 16) + (0 << 8) + Cmd3d::VirglCcmdClear as u32);
                //Buffer select
                buffer.push(buffers);
                buffer.push(rgba[0] as u32);
                buffer.push(rgba[1] as u32);
                buffer.push(rgba[2] as u32);
                buffer.push(rgba[3] as u32);
                //Depth
                buffer.push(0);
                buffer.push(0);
                //stencil
                buffer.push(0);
            }

            fn cmd_set_framebuffer_state(buffer: &mut Vec<u32>, handles: &[u32]) {
                let len = handles.len() as u32 + 2;
                buffer.push((len << 16) + (0 << 8) + Cmd3d::VirglCcmdSetFramebufferState as u32);
                //Buffer select
                buffer.push(handles.len() as u32);
                buffer.push(0);
                for handle in handles.iter() {
                    buffer.push(*handle);
                }
            }

            fn cmd_create_surface(buffer: &mut Vec<u32>, handle: u32, format: VirglFormats) {
                let len = 5;
                buffer.push(
                    (len << 16)
                        | ((VirglObjectType::VirglObjectSurface as u32) << 8)
                        | Cmd3d::VirglCcmdCreateObject as u32,
                );
                //Buffer select
                buffer.push(handle);
                buffer.push(handle);
                buffer.push(format as u32);
                buffer.push(0);
                buffer.push(0);
            }

            let res_handle = 2;

            let mut args = VirglRendererResourceCreateArgs::default();
            args.width = 256;
            args.height = 256;
            args.handle = res_handle;
            // args.target = PipeTextureTarget::PIPE_BUFFER;
            args.bind = PIPE_BIND_SAMPLER_VIEW;

            let response_desc = request(
                Arc::clone(&virtio),
                VirtioGpuCmdResourceCreate3d {
                    header: VirtioGpuCtrlHdr {
                        type_: VirtioGpuCtrlType::VirtioGpuCmdResourceCreate3d,
                        ctx_id: 1,
                        ..Default::default()
                    },
                    args,
                    padding: 0,
                },
            )
            .await;
            let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
            // log::info!("VirtioGpuCmdResourceCreate3d {:?}", nodata.type_);

            let response_desc = request(
                Arc::clone(&virtio),
                VirtioGpuCmdCtxAttachResource {
                    header: VirtioGpuCtrlHdr {
                        type_: VirtioGpuCtrlType::VirtioGpuCmdCtxAttachResource,
                        ctx_id: 1,
                        ..Default::default()
                    },
                    handle: res_handle,
                    padding: 0,
                },
            )
            .await;
            let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();

            // log::info!("VirtioGpuCmdCtxAttachResource {:?}", nodata.type_);

            let resource_example = {
                let capacity = (256 * 256) as usize;
                let framebuffer_bytes = capacity * 4;
                let pages_needed = 1 + framebuffer_bytes / 4096;
                let pages = create_identity_virt_from_phys_n(pages_needed).unwrap();
                let addr = pages.start_address().as_u64();
                let framebuffer: Vec<RGBA> =
                    Vec::from_raw_parts(addr as *mut RGBA, capacity, capacity);
                framebuffer
            };

            let response_desc = request(
                Arc::clone(&virtio),
                VirtioGpuCmdResourceAttachBacking {
                    header: VirtioGpuCtrlHdr {
                        type_: VirtioGpuCtrlType::VirtioGpuCmdResourceAttachBacking,
                        ctx_id: 1,
                        ..Default::default()
                    },
                    resource_id: res_handle,
                    nr_entries: 1,
                    //mem
                    addr: resource_example.as_ptr() as u64,
                    length: (core::mem::size_of::<RGBA>() * resource_example.len()) as u32,
                    padding: 0,
                },
            )
            .await;
            let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
            // log::info!("{:?}", nodata.type_);

            cmd_create_surface(&mut buffer, res_handle, args.format);
            cmd_set_framebuffer_state(&mut buffer, &[res_handle]);
            cmd_clear(&mut buffer, PIPE_CLEAR_COLOR, [255, 0, 0, 255], 0.0, 0);

            let len = buffer.len() as u32;
            // pad to 512
            while buffer.len() < 512 {
                buffer.push(0);
            }
            let buffer: [u32; 512] = buffer.try_into().unwrap();
            let response_desc = request(
                Arc::clone(&virtio),
                VirtioGpuCmdSubmit3d {
                    header: VirtioGpuCtrlHdr {
                        type_: VirtioGpuCtrlType::VirtioGpuCmdSubmit3d,
                        ctx_id: 1,
                        ..Default::default()
                    },
                    len,
                    buffer,
                },
            )
            .await;
            let _nodata = (response_desc.addr as *const VirtioGpuCtrlHdr).read_volatile();
            // log::info!("VirtioGpuCmdSubmit3d {:?}", nodata.type_);
        }

        // test fb manipulation
        // let mut b: u8 = 0;

        // spawner.run(async move {
        //     loop {
        //         b = b.wrapping_add(1);
        //         for i in 0..capacity {
        //             framebuffer[i] = RGBA {
        //                 r: 100,
        //                 g: 120,
        //                 b: b.wrapping_add((i % 256) as u8),
        //                 a: 125,
        //             };
        //         }

        //         yield_once().await;
        //     }
        // });

        if true {
            let virtio_2 = Arc::clone(&virtio);
            spawner.run(async move {
                loop {
                    request(
                        Arc::clone(&virtio_2),
                        VirtioGpuCmdTransferToHost2d {
                            header: VirtioGpuCtrlHdr {
                                type_: VirtioGpuCtrlType::VirtioGpuCmdTransferToHost2d,
                                ..Default::default()
                            },
                            r: display_info.pmodes.rect,
                            resource_id: 1,
                            padding: 0,
                            offset: 0,
                        },
                    )
                    .await;
                }
            });
        }
        loop {
            use futures::join;
            join!(
                // request(
                //     Arc::clone(&virtio),
                //     VirtioGpuCmdTransferToHost2d {
                //         header: VirtioGpuCtrlHdr {
                //             type_: VirtioGpuCtrlType::VirtioGpuCmdTransferToHost2d,
                //             ..Default::default()
                //         },
                //         r: display_info.pmodes.rect,
                //         resource_id: 1,
                //         padding: 0,
                //         offset: 0,
                //     },
                //
                // ),
                request(
                    Arc::clone(&virtio),
                    VirtioGpuCmdResourceFlush {
                        header: VirtioGpuCtrlHdr {
                            type_: VirtioGpuCtrlType::VirtioGpuCmdResourceFlush,
                            ..Default::default()
                        },
                        r: display_info.pmodes.rect,
                        resource_id: 1,
                        padding: 0,
                    },
                )
            );
            let now = global_time_ms();
            let _elapsed = now - LAST_FLUSH_MS.load(Ordering::Relaxed);
            LAST_FLUSH_MS.store(now, Ordering::Relaxed);
            // log::info!("gpu start {} elapsed {}", now, elapsed);
            // while elapsed < 10 {
            //     yield_once().await;
            //     elapsed = get_time_ms() - start;
            // }

            // yield_once().await;
        }
    }
}

pub async fn request<T>(virtio: Arc<Mutex<Virtio>>, data: T) -> Desc {
    let twice = { virtio.lock().get_free_twice_desc_id() };
    if let Some((desc_id, desc_next_id)) = twice {
        {
            virtio.lock().add_request(desc_id, desc_next_id, data);
            virtio.lock().kick(0);
        }
        wait_for(desc_id as usize).await;

        {
            let v = &mut virtio.lock();
            v.set_free_desc_id(desc_id);
            v.set_free_desc_id(desc_next_id);
        }

        virtio.lock().read_desc(desc_next_id)
    } else {
        panic!("No more desc available")
    }
}

pub async fn wait_for(id: usize) {
    IdWait::new(id).await;
}

#[derive(Debug)]
pub struct IdWait {
    id: usize,
}

impl IdWait {
    pub fn new(id: usize) -> Self {
        IdWait { id }
    }
}

impl futures::future::Future for IdWait {
    type Output = ();

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        {
            let val = &mut WAKERS.lock()[self.id];
            match val {
                IdWaker::None => {
                    let aw = AtomicWaker::new();
                    aw.register(&cx.waker());
                    *val = IdWaker::Waker(aw);

                    return core::task::Poll::Pending;
                }
                IdWaker::Done => {
                    *val = IdWaker::None;
                    return core::task::Poll::Ready(());
                }
                IdWaker::Waker(e) => {
                    e.register(&cx.waker());
                }
            }
        }
        {
            let val = &mut WAKERS.lock()[self.id];
            match val {
                IdWaker::Done => {
                    *val = IdWaker::None;
                    core::task::Poll::Ready(())
                }
                _ => core::task::Poll::Pending,
            }
        }
    }
}

#[repr(u32)]
#[derive(Clone, Debug)]
enum VirtioGpuCtrlType {
    /* 2d commands */
    VirtioGpuCmdGetDisplayInfo = 0x0100,
    VirtioGpuCmdResourceCreate2d,
    VirtioGpuCmdResourceUnref,
    VirtioGpuCmdSetScanout,
    VirtioGpuCmdResourceFlush,
    VirtioGpuCmdTransferToHost2d,
    VirtioGpuCmdResourceAttachBacking,
    VirtioGpuCmdResourceDetachBacking,
    VirtioGpuCmdGetCapsetInfo,
    VirtioGpuCmdGetCapset,
    VirtioGpuCmdGetEdid,
    VirtioGpuCmdResourceAssignUuid,
    VirtioGpuCmdResourceCreateBlob,
    VirtioGpuCmdSetScanoutBlob,

    /* 3d commands */
    VirtioGpuCmdCtxCreate = 0x0200,
    VirtioGpuCmdCtxDestroy,
    VirtioGpuCmdCtxAttachResource,
    VirtioGpuCmdCtxDetachResource,
    VirtioGpuCmdResourceCreate3d,
    VirtioGpuCmdTransferToHost3d,
    VirtioGpuCmdTransferFromHost3d,
    VirtioGpuCmdSubmit3d,
    VirtioGpuCmdResourceMapBlob,
    VirtioGpuCmdResourceUnmapBlob,

    /* cursor commands */
    VirtioGpuCmdUpdateCursor = 0x0300,
    VirtioGpuCmdMoveCursor,

    /* success responses */
    VirtioGpuRespOkNoData = 0x1100,
    VirtioGpuRespOkDisplayInfo,
    VirtioGpuRespOkCapsetInfo,
    VirtioGpuRespOkCapset,
    VirtioGpuRespOkEdid,
    VirtioGpuRespOkResourceUuid,
    VirtioGpuRespOkMapInfo,

    /* error responses */
    VirtioGpuRespErrUnspec = 0x1200,
    VirtioGpuRespErrOutOfMemory,
    VirtioGpuRespErrInvalidScanoutId,
    VirtioGpuRespErrInvalidResourceId,
    VirtioGpuRespErrInvalidContextId,
    VirtioGpuRespErrInvalidParameter,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCtrlHdr {
    type_: VirtioGpuCtrlType,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    ring_idx: u8,
    padding: [u8; 3],
}

impl Default for VirtioGpuCtrlHdr {
    fn default() -> Self {
        Self {
            type_: VirtioGpuCtrlType::VirtioGpuCmdGetDisplayInfo,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
            padding: [0, 0, 0],
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct VirtioGpuRect {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
}

#[repr(C)]
#[derive(Clone, Debug, Copy)]
struct VirtioGpuDisplay {
    rect: VirtioGpuRect,
    enabled: u32,
    flags: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuRespOkDisplayInfo {
    header: VirtioGpuCtrlHdr,
    pmodes: VirtioGpuDisplay,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdGetEdid {
    header: VirtioGpuCtrlHdr,
    scanout: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuRespEdid {
    header: VirtioGpuCtrlHdr,
    size: u32,
    padding: u32,
    edid: [u8; 1024],
}

#[repr(u32)]
#[derive(Clone, Debug)]
enum VirtioGpuFormats {
    VirtioGpuFormatB8g8r8a8Unorm = 1,
    VirtioGpuFormatB8g8r8x8Unorm = 2,
    VirtioGpuFormatA8r8g8b8Unorm = 3,
    VirtioGpuFormatX8r8g8b8Unorm = 4,
    VirtioGpuFormatR8g8b8a8Unorm = 67,
    VirtioGpuFormatX8b8g8r8Unorm = 68,
    VirtioGpuFormatA8b8g8r8Unorm = 121,
    VirtioGpuFormatR8g8b8x8Unorm = 134,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdResourceCreate2d {
    header: VirtioGpuCtrlHdr,
    resource_id: u32,
    format: VirtioGpuFormats,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdResourceAttachBacking {
    header: VirtioGpuCtrlHdr,
    resource_id: u32,
    nr_entries: u32,
    addr: u64,
    length: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdSetScanout {
    header: VirtioGpuCtrlHdr,
    r: VirtioGpuRect,
    scanout_id: u32,
    resource_id: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdTransferToHost2d {
    header: VirtioGpuCtrlHdr,
    r: VirtioGpuRect,
    offset: u64,
    resource_id: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdResourceFlush {
    header: VirtioGpuCtrlHdr,
    r: VirtioGpuRect,
    resource_id: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdCtxCreate {
    header: VirtioGpuCtrlHdr,
    nlen: u32,
    context_init: u32,
    debug_name: [char; 64],
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdGetCapsetInfo {
    header: VirtioGpuCtrlHdr,
    capset_index: u32,
    padding: u32,
}

#[repr(u32)]
#[derive(Clone, Debug)]
enum CapsetId {
    VirtioGpuCapsetVirgl = 1,
    VirtioGpuCapsetVirgl2 = 2,
    VirtioGpuCapsetGfxstream = 3,
    VirtioGpuCapsetVenus = 4,
    VirtioGpuCapsetCrossDomain = 5,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuRespCapsetInfo {
    header: VirtioGpuCtrlHdr,
    capset_id: CapsetId,
    capset_max_version: u32,
    capset_max_size: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdSubmit3d {
    header: VirtioGpuCtrlHdr,
    len: u32,
    buffer: [u32; 512],
}

#[repr(u32)]
#[derive(Clone, Debug)]
pub enum Cmd3d {
    VirglCcmdNop = 0,
    VirglCcmdCreateObject = 1,
    VirglCcmdBindObject,
    VirglCcmdDestroyObject,
    VirglCcmdSetViewportState,
    VirglCcmdSetFramebufferState,
    VirglCcmdSetVertexBuffers,
    VirglCcmdClear,
    VirglCcmdDrawVbo,
    VirglCcmdResourceInlineWrite,
    VirglCcmdSetSamplerViews,
    VirglCcmdSetIndexBuffer,
    VirglCcmdSetConstantBuffer,
    VirglCcmdSetStencilRef,
    VirglCcmdSetBlendColor,
    VirglCcmdSetScissorState,
    VirglCcmdBlit,
    VirglCcmdResourceCopyRegion,
    VirglCcmdBindSamplerStates,
    VirglCcmdBeginQuery,
    VirglCcmdEndQuery,
    VirglCcmdGetQueryResult,
    VirglCcmdSetPolygonStipple,
    VirglCcmdSetClipState,
    VirglCcmdSetSampleMask,
    VirglCcmdSetStreamoutTargets,
    VirglCcmdSetRenderCondition,
    VirglCcmdSetUniformBuffer,

    VirglCcmdSetSubCtx,
    VirglCcmdCreateSubCtx,
    VirglCcmdDestroySubCtx,
    VirglCcmdBindShader,
    VirglCcmdSetTessState,
    VirglCcmdSetMinSamples,
    VirglCcmdSetShaderBuffers,
    VirglCcmdSetShaderImages,
    VirglCcmdMemoryBarrier,
    VirglCcmdLaunchGrid,
    VirglCcmdSetFramebufferStateNoAttach,
    VirglCcmdTextureBarrier,
    VirglCcmdSetAtomicBuffers,
    VirglCcmdSetDebugFlags,
    VirglCcmdGetQueryResultQbo,
    VirglCcmdTransfer3d,
    VirglCcmdEndTransfers,
    VirglCcmdCopyTransfer3d,
    VirglCcmdSetTweaks,
    VirglCcmdClearTexture,
    VirglCcmdPipeResourceCreate,
    VirglCcmdPipeResourceSetType,
    VirglCcmdGetMemoryInfo,
    VirglCcmdSendStringMarker,
    VirglCcmdLinkShader,

    /* video codec */
    VirglCcmdCreateVideoCodec,
    VirglCcmdDestroyVideoCodec,
    VirglCcmdCreateVideoBuffer,
    VirglCcmdDestroyVideoBuffer,
    VirglCcmdBeginFrame,
    VirglCcmdDecodeMacroblock,
    VirglCcmdDecodeBitstream,
    VirglCcmdEncodeBitstream,
    VirglCcmdEndFrame,

    VirglMaxCommands,
}

const PIPE_CLEAR_DEPTH: u32 = 1 << 0;
const PIPE_CLEAR_STENCIL: u32 = 1 << 1;
const PIPE_CLEAR_COLOR0: u32 = 1 << 2;
const PIPE_CLEAR_COLOR1: u32 = 1 << 3;
const PIPE_CLEAR_COLOR2: u32 = 1 << 4;
const PIPE_CLEAR_COLOR3: u32 = 1 << 5;
const PIPE_CLEAR_COLOR4: u32 = 1 << 6;
const PIPE_CLEAR_COLOR5: u32 = 1 << 7;
const PIPE_CLEAR_COLOR6: u32 = 1 << 8;
const PIPE_CLEAR_COLOR7: u32 = 1 << 9;

/** Combined flags */
/** All color buffers currently bound */
const PIPE_CLEAR_COLOR: u32 = PIPE_CLEAR_COLOR0
    | PIPE_CLEAR_COLOR1
    | PIPE_CLEAR_COLOR2
    | PIPE_CLEAR_COLOR3
    | PIPE_CLEAR_COLOR4
    | PIPE_CLEAR_COLOR5
    | PIPE_CLEAR_COLOR6
    | PIPE_CLEAR_COLOR7;

const PIPE_CLEAR_DEPTHSTENCIL: u32 = PIPE_CLEAR_DEPTH | PIPE_CLEAR_STENCIL;

#[repr(C)]
#[derive(Clone, Debug, Copy)]
pub struct VirglRendererResourceCreateArgs {
    handle: u32,
    target: PipeTextureTarget,
    format: VirglFormats,
    bind: u32,
    width: u32,
    height: u32,
    depth: u32,
    array_size: u32,
    last_level: u32,
    nr_samples: u32,
    flags: u32,
}

impl Default for VirglRendererResourceCreateArgs {
    fn default() -> Self {
        Self {
            handle: 0,
            target: PipeTextureTarget::PipeTexture2d,
            format: VirglFormats::VirglFormatR8g8b8a8Unorm,
            bind: PIPE_BIND_RENDER_TARGET,
            width: 128,
            height: 128,
            depth: 1,
            array_size: 1,
            last_level: 0,
            nr_samples: 0,
            flags: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdCtxAttachResource {
    header: VirtioGpuCtrlHdr,
    handle: u32,
    padding: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
struct VirtioGpuCmdResourceCreate3d {
    header: VirtioGpuCtrlHdr,
    args: VirglRendererResourceCreateArgs,
    padding: u32,
}

const PIPE_BIND_DEPTH_STENCIL: u32 = 1 << 0; // create_surface
const PIPE_BIND_RENDER_TARGET: u32 = 1 << 1; // create_surface
const PIPE_BIND_BLENDABLE: u32 = 1 << 2; // create_surface
const PIPE_BIND_SAMPLER_VIEW: u32 = 1 << 3; // create_sampler_view
const PIPE_BIND_VERTEX_BUFFER: u32 = 1 << 4; // set_vertex_buffers
const PIPE_BIND_INDEX_BUFFER: u32 = 1 << 5; // draw_elements
const PIPE_BIND_CONSTANT_BUFFER: u32 = 1 << 6; // set_constant_buffer
const PIPE_BIND_DISPLAY_TARGET: u32 = 1 << 8; // flush_front_buffer
const PIPE_BIND_TRANSFER_WRITE: u32 = 1 << 9; // transfer_map
const PIPE_BIND_TRANSFER_READ: u32 = 1 << 10; // transfer_map
const PIPE_BIND_STREAM_OUTPUT: u32 = 1 << 11; // set_stream_output_buffers
const PIPE_BIND_CURSOR: u32 = 1 << 16; // mouse cursor
const PIPE_BIND_CUSTOM: u32 = 1 << 17; // state-tracker/winsys usages
const PIPE_BIND_GLOBAL: u32 = 1 << 18; // set_global_binding
const PIPE_BIND_SHADER_RESOURCE: u32 = 1 << 19; // set_shader_resources
const PIPE_BIND_COMPUTE_RESOURCE: u32 = 1 << 20; // set_compute_resources
const PIPE_BIND_COMMAND_ARGS_BUFFER: u32 = 1 << 21; // pipe_draw_info.indirect
const PIPE_BIND_QUERY_BUFFER: u32 = 1 << 22; // get_query_result_resource
const PIPE_BIND_SCANOUT: u32 = 1 << 14;
const PIPE_BIND_SHARED: u32 = 1 << 15;
const PIPE_BIND_LINEAR: u32 = 1 << 21;

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum PipeTextureTarget {
    PipeBuffer = 0,
    PipeTexture1d,
    PipeTexture2d,
    PipeTexture3d,
    PipeTextureCube,
    PipeTextureRect,
    PipeTexture1dArray,
    PipeTexture2dArray,
    PipeTextureCubeArray,
    PipeMaxTextureTypes,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum VirglFormats {
    VirglFormatNone = 0,
    VirglFormatB8g8r8a8Unorm = 1,
    VirglFormatB8g8r8x8Unorm = 2,
    VirglFormatA8r8g8b8Unorm = 3,
    VirglFormatX8r8g8b8Unorm = 4,
    VirglFormatB5g5r5a1Unorm = 5,
    VirglFormatB4g4r4a4Unorm = 6,
    VirglFormatB5g6r5Unorm = 7,
    VirglFormatR10g10b10a2Unorm = 8,
    VirglFormatL8Unorm = 9,
    /**< ubyte luminance */
    VirglFormatA8Unorm = 10,
    /**< ubyte alpha */
    VirglFormatI8Unorm = 11,
    VirglFormatL8a8Unorm = 12,
    /**< ubyte alpha, luminance */
    VirglFormatL16Unorm = 13,
    /**< ushort luminance */
    VirglFormatUyvy = 14,
    VirglFormatYuyv = 15,
    VirglFormatZ16Unorm = 16,
    VirglFormatZ32Unorm = 17,
    VirglFormatZ32Float = 18,
    VirglFormatZ24UnormS8Uint = 19,
    VirglFormatS8UintZ24Unorm = 20,
    VirglFormatZ24x8Unorm = 21,
    VirglFormatX8z24Unorm = 22,
    VirglFormatS8Uint = 23,
    /**< ubyte stencil */
    VirglFormatR64Float = 24,
    VirglFormatR64g64Float = 25,
    VirglFormatR64g64b64Float = 26,
    VirglFormatR64g64b64a64Float = 27,
    VirglFormatR32Float = 28,
    VirglFormatR32g32Float = 29,
    VirglFormatR32g32b32Float = 30,
    VirglFormatR32g32b32a32Float = 31,

    VirglFormatR32Unorm = 32,
    VirglFormatR32g32Unorm = 33,
    VirglFormatR32g32b32Unorm = 34,
    VirglFormatR32g32b32a32Unorm = 35,
    VirglFormatR32Uscaled = 36,
    VirglFormatR32g32Uscaled = 37,
    VirglFormatR32g32b32Uscaled = 38,
    VirglFormatR32g32b32a32Uscaled = 39,
    VirglFormatR32Snorm = 40,
    VirglFormatR32g32Snorm = 41,
    VirglFormatR32g32b32Snorm = 42,
    VirglFormatR32g32b32a32Snorm = 43,
    VirglFormatR32Sscaled = 44,
    VirglFormatR32g32Sscaled = 45,
    VirglFormatR32g32b32Sscaled = 46,
    VirglFormatR32g32b32a32Sscaled = 47,

    VirglFormatR16Unorm = 48,
    VirglFormatR16g16Unorm = 49,
    VirglFormatR16g16b16Unorm = 50,
    VirglFormatR16g16b16a16Unorm = 51,

    VirglFormatR16Uscaled = 52,
    VirglFormatR16g16Uscaled = 53,
    VirglFormatR16g16b16Uscaled = 54,
    VirglFormatR16g16b16a16Uscaled = 55,

    VirglFormatR16Snorm = 56,
    VirglFormatR16g16Snorm = 57,
    VirglFormatR16g16b16Snorm = 58,
    VirglFormatR16g16b16a16Snorm = 59,

    VirglFormatR16Sscaled = 60,
    VirglFormatR16g16Sscaled = 61,
    VirglFormatR16g16b16Sscaled = 62,
    VirglFormatR16g16b16a16Sscaled = 63,

    VirglFormatR8Unorm = 64,
    VirglFormatR8g8Unorm = 65,
    VirglFormatR8g8b8Unorm = 66,
    VirglFormatR8g8b8a8Unorm = 67,
    VirglFormatX8b8g8r8Unorm = 68,

    VirglFormatR8Uscaled = 69,
    VirglFormatR8g8Uscaled = 70,
    VirglFormatR8g8b8Uscaled = 71,
    VirglFormatR8g8b8a8Uscaled = 72,

    VirglFormatR8Snorm = 74,
    VirglFormatR8g8Snorm = 75,
    VirglFormatR8g8b8Snorm = 76,
    VirglFormatR8g8b8a8Snorm = 77,

    VirglFormatR8Sscaled = 82,
    VirglFormatR8g8Sscaled = 83,
    VirglFormatR8g8b8Sscaled = 84,
    VirglFormatR8g8b8a8Sscaled = 85,

    VirglFormatR32Fixed = 87,
    VirglFormatR32g32Fixed = 88,
    VirglFormatR32g32b32Fixed = 89,
    VirglFormatR32g32b32a32Fixed = 90,

    VirglFormatR16Float = 91,
    VirglFormatR16g16Float = 92,
    VirglFormatR16g16b16Float = 93,
    VirglFormatR16g16b16a16Float = 94,

    VirglFormatL8Srgb = 95,
    VirglFormatL8a8Srgb = 96,
    VirglFormatR8g8b8Srgb = 97,
    VirglFormatA8b8g8r8Srgb = 98,
    VirglFormatX8b8g8r8Srgb = 99,
    VirglFormatB8g8r8a8Srgb = 100,
    VirglFormatB8g8r8x8Srgb = 101,
    VirglFormatA8r8g8b8Srgb = 102,
    VirglFormatX8r8g8b8Srgb = 103,
    VirglFormatR8g8b8a8Srgb = 104,

    /* compressed formats */
    VirglFormatDxt1Rgb = 105,
    VirglFormatDxt1Rgba = 106,
    VirglFormatDxt3Rgba = 107,
    VirglFormatDxt5Rgba = 108,

    /* sRGB, compressed */
    VirglFormatDxt1Srgb = 109,
    VirglFormatDxt1Srgba = 110,
    VirglFormatDxt3Srgba = 111,
    VirglFormatDxt5Srgba = 112,

    /* rgtc compressed */
    VirglFormatRgtc1Unorm = 113,
    VirglFormatRgtc1Snorm = 114,
    VirglFormatRgtc2Unorm = 115,
    VirglFormatRgtc2Snorm = 116,

    VirglFormatR8g8B8g8Unorm = 117,
    VirglFormatG8r8G8b8Unorm = 118,

    VirglFormatR8sg8sb8ux8uNorm = 119,
    VirglFormatR5sg5sb6uNorm = 120,

    VirglFormatA8b8g8r8Unorm = 121,
    VirglFormatB5g5r5x1Unorm = 122,
    VirglFormatR10g10b10a2Uscaled = 123,
    VirglFormatR11g11b10Float = 124,
    VirglFormatR9g9b9e5Float = 125,
    VirglFormatZ32FloatS8x24Uint = 126,
    VirglFormatR1Unorm = 127,
    VirglFormatR10g10b10x2Uscaled = 128,
    VirglFormatR10g10b10x2Snorm = 129,

    VirglFormatL4a4Unorm = 130,
    VirglFormatB10g10r10a2Unorm = 131,
    VirglFormatR10sg10sb10sa2uNorm = 132,
    VirglFormatR8g8bxSnorm = 133,
    VirglFormatR8g8b8x8Unorm = 134,
    VirglFormatB4g4r4x4Unorm = 135,
    VirglFormatX24s8Uint = 136,
    VirglFormatS8x24Uint = 137,
    VirglFormatX32S8x24Uint = 138,
    VirglFormatB2g3r3Unorm = 139,

    VirglFormatL16a16Unorm = 140,
    VirglFormatA16Unorm = 141,
    VirglFormatI16Unorm = 142,

    VirglFormatLatc1Unorm = 143,
    VirglFormatLatc1Snorm = 144,
    VirglFormatLatc2Unorm = 145,
    VirglFormatLatc2Snorm = 146,

    VirglFormatA8Snorm = 147,
    VirglFormatL8Snorm = 148,
    VirglFormatL8a8Snorm = 149,
    VirglFormatI8Snorm = 150,
    VirglFormatA16Snorm = 151,
    VirglFormatL16Snorm = 152,
    VirglFormatL16a16Snorm = 153,
    VirglFormatI16Snorm = 154,

    VirglFormatA16Float = 155,
    VirglFormatL16Float = 156,
    VirglFormatL16a16Float = 157,
    VirglFormatI16Float = 158,
    VirglFormatA32Float = 159,
    VirglFormatL32Float = 160,
    VirglFormatL32a32Float = 161,
    VirglFormatI32Float = 162,

    VirglFormatYv12 = 163,
    VirglFormatYv16 = 164,
    VirglFormatIyuv = 165,
    /**< aka I420 */
    VirglFormatNv12 = 166,
    VirglFormatNv21 = 167,

    VirglFormatA4r4Unorm = 168,
    VirglFormatR4a4Unorm = 169,
    VirglFormatR8a8Unorm = 170,
    VirglFormatA8r8Unorm = 171,

    VirglFormatR10g10b10a2Sscaled = 172,
    VirglFormatR10g10b10a2Snorm = 173,
    VirglFormatB10g10r10a2Uscaled = 174,
    VirglFormatB10g10r10a2Sscaled = 175,
    VirglFormatB10g10r10a2Snorm = 176,

    VirglFormatR8Uint = 177,
    VirglFormatR8g8Uint = 178,
    VirglFormatR8g8b8Uint = 179,
    VirglFormatR8g8b8a8Uint = 180,

    VirglFormatR8Sint = 181,
    VirglFormatR8g8Sint = 182,
    VirglFormatR8g8b8Sint = 183,
    VirglFormatR8g8b8a8Sint = 184,

    VirglFormatR16Uint = 185,
    VirglFormatR16g16Uint = 186,
    VirglFormatR16g16b16Uint = 187,
    VirglFormatR16g16b16a16Uint = 188,

    VirglFormatR16Sint = 189,
    VirglFormatR16g16Sint = 190,
    VirglFormatR16g16b16Sint = 191,
    VirglFormatR16g16b16a16Sint = 192,
    VirglFormatR32Uint = 193,
    VirglFormatR32g32Uint = 194,
    VirglFormatR32g32b32Uint = 195,
    VirglFormatR32g32b32a32Uint = 196,

    VirglFormatR32Sint = 197,
    VirglFormatR32g32Sint = 198,
    VirglFormatR32g32b32Sint = 199,
    VirglFormatR32g32b32a32Sint = 200,

    VirglFormatA8Uint = 201,
    VirglFormatI8Uint = 202,
    VirglFormatL8Uint = 203,
    VirglFormatL8a8Uint = 204,

    VirglFormatA8Sint = 205,
    VirglFormatI8Sint = 206,
    VirglFormatL8Sint = 207,
    VirglFormatL8a8Sint = 208,

    VirglFormatA16Uint = 209,
    VirglFormatI16Uint = 210,
    VirglFormatL16Uint = 211,
    VirglFormatL16a16Uint = 212,

    VirglFormatA16Sint = 213,
    VirglFormatI16Sint = 214,
    VirglFormatL16Sint = 215,
    VirglFormatL16a16Sint = 216,

    VirglFormatA32Uint = 217,
    VirglFormatI32Uint = 218,
    VirglFormatL32Uint = 219,
    VirglFormatL32a32Uint = 220,

    VirglFormatA32Sint = 221,
    VirglFormatI32Sint = 222,
    VirglFormatL32Sint = 223,
    VirglFormatL32a32Sint = 224,

    VirglFormatB10g10r10a2Uint = 225,
    VirglFormatEtc1Rgb8 = 226,
    VirglFormatR8g8R8b8Unorm = 227,
    VirglFormatG8r8B8r8Unorm = 228,
    VirglFormatR8g8b8x8Snorm = 229,

    VirglFormatR8g8b8x8Srgb = 230,

    VirglFormatR8g8b8x8Uint = 231,
    VirglFormatR8g8b8x8Sint = 232,
    VirglFormatB10g10r10x2Unorm = 233,
    VirglFormatR16g16b16x16Unorm = 234,
    VirglFormatR16g16b16x16Snorm = 235,
    VirglFormatR16g16b16x16Float = 236,
    VirglFormatR16g16b16x16Uint = 237,
    VirglFormatR16g16b16x16Sint = 238,
    VirglFormatR32g32b32x32Float = 239,
    VirglFormatR32g32b32x32Uint = 240,
    VirglFormatR32g32b32x32Sint = 241,
    VirglFormatR8a8Snorm = 242,
    VirglFormatR16a16Unorm = 243,
    VirglFormatR16a16Snorm = 244,
    VirglFormatR16a16Float = 245,
    VirglFormatR32a32Float = 246,
    VirglFormatR8a8Uint = 247,
    VirglFormatR8a8Sint = 248,
    VirglFormatR16a16Uint = 249,
    VirglFormatR16a16Sint = 250,
    VirglFormatR32a32Uint = 251,
    VirglFormatR32a32Sint = 252,

    VirglFormatR10g10b10a2Uint = 253,
    VirglFormatB5g6r5Srgb = 254,

    VirglFormatBptcRgbaUnorm = 255,
    VirglFormatBptcSrgba = 256,
    VirglFormatBptcRgbFloat = 257,
    VirglFormatBptcRgbUfloat = 258,

    VirglFormatA16l16Unorm = 262,

    VirglFormatG8r8Unorm = 263,
    VirglFormatG8r8Snorm = 264,
    VirglFormatG16r16Unorm = 265,
    VirglFormatG16r16Snorm = 266,
    VirglFormatA8b8g8r8Snorm = 267,

    VirglFormatA8l8Unorm = 259,
    VirglFormatA8l8Snorm = 260,
    VirglFormatA8l8Srgb = 261,

    // VirglFormatA1b5g5r5Unorm = 262,
    // VirglFormatA1r5g5b5Unorm = 263,
    // VirglFormatA2b10g10r10Unorm = 264,
    // VirglFormatA2r10g10b10Unorm = 265,
    // VirglFormatA4r4g4b4Unorm = 266,
    VirglFormatX8b8g8r8Snorm = 268,

    /* etc2 compressed */
    VirglFormatEtc2Rgb8 = 269,
    VirglFormatEtc2Srgb8 = 270,
    VirglFormatEtc2Rgb8a1 = 271,
    VirglFormatEtc2Srgb8a1 = 272,
    VirglFormatEtc2Rgba8 = 273,
    VirglFormatEtc2Srgba8 = 274,
    VirglFormatEtc2R11Unorm = 275,
    VirglFormatEtc2R11Snorm = 276,
    VirglFormatEtc2Rg11Unorm = 277,
    VirglFormatEtc2Rg11Snorm = 278,

    VirglFormatAstc4x4 = 279,
    VirglFormatAstc5x4 = 280,
    VirglFormatAstc5x5 = 281,
    VirglFormatAstc6x5 = 282,
    VirglFormatAstc6x6 = 283,
    VirglFormatAstc8x5 = 284,
    VirglFormatAstc8x6 = 285,
    VirglFormatAstc8x8 = 286,
    VirglFormatAstc10x5 = 287,
    VirglFormatAstc10x6 = 288,
    VirglFormatAstc10x8 = 289,
    VirglFormatAstc10x10 = 290,
    VirglFormatAstc12x10 = 291,
    VirglFormatAstc12x12 = 292,
    VirglFormatAstc4x4Srgb = 293,
    VirglFormatAstc5x4Srgb = 294,
    VirglFormatAstc5x5Srgb = 295,
    VirglFormatAstc6x5Srgb = 296,
    VirglFormatAstc6x6Srgb = 297,
    VirglFormatAstc8x5Srgb = 298,
    VirglFormatAstc8x6Srgb = 299,
    VirglFormatAstc8x8Srgb = 300,
    VirglFormatAstc10x5Srgb = 301,
    VirglFormatAstc10x6Srgb = 302,
    VirglFormatAstc10x8Srgb = 303,
    VirglFormatAstc10x10Srgb = 304,
    VirglFormatAstc12x10Srgb = 305,
    VirglFormatAstc12x12Srgb = 306,

    VirglFormatR10g10b10x2Unorm = 308,
    VirglFormatA4b4g4r4Unorm = 311,

    VirglFormatR8Srgb = 312,
    VirglFormatR8g8Srgb = 313,

    VirglFormatP010 = 314,
    VirglFormatP012 = 315,
    VirglFormatP016 = 316,

    VirglFormatB8g8r8Unorm = 317,
    VirglFormatR3g3b2Unorm = 318,
    VirglFormatR4g4b4a4Unorm = 319,
    VirglFormatR5g5b5a1Unorm = 320,
    VirglFormatR5g6b5Unorm = 321,

    VirglFormatMax, /* = PIPE_FORMAT_COUNT */

    /* Below formats must not be used in the guest. */
    VirglFormatB8g8r8x8UnormEmulated,
    VirglFormatB8g8r8a8UnormEmulated,
    VirglFormatMaxExtended,
}

impl VirglFormats {
    pub const VIRGL_FORMAT_A1B5G5R5_UNORM: VirglFormats = VirglFormats::VirglFormatA16l16Unorm;
    pub const VIRGL_FORMAT_A1R5G5B5_UNORM: VirglFormats = VirglFormats::VirglFormatG8r8Unorm;
    pub const VIRGL_FORMAT_A2B10G10R10_UNORM: VirglFormats = VirglFormats::VirglFormatG8r8Snorm;
    pub const VIRGL_FORMAT_A2R10G10B10_UNORM: VirglFormats = VirglFormats::VirglFormatG16r16Unorm;
    pub const VIRGL_FORMAT_A4R4G4B4_UNORM: VirglFormats = VirglFormats::VirglFormatG16r16Snorm;
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum VirglObjectType {
    VirglObjectNull,
    VirglObjectBlend,
    VirglObjectRasterizer,
    VirglObjectDsa,
    VirglObjectShader,
    VirglObjectVertexElements,
    VirglObjectSamplerView,
    VirglObjectSamplerState,
    VirglObjectSurface,
    VirglObjectQuery,
    VirglObjectStreamoutTarget,
    VirglObjectMsaaSurface,
    VirglMaxObjects,
}
