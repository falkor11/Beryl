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

use crate::{core, core_locals, fb_print, serial_print};
use log::{Level, LevelFilter, Log, Metadata, Record};
use spin::Mutex;

static LOGGER_LOCK: Mutex<()> = Mutex::new(());
static LOGGER: Logger = Logger;

pub unsafe fn unlock() {
    LOGGER_LOCK.force_unlock()
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Trace
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let _logger = LOGGER_LOCK.lock();

            let file = record.file().unwrap_or("unknown");
            let line = record.line().unwrap_or(0);
            let level = record.level();

            macro generic_log($($arg:tt)*) {
                {
                    serial_print!("{}", format_args!($($arg)*));

                    if !matches!(record.metadata().level(), Level::Trace | Level::Debug) {
                        fb_print!("{}", format_args!($($arg)*));
                    }
                }
            }

            let core_id = if core_locals::initialized() {
                core!().id
            } else {
                0
            };
            generic_log!("\x1b[37;1m[{core_id}] {file}:{line} ");

            match record.level() {
                Level::Info => generic_log!("\x1b[32;1minfo "), // green info
                Level::Warn => generic_log!("\x1b[33;1mwarn "), // yellow warn
                Level::Error => generic_log!("\x1b[31;1merror "), // red error
                Level::Debug => generic_log!("\x1b[35;1mdebug "), // gray debug
                Level::Trace => generic_log!("\x1b[34;1mtrace "), // blue trace
            }

            generic_log!("\x1b[0m");
            generic_log!("{}\n", record.args());
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .unwrap();
}
