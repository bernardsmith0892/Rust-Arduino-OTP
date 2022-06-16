use arduino_hal::I2c;
use embedded_hal::prelude::_embedded_hal_blocking_i2c_WriteRead;

const DS3231_I2C_ADDRESS: u8 = 0x68;
const EEPROM_I2C_ADDRESS: u8 = 0x57;

pub struct Datetime {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub date: u8,
    pub month: u8,
    pub year: u32,
}

pub fn now(mut i2c: I2c) -> Result<Datetime, arduino_hal::i2c::Error> {
    let mut buffer = [0_u8; 7];
    i2c.write_read(DS3231_I2C_ADDRESS, &[0x00], &mut buffer)?;

    Ok(bytes_to_datetime(buffer))
}

pub fn bytes_to_datetime(bytes: [u8; 7]) -> Datetime {
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
