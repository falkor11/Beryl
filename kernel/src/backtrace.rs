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

use limine::LimineKernelFileRequest;
use xmas_elf::symbol_table::Entry;
use xmas_elf::{
    sections::{SectionData, ShType},
    ElfFile,
};

static KERNEL_FILE: LimineKernelFileRequest = LimineKernelFileRequest::new(0);

pub fn backtrace(rbp: Option<u64>) {
    let kernel_elf = KERNEL_FILE
        .get_response()
        .get()
        .unwrap()
        .kernel_file
        .get()
        .unwrap();
    let kernel_elf = unsafe {
        core::slice::from_raw_parts(
            kernel_elf.base.as_ptr().unwrap(),
            kernel_elf.length as usize,
        )
    };
    let kernel_elf = ElfFile::new(kernel_elf).unwrap();

    let mut symbol_table = None;

    for section in kernel_elf.section_iter() {
        if section.get_type() == Ok(ShType::SymTab) {
            let section_data = section
                .get_data(&kernel_elf)
                .expect("Failed to get kernel section data information");

            if let SectionData::SymbolTable64(symtab) = section_data {
                symbol_table = Some(symtab);
            }
        }
    }
    let symbol_table = symbol_table.unwrap();

    let rbp = if let Some(rbp) = rbp {
        rbp
    } else {
        unsafe {
            let rbp: u64;
            core::arch::asm!("mov rbp, {}", out(reg) rbp);
            rbp
        }
    };

    log::info!("======== BACKTRACE ===========");

    let mut rbp: *const u64 = rbp as _;
    for i in 0.. {
        if rbp.is_null() {
            break;
        }

        let rip = unsafe { *(rbp.offset(1)) } as usize;
        let mut name = None;

        for data in symbol_table {
            let st_value = data.value() as usize;
            let st_size = data.size() as usize;

            if rip >= st_value && rip < (st_value + st_size) {
                let mangled_name = data.get_name(&kernel_elf).unwrap_or("<unknown>");
                let demangled_name = rustc_demangle::demangle(mangled_name);

                name = Some(demangled_name);
            }
        }

        if let Some(name) = name {
            log::info!("{:>2}: 0x{:016x} - {}", i, rip, name);
        } else {
            log::info!("{:>2}: 0x{:016x} - <unknown>", i, rip);
        }
        rbp = unsafe { (*rbp) as *const u64 };
    }
}
