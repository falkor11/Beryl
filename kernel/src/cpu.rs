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

use crate::mm::VirtAddr;

pub const IA32_GS_BASE: u32 = 0xc0000101;

pub fn get_cr2() -> VirtAddr {
    let cr2: u64;
    unsafe { core::arch::asm!("mov {}, cr2", out(reg) cr2) };
    VirtAddr::new(cr2)
}

#[inline]
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    core::arch::asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem));
}

#[inline]
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);

    core::arch::asm!("rdmsr", out("eax") low, out("edx") high, in("ecx") msr, options(nomem));

    ((high as u64) << 32) | (low as u64)
}

#[inline]
pub unsafe fn rdtsc() -> u64 {
    let (high, low): (u32, u32);

    core::arch::asm!("lfence; rdtsc", out("eax") low, out("edx") high, options(nomem));

    ((high as u64) << 32) | (low as u64)
}
