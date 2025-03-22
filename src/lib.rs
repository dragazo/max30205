#![no_std]
#![forbid(unsafe_code)]

#![doc = include_str!("../README.md")]

use embedded_hal::i2c::I2c;

#[repr(u8)]
enum Register {
    Temp   = 0,
    Config = 1,
    Thyst  = 2,
    Tos    = 3,
}

const ADDRESSES: &'static [u8] = &[0x49, 0x48];

/// A MAX30205 sensor wrapper.
pub struct MAX30205<T: I2c> {
    i2c: T,
    addr: u8,
}
impl<T: I2c> MAX30205<T> {
    /// Scans for available devices on the expected set of addresses.
    /// Returns `Some(addr)` with the first found valid address, or `None` if no devices are found.
    ///
    /// Note that a found device is not necessarily a MAX30205 sensor,
    /// as it could be that some other device has the same address as a MAX30205 device.
    pub fn scan(i2c: &mut T) -> Option<u8> {
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
    pub fn new(addr: u8, mut i2c: T) -> Result<Self, T::Error> {
        i2c.write(addr, &[Register::Config as u8, 0x00])?;
        i2c.write(addr, &[Register::Thyst  as u8, 0x00])?;
        i2c.write(addr, &[Register::Tos    as u8, 0x00])?;
        Ok(Self { i2c, addr })
    }

    fn transform_config(&mut self, trans: fn(u8) -> u8) -> Result<(), T::Error> {
        let mut reg = [0u8];
        self.i2c.write_read(self.addr, &[Register::Config as u8], &mut reg)?;
        self.i2c.write(self.addr, &[Register::Config as u8, trans(reg[0])])?;
        Ok(())
    }

    /// Transitions the device into power saving mode.
    /// In power saving mode, the device will not update its stored temperature,
    /// meaning subsequent calls to [`MAX30205::get_temperature`] will return the same value.
    ///
    /// You may use [`MAX30205::power_up`] to exit power saving mode and resume continuous updates,
    /// or [`MAX30205::update_once`] to get on-demand temperature updates while staying in power saving mode.
    pub fn power_down(&mut self) -> Result<(), T::Error> {
        self.transform_config(|x| x | 0x01)
    }
    /// Exits power saving mode and resumes continuous temperature updates. See [`MAX30205::power_down`] for details.
    pub fn power_up(&mut self) -> Result<(), T::Error> {
        self.transform_config(|x| x & !0x01)
    }
    /// Performs a single temperature update while in power saving mode.
    /// When not in power saving mode, this has no effect.
    /// See [`MAX30205::power_down`] for more details.
    pub fn update_once(&mut self) -> Result<(), T::Error> {
        self.transform_config(|x| x | 0x80)
    }

    /// Gets an instantaneous temperature reading (in Celsius) from the device.
    pub fn get_temperature(&mut self) -> Result<f64, T::Error> {
        let mut res = [0; 2];
        self.i2c.write_read(self.addr, &[Register::Temp as u8], &mut res)?;
        let res = ((res[0] as u16) << 8) | (res[1] as u16);
        Ok(res as i16 as f64 * 0.00390625)
    }
}
