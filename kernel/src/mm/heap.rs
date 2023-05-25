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
use super::{addr::VirtAddr, align_up, pmm, slab::Slab};
use core::alloc::{GlobalAlloc, Layout};
use spin::Mutex;

struct Alloc {
    slabs: [Slab; 10],
    mem_used: usize,
}

impl Alloc {
    pub const fn new() -> Alloc {
        Alloc {
            slabs: [
                Slab::new(8),
                Slab::new(16),
                Slab::new(24),
                Slab::new(32),
                Slab::new(48),
                Slab::new(64),
                Slab::new(128),
                Slab::new(256),
                Slab::new(512),
                Slab::new(1024),
            ],
            mem_used: 0,
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> *mut u8 {
        self.mem_used += layout.size();
        let slab_i = [8, 16, 24, 32, 48, 64, 128, 256, 512, 1024]
            .into_iter()
            .enumerate()
            .find_map(|(i, s)| if s >= layout.size() { Some(i) } else { None });
        if let Some(i) = slab_i {
            return self.slabs[i].alloc();
        }

        let pages = align_up(layout.size() as u64, 4096) / 4096;
        let ret = pmm::alloc(pages as usize).as_hhdm();

        unsafe { ret.as_mut_ptr() }
    }

    pub fn free(&mut self, ptr: *mut u8, layout: Layout) {
        self.mem_used -= layout.size();
        if (ptr as u64) & 0xFFF == 0 {
            let pages = align_up(layout.size() as u64, 4096) / 4096;
            pmm::free(VirtAddr::new(ptr as u64).as_phys_hhdm(), pages as usize);
        }

        let slab: &mut Slab = unsafe { &mut *((ptr as u64 & !0xFFF) as *mut Slab) };
        slab.free(ptr);
    }

    pub fn realloc(&mut self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        if ptr.is_null() {
            return self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());
        }

        if (ptr as u64) & 0xFFF == 0 {
            if align_up(layout.size() as u64, 4096) == new_size as u64 {
                return ptr;
            }

            let new_ptr = self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());

            if layout.size() > new_size {
                unsafe {
                    core::ptr::copy_nonoverlapping(ptr, new_ptr, new_size);
                }
            } else {
                unsafe {
                    core::ptr::copy_nonoverlapping(ptr, new_ptr, layout.size());
                }
            }

            return new_ptr;
        }

        let slab: &mut Slab = unsafe { &mut *((ptr as u64 & !0xFFF) as *mut Slab) };

        if new_size > slab.size {
            let new_ptr = self.alloc(Layout::from_size_align(new_size, layout.align()).unwrap());

            unsafe {
                core::ptr::copy_nonoverlapping(ptr, new_ptr, slab.size);
            }

            slab.free(ptr);
            return new_ptr;
        }   

        ptr
    }
}

struct LockedAlloc(Mutex<Alloc>);

unsafe impl Send for LockedAlloc {}
unsafe impl Sync for LockedAlloc {}

unsafe impl GlobalAlloc for LockedAlloc {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        self.0.lock().alloc(l)
    }

    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        self.0.lock().free(p, l)
    }

    unsafe fn realloc(&self, p: *mut u8, l: Layout, ns: usize) -> *mut u8 {
        self.0.lock().realloc(p, l, ns)
    }
}

pub fn used() -> usize {
    GLOBAL_ALLOC.0.lock().mem_used
}

#[global_allocator]
static GLOBAL_ALLOC: LockedAlloc = LockedAlloc(Mutex::new(Alloc::new()));
