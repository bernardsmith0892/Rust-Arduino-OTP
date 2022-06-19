use arduino_hal::I2c;
use embedded_hal::prelude::{_embedded_hal_blocking_i2c_WriteRead, _embedded_hal_blocking_i2c_Write};
use ufmt::derive::uDebug;

use crate::byte_helper;

const DS3231_I2C_ADDRESS: u8 = 0x68;
const EEPROM_I2C_ADDRESS: u8 = 0x57;

#[derive(uDebug)]
pub struct Datetime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub date: u8,
    pub month: u8,
    pub year: u32,
}

const SECONDS_PER_MINUTE: u8 = 60;
const SECONDS_PER_HOUR: u32 = 3_600;
const SECONDS_PER_DAY: u32 = 86_400;

const DAYS_PER_YEAR: u16 = 365;
const DAYS_PER_LEAP_YEAR: u16 = DAYS_PER_YEAR + 1;
const DAYS_PER_MONTH: [u8; 12] = [
    31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31
];
fn days_this_month(month: u8, year: u32) -> u8 {
    match month - 1 {
        2 if is_leap_year(year) => DAYS_PER_MONTH[month as usize] + 1,
        _ => DAYS_PER_MONTH[month as usize],
    }
}

const EPOCH: Datetime = Datetime {
    seconds: 0, minutes: 0, hours: 0,
    date: 1, month: 1, year: 1970,
};

impl Datetime {
    pub fn unix_timestamp(self) -> u64 {
        let mut days: u32 = 0;
        days += days_since_epoch(self.year);
        days += days_this_year(self.year, self.month, self.date);

        let mut seconds: u64 = 0;
        seconds += days as u64 * SECONDS_PER_DAY as u64;
        seconds += seconds_this_day(self.hours, self.minutes, self.seconds) as u64;

        seconds
    }

    pub fn from_timestamp(timestamp: u64) -> Self {
        let mut days: u32 = (timestamp / SECONDS_PER_DAY as u64) as u32;
        let mut seconds: u32 = (timestamp % SECONDS_PER_DAY as u64) as u32;

        // Calculate years since Epoch
        let mut current_year: u32 = EPOCH.year;
        while days > DAYS_PER_YEAR as u32 {
            days -= match is_leap_year(current_year) {
                true => DAYS_PER_LEAP_YEAR as u32,
                false => DAYS_PER_YEAR as u32,
            };
            
            current_year += 1;
        }

        // Calculate months from the start of this year
        let mut months: u8 = 1;
        while days > DAYS_PER_MONTH[0] as u32 {
            days -= days_this_month(months, current_year) as u32;
            months += 1;
        }

        // Calculate hours from the start of the day
        let hours: u32 = seconds / SECONDS_PER_HOUR as u32;
        seconds %= SECONDS_PER_HOUR as u32;
        

        // Calculate minutes from the start of the hour
        let minutes: u32 = seconds / SECONDS_PER_MINUTE as u32;
        seconds %= SECONDS_PER_MINUTE as u32;

        Datetime { seconds: seconds as u8, minutes: minutes as u8, hours: hours as u8, date: days as u8, month: months as u8, year: current_year }
    }

    pub fn from_bytes(bytes: [u8; 7]) -> Self {
        let [seconds_byte, minutes_byte, 
            hours_byte, _, date_byte, 
            month_byte, year_byte] = bytes;
        
        let bcd_seconds = [seconds_byte >> 4, seconds_byte & 0b1111];
        let bcd_minutes = [minutes_byte >> 4, minutes_byte & 0b1111];
        let military_time = (hours_byte & 0b100_0000) == 0b100_00000;
        let bcd_hours = [(hours_byte & 0b11_0000) >> 4, hours_byte & 0b1111];
        let bcd_date = [date_byte >> 4, date_byte & 0b1111];
        let bcd_month = [(month_byte & 0b1_0000) >> 4, month_byte & 0b1111];
        let century = ((month_byte & 0b1000_0000) >> 7) * 100;
        let bcd_year = [year_byte >> 4, year_byte & 0b1111];

        Datetime { 
            seconds: bcd_seconds[0] * 10 + bcd_seconds[1],
            minutes: bcd_minutes[0] * 10 + bcd_minutes[1],
            hours: match military_time {
                true => bcd_hours[0] * 10 + bcd_hours[1],
                false => {
                    let pm = (bcd_hours[0] & 0b0010) == 0b0010;
                    match pm {
                        true => 12 + (bcd_hours[0] & 0b0001) * 10 + bcd_hours[1],
                        false => ((bcd_hours[0] & 0b0001) * 10 + bcd_hours[1]) % 12,
                    }
                },
            },
            date: bcd_date[0] * 10 + bcd_date[1],
            month: bcd_month[0] * 10 + bcd_month[1],
            year: 1900 + century as u32 + bcd_year[0] as u32 * 10 + bcd_year[1] as u32,
        }
    }

    pub fn to_bytes(self) -> [u8; 8] {
        [
        0x00, // Destination register on the DS3231
        (((self.seconds / 10) << 4) & 0b0111_0000) | ((self.seconds % 10) & 0b1111),
        (((self.minutes / 10) << 4) & 0b0111_0000) | ((self.minutes % 10) & 0b1111),
        0b0100_0000 | (((self.hours / 10) << 4) & 0b0011_0000) | ((self.hours % 10) & 0b1111), // Set to military time
        0b0000_0001, // Don't care - set to start of the week
        (((self.date / 10) << 4) & 0b0011_0000) | ((self.date % 10) & 0b1111),
        //Set the century marker if the year is in the 2001's
        if self.year >= 2000 {0b1000_0000} else {0b0000_0000} | (((self.month / 10) << 4) & 0b0001_0000) | ((self.month % 10) & 0b1111),
        (((((self.year % 100) / 10) << 4) & 0b1111_0000) | ((self.year % 10) & 0b1111)) as u8,
        ]
    }

}

