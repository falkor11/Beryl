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
use crate::mm::PhysAddr;
use core::mem::size_of;

#[repr(C)]
pub struct SdtHeader {
    signature: [u8; 4],
    lenght: u32,
    revision: u8,
    checksum: u8,
    oemid: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: [u8; 4],
    creator_revision: u32,
}

impl SdtHeader {
    pub fn signature(&self) -> &str {
        unsafe { core::str::from_utf8_unchecked(&self.signature) }
    }

    pub fn data_len(&self) -> usize {
        (self.lenght as usize) - size_of::<Self>()
    }

    pub fn data(&self) -> *const u8 {
        unsafe { (self as *const SdtHeader).add(1).cast() }
    }
}

#[repr(C)]
pub struct Xsdt {
    hdr: SdtHeader,
    tables: [u64; 0],
}

impl Xsdt {
    pub unsafe fn from_phys<'a>(ptr: PhysAddr) -> &'a Xsdt {
        &*ptr.as_hhdm().as_ptr()
    }

    pub fn len(&self) -> usize {
        self.hdr.data_len() / 8
    }

    pub fn tables(&self) -> &[*const SdtHeader] {
        unsafe { core::slice::from_raw_parts(self.hdr.data().cast(), self.len()) }
    }
}
