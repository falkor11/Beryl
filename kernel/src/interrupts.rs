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
use alloc::{boxed::Box, vec};
use core::mem::size_of;
use spin::Mutex;

#[repr(C, packed)]
#[derive(Default, Clone, Copy, Debug)]
pub struct Tss {
    reserved: u32, // offset 0x00

    /// The full 64-bit canonical forms of the stack pointers (RSP) for
    /// privilege levels 0-2.
    pub rsp: [u64; 3], // offset 0x04
    reserved2: u64, // offset 0x1C

    /// The full 64-bit canonical forms of the interrupt stack table
    /// (IST) pointers.
    pub ist: [u64; 7], // offset 0x24
    reserved3: u64, // offset 0x5c
    reserved4: u16, // offset 0x64

    /// The 16-bit offset to the I/O permission bit map from the 64-bit
    /// TSS base.
    pub iomap_base: u16, // offset 0x66
}

impl Tss {
    pub fn new() -> Tss {
        let kstack = unsafe { vec![0u8; 64 * 1024].leak().as_mut_ptr().add(64 * 1024) };
        let mut ists = [0u64; 7];
        ists.iter_mut().for_each(|ist| {
            *ist = unsafe { vec![0u8; 64 * 1024].leak().as_mut_ptr().add(64 * 1024) } as u64;
        });

        Tss {
            rsp: [kstack as u64; 3],
            ist: ists,
            ..Default::default()
        }
    }

    pub fn as_ptr(&self) -> *const Tss {
        self as *const Tss
    }
}

#[derive(Clone, Copy, Default)]
#[repr(C, packed)]
pub struct IDTDescriptor {
    base_low: u16,
    code_selector: u16,
    ist: u8,
    type_attributes: u8,
    base_mid: u16,
    base_high: u32,
    reserved: u32,
}

#[allow(dead_code)]
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum ISTType {
    None = 0b0000_1110,
    KernelModeIntGate = 0b1000_1110,
    Ring1IntGate = 0b1010_1110,
    Ring2ModeIntGate = 0b1100_1110,
    UserModeIntGate = 0b1110_1110,
}

impl IDTDescriptor {
    pub fn new(ist: u8, typ: ISTType, gdt_selector: u16, handler: unsafe extern "C" fn()) -> Self {
        Self {
            code_selector: gdt_selector,
            base_low: handler as u64 as u16,
            base_mid: ((handler as u64) >> 16) as u16,
            base_high: ((handler as u64) >> 32) as u32,
            ist,
            type_attributes: typ as u8,
            reserved: 0,
        }
    }
}

pub fn init() {
    let idt: &mut [IDTDescriptor; 256] = Box::leak(Box::new([IDTDescriptor::default(); 256]));

    unsafe {
        for (i, &ist) in HANDLERS.iter().enumerate() {
            idt[i] = IDTDescriptor::new(0, ISTType::KernelModeIntGate, 0x08, ist);
        }
    }

    let desc = Descriptor {
        size: size_of::<[IDTDescriptor; 256]>() as u16,
        ptr: idt.as_ptr() as u64,
    };

    unsafe { load_idt(&desc) };
}

#[repr(C, packed)]
struct Descriptor {
    size: u16,
    ptr: u64,
}

