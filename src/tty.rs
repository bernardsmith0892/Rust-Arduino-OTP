use arduino_hal::{hal::{port::{PD0, PD1}, Usart}, port::{Pin, mode::{Output, Input}}, clock::MHz16, pac::USART0, I2c};

pub struct TTY {
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>,
    i2c: I2c,
    buffer: [u8; 128],
    key: [u8; 256],
    key_length: usize,
    cursor_position: usize,
    digits: u8,
}

impl TTY {
    pub fn new(serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>, i2c: I2c) -> Self {
        let mut new_tty = Self {
            serial: serial,
            i2c: i2c,
            buffer: [0; 128],
            key: [0; 256],
            key_length: 0,
            cursor_position: 0,
            digits: 6,
        };

        // Attempt to load a saved key from the RTC EEPROM
        tty_commands::read_key(&mut new_tty, None);
        
        new_tty.newline();

        new_tty
    }

    fn newline(&mut self) {
        self.cursor_position = 0;
        self.buffer = [0; 128];
        ufmt::uwrite!(&mut self.serial, "\n$ ").unwrap();
    }

    pub fn wait_for_byte(&mut self) {
        let byte = self.serial.read_byte();
        self.process_byte(byte);
    }

    fn process_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                ufmt::uwrite!(&mut self.serial, "\n").unwrap();
                self.process_input();
                self.newline();
            },
            // Backspace
            b'\x08' => {
                if self.cursor_position > 0 {
                    ufmt::uwrite!(&mut self.serial, "\x08 \x08").unwrap();
                    self.cursor_position -= 1;
                }

                self.buffer[self.cursor_position] = 0;
            },
            b'\r' => { },
            _ => {
                if self.cursor_position < self.buffer.len() {
                    self.buffer[self.cursor_position] = byte;
                    self.cursor_position += 1;
                    ufmt::uwrite!(&mut self.serial, "{}", byte as char).unwrap();
                }
            },
        }
    }

    fn process_input(&mut self) {
        let args_buffer = self.buffer.clone();
        let mut args = args_buffer[0..self.cursor_position].splitn(2, |byte| *byte == b' ');
        let name = args.next();
        let params = args.next();

        for command in tty_commands::COMMANDS {
            if let Some(input_name) = name {
                if input_name.len() == command.name_length && 
                   input_name == &command.name[0..command.name_length] {
                    (command.function)(self, params);
                }
            }
        }
    }

}

mod tty_commands {
    use crate::{sha1, rtc, byte_helper};
    use avr_progmem::progmem_display as D;

    use super::TTY;

    pub struct Command {
        pub name: [u8; 5],
        pub name_length: usize,
        pub function: fn(&mut TTY, Option<&[u8]>),
    }

    #[macro_export]
    macro_rules! command {
        ($n:tt, $l:tt, $f:tt) => {
            Command {
                name: *$n,
                name_length: $l,
                function: $f,
            }
        };
    }

    pub const COMMANDS: [Command; 10] = [
        command!(b"key  ", 3, key),
        command!(b"digit", 5, digit),
        command!(b"hotp ", 4, hotp),
        command!(b"totp ", 4, totp),
        command!(b"time ", 4, time_i2c),
        command!(b"read ", 4, read_i2c),
        command!(b"write", 5, write_i2c),
        command!(b"load ", 4, read_key),
        command!(b"save ", 4, write_key),
        command!(b"help ", 4, help_screen),
    ];

    // Functions
    fn key(context: &mut TTY, param: Option<&[u8]>) {
        match param {
            Some(new_key) => {
                context.key = [0; 256];
                context.key_length = new_key.len();
                for (i, new_byte) in new_key.iter().enumerate() {
                    context.key[i] = *new_byte;
                }
            },
            None => {
                for byte in &context.key[0..context.key_length] {
                    ufmt::uwrite!(context.serial, "{}", *byte as char).unwrap();
                }
            },
        }
    }

    fn digit(context: &mut TTY, param: Option<&[u8]>) {
        match param {
            Some(digit_param) => {
                context.digits = 0;
                for (i, byte) in digit_param.iter().enumerate() {
                    context.digits += (*byte as u8 - 0x30) * 10_u8.pow(digit_param.len() as u32 - i as u32 - 1);
                }
            },
            None => {
                ufmt::uwrite!(context.serial, "{}", context.digits).unwrap();
            },
        }
    }

    fn hotp(context: &mut TTY, param: Option<&[u8]>) {
        if let Some(counter_param) = param {
            let mut counter = 0;
            for (i, byte) in counter_param.iter().enumerate() {
                counter += (*byte as u64 - 0x30) * 10_u64.pow(counter_param.len() as u32 - i as u32 - 1);
            }

            let otp = sha1::gen_sha1_hotp(&context.key[0..context.key_length], counter, context.digits as u32).unwrap();
            for i in 0..context.digits {
                let digit = otp as u64 / 10_u64.pow((context.digits - i) as u32 - 1) % 10;
                ufmt::uwrite!(&mut context.serial, "{}", digit as u8).unwrap();
            }
        }
    }
    fn totp(context: &mut TTY, _: Option<&[u8]>) {
        let timestamp = rtc::now(&mut context.i2c).unwrap().unix_timestamp();
        let counter = timestamp / 30;

        ufmt::uwriteln!(&mut context.serial, "Timestamp: {}", timestamp).unwrap();
        ufmt::uwriteln!(&mut context.serial, "Counter: {}", counter).unwrap();
        let otp = sha1::gen_sha1_hotp(&context.key[0..context.key_length], counter, context.digits as u32).unwrap();
        for i in 0..context.digits {
            let digit = otp as u64 / 10_u64.pow((context.digits - i) as u32 - 1) % 10;
            ufmt::uwrite!(&mut context.serial, "{}", digit as u8).unwrap();
        }
    }

