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
use crate::hpet;
use limine::LimineRsdpRequest;
use rsdp::Rsdp;
use sdt::{SdtHeader, Xsdt};
use spin::Mutex;

mod rsdp;
pub mod sdt;

static RSDP_REQ: LimineRsdpRequest = LimineRsdpRequest::new(0);
static XSDT: Mutex<Option<&'static Xsdt>> = Mutex::new(None);

pub fn init() {
    let rsdp = RSDP_REQ.get_response().get().unwrap();
    let rsdp: *const Rsdp = rsdp.address.as_ptr().unwrap().cast();
    let rsdp = unsafe { Rsdp::from_ptr(rsdp) };
    assert!(rsdp.revision() >= 2);

    let xsdt = unsafe { rsdp.get_xsdt() };
    *XSDT.lock() = Some(xsdt);

    for &table in xsdt.tables() {
        let signature = unsafe { &*table }.signature();
        log::info!("Table @ {table:#p} {signature}");

        if signature == "HPET" {
            hpet::init(table);
        }
    }
}

pub fn get_table(signature: &str, index: usize) -> Option<*const SdtHeader> {
    if signature == "DSDT" {
        #[repr(C, packed)]
        struct Fadt {
            firmware_ctrl: u32,
            dsdt: u32,
        }

        let fadt = get_table("FACP", 0)?;
        let fadt: Fadt = unsafe { core::ptr::read_unaligned((*fadt).data().cast()) };
        return Some(PhysAddr::new(fadt.dsdt as u64).as_hhdm().as_ptr()) 
    }

    let xsdt = XSDT.lock();
    let xsdt = xsdt.as_ref().unwrap();

    xsdt
        .tables()
        .iter()
        .filter(|&&p| unsafe { &*p }.signature() == signature)
        .nth(index)
        .copied()
}
