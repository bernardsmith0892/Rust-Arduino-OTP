#![no_std]
#![no_main]

use panic_halt as _;

mod sha1_tests;

pub mod byte_helper;
pub mod sha1;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 9600);

    /*
     * For examples (and inspiration), head to
     *
     *     https://github.com/Rahix/avr-hal/tree/main/examples
     *
     * NOTE: Not all examples were ported to all boards!  There is a good chance though, that code
     * for a different board can be adapted for yours.  The Arduino Uno currently has the most
     * examples available.
     */

    let mut led = pins.d13.into_output();

    let key = b"12345678901234567890";
    let mut counter = 0;
    loop {
        let otp = sha1::gen_sha1_hotp(key, counter, 6).unwrap();
        let otp_chars = byte_helper::otp6_to_chars(otp);
        for digit in otp_chars {
            ufmt::uwrite!(&mut serial, "{}", digit).unwrap();
        }
        ufmt::uwrite!(&mut serial, "\n").unwrap();
        
        // *** ufmt can only support up to u16/i16 numbers ***
        // ufmt::uwriteln!(&mut serial, "{}", 65_535_u16).unwrap();
        // ufmt::uwriteln!(&mut serial, "{}", 65_536_u16).unwrap();
        // ufmt::uwriteln!(&mut serial, "{}", 655_536_u32).unwrap();

        led.toggle();
        counter += 1;
        arduino_hal::delay_ms(1000);
    }
}
