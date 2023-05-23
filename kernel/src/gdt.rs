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

use crate::interrupts::Tss;
use alloc::vec;
use core::mem::size_of;

#[repr(u16)]
pub enum SegmentSelector {
    KernelCode = 0x08,
    KernelData = 0x10,
    UserNull = 0x18,
    UserData = 0x20,
    UserCode64 = 0x28,
    Tss = 0x30,
}

pub fn init() {
    let gdt: &mut [u64] = vec![0; 8].leak();

    gdt[0] = 0;
    gdt[1] = 0x00209a0000000000; // 0x08 KC
    gdt[2] = 0x0000920000000000; // 0x10 KD
    gdt[3] = 0;
    gdt[5] = 0x0000f30000000000; // 0x20 UD
    gdt[4] = 0x0020fb0000000000; // 0x28 UC64

    let tss = core!().tss.lock();
    let tss = tss.as_ptr();

    let tss_base = tss as u64;
    let tss_low = 0x890000000000
        | (((tss_base >> 24) & 0xff) << 56)
        | ((tss_base & 0xffffff) << 16)
        | (size_of::<Tss>() as u64 - 1);
    let tss_high = tss_base >> 32;

    gdt[6] = tss_low;
    gdt[7] = tss_high;

    let gdt_desc = Descriptor {
        size: size_of::<[u64; 8]>() as u16 - 1,
        ptr: gdt.as_ptr() as u64,
    };

    unsafe {
        load_gdt(&gdt_desc);

        load_cs(SegmentSelector::KernelCode);
        load_ds(SegmentSelector::KernelData);
        load_es(SegmentSelector::KernelData);
        load_fs(SegmentSelector::KernelData);
        load_gs(SegmentSelector::KernelData);
        load_ss(SegmentSelector::KernelData);

        load_tss(SegmentSelector::Tss);
    }
}

#[inline(always)]
unsafe fn load_cs(selector: SegmentSelector) {
    core::arch::asm!(
        "push {selector}",
        "lea {tmp}, [1f + rip]",
        "push {tmp}",
        "retfq",
        "1:",
        selector = in(reg) u64::from(selector as u16),
        tmp = lateout(reg) _,
    );
}

#[inline(always)]
unsafe fn load_ds(selector: SegmentSelector) {
    core::arch::asm!("mov ds, {0:x}", in(reg) selector as u16, options(nomem, nostack))
}

#[inline(always)]
unsafe fn load_es(selector: SegmentSelector) {
    core::arch::asm!("mov es, {0:x}", in(reg) selector as u16, options(nomem, nostack))
}

#[inline(always)]
unsafe fn load_fs(selector: SegmentSelector) {
    core::arch::asm!("mov fs, {0:x}", in(reg) selector as u16, options(nomem, nostack))
}

#[inline(always)]
unsafe fn load_gs(selector: SegmentSelector) {
    core::arch::asm!("swapgs; mov gs, {0:x}; swapgs", in(reg) selector as u16, options(nomem, nostack))
}

#[inline(always)]
unsafe fn load_ss(selector: SegmentSelector) {
    core::arch::asm!("mov ss, {0:x}", in(reg) selector as u16, options(nomem, nostack))
}

#[inline(always)]
unsafe fn load_tss(selector: SegmentSelector) {
    core::arch::asm!("ltr {0:x}", in(reg) selector as u16, options(nostack, nomem));
}

#[repr(C, packed)]
struct Descriptor {
    size: u16,
    ptr: u64,
}

#[inline(always)]
unsafe fn load_gdt(gdt_descriptor: &Descriptor) {
    core::arch::asm!("lgdt [{}]", in(reg) gdt_descriptor, options(nostack));
}
