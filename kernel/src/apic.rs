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

use crate::cpu;
use crate::hpet;
use crate::mm::{PhysAddr, VirtAddr};

/// The x2apic enable bit in the `IA32_APIC_BASE` MSR
const IA32_APIC_BASE_EXTD: u64 = 1 << 10;

/// The global enable bit in the `IA32_APIC_BASE` MSR
const IA32_APIC_BASE_EN: u64 = 1 << 11;

/// MSR for the IA32_APIC_BASE
const IA32_APIC_BASE: u32 = 0x1b;

/// Physical address we want the local APIC to be mapped at
const APIC_BASE: u64 = 0xfee0_0000;

#[derive(Clone, Copy)]
#[repr(usize)]
pub enum Register {
    EndOfInterrupt = 0xb0,
    SpuriousInterruptVector = 0xf0,
    ICRHigh = 0x310,
    ICRLow = 0x300,
    LvtTimer = 0x320,
    InitialCount = 0x380,
    CurrentCount = 0x390,
    DivideConfiguration = 0x3e0,
}

pub struct Apic {
    mode: ApicMode,
    timer_freq: usize,
}

enum ApicMode {
    XApic(VirtAddr),
    X2Apic,
}

impl Apic {
    pub fn new() -> Apic {
        unsafe {
            cpu::wrmsr(
                IA32_APIC_BASE,
                cpu::rdmsr(IA32_APIC_BASE) | IA32_APIC_BASE_EN | IA32_APIC_BASE_EXTD,
            )
        };

        let mode = ApicMode::X2Apic;

        Apic {
            mode,
            timer_freq: 0,
        }
    }

    pub fn enable(&mut self) {
        unsafe {
            self.write(Register::LvtTimer, 1 << 16);
            self.write(Register::DivideConfiguration, 0b1010);
            self.write(Register::InitialCount, 0);
            self.write(Register::SpuriousInterruptVector, 0x100 | 0xFF);

            let mut ticks = 0;

            for i in 0..16 {
                self.write(Register::InitialCount, 0xFFFFFFFF);
                hpet::sleep(10 * 1000 * 1000);
                self.write(Register::LvtTimer, 1 << 16);
                ticks += 0xFFFFFFFF - self.read(Register::CurrentCount);
            }

            log::debug!("{} APIC ticks/ms", ticks / 16);
            self.timer_freq = (ticks / 16) as usize;
        }
    }

    pub unsafe fn ipi(&mut self, dest_apic_id: u32, ipi: u32) {
        let dest_apic_id = match self.mode {
            ApicMode::XApic(_) => todo!(),
            ApicMode::X2Apic => dest_apic_id,
        };

        cpu::wrmsr(0x830, ((dest_apic_id as u64) << 32) | ipi as u64);
    }

    unsafe fn write(&mut self, register: Register, value: u32) {
        let register = register as usize;

        match self.mode {
            ApicMode::XApic(base) => {
                let addr = VirtAddr::new(base.as_u64() + register as u64);
                core::ptr::write_volatile(addr.as_mut_ptr(), value);
            }

            ApicMode::X2Apic => {
                let msr = 0x800u32 + ((register as u32) >> 4);
                cpu::wrmsr(msr, value as u64);
            }
        }
    }

    unsafe fn read(&mut self, register: Register) -> u32 {
        let register = register as usize;

        match self.mode {
            ApicMode::XApic(base) => {
                let addr = VirtAddr::new(base.as_u64() + register as u64);
                core::ptr::read_volatile(addr.as_ptr())
            }

            ApicMode::X2Apic => {
                let msr = 0x800u32 + ((register as u32) >> 4);
                cpu::rdmsr(msr) as u32
            }
        }
    }
}
