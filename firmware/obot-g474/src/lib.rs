#![cfg_attr(target_os = "none", no_std)]

#[cfg(target_os = "none")]
pub mod adc;
pub mod cycle_counter;
#[cfg(target_os = "none")]
pub mod hall;
#[cfg(target_os = "none")]
pub mod pwm;
