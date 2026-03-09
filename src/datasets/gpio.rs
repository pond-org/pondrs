//! GPIO pin dataset — reads/writes a single bit from a memory-mapped register.

#[cfg(feature = "std")]
use std::prelude::v1::*;

use serde::Serialize;

use crate::error::PondError;
use super::Dataset;

/// A dataset representing a single GPIO pin within a memory-mapped register.
///
/// Reads and writes a single bit at position `bit` within a 32-bit register
/// at `address`. The register is accessed via volatile operations.
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid, aligned u32
/// memory location for the lifetime of this dataset.
#[derive(Serialize)]
pub struct GpioDataset {
    address: usize,
    bit: u8,
    label: &'static str,
}

// SAFETY: Same as RegisterDataset — single-threaded access assumed.
unsafe impl Send for GpioDataset {}
unsafe impl Sync for GpioDataset {}

impl GpioDataset {
    /// Create a new GPIO pin dataset.
    ///
    /// # Arguments
    ///
    /// * `address` — memory address of the GPIO port register
    /// * `bit` — bit index (0–31) within the register
    /// * `label` — display label (e.g. "PA5", "LED1")
    ///
    /// # Safety
    ///
    /// The address must point to a valid, aligned u32 memory location.
    pub const unsafe fn new(address: usize, bit: u8, label: &'static str) -> Self {
        Self { address, bit, label }
    }

    /// Returns the memory address of the GPIO port register.
    pub const fn address(&self) -> usize {
        self.address
    }

    /// Returns the bit index within the register.
    pub const fn bit(&self) -> u8 {
        self.bit
    }

    /// Returns the display label.
    pub const fn label(&self) -> &'static str {
        self.label
    }
}

impl Dataset for GpioDataset {
    type LoadItem = bool;
    type SaveItem = bool;
    type Error = PondError;

    fn load(&self) -> Result<bool, PondError> {
        let ptr = self.address as *const u32;
        let reg = unsafe { core::ptr::read_volatile(ptr) };
        Ok((reg >> self.bit) & 1 == 1)
    }

    fn save(&self, output: bool) -> Result<(), PondError> {
        let ptr = self.address as *mut u32;
        let mut reg = unsafe { core::ptr::read_volatile(ptr) };
        if output {
            reg |= 1 << self.bit;
        } else {
            reg &= !(1 << self.bit);
        }
        unsafe { core::ptr::write_volatile(ptr, reg) };
        Ok(())
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        let state = self.load().ok()?;
        let (color, glow, text) = if state {
            ("#22c55e", "0 0 12px #22c55e, 0 0 24px #22c55e80", "ON")
        } else {
            ("#374151", "none", "OFF")
        };

        Some(format!(
            "<div style=\"font-family:monospace;font-size:13px;padding:12px;\
             display:flex;flex-direction:column;align-items:center;gap:8px\">\
             <div style=\"font-weight:bold;font-size:14px\">{label}</div>\
             <div style=\"width:40px;height:40px;border-radius:50%;\
             background:{color};box-shadow:{glow};\
             border:2px solid #555\"></div>\
             <div style=\"font-size:12px;color:#888\">{text}</div>\
             <div style=\"font-size:11px;color:#666\">\
             Register 0x{addr:x} bit {bit}</div>\
             </div>",
            label = self.label,
            addr = self.address,
            bit = self.bit,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpio_read_write_single_bit() {
        let storage = Box::new(0u32);
        let address = &*storage as *const u32 as usize;

        let pin = unsafe { GpioDataset::new(address, 5, "PA5") };

        assert!(!pin.load().unwrap());

        pin.save(true).unwrap();
        assert!(pin.load().unwrap());

        // Verify it set only bit 5
        let raw = unsafe { core::ptr::read_volatile(address as *const u32) };
        assert_eq!(raw, 1 << 5);

        pin.save(false).unwrap();
        assert!(!pin.load().unwrap());

        let raw = unsafe { core::ptr::read_volatile(address as *const u32) };
        assert_eq!(raw, 0);
    }

    #[test]
    fn gpio_preserves_other_bits() {
        let storage = Box::new(0xFFFF_FFFFu32);
        let address = &*storage as *const u32 as usize;

        let pin = unsafe { GpioDataset::new(address, 3, "PB3") };

        assert!(pin.load().unwrap());

        pin.save(false).unwrap();
        let raw = unsafe { core::ptr::read_volatile(address as *const u32) };
        assert_eq!(raw, 0xFFFF_FFF7);
    }

    #[test]
    fn gpio_multiple_pins_same_register() {
        let storage = Box::new(0u32);
        let address = &*storage as *const u32 as usize;

        let led1 = unsafe { GpioDataset::new(address, 0, "LED1") };
        let led2 = unsafe { GpioDataset::new(address, 4, "LED2") };
        let led3 = unsafe { GpioDataset::new(address, 7, "LED3") };

        led1.save(true).unwrap();
        led2.save(true).unwrap();
        led3.save(true).unwrap();

        let raw = unsafe { core::ptr::read_volatile(address as *const u32) };
        assert_eq!(raw, (1 << 0) | (1 << 4) | (1 << 7));

        assert!(led1.load().unwrap());
        assert!(led2.load().unwrap());
        assert!(led3.load().unwrap());
    }

    #[cfg(feature = "std")]
    #[test]
    fn gpio_html_shows_led() {
        let storage = Box::new(0u32);
        let address = &*storage as *const u32 as usize;
        let pin = unsafe { GpioDataset::new(address, 0, "LED1") };

        pin.save(true).unwrap();
        let html = pin.html().unwrap();
        assert!(html.contains("LED1"), "should contain label: {html}");
        assert!(html.contains("#22c55e"), "should show green when on: {html}");

        pin.save(false).unwrap();
        let html = pin.html().unwrap();
        assert!(html.contains("OFF"), "should show OFF: {html}");
    }
}
