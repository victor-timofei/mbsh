#![no_main]
#![no_std]

use cortex_m_rt::entry;
use heapless::Vec;
use rtt_target::{rtt_init_print, rprintln};
use panic_rtt_target as _;
use core::fmt::Write;

use microbit::hal::prelude::*;

#[cfg(feature = "v1")]
use microbit::{
    hal::twi,
    pac::twi0::frequency::FREQUENCY_A,
};

#[cfg(feature = "v2")]
use microbit::{
    hal::twim,
    pac::twim0::frequency::FREQUENCY_A,
};

use lsm303agr::{
    AccelOutputDataRate, Lsm303agr,
};

#[cfg(feature = "v1")]
use microbit::{
    hal::prelude::*,
    hal::uart,
    hal::uart::{Baudrate, Parity},
};

#[cfg(feature = "v2")]
use microbit::{
    hal::prelude::*,
    hal::uarte,
    hal::uarte::{Baudrate, Parity},
};

#[cfg(feature = "v2")]
mod serial_setup;
#[cfg(feature = "v2")]
use serial_setup::UartePort;

#[entry]
fn main() -> ! {
    rtt_init_print!();
    let board = microbit::Board::take().unwrap();


    #[cfg(feature = "v1")]
    let mut i2c = { twi::Twi::new(board.TWI0, board.i2c.into(), FREQUENCY_A::K100) };

    #[cfg(feature = "v2")]
    let mut i2c = { twim::Twim::new(board.TWIM0, board.i2c_internal.into(), FREQUENCY_A::K100) };

    #[cfg(feature = "v1")]
    let mut serial = {
        uart::Uart::new(
            board.UART0,
            board.uart.into(),
            Parity::EXCLUDED,
            Baudrate::BAUD115200,
        )
    };

    #[cfg(feature = "v2")]
    let mut serial = {
        let serial = uarte::Uarte::new(
            board.UARTE0,
            board.uart.into(),
            Parity::EXCLUDED,
            Baudrate::BAUD115200,
        );
        UartePort::new(serial)
    };

    let mut sensor = Lsm303agr::new_with_i2c(i2c);
    sensor.init().unwrap();
    sensor.set_accel_odr(AccelOutputDataRate::Hz50);

    let mut buffer: Vec<u8, 1024> = Vec::new();

    write!(serial, "{}[2J$ ", 27 as char).unwrap();

    loop {
        let byte = nb::block!(serial.read()).unwrap();
        nb::block!(serial.write(byte)).unwrap();

        if byte == b'\r' {
            nb::block!(serial.write(b'\n')).unwrap();
            while let Some(b' ') = buffer.last() {
                buffer.pop();
            }
            let cmd = core::str::from_utf8(buffer.as_slice());
            match cmd {
                core::result::Result::Ok("accelerometer") => {
                    rprintln!("INFO: Calculating accelerometer");
                    let data = sensor.accel_data();
                    match data {
                        Result::Ok(data) => write!(serial, "x: {}, y: {}, z: {}\r\n", data.x, data.y, data.z).unwrap(),
                        Result::Err(e) => {
                            rprintln!("ERR: {:?}", e);
                            write!(serial, "ERR: {:?}", e).unwrap();
                        },
                    }
                },
                core::result::Result::Ok("magnetometer") => {
                    rprintln!("INFO: Calculating magnetometer");
                    let data = nb::block!(sensor.mag_data()).unwrap();
                    write!(serial, "x: {}, y: {}, z: {}\r\n", data.x, data.y, data.z).unwrap();
                    
                },
                core::result::Result::Ok("clear") => {
                    write!(serial, "{}[2J", 27 as char).unwrap();
                },
                core::result::Result::Ok("") => {},
                core::result::Result::Ok(cmd) => write!(serial, "Unknown command: `{}`\r\n", cmd).unwrap(),
                core::result::Result::Err(e) => rprintln!("ERR: couldn't unwrap cmd: {}\r\n", e),
            }

            buffer.clear();
            write!(serial, "$ ").unwrap();
        } else if byte == 8 {
            buffer.pop();
        } else if !(buffer.len() == 0 && byte == b' ') {
            buffer.push(byte);
        }

        nb::block!(serial.flush()).unwrap();
    }
}
