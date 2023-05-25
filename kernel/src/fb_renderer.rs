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

use crate::framebuffer::Framebuffer;
use core::fmt::{self, Arguments, Write};
use limine::LimineFramebufferRequest;
use psf2::Font;
use spin::Mutex;
use vte::{Params, Parser, Perform};

struct Performer<'fb, 'font> {
    framebuffer: Framebuffer<'fb>,
    font: Font<&'font [u8]>,
    cursor_x: usize,
    cursor_y: usize,
    offset: (usize, usize),
    max: (usize, usize),
    color: u32,
    bg: u32,
}

impl<'fb, 'font> Performer<'fb, 'font> {
    pub fn new(
        framebuffer: Framebuffer<'fb>,
        font: &'font [u8],
        offset: (usize, usize),
        max: (usize, usize),
    ) -> Performer<'fb, 'font> {
        Performer {
            framebuffer,
            font: Font::new(font).unwrap(),
            cursor_x: 0,
            cursor_y: 0,
            offset,
            max,
            color: 0,
            bg: !0,
        }
    }

    pub fn write_char(&mut self, chr: char, x: usize, y: usize) {
        let chr = self.font.get_ascii(chr as u8).expect("A");

        for (y_idx, row) in chr.enumerate() {
            for (x_idx, pixel) in row.enumerate() {
                self.framebuffer.write(
                    x + x_idx + self.offset.0,
                    y + y_idx + self.offset.1,
                    self.bg,
                );
                if pixel {
                    self.framebuffer.write(
                        x + x_idx + self.offset.0,
                        y + y_idx + self.offset.1,
                        self.color,
                    );
                }
            }
        }
    }

    pub fn clear(&mut self) {
        let offset = self.offset;
        let max = self.max;

        self.framebuffer
            .clear_part(!0, offset.0, offset.1, max.0 + 3, max.1 + 3);
    }
}

impl Perform for Performer<'_, '_> {
    fn print(&mut self, chr: char) {
        self.write_char(chr, self.cursor_x, self.cursor_y);

        self.cursor_x += self.font.width() as usize;
        if self.cursor_x >= self.max.0 {
            self.cursor_x = 0;
            self.cursor_y += self.font.height() as usize;
        }

        if self.cursor_y >= self.max.1 {
            self.cursor_y = 0;
            self.cursor_x = 0;
            self.clear();
        }
    }

    fn execute(&mut self, b: u8) {
        match b {
            b'\n' => {
                self.cursor_y += self.font.height() as usize;
                self.cursor_x = 0;

                if self.cursor_y >= self.max.1 {
                    self.cursor_y = 0;
                    self.cursor_x = 0;
                    self.clear();
                }
            }
            _ => unimplemented!("Unknown byte: {b:#x}"),
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], ignore: bool, action: char) {
        if ignore {
            return;
        }

        for param in params.iter() {
            match &param {
                &[0] => self.color = 0,
                &[0x1] => {}
                &[0x25] => {}
                &[31] => {
                    self.color = u32::from_le_bytes([0, 0, 170, 255]);
                }
                &[32] => self.color = u32::from_le_bytes([0, 170, 0, 255]),
                &[33] => {
                    self.color = u32::from_le_bytes([6, 159, 255, 255]);
                }
                &[34] => {
                    self.color = u32::from_le_bytes([170, 0, 0, 255]);
                }
                &[35] => {
                    self.color = u32::from_le_bytes([170, 0, 170, 255]);
                }
                x => unimplemented!("Unknown param: {x:#x?}"),
            }
        }
    }
}

pub struct Writer<'fb, 'font> {
    parser: Parser,
    performer: Performer<'fb, 'font>,
}

impl<'fb, 'font> Writer<'fb, 'font> {
    pub fn new(
        framebuffer: Framebuffer<'fb>,
        font: &'font [u8],
        offset: (usize, usize),
        max: (usize, usize),
    ) -> Writer<'fb, 'font> {
        Writer {
            parser: Parser::new(),
            performer: Performer::new(framebuffer, font, offset, max),
        }
    }
}

impl Write for Writer<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for c in s.bytes() {
            self.parser.advance(&mut self.performer, c);
        }

        Ok(())
    }
}

static FB_INFO: LimineFramebufferRequest = LimineFramebufferRequest::new(0);
static FONT: &[u8] = include_bytes!("../cozette.psf");
static WRITER: Mutex<Option<Writer>> = Mutex::new(None);

pub fn init() {
    let mut fb = {
        let fb_info = FB_INFO.get_response().get().unwrap();
        Framebuffer::from_limine(fb_info).unwrap()
    };
    fb.clear(0x00_00_80_83);

    // Pseudo console window
    fb.clear_part(0, 100, 100, fb.width() - 200, fb.height() - 200);
    fb.clear_part(!0, 101, 101, fb.width() - 202, fb.height() - 202);
    fb.clear_part(0xE0_E0_E0_E0, 102, 102, fb.width() - 204, fb.height() - 204);
    fb.clear_part(0xE0_E0_E0_E0, 103, 103, fb.width() - 206, fb.height() - 206);
    fb.clear_part(0xB7_B7_B7_B7, 104, 104, fb.width() - 208, fb.height() - 208);
    fb.clear_part(0, 105, 105, fb.width() - 210, fb.height() - 210);
    fb.clear_part(!0, 106, 106, fb.width() - 212, fb.height() - 212);

    let width = fb.width();
    let height = fb.height();
    let writer = Writer::new(fb, FONT, (110, 110), (width - 225, height - 225));
    *WRITER.lock() = Some(writer);
}

pub unsafe fn unlock() {
    WRITER.force_unlock()
}

#[doc(hidden)]
pub fn _print(args: Arguments) {
    let mut w = WRITER.lock();
    let w = w.as_mut().unwrap();
    let _ = w.write_fmt(args);
}

#[macro_export]
macro_rules! fb_print {
    ($($arg:tt)*) => {
        $crate::fb_renderer::_print(format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! fb_println {
    ($($arg:tt)*) => {
        $crate::fb_renderer::_print(format_args_nl!($($arg)*))
    };
}
