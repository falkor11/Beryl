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

use limine::LimineFramebufferResponse;

pub struct Framebuffer<'backing> {
    backing: &'backing mut [u32],
    width: usize,
    stride: usize,
    height: usize,
}

impl Framebuffer<'static> {
    pub fn from_limine(response: &LimineFramebufferResponse) -> Option<Framebuffer> {
        let fbs = response.framebuffers();
        let fb = fbs.first()?;

        let framebuffer_ptr = fb.address.as_ptr()? as *mut u32;
        let width = fb.width as usize;
        let stride = (fb.pitch / 4) as usize;
        let height = fb.height as usize;

        let backing = unsafe { core::slice::from_raw_parts_mut(framebuffer_ptr, stride * height) };

        Some(Framebuffer {
            backing,
            width,
            stride,
            height,
        })
    }
}

impl<'backing> Framebuffer<'backing> {
    pub fn new(width: usize, height: usize) -> Framebuffer<'backing> {
        todo!()
    }
}

impl Framebuffer<'_> {
    pub fn width(&self) -> usize {
        self.width
    }
    pub fn stride(&self) -> usize {
        self.stride
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn write(&mut self, x: usize, y: usize, color: u32) {
        self.backing[x + y * self.stride] = color;
    }

    pub fn clear(&mut self, color: u32) {
        self.backing.fill(color);
    }

    pub fn clear_part(&mut self, color: u32, x: usize, y: usize, width: usize, height: usize) {
        for cy in 0..height {
            for cx in 0..width {
                self.write(cx + x, cy + y, color);
            }
        }
    }
}
