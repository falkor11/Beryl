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

#![no_std]
#![no_main]
#![feature(format_args_nl)]
#![feature(decl_macro)]

use limine::LimineBootInfoRequest;

extern crate alloc;

mod acpi;
mod apic;
#[macro_use]
mod core_locals;
mod cpu;
#[macro_use]
mod fb_renderer;
mod framebuffer;
mod gdt;
mod hpet;
mod interrupts;
mod logging;
mod mm;
#[macro_use]
mod serial;
mod smp;
mod utils;

static BOOT_INFO: LimineBootInfoRequest = LimineBootInfoRequest::new(0);

#[no_mangle]
extern "C" fn _start() -> ! {
    logging::init();
    fb_renderer::init();

    log::info!("Beryl v{} loading", env!("CARGO_PKG_VERSION"));
    let boot_info = BOOT_INFO.get_response().get().unwrap();
    log::info!(
        "Booted by {:?} ({:?})",
        boot_info.name.to_str().unwrap(),
        boot_info.version.to_str().unwrap()
    );

    mm::init();
    core_locals::init();
    gdt::init();
    interrupts::init();
    acpi::init();

    {
        let mut apic = core!().apic.lock();
        apic.enable();
    }

    log::info!("Finished intializzation, starting other cores!");

    smp::init();

    hcf();
}

#[panic_handler]
fn rust_panic(info: &core::panic::PanicInfo) -> ! {
    unsafe {
        logging::unlock();
        fb_renderer::unlock();
    }

    log::error!("PANIC: {info:#?}");

    // TODO: Panic on every core

    hcf();
}

#[inline]
pub fn hcf() -> ! {
    use core::arch::asm;

    unsafe {
        asm!("cli");
        loop {
            asm!("hlt");
        }
    }
}
