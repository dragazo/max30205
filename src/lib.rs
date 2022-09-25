//! A driver for the MAX30205 temperature sensor communicating over I2C.

#![forbid(unsafe_code)]

use embedded_hal::blocking::i2c;

#[repr(u8)]
enum Register {
    Temp   = 0,
    Config = 1,
    Thyst  = 2,
    Tos    = 3,
}

const ADDRESSES: &'static [u8] = &[0x49, 0x48];

/// A MAX30205 sensor wrapper.
pub struct MAX30205 {
    addr: u8
}
impl MAX30205 {
    /// Scans for available devices on the expected set of addresses.
    /// Returns `Some(addr)` with the first found valid address, or `None` if no devices are found.
    ///
    /// Note that a found device is not necessarily a MAX30205 sensor,
    /// as it could be that some other device has the same address as a MAX30205 device.
    pub fn scan<I2C>(i2c: &mut I2C) -> Option<u8> where I2C: i2c::Write<u8> {
        for addr in ADDRESSES.iter().copied() {
            if i2c.write(addr, &[]).is_ok() { return Some(addr) }
        }
        None
    }
    /// Constructs a MAX30205 sensor wrapper targeting the given address.
    /// If the address is unknown, [`MAX30205::scan`] can be used.
    ///
    /// Also initializes the device for usage, which requires the I2C bus for communication.
    /// The initial state disables power saving mode.
    /// See [`MAX30205::power_down`] for details.
    pub fn new<I2C, E>(addr: u8, i2c: &mut I2C) -> Result<Self, E> where I2C: i2c::Write<u8, Error = E> {
        i2c.write(addr, &[Register::Config as u8, 0x00])?;
        i2c.write(addr, &[Register::Thyst  as u8, 0x00])?;
        i2c.write(addr, &[Register::Tos    as u8, 0x00])?;
        Ok(Self { addr })
    }

    fn transform_config<I2C, E>(&self, i2c: &mut I2C, trans: fn(u8) -> u8) -> Result<(), E> where I2C: i2c::WriteRead<u8, Error = E> + i2c::Write<u8, Error = E> {
        let mut reg = [0u8];
        i2c.write_read(self.addr, &[Register::Config as u8], &mut reg)?;
        i2c.write(self.addr, &[Register::Config as u8, trans(reg[0])])?;
        Ok(())
    }

    /// Transitions the device into power saving mode.
    /// In power saving mode, the device will not update its stored temperature,
    /// meaning subsequent calls to [`MAX30205::get_temperature`] will return the same value.
    ///
    /// You may use [`MAX30205::power_up`] to exit power saving mode and resume continuous updates,
    /// or [`MAX30205::update_once`] to get on-demand temperature updates while staying in power saving mode.
    pub fn power_down<I2C, E>(&self, i2c: &mut I2C) -> Result<(), E> where I2C: i2c::WriteRead<u8, Error = E> + i2c::Write<u8, Error = E> {
        self.transform_config(i2c, |x| x | 0x01)
    }
    /// Exits power saving mode and resumes continuous temperature updates. See [`MAX30205::power_down`] for details.
    pub fn power_up<I2C, E>(&self, i2c: &mut I2C) -> Result<(), E> where I2C: i2c::WriteRead<u8, Error = E> + i2c::Write<u8, Error = E> {
        self.transform_config(i2c, |x| x & !0x01)
    }
    /// Performs a single temperature update while in power saving mode.
    /// When not in power saving mode, this has no effect.
    /// See [`MAX30205::power_down`] for more details.
    pub fn update_once<I2C, E>(&self, i2c: &mut I2C) -> Result<(), E> where I2C: i2c::WriteRead<u8, Error = E> + i2c::Write<u8, Error = E> {
        self.transform_config(i2c, |x| x | 0x80)
    }

    /// Gets an instantaneous temperature reading (in Celsius) from the device.
    pub fn get_temperature<I2C, E>(&self, i2c: &mut I2C) -> Result<f64, E> where I2C: i2c::WriteRead<u8, Error = E> {
        let mut res = [0; 2];
        i2c.write_read(self.addr, &[Register::Temp as u8], &mut res)?;
        let res = ((res[0] as u16) << 8) | (res[1] as u16);
        Ok(res as i16 as f64 * 0.00390625)
    }
}