fn days_since_epoch(year: u32) -> u32 {
    let mut days: u32 = 0;
    for y in EPOCH.year..year {
        days += match is_leap_year(y) {
            true  => DAYS_PER_LEAP_YEAR as u32,
            false => DAYS_PER_YEAR as u32,
        }
    }

    days 
}

fn days_this_year(year: u32, month: u8, day: u8) -> u32 {
    let mut days: u32 = 0;
    for m in 1..month {
        days += days_this_month(m, year) as u32;
    }

    days + day as u32
}

fn seconds_this_day(hours: u8, minutes: u8, seconds: u8) -> u32 {
    hours as u32 * SECONDS_PER_HOUR as u32 + minutes as u32 * SECONDS_PER_MINUTE as u32 + seconds as u32
}

// https://en.wikipedia.org/wiki/File:Leap_Year_Algorithm.png
fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0)
    ||
    year % 400 == 0
}

pub fn now(i2c: &mut I2c) -> Result<Datetime, arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 7];
    i2c.write_read(DS3231_I2C_ADDRESS, &[0x00], &mut buffer)?;

    Ok(Datetime::from_bytes(buffer))
}


pub fn set(i2c: &mut I2c, new_time: [u8; 8]) -> Result<(), arduino_hal::i2c::Error> {
    i2c.write(DS3231_I2C_ADDRESS, &new_time)
}

// Read the current temperature value in Celsius 
// Return: (whole numbers, 0.25 resolution value)
pub fn read_temperature(i2c: &mut I2c) -> Result<(i8, u8), arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 2];
    i2c.write_read(DS3231_I2C_ADDRESS, &[0x11], &mut buffer)?;

    buffer[1] = (buffer[1] >> 6) & 0b11;

    Ok((buffer[0] as i8, buffer[1]))
}

// Force a temperature update in the RTC
// Return: true if we forced an update, false if an update is already in progress
pub fn update_temperature(i2c: &mut I2c) -> Result<bool, arduino_hal::i2c::Error> {
    let mut current_settings = [0_u8; 2];
    i2c.write_read(DS3231_I2C_ADDRESS, &[0x0e], &mut current_settings)?;

    // Don't update if the busy flag is set
    if (current_settings[1] & 0b0100) != 0b0100 {
        let new_control_settings = current_settings[0] | 0b0010_0000;
        i2c.write(DS3231_I2C_ADDRESS, &[0x0e, new_control_settings])?;

        Ok(true)
    }
    else {
        Ok(false)
    }
}

// Read a single byte from the RTC EEPROM
pub fn read_byte_eeprom(i2c: &mut I2c, address: [u8; 2]) -> Result<[u8; 1], arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 1];
    i2c.write_read(EEPROM_I2C_ADDRESS, &address, &mut buffer)?;

    Ok(buffer)
}

// Read a 32-byte page from the RTC EEPROM
pub fn read_page_eeprom(i2c: &mut I2c, address: [u8; 2]) -> Result<[u8; 32], arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 32];
    i2c.write_read(EEPROM_I2C_ADDRESS, &address, &mut buffer)?;

    Ok(buffer)
}

pub fn write_byte_eeprom(i2c: &mut I2c, address: [u8; 2], input: u8) -> Result<(), arduino_hal::i2c::Error> {
    let buffer = [
        address[0], address[1],
        input
    ];
    i2c.write(EEPROM_I2C_ADDRESS, &buffer)
}

pub fn write_page_eeprom(i2c: &mut I2c, address: [u8; 2], input: [u8; 32]) -> Result<(), arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 34];

    // Address must be at the start of a 32-byte page boundary
    if address[1] % 32 != 0 {
        return Err(arduino_hal::i2c::Error::Unknown);
    }

    buffer[0..2].copy_from_slice(&address);
    buffer[2..34].copy_from_slice(&input);

    i2c.write(EEPROM_I2C_ADDRESS, &buffer)
}

// Key is stored starting at address 0x0000 of the RTC EEPROM
// 0x00_00 => length: u8
// 0x00_20..0x01_20 => key_byte: u8
//   (start key at 0x00_20 to ensure we only write within 32-byte page boundaries)
pub fn read_key_eeprom(i2c: &mut I2c) -> Result<(usize, [u8; 256]), arduino_hal::i2c::Error> {
    let mut length = [0_u8; 1];
    i2c.write_read(EEPROM_I2C_ADDRESS, &[0x00, 0x00], &mut length)?;

    let mut key = [0_u8; 256];
    i2c.write_read(EEPROM_I2C_ADDRESS, &[0x00, 0x20], &mut key)?;

    Ok((length[0] as usize, key))
}

pub fn write_key_eeprom(i2c: &mut I2c, length: usize, key: [u8; 256]) -> Result<(), arduino_hal::i2c::Error> {
    // Write key length to 0x00_00
    write_byte_eeprom(i2c, byte_helper::u16_to_bytes(0x00_00), length as u8)?;
    arduino_hal::delay_ms(10); // Wait for EEPROM to finish writing

    // Write key in 32-byte pages from 0x00_20 to 0x01_20
    for (address, key_page) in (0x00_20..0x01_20 as u16).step_by(32).zip(key.chunks(32)) {
        let address_bytes = byte_helper::u16_to_bytes(address);

        let mut page = [0u8; 32];
        page.copy_from_slice(key_page);

        write_page_eeprom(i2c, address_bytes, page)?;
        arduino_hal::delay_ms(10); // Wait for EEPROM to finish writing
    }

    Ok(())
}