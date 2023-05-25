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
use super::sdt::Xsdt;
use crate::mm::PhysAddr;

#[repr(C)]
pub struct Rsdp {
    signature: [u8; 8],
    checksum: u8,
    oemid: [u8; 6],
    revision: u8,
    rsdt_address: u32,
    lenght: u32,
    xsdt_address: *const Xsdt,
    ext_checksum: u8,
    _reserved: [u8; 3],
}

impl Rsdp {
    pub unsafe fn from_ptr(ptr: *const Rsdp) -> Rsdp {
        core::ptr::read_unaligned(ptr)
    }

    #[inline]
    pub const fn revision(&self) -> u8 {
        self.revision
    }

    #[inline]
    pub unsafe fn get_xsdt(&self) -> &'static Xsdt {
        Xsdt::from_phys(PhysAddr::new(self.xsdt_address as u64))
    }
}
