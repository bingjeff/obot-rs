#![cfg_attr(target_os = "none", no_std)]

#[cfg(target_os = "none")]
pub mod adc;
pub mod cycle_counter;
#[cfg(target_os = "none")]
pub mod driver;
pub mod drv8323s;
#[cfg(target_os = "none")]
pub mod hall;
pub mod led;
pub mod pwm;
#[cfg(target_os = "none")]
pub mod usb;
