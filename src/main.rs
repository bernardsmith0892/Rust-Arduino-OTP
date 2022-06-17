#![no_std]
#![no_main]

use panic_halt as _;

mod sha1_tests;

pub mod tty;
pub mod rtc;
pub mod byte_helper;
pub mod sha1;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    /*
     * For examples (and inspiration), head to
     *
     *     https://github.com/Rahix/avr-hal/tree/main/examples
     *
     * NOTE: Not all examples were ported to all boards!  There is a good chance though, that code
     * for a different board can be adapted for yours.  The Arduino Uno currently has the most
     * examples available.
     */

    // let mut led = pins.d13.into_output();

    let mut tty = tty::TTY::new(
        arduino_hal::default_serial!(dp, pins, 9600), 
        arduino_hal::I2c::new(
            dp.TWI,
            pins.a4.into_pull_up_input(),
            pins.a5.into_pull_up_input(),
            50000
        )
    );

    // ufmt::uwriteln!(&mut serial, "Write direction test:\r").unwrap();
    // i2c.i2cdetect(&mut serial, arduino_hal::i2c::Direction::Write)
        // .unwrap();
    // ufmt::uwriteln!(&mut serial, "\r\nRead direction test:\r").unwrap();
    // i2c.i2cdetect(&mut serial, arduino_hal::i2c::Direction::Read)
        // .unwrap();
    loop {
        // if let Ok(date) = rtc::now(&mut i2c) {
            // ufmt::uwriteln!(&mut serial, "{}", date.unix_timestamp()).unwrap();
            // //ufmt::uwriteln!(&mut serial, "{}-{}-{} - {}:{}:{}", date.year, date.month, date.date, date.hours, date.minutes, date.seconds).unwrap();
        // }

        // let mut buffer = [0_u8; 7];
        // i2c.write_read(0x68, &[0x00], &mut buffer).unwrap();

        // for (addr, byte) in buffer.iter().enumerate() {
            // ufmt::uwrite!(&mut serial, "{}, ", byte).unwrap();
        // }
        // ufmt::uwriteln!(&mut serial, "").unwrap();
        // arduino_hal::delay_ms(1000);
        tty.wait_for_byte();

        //led.toggle();
        //arduino_hal::delay_ms(10);
        //led.toggle();
    }
}
