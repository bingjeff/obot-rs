#![cfg_attr(target_os = "none", no_std)]

pub mod cycle_counter;
#[cfg(target_os = "none")]
pub mod hall;
#[cfg(target_os = "none")]
pub mod pwm;
