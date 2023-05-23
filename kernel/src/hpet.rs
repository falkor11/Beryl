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

use crate::acpi::sdt::SdtHeader;
use bilge::prelude::*;
use spin::Mutex;

#[bitsize(32)]
struct EventTimerBlockId {
    hardware_rev_id: u8,
    comparator_count: u5,
    counter_size: u1,
    _reserved: u1,
    legacy_replacement: u1,
    pci_vendor_id: u16,
}

#[repr(C, packed)]
struct Address {
    asid: u8,
    bit_width: u8,
    bit_offset: u8,
    _reserved: u8,
    address: u64,
}

#[repr(C, packed)]
struct HpetTable {
    event_timer_block_id: EventTimerBlockId,
    address: Address,
    hpet_number: u8,
    minimum_tick: u16,
    page_protection: u8,
}

#[bitsize(64)]
#[derive(DebugBits)]
struct HpetGeneralCaps {
    rev_id: u8,
    num_tim_cap: u5,
    count_size_cap: u1,
    _reserved: u1,
    leg_route_cap: u1,
    vendor_id: u16,
    counter_clock_period: u32,
}

#[repr(C)]
struct HpetTimerInfo {
    config_and_caps: u64,
    comparator_value: u64,
    fsb_interrupt_route: u64,
    _reserved: u64,
}

#[repr(C)]
struct HpetRegisters {
    caps: HpetGeneralCaps,
    _res0: u64,
    general_config: u64,
    _res1: u64,
    general_irq_status: u64,
    _res2: [u64; 25],
    counter_val: u64,
    _res3: u64,
    timers: [HpetTimerInfo; 32],
}

pub struct Hpet {
    regs: &'static mut HpetRegisters,
}

impl Hpet {
    fn new(table: *const SdtHeader) -> Hpet {
        let table: &HpetTable = unsafe { &*(&*table).data().cast() };
        let regs = unsafe { &mut *(table.address.address as *mut HpetRegisters) };

        log::info!("Caps: {:x?}", regs.caps);

        regs.general_config = 0;
        regs.counter_val = 0;
        regs.general_config = 1;

        Hpet { regs }
    }

    fn raw_tick_count(&self) -> u64 {
        self.regs.counter_val
    }

    fn sleep(&mut self, nano: u64) {
        let time = nano * 1_000_000 / (self.regs.caps.counter_clock_period() as u64);
        let now = self.raw_tick_count();
        let target = now + time;

        while self.raw_tick_count() < target {
            core::hint::spin_loop();
        }
    }
}

unsafe impl Sync for Hpet {}
unsafe impl Send for Hpet {}

static HPET: Mutex<Option<Hpet>> = Mutex::new(None);

pub fn init(table: *const SdtHeader) {
    log::info!("Initializing the HPET");
    *HPET.lock() = Some(Hpet::new(table));
}

pub fn sleep(nano: u64) {
    HPET.lock().as_mut().unwrap().sleep(nano)
}
