use arduino_hal::{hal::{port::{PD0, PD1}, Usart}, port::{Pin, mode::{Output, Input}}, clock::MHz16, pac::USART0};
use crate::sha1;

const COMMANDS: [Command; 3] = [
    Command {
        name: *b"key",
        function: | context, param | {
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
                        ufmt::uwrite!(context.serial, "{}", *byte as char);
                    }
                    ufmt::uwrite!(context.serial, "\n");
                },
            }
        },
    },
    Command {
        name: *b"dig",
        function: | context, param | {
            match param {
                Some(digit_param) => {
                    context.digits = 0;
                    for (i, byte) in digit_param.iter().enumerate() {
                        context.digits += (*byte as u8 - 0x30) * 10_u8.pow(digit_param.len() as u32 - i as u32 - 1);
                    }
                },
                None => {
                    ufmt::uwriteln!(context.serial, "{}", context.digits);
                },
            }
        },
    },
    Command {
        name: *b"otp",
        function: | context, param | {
            if let Some(counter_param) = param {
                let mut counter = 0;
                for (i, byte) in counter_param.iter().enumerate() {
                    counter += (*byte as u64 - 0x30) * 10_u64.pow(counter_param.len() as u32 - i as u32 - 1);
                }

                let otp = sha1::gen_sha1_hotp(&context.key[0..context.key_length], counter, context.digits as u32).unwrap();
                for i in 0..context.digits {
                    let digit = otp as u64 / 10_u64.pow((context.digits - i) as u32 - 1) % 10;
                    ufmt::uwrite!(&mut context.serial, "{}", digit);
                }
                ufmt::uwrite!(&mut context.serial, "\n");
            }
        },
    }
];

pub struct TTY {
    serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>,
    buffer: [u8; 128],
    key: [u8; 256],
    key_length: usize,
    cursor_position: usize,
    digits: u8,
}

impl TTY {
    pub fn new(serial: Usart<USART0, Pin<Input, PD0>, Pin<Output, PD1>, MHz16>) -> Self {
        Self {
            serial: serial,
            buffer: [0; 128],
            key: [0; 256],
            key_length: 0,
            cursor_position: 0,
            digits: 6,
        }
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
        let mut args = args_buffer[0..self.cursor_position].split(|byte| *byte == b' ');
        let name = args.next();
        let param = args.next();

        for command in COMMANDS {
            if let Some(input_name) = name {
                if input_name == command.name {
                    (command.function)(self, param);
                }
            }
        }
    }

}

struct Command {
    name: [u8; 3],
    function: fn(&mut TTY, Option<&[u8]>),
}