#[inline(always)]
unsafe fn load_idt(idt_descriptor: &Descriptor) {
    core::arch::asm!("lidt [{}]", in(reg) idt_descriptor, options(nostack));
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct InterruptStack {
    pub code: u64,
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub rbp: u64,
    pub rbx: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rax: u64,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

static INTERRUPT_HANDLERS: Mutex<[Option<fn(&mut InterruptStack)>; 256]> = Mutex::new([None; 256]);

pub fn register_handler(ist: usize, handler: fn(&mut InterruptStack)) {
    INTERRUPT_HANDLERS.lock()[ist] = Some(handler);
}

#[no_mangle]
unsafe extern "C" fn generic_interrupt_handler(ist: usize, stack: *mut InterruptStack) {
    let stack = &mut *stack;

    if ist == 0xE && stack.cs & 3 == 3 {
        log::info!("USER MODE PAGE FAULT: Error code {:#x}", stack.code);
    } else if ist == 0xE {
        log::error!("KERNEL MODE PAGE FAULT: Error code {:#x}", stack.code);
    }

    let handler = {
        let handlers = INTERRUPT_HANDLERS.lock();
        handlers[ist]
    };

    match handler {
        Some(handler) => handler(stack),
        None => {
            panic!(
                r#"Interrupt {:#x}, error code {:#x} on core {}
                Registers at exception:
                    rax {:016x} rcx {:016x} rdx {:016x} rbx {:016x}
                    rsp {:016x} rbp {:016x} rsi {:016x} rdi {:016x}
                    r8  {:016x} r9  {:016x} r10 {:016x} r11 {:016x}
                    r12 {:016x} r13 {:016x} r14 {:016x} r15 {:016x}
                    rfl {:016x}
                    rip {:016x}
                    cr2 {:016x}
                    cs {:02x} ss {:02x}
                "#,
                ist,
                stack.code,
                core!().id,
                stack.rax,
                stack.rcx,
                stack.rdx,
                stack.rbx,
                stack.rsp,
                stack.rbp,
                stack.rsi,
                stack.rdi,
                stack.r8,
                stack.r9,
                stack.r10,
                stack.r11,
                stack.r12,
                stack.r13,
                stack.r14,
                stack.r15,
                stack.rflags,
                stack.rip,
                cpu::get_cr2().as_u64(),
                stack.cs,
                stack.ss
            );
        }
    }
}

extern "C" {
    fn interrupt_handler_0();
    fn interrupt_handler_1();
    fn interrupt_handler_2();
    fn interrupt_handler_3();
    fn interrupt_handler_4();
    fn interrupt_handler_5();
    fn interrupt_handler_6();
    fn interrupt_handler_7();
    fn interrupt_handler_8();
    fn interrupt_handler_9();
    fn interrupt_handler_10();
    fn interrupt_handler_11();
    fn interrupt_handler_12();
    fn interrupt_handler_13();
    fn interrupt_handler_14();
    fn interrupt_handler_15();
    fn interrupt_handler_16();
    fn interrupt_handler_17();
    fn interrupt_handler_18();
    fn interrupt_handler_19();
    fn interrupt_handler_20();
    fn interrupt_handler_21();
    fn interrupt_handler_22();
    fn interrupt_handler_23();
    fn interrupt_handler_24();
    fn interrupt_handler_25();
    fn interrupt_handler_26();
    fn interrupt_handler_27();
    fn interrupt_handler_28();
    fn interrupt_handler_29();
    fn interrupt_handler_30();
    fn interrupt_handler_31();
    fn interrupt_handler_32();
    fn interrupt_handler_33();
    fn interrupt_handler_34();
    fn interrupt_handler_35();
    fn interrupt_handler_36();
    fn interrupt_handler_37();
    fn interrupt_handler_38();
    fn interrupt_handler_39();
    fn interrupt_handler_40();
    fn interrupt_handler_41();
    fn interrupt_handler_42();
    fn interrupt_handler_43();
    fn interrupt_handler_44();
    fn interrupt_handler_45();
    fn interrupt_handler_46();
    fn interrupt_handler_47();
    fn interrupt_handler_48();
    fn interrupt_handler_49();
    fn interrupt_handler_50();
    fn interrupt_handler_51();
    fn interrupt_handler_52();
    fn interrupt_handler_53();
    fn interrupt_handler_54();
    fn interrupt_handler_55();
    fn interrupt_handler_56();
    fn interrupt_handler_57();
    fn interrupt_handler_58();
    fn interrupt_handler_59();
    fn interrupt_handler_60();
    fn interrupt_handler_61();
    fn interrupt_handler_62();
    fn interrupt_handler_63();
    fn interrupt_handler_64();
    fn interrupt_handler_65();
    fn interrupt_handler_66();
    fn interrupt_handler_67();
    fn interrupt_handler_68();
    fn interrupt_handler_69();
    fn interrupt_handler_70();
    fn interrupt_handler_71();
    fn interrupt_handler_72();
    fn interrupt_handler_73();
    fn interrupt_handler_74();
    fn interrupt_handler_75();
    fn interrupt_handler_76();
    fn interrupt_handler_77();
    fn interrupt_handler_78();
    fn interrupt_handler_79();
    fn interrupt_handler_80();
    fn interrupt_handler_81();
    fn interrupt_handler_82();
    fn interrupt_handler_83();
    fn interrupt_handler_84();
    fn interrupt_handler_85();
    fn interrupt_handler_86();
    fn interrupt_handler_87();
    fn interrupt_handler_88();
    fn interrupt_handler_89();
    fn interrupt_handler_90();
    fn interrupt_handler_91();
    fn interrupt_handler_92();
    fn interrupt_handler_93();
    fn interrupt_handler_94();
    fn interrupt_handler_95();
    fn interrupt_handler_96();
    fn interrupt_handler_97();
    fn interrupt_handler_98();
    fn interrupt_handler_99();
    fn interrupt_handler_100();
    fn interrupt_handler_101();
    fn interrupt_handler_102();
    fn interrupt_handler_103();
    fn interrupt_handler_104();
    fn interrupt_handler_105();
    fn interrupt_handler_106();
    fn interrupt_handler_107();
    fn interrupt_handler_108();
    fn interrupt_handler_109();
    fn interrupt_handler_110();
    fn interrupt_handler_111();
    fn interrupt_handler_112();
    fn interrupt_handler_113();
    fn interrupt_handler_114();
    fn interrupt_handler_115();
    fn interrupt_handler_116();
    fn interrupt_handler_117();
    fn interrupt_handler_118();
    fn interrupt_handler_119();
    fn interrupt_handler_120();
    fn interrupt_handler_121();
    fn interrupt_handler_122();
    fn interrupt_handler_123();
    fn interrupt_handler_124();
    fn interrupt_handler_125();
    fn interrupt_handler_126();
    fn interrupt_handler_127();
    fn interrupt_handler_128();
    fn interrupt_handler_129();
    fn interrupt_handler_130();
    fn interrupt_handler_131();
    fn interrupt_handler_132();
    fn interrupt_handler_133();
    fn interrupt_handler_134();
    fn interrupt_handler_135();
    fn interrupt_handler_136();
    fn interrupt_handler_137();
    fn interrupt_handler_138();
    fn interrupt_handler_139();
    fn interrupt_handler_140();
    fn interrupt_handler_141();
    fn interrupt_handler_142();
    fn interrupt_handler_143();
    fn interrupt_handler_144();
    fn interrupt_handler_145();
    fn interrupt_handler_146();
    fn interrupt_handler_147();
    fn interrupt_handler_148();
    fn interrupt_handler_149();
    fn interrupt_handler_150();
    fn interrupt_handler_151();
    fn interrupt_handler_152();
    fn interrupt_handler_153();
    fn interrupt_handler_154();
    fn interrupt_handler_155();
    fn interrupt_handler_156();
    fn interrupt_handler_157();
    fn interrupt_handler_158();
    fn interrupt_handler_159();
    fn interrupt_handler_160();
    fn interrupt_handler_161();
    fn interrupt_handler_162();
    fn interrupt_handler_163();
    fn interrupt_handler_164();
    fn interrupt_handler_165();
    fn interrupt_handler_166();
    fn interrupt_handler_167();
    fn interrupt_handler_168();
    fn interrupt_handler_169();
    fn interrupt_handler_170();
    fn interrupt_handler_171();
    fn interrupt_handler_172();
    fn interrupt_handler_173();
    fn interrupt_handler_174();
    fn interrupt_handler_175();
    fn interrupt_handler_176();
    fn interrupt_handler_177();
    fn interrupt_handler_178();
    fn interrupt_handler_179();
    fn interrupt_handler_180();
    fn interrupt_handler_181();
    fn interrupt_handler_182();
    fn interrupt_handler_183();
    fn interrupt_handler_184();
    fn interrupt_handler_185();
    fn interrupt_handler_186();
    fn interrupt_handler_187();
    fn interrupt_handler_188();
    fn interrupt_handler_189();
    fn interrupt_handler_190();
    fn interrupt_handler_191();
    fn interrupt_handler_192();
    fn interrupt_handler_193();
    fn interrupt_handler_194();
    fn interrupt_handler_195();
    fn interrupt_handler_196();
    fn interrupt_handler_197();
    fn interrupt_handler_198();
    fn interrupt_handler_199();
    fn interrupt_handler_200();
    fn interrupt_handler_201();
    fn interrupt_handler_202();
    fn interrupt_handler_203();
    fn interrupt_handler_204();
    fn interrupt_handler_205();
    fn interrupt_handler_206();
    fn interrupt_handler_207();
    fn interrupt_handler_208();
    fn interrupt_handler_209();
    fn interrupt_handler_210();
    fn interrupt_handler_211();
    fn interrupt_handler_212();
    fn interrupt_handler_213();
    fn interrupt_handler_214();
    fn interrupt_handler_215();
    fn interrupt_handler_216();
    fn interrupt_handler_217();
    fn interrupt_handler_218();
    fn interrupt_handler_219();
    fn interrupt_handler_220();
    fn interrupt_handler_221();
    fn interrupt_handler_222();
    fn interrupt_handler_223();
    fn interrupt_handler_224();
    fn interrupt_handler_225();
    fn interrupt_handler_226();
    fn interrupt_handler_227();
    fn interrupt_handler_228();
    fn interrupt_handler_229();
    fn interrupt_handler_230();
    fn interrupt_handler_231();
    fn interrupt_handler_232();
    fn interrupt_handler_233();
    fn interrupt_handler_234();
    fn interrupt_handler_235();
    fn interrupt_handler_236();
    fn interrupt_handler_237();
    fn interrupt_handler_238();
    fn interrupt_handler_239();
    fn interrupt_handler_240();
    fn interrupt_handler_241();
    fn interrupt_handler_242();
    fn interrupt_handler_243();
    fn interrupt_handler_244();
    fn interrupt_handler_245();
    fn interrupt_handler_246();
    fn interrupt_handler_247();
    fn interrupt_handler_248();
    fn interrupt_handler_249();
    fn interrupt_handler_250();
    fn interrupt_handler_251();
    fn interrupt_handler_252();
    fn interrupt_handler_253();
    fn interrupt_handler_254();
    fn interrupt_handler_255();
}

static HANDLERS: [unsafe extern "C" fn(); 256] = [
    interrupt_handler_0,
    interrupt_handler_1,
    interrupt_handler_2,
    interrupt_handler_3,
    interrupt_handler_4,
    interrupt_handler_5,
    interrupt_handler_6,
    interrupt_handler_7,
    interrupt_handler_8,
    interrupt_handler_9,
    interrupt_handler_10,
    interrupt_handler_11,
    interrupt_handler_12,
    interrupt_handler_13,
    interrupt_handler_14,
    interrupt_handler_15,
    interrupt_handler_16,
    interrupt_handler_17,
    interrupt_handler_18,
    interrupt_handler_19,
    interrupt_handler_20,
    interrupt_handler_21,
    interrupt_handler_22,
    interrupt_handler_23,
    interrupt_handler_24,
    interrupt_handler_25,
    interrupt_handler_26,
    interrupt_handler_27,
    interrupt_handler_28,
    interrupt_handler_29,
    interrupt_handler_30,
    interrupt_handler_31,
    interrupt_handler_32,
    interrupt_handler_33,
    interrupt_handler_34,
    interrupt_handler_35,
    interrupt_handler_36,
    interrupt_handler_37,
    interrupt_handler_38,
    interrupt_handler_39,
    interrupt_handler_40,
    interrupt_handler_41,
    interrupt_handler_42,
    interrupt_handler_43,
    interrupt_handler_44,
    interrupt_handler_45,
    interrupt_handler_46,
    interrupt_handler_47,
    interrupt_handler_48,
    interrupt_handler_49,
    interrupt_handler_50,
    interrupt_handler_51,
    interrupt_handler_52,
    interrupt_handler_53,
    interrupt_handler_54,
    interrupt_handler_55,
    interrupt_handler_56,
    interrupt_handler_57,
    interrupt_handler_58,
    interrupt_handler_59,
    interrupt_handler_60,
    interrupt_handler_61,
    interrupt_handler_62,
    interrupt_handler_63,
    interrupt_handler_64,
    interrupt_handler_65,
    interrupt_handler_66,
    interrupt_handler_67,
    interrupt_handler_68,
    interrupt_handler_69,
    interrupt_handler_70,
    interrupt_handler_71,
    interrupt_handler_72,
    interrupt_handler_73,
    interrupt_handler_74,
    interrupt_handler_75,
    interrupt_handler_76,
    interrupt_handler_77,
    interrupt_handler_78,
    interrupt_handler_79,
    interrupt_handler_80,
    interrupt_handler_81,
    interrupt_handler_82,
    interrupt_handler_83,
    interrupt_handler_84,
    interrupt_handler_85,
    interrupt_handler_86,
    interrupt_handler_87,
    interrupt_handler_88,
    interrupt_handler_89,
    interrupt_handler_90,
    interrupt_handler_91,
    interrupt_handler_92,
    interrupt_handler_93,
    interrupt_handler_94,
    interrupt_handler_95,
    interrupt_handler_96,
    interrupt_handler_97,
    interrupt_handler_98,
    interrupt_handler_99,
    interrupt_handler_100,
    interrupt_handler_101,
    interrupt_handler_102,
    interrupt_handler_103,
    interrupt_handler_104,
    interrupt_handler_105,
    interrupt_handler_106,
    interrupt_handler_107,
    interrupt_handler_108,
    interrupt_handler_109,
    interrupt_handler_110,
    interrupt_handler_111,
    interrupt_handler_112,
    interrupt_handler_113,
    interrupt_handler_114,
    interrupt_handler_115,
    interrupt_handler_116,
    interrupt_handler_117,
    interrupt_handler_118,
    interrupt_handler_119,
    interrupt_handler_120,
    interrupt_handler_121,
    interrupt_handler_122,
    interrupt_handler_123,
    interrupt_handler_124,
    interrupt_handler_125,
    interrupt_handler_126,
    interrupt_handler_127,
    interrupt_handler_128,
    interrupt_handler_129,
    interrupt_handler_130,
    interrupt_handler_131,
    interrupt_handler_132,
    interrupt_handler_133,
    interrupt_handler_134,
    interrupt_handler_135,
    interrupt_handler_136,
    interrupt_handler_137,
    interrupt_handler_138,
    interrupt_handler_139,
    interrupt_handler_140,
    interrupt_handler_141,
    interrupt_handler_142,
    interrupt_handler_143,
    interrupt_handler_144,
    interrupt_handler_145,
    interrupt_handler_146,
    interrupt_handler_147,
    interrupt_handler_148,
    interrupt_handler_149,
    interrupt_handler_150,
    interrupt_handler_151,
    interrupt_handler_152,
    interrupt_handler_153,
    interrupt_handler_154,
    interrupt_handler_155,
    interrupt_handler_156,
    interrupt_handler_157,
    interrupt_handler_158,
    interrupt_handler_159,
    interrupt_handler_160,
    interrupt_handler_161,
    interrupt_handler_162,
    interrupt_handler_163,
    interrupt_handler_164,
    interrupt_handler_165,
    interrupt_handler_166,
    interrupt_handler_167,
    interrupt_handler_168,
    interrupt_handler_169,
    interrupt_handler_170,
    interrupt_handler_171,
    interrupt_handler_172,
    interrupt_handler_173,
    interrupt_handler_174,
    interrupt_handler_175,
    interrupt_handler_176,
    interrupt_handler_177,
    interrupt_handler_178,
    interrupt_handler_179,
    interrupt_handler_180,
    interrupt_handler_181,
    interrupt_handler_182,
    interrupt_handler_183,
    interrupt_handler_184,
    interrupt_handler_185,
    interrupt_handler_186,
    interrupt_handler_187,
    interrupt_handler_188,
    interrupt_handler_189,
    interrupt_handler_190,
    interrupt_handler_191,
    interrupt_handler_192,
    interrupt_handler_193,
    interrupt_handler_194,
    interrupt_handler_195,
    interrupt_handler_196,
    interrupt_handler_197,
    interrupt_handler_198,
    interrupt_handler_199,
    interrupt_handler_200,
    interrupt_handler_201,
    interrupt_handler_202,
    interrupt_handler_203,
    interrupt_handler_204,
    interrupt_handler_205,
    interrupt_handler_206,
    interrupt_handler_207,
    interrupt_handler_208,
    interrupt_handler_209,
    interrupt_handler_210,
    interrupt_handler_211,
    interrupt_handler_212,
    interrupt_handler_213,
    interrupt_handler_214,
    interrupt_handler_215,
    interrupt_handler_216,
    interrupt_handler_217,
    interrupt_handler_218,
    interrupt_handler_219,
    interrupt_handler_220,
    interrupt_handler_221,
    interrupt_handler_222,
    interrupt_handler_223,
    interrupt_handler_224,
    interrupt_handler_225,
    interrupt_handler_226,
    interrupt_handler_227,
    interrupt_handler_228,
    interrupt_handler_229,
    interrupt_handler_230,
    interrupt_handler_231,
    interrupt_handler_232,
    interrupt_handler_233,
    interrupt_handler_234,
    interrupt_handler_235,
    interrupt_handler_236,
    interrupt_handler_237,
    interrupt_handler_238,
    interrupt_handler_239,
    interrupt_handler_240,
    interrupt_handler_241,
    interrupt_handler_242,
    interrupt_handler_243,
    interrupt_handler_244,
    interrupt_handler_245,
    interrupt_handler_246,
    interrupt_handler_247,
    interrupt_handler_248,
    interrupt_handler_249,
    interrupt_handler_250,
    interrupt_handler_251,
    interrupt_handler_252,
    interrupt_handler_253,
    interrupt_handler_254,
    interrupt_handler_255,
];