    fn time_i2c(context: &mut TTY, param: Option<&[u8]>) {
        if let Some(timestamp_param) = param {
            let mut timestamp = 0;
            for (i, byte) in timestamp_param.iter().enumerate() {
                timestamp += (*byte as u64 - 0x30) * 10_u64.pow(timestamp_param.len() as u32 - i as u32 - 1);
            }

            let new_date = rtc::Datetime::from_timestamp(timestamp);
            let date_bytes = new_date.to_bytes();
            rtc::set(&mut context.i2c, date_bytes).unwrap();
        }

        match rtc::now(&mut context.i2c) {
            Ok(stored_date) => {
                ufmt::uwriteln!(&mut context.serial, "Date: {}/{}/{} - {}:{}:{}", 
                    stored_date.year, stored_date.month, stored_date.date, 
                    stored_date.hours, stored_date.minutes, stored_date.seconds)
                .unwrap();
                ufmt::uwriteln!(&mut context.serial, "Timestamp: {}", stored_date.unix_timestamp()).unwrap();
            }
            Err(e) => {
                ufmt::uwriteln!(&mut context.serial, "Error reading time from RTC - {:?}", e).unwrap();
            }
        }
    }

    fn read_i2c(context: &mut TTY, param: Option<&[u8]>) {
        if let Some(address_bytes) = param {
            if address_bytes.len() == 4 {
                let mut address = [0_u8; 2];
                for i in (0..address_bytes.len()).step_by(2) {
                    address[i/2] = byte_helper::hex_to_byte([address_bytes[i], address_bytes[i+1]]);
                }
                match rtc::read_byte_eeprom(&mut context.i2c, address) {
                    Ok(byte) => {
                        ufmt::uwriteln!(&mut context.serial, "Byte: {}", byte[0]).unwrap();
                    },
                    Err(e) => {
                        ufmt::uwriteln!(&mut context.serial, "Error reading RTC EEPROM: {:?}", e).unwrap();
                    },
                }
            }
        }
    }
    fn write_i2c(context: &mut TTY, param: Option<&[u8]>) {
        if let Some(input_bytes) = param {
            let mut args = input_bytes.split(|byte| byte == &b' ');
            let address_bytes = args.next();
            let data_bytes = args.next();
            if address_bytes.is_some() && data_bytes.is_some() {

                let address_bytes = address_bytes.unwrap(); 
                let data_bytes = data_bytes.unwrap(); 
                if address_bytes.len() == 4 && data_bytes.len() == 2 {
                    let address = [
                        byte_helper::hex_to_byte([address_bytes[0], address_bytes[1]]),
                        byte_helper::hex_to_byte([address_bytes[2], address_bytes[3]]),
                    ];
                    let input = byte_helper::hex_to_byte([data_bytes[0], data_bytes[1]]);

                    let write_result = rtc::write_byte_eeprom(&mut context.i2c, address, input);
                    ufmt::uwriteln!(&mut context.serial, "{:?}", write_result).unwrap();
                }
            }
        }
    }

    // Write the currently saved key to EEPROM
    fn write_key(context: &mut TTY, _: Option<&[u8]>) {
        match rtc::write_key_eeprom(&mut context.i2c, context.key_length, context.key) {
            Ok(_) => {
                ufmt::uwriteln!(&mut context.serial, "Saved key to RTC EEPROM").unwrap();
            },
            Err(e) => {
                ufmt::uwriteln!(&mut context.serial, "Error writing key to RTC EEPROM - {:?}", e).unwrap();
            },
        }
    }
    // Read the currently saved key from EEPROM
    pub fn read_key(context: &mut TTY, _: Option<&[u8]>) {
        match rtc::read_key_eeprom(&mut context.i2c) {
            Ok((length, key)) => {
                context.key_length = length;
                context.key = key;
                ufmt::uwriteln!(&mut context.serial, "Loaded key of length {} from RTC EEPROM", length).unwrap();
            },
            Err(e) => {
                ufmt::uwriteln!(&mut context.serial, "Error loading previous key from EEPROM - {:?}", e).unwrap();
            },
        }
    }

    fn help_screen(context: &mut TTY, _: Option<&[u8]>) {
        ufmt::uwriteln!(&mut context.serial, "{}",
            D!("key <OTP Key> - Set OTP key. \n\
            key - Show current OTP key. \n\
            digit <OTP Digits> - Set digits of OTP. (default is 6) \n\
            digit - Show OTP digits setting. \n\
            hotp <HOTP Counter> - Calculate OTP for a given counter value. \n\
            totp - Calculate OTP for the current time. (step of 30) \n\
            time <UNIX timestamp> - Set date and time. \n\
            time - Show current date and time. \n\
            read <xxxx> - Read RTC EEPROM at the given 2-byte address. Must provide four hex digits. \n\
            write <xxxx> <xx> - Read RTC EEPROM at the given 2-byte address. Must provide four and two hex digits. \n\
            save - Save the current key into RTC EEPROM. \n\
            load - Load the saved key from RTC EEPROM. \n\
            help - Show this help menu.")
            ).unwrap();
    }
}
