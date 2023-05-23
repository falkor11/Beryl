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

use crate::{
    apic::Apic,
    cpu::{self, IA32_GS_BASE},
    interrupts::Tss,
    mm::VirtAddr,
};
use alloc::boxed::Box;
use core::{
    mem::size_of,
    sync::atomic::{AtomicUsize, Ordering},
};
use spin::Mutex;

static CORES_ONLINE: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
pub struct CoreLocals {
    address: u64,

    pub id: usize,
    pub tss: Mutex<Box<Tss>>,
    pub apic: Mutex<Apic>,
}

trait CoreGuard: Sync + Sized {}
impl CoreGuard for CoreLocals {}

#[macro_export]
macro_rules! core {
    () => {
        $crate::core_locals::get_core_locals()
    };
}

#[inline]
pub fn initialized() -> bool {
    unsafe { cpu::rdmsr(IA32_GS_BASE) != 0 }
}

#[inline]
pub fn get_core_locals() -> &'static CoreLocals {
    unsafe {
        let ptr: usize;
        core::arch::asm!("mov {}, gs:[0]", out(reg) ptr);

        &*(ptr as *const CoreLocals)
    }
}

pub fn cores_online() -> usize {
    CORES_ONLINE.load(Ordering::SeqCst)
}

pub fn init() {
    let core_locals_ptr =
        VirtAddr::new(Box::leak(Box::new([0u8; size_of::<CoreLocals>()])).as_ptr() as u64);

    let core_locals = CoreLocals {
        address: core_locals_ptr.as_u64(),
        id: CORES_ONLINE.fetch_add(1, Ordering::SeqCst),
        tss: Mutex::new(Box::new(Tss::new())),
        apic: Mutex::new(Apic::new()),
    };

    unsafe {
        core::ptr::write(core_locals_ptr.as_mut_ptr(), core_locals);
        cpu::wrmsr(IA32_GS_BASE, core_locals_ptr.as_u64());
    }
}
