/*
 * Beryl: A pragmatic microkernel written in rust
 * Copyright (C) 2023  Franco Longo

 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.

 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.

 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/
use super::{align_up, pmm};

pub(super) struct Slab {
    pub(super) size: usize,
    first_free: *mut *mut (),
}

impl Slab {
    pub const fn new(size: usize) -> Slab {
        Slab {
            size,
            first_free: core::ptr::null_mut(),
        }
    }

    fn init(&mut self) {
        let addr = pmm::alloc(1).as_hhdm();

        let hdr_offset = align_up(8, self.size as u64) as usize;
        let avl = 0x1000 - hdr_offset;

        let hdr = unsafe { &mut *(addr.as_mut_ptr::<*mut Slab>()) };
        *hdr = self;

        self.first_free = unsafe { addr.as_mut_ptr::<*mut ()>().add(hdr_offset) };

        let arr = self.first_free;
        let max = avl / self.size - 1;
        let fact = self.size / 8;

        for i in 0..max {
            unsafe { *arr.add(i * fact) = arr.add((i + 1) * fact).cast() };
        }
        unsafe { *arr.add(max * fact) = core::ptr::null_mut() };
    }

    pub fn alloc(&mut self) -> *mut u8 {
        if self.first_free.is_null() {
            // Initialize or add a page to the slab alloc
            self.init();
        }

        let old_free = self.first_free;
        self.first_free = unsafe { (*old_free).cast() };

        let ret: *mut u8 = old_free.cast();
        unsafe { core::ptr::write_bytes(ret, 0, self.size) };

        ret
    }

    pub fn free(&mut self, ptr: *mut u8) {
        let new_head: *mut *mut () = ptr.cast();
        unsafe { *new_head = self.first_free.cast() };
        self.first_free = new_head;
    }
}
