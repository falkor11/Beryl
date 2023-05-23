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
use crate::hpet;
use limine::LimineRsdpRequest;
use rsdp::Rsdp;

mod rsdp;
pub mod sdt;

static RSDP_REQ: LimineRsdpRequest = LimineRsdpRequest::new(0);

pub fn init() {
    let rsdp = RSDP_REQ.get_response().get().unwrap();
    let rsdp: *const Rsdp = rsdp.address.as_ptr().unwrap().cast();
    let rsdp = unsafe { Rsdp::from_ptr(rsdp) };
    assert!(rsdp.revision() >= 2);
    let xsdt = unsafe { rsdp.get_xsdt() };
    for &table in xsdt.tables() {
        let signature = unsafe { &*table }.signature();
        log::info!("Table @ {table:#p} {signature}");

        if signature == "HPET" {
            hpet::init(table);
        }
    }
}
