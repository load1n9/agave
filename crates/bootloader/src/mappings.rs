use crate::{
    jump_to_kernel,
    memory::{Frame, FrameAllocator, Page, PhysicalAddress, PteFlags, VirtualAddress},
    FrameBuffer, RuntimeContext,
};

impl RuntimeContext {
    pub(crate) fn set_up_mappings(
        &mut self,
        frame_buffer: Option<&mut FrameBuffer>,
    ) -> VirtualAddress {
        // TODO: Enable nxe and write protect bits on x86_64.

        // TODO: Depend on kernel_config?
        const STACK_SIZE: usize = 18 * 4096;

        let stack_start_address = self.page_allocator.get_free_address(STACK_SIZE);

        let stack_start = Page::containing_address(stack_start_address);
        let stack_end = {
            let end_address = stack_start_address + STACK_SIZE;
            Page::containing_address(end_address - 1)
        };

        // The +1 means the guard page isn't mapped to a frame.
        for page in (stack_start + 1)..=stack_end {
            let frame = self
                .frame_allocator
                .allocate_frame()
                .expect("failed to allocate stack frame");
            self.mapper.map(
                page,
                frame,
                PteFlags::new()
                    .present(true)
                    .writable(true)
                    .no_execute(true),
                &mut self.frame_allocator,
            );
        }

        if let Some(frame_buffer) = frame_buffer {
            let frame_buffer_start_address =
                self.page_allocator.get_free_address(frame_buffer.info.size);
            let frame_buffer_virtual_start = Page::containing_address(frame_buffer_start_address);
            let frame_buffer_virtual_end = {
                let end_address =
                    frame_buffer_virtual_start.start_address() + frame_buffer.info.size;
                Page::containing_address(end_address - 1)
            };

            let frame_buffer_physical_start =
                Frame::containing_address(PhysicalAddress::new_canonical(frame_buffer.physical));
            let frame_buffer_physical_end = {
                let end_address =
                    frame_buffer_physical_start.start_address() + frame_buffer.info.size;
                Frame::containing_address(end_address - 1)
            };

            for (page, frame) in (frame_buffer_virtual_start..=frame_buffer_virtual_end)
                .zip(frame_buffer_physical_start..frame_buffer_physical_end)
            {
                self.mapper.map(
                    page,
                    frame,
                    PteFlags::new()
                        .present(true)
                        .writable(true)
                        .no_execute(true),
                    &mut self.frame_allocator,
                );
            }

            frame_buffer.virt = frame_buffer_start_address.value();
        }

        // Identity-map the context switch function so that when it switches to the new
        // page table, it continues executing.
        self.mapper.map(
            Page::containing_address(VirtualAddress::new_canonical(jump_to_kernel as usize)),
            Frame::containing_address(PhysicalAddress::new_canonical(jump_to_kernel as usize)),
            PteFlags::new().present(true),
            &mut self.frame_allocator,
        );

        crate::memory::set_up_arch_specific_mappings(self);

        (stack_end + 1).start_address()
    }
}
