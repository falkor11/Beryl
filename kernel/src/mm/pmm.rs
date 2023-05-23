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
use super::PhysAddr;
use crate::utils::Bitmap;
use core::sync::atomic::{AtomicUsize, Ordering};
use limine::{LimineMemmapRequest, LimineMemoryMapEntryType};
use spin::Mutex;

static BITMAP: Mutex<Option<Bitmap>> = Mutex::new(None);
static MEMMAP: LimineMemmapRequest = LimineMemmapRequest::new(0);
static LAST_USED_INDEX: AtomicUsize = AtomicUsize::new(0);

pub(super) fn init() {
    log::info!("Initializing the pmm");

    let memmap = MEMMAP.get_response().get_mut().expect("No memory map");
    let mut highest_addr = 0u64;

    for entry in memmap.memmap() {
        log::info!(
            "Memmap entry -> base: {:#x}, len: {:#x}, typ: {:?}",
            entry.base,
            entry.len,
            entry.typ
        );

        if entry.typ == LimineMemoryMapEntryType::Usable {
            highest_addr = core::cmp::max(highest_addr, entry.base + entry.len);
        }
    }

    log::info!("Highest addr: {:#x}", highest_addr);

    let bitmap_size = super::align_up(highest_addr / 4096 / 8, 4096);
    log::info!("Bitmap size: {:#x}", bitmap_size);

    let mut bitmap: Option<Bitmap> = None;
    for entry in memmap.memmap_mut() {
        if entry.typ != LimineMemoryMapEntryType::Usable {
            continue;
        }

        if entry.len >= bitmap_size {
            let bitmap_addr = PhysAddr::new(entry.base).as_hhdm();
            let bitmap_slice = unsafe {
                core::slice::from_raw_parts_mut(bitmap_addr.as_mut_ptr(), bitmap_size as usize)
            };
            bitmap_slice.fill(0xFF);
            bitmap = Some(Bitmap::new(bitmap_slice));

            entry.len -= bitmap_size;
            entry.base += bitmap_size;
        }
    }

    let mut bitmap = bitmap.unwrap();

    for entry in memmap.memmap() {
        if entry.typ != LimineMemoryMapEntryType::Usable {
            continue;
        }

        for i in (0..entry.len).step_by(4096) {
            let idx = (entry.base + i) / 4096;
            bitmap.unset(idx as usize);
        }
    }

    *BITMAP.lock() = Some(bitmap);
}

pub fn alloc(pages: usize) -> PhysAddr {
    let ret = alloc_nozero(pages);

    unsafe {
        core::ptr::write_bytes::<u8>(ret.as_hhdm().as_mut_ptr(), 0, pages * 0x1000);
    }

    ret
}

pub fn alloc_nozero(pages: usize) -> PhysAddr {
    alloc_inner(pages).unwrap_or_else(|| {
        LAST_USED_INDEX.store(0, Ordering::Relaxed);
        alloc_inner(pages).expect("OOM")
    })
}

pub fn free(phys: PhysAddr, pages: usize) {
    let mut bitmap = BITMAP.lock();
    let bitmap = bitmap.as_mut().unwrap();

    let page = (phys.as_u64() / 0x1000) as usize;
    LAST_USED_INDEX.store(page, Ordering::Relaxed);
    for i in page..(page + pages) {
        bitmap.unset(i);
    }
}

fn alloc_inner(pages: usize) -> Option<PhysAddr> {
    let mut bitmap = BITMAP.lock();
    let bitmap = bitmap.as_mut().unwrap();

    let mut p = 0;

    while LAST_USED_INDEX.load(Ordering::Relaxed) < bitmap.len() {
        if bitmap.test(LAST_USED_INDEX.fetch_add(1, Ordering::Relaxed)) == false {
            p += 1;

            if p == pages {
                let page = LAST_USED_INDEX.load(Ordering::Relaxed) - pages;
                for i in page..LAST_USED_INDEX.load(Ordering::Relaxed) {
                    bitmap.set(i);
                }

                return Some(PhysAddr::new((page * 0x1000) as u64));
            }
        } else {
            p = 0;
        }
    }

    None
}
