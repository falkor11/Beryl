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

use limine::{LimineSmpInfo, LimineSmpRequest};

static SMP: LimineSmpRequest = LimineSmpRequest::new(0).flags(1);

pub fn init() {
    let smp = SMP.get_response().get_mut().unwrap();

    for cpu in smp.cpus() {
        cpu.goto_address = ap_init;
    }
}

extern "C" fn ap_init(info: *const LimineSmpInfo) -> ! {
    let info = unsafe { &*info };

    crate::core_locals::init();
    crate::gdt::init();
    crate::interrupts::init();

    {
        let mut apic = core!().apic.lock();
        apic.enable();
    }

    log::info!("Hello from core: {}", info.processor_id);

    crate::hcf()
}
