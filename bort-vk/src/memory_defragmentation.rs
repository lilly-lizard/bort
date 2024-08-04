use crate::MemoryAllocator;
use ash::vk;
use bort_vma::ffi;

pub struct DefragmentationContext<'a> {
    handle: ffi::VmaDefragmentationContext,
    allocator: &'a MemoryAllocator,
}

impl<'a> Drop for DefragmentationContext<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::vmaEndDefragmentation(self.allocator.handle(), self.handle, std::ptr::null_mut());
        }
    }
}

impl<'a> DefragmentationContext<'a> {
    #[inline]
    pub(crate) fn new(
        handle: ffi::VmaDefragmentationContext,
        allocator: &'a MemoryAllocator,
    ) -> Self {
        Self { handle, allocator }
    }

    /// Ends defragmentation process.
    pub fn end(self) -> ffi::VmaDefragmentationStats {
        let mut stats = ffi::VmaDefragmentationStats {
            bytesMoved: 0,
            bytesFreed: 0,
            allocationsMoved: 0,
            deviceMemoryBlocksFreed: 0,
        };
        unsafe {
            ffi::vmaEndDefragmentation(self.allocator.handle(), self.handle, &mut stats);
        }
        std::mem::forget(self);
        stats
    }

    /// Returns `false` if no more moves are possible or `true` if more defragmentations are possible.
    pub fn begin_pass(&self, mover: impl FnOnce(&mut [ffi::VmaDefragmentationMove])) -> bool {
        let mut pass_info = ffi::VmaDefragmentationPassMoveInfo {
            moveCount: 0,
            pMoves: std::ptr::null_mut(),
        };
        let result = unsafe {
            ffi::vmaBeginDefragmentationPass(self.allocator.handle(), self.handle, &mut pass_info)
        };
        if result == vk::Result::SUCCESS {
            return false;
        }
        debug_assert_eq!(result, vk::Result::INCOMPLETE);
        let moves = unsafe {
            std::slice::from_raw_parts_mut(pass_info.pMoves, pass_info.moveCount as usize)
        };
        mover(moves);

        let result = unsafe {
            ffi::vmaEndDefragmentationPass(self.allocator.handle(), self.handle, &mut pass_info)
        };

        return result == vk::Result::INCOMPLETE;
    }
}
