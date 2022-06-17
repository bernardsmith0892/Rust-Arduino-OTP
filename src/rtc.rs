use arduino_hal::I2c;
use embedded_hal::prelude::{_embedded_hal_blocking_i2c_WriteRead, _embedded_hal_blocking_i2c_Write};
use ufmt::derive::uDebug;

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
        let military_time = (hours_byte & 0b10_0000) == 0b10_00000;
        let bcd_hours = match military_time {
            true  => [(hours_byte & 0b11_0000) >> 4, hours_byte & 0b1111],
            false => [(hours_byte & 0b1_0000) >> 4, hours_byte & 0b1111],
        };
        let bcd_date = [date_byte >> 4, date_byte & 0b1111];
        let bcd_month = [(month_byte & 0b1_0000) >> 4, month_byte & 0b1111];
        let century = ((month_byte & 0b1000_0000) >> 7) * 100;
        let bcd_year = [year_byte >> 4, year_byte & 0b1111];

        Datetime { 
            seconds: bcd_seconds[0] * 10 + bcd_seconds[1],
            minutes: bcd_minutes[0] * 10 + bcd_minutes[1],
            hours: bcd_hours[0] * 10 + bcd_hours[1],
            date: bcd_date[0] * 10 + bcd_date[1],
            month: bcd_month[0] * 10 + bcd_month[1],
            year: 1900 + century as u32 + bcd_year[0] as u32 * 10 + bcd_year[1] as u32,
        }
    }

    pub fn to_bytes(self) -> [u8; 8] {
        let mut bytes = [0_u8; 8];

        bytes[0] = 0x00; // Destination register on the DS3231
        bytes[1] = (((self.seconds / 10) << 4) & 0b0111_0000) | ((self.seconds % 10) & 0b1111);
        bytes[2] = (((self.minutes / 10) << 4) & 0b0111_0000) | ((self.minutes % 10) & 0b1111);
        bytes[3] = 0b0100_0000 | (((self.hours / 10) << 4) & 0b0011_0000) | ((self.hours % 10) & 0b1111); // Set to military time
        bytes[4] = 0b0000_0001; // Don't care - set to start of the week
        bytes[5] = (((self.date / 10) << 4) & 0b0011_0000) | ((self.date % 10) & 0b1111);
        // Set the century marker if the year is in the 2001's
        bytes[6] = if self.year >= 2000 {0b1000_0000} else {0b0000_0000} | (((self.month / 10) << 4) & 0b0001_0000) | ((self.month % 10) & 0b1111);
        bytes[7] = (((((self.year % 100) / 10) << 4) & 0b1111_0000) | ((self.year % 10) & 0b1111)) as u8;

        bytes
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

    days += day as u32;

    days
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
    i2c.write(DS3231_I2C_ADDRESS, &new_time)?;

    Ok(())
}

pub fn read(i2c: &mut I2c, address: [u8; 2]) -> Result<[u8; 1], arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 1];
    i2c.write_read(EEPROM_I2C_ADDRESS, &address, &mut buffer)?;

    Ok(buffer)
}

pub fn write(i2c: &mut I2c, input: [u8; 3]) -> Result<(), arduino_hal::i2c::Error> {
    i2c.write(EEPROM_I2C_ADDRESS, &input)?;

    Ok(())
}