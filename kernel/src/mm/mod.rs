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
use limine::LimineHhdmRequest;

pub mod addr;
pub mod heap;
pub mod pmm;
pub mod slab;

pub use addr::*;

static HHDM_ADDRESS_REQUEST: LimineHhdmRequest = LimineHhdmRequest::new(0);

#[inline]
pub const fn align_down(addr: u64, align: u64) -> u64 {
    addr & !(align - 1)
}

#[inline]
pub const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}

pub fn init() {
    {
        let hhdm = HHDM_ADDRESS_REQUEST
            .get_response()
            .get()
            .expect("Cannot get the HHDM address");

        log::info!("HHDM @ {:#x}", hhdm.offset);

        unsafe {
            core::ptr::write(&mut HHDM_ADDRESS, hhdm.offset);
        }
    }

    pmm::init();
}
