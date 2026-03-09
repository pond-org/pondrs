//! Register-mapped dataset for volatile memory access.
//!
//! Reads and writes values at a raw memory address using volatile operations.
//! Works on both no_std (embedded registers) and std (memory-mapped I/O).

#[cfg(feature = "std")]
use std::prelude::v1::*;

use core::marker::PhantomData;

use serde::Serialize;

use crate::error::PondError;
use super::Dataset;

/// A dataset backed by a volatile memory-mapped register.
///
/// `T` is the register width (typically `u8`, `u16`, or `u32`).
/// Reads use `read_volatile`, writes use `write_volatile`.
///
/// # Safety
///
/// The caller must ensure that `address` points to a valid, aligned memory
/// location for the lifetime of this dataset. For embedded use, addresses
/// come from the hardware memory map. For testing, use heap-allocated memory.
#[derive(Serialize)]
pub struct RegisterDataset<T: Copy> {
    address: usize,
    #[serde(skip)]
    _marker: PhantomData<T>,
}

// SAFETY: Register access is inherently single-threaded in embedded contexts.
// For std use, the caller is responsible for ensuring no data races.
unsafe impl<T: Copy + Send> Send for RegisterDataset<T> {}
unsafe impl<T: Copy + Send> Sync for RegisterDataset<T> {}

impl<T: Copy> RegisterDataset<T> {
    /// Create a new register dataset at the given memory address.
    ///
    /// # Safety
    ///
    /// The address must point to valid, aligned memory for `T`.
    pub const unsafe fn new(address: usize) -> Self {
        Self {
            address,
            _marker: PhantomData,
        }
    }

    /// Returns the memory address of this register.
    pub const fn address(&self) -> usize {
        self.address
    }
}

impl<T: Copy> Dataset for RegisterDataset<T> {
    type LoadItem = T;
    type SaveItem = T;
    type Error = PondError;

    fn load(&self) -> Result<T, PondError> {
        let ptr = self.address as *const T;
        Ok(unsafe { core::ptr::read_volatile(ptr) })
    }

    fn save(&self, output: T) -> Result<(), PondError> {
        let ptr = self.address as *mut T;
        unsafe { core::ptr::write_volatile(ptr, output) };
        Ok(())
    }

    #[cfg(feature = "std")]
    fn html(&self) -> Option<String> {
        register_html(self.address, self.load().ok()?)
    }
}

#[cfg(feature = "std")]
fn register_html<T: Copy>(address: usize, value: T) -> Option<String> {
    let size = core::mem::size_of::<T>();
    let bytes = unsafe {
        core::slice::from_raw_parts(&value as *const T as *const u8, size)
    };

    // Build integer value from little-endian bytes
    let mut int_val: u64 = 0;
    for (i, &b) in bytes.iter().enumerate() {
        int_val |= (b as u64) << (i * 8);
    }

    let bits = size * 8;
    let hex_width = size * 2;

    let hex_raw = format!("{int_val:0>width$x}", width = hex_width);
    let hex = insert_separators(&hex_raw, 4);

    let bin_raw = format!("{int_val:0>width$b}", width = bits);
    let bin = insert_separators(&bin_raw, 4);

    // Bit grid: colored cells, MSB first
    let mut grid = String::from(
        "<div style=\"display:flex;gap:1px;margin-top:6px;font-family:monospace;font-size:11px\">"
    );
    for i in (0..bits).rev() {
        let bit = (int_val >> i) & 1;
        let bg = if bit == 1 { "#4ade80" } else { "#e5e7eb" };
        let fg = if bit == 1 { "#000" } else { "#888" };
        grid.push_str(&format!(
            "<div style=\"width:18px;height:24px;background:{bg};color:{fg};\
             display:flex;align-items:center;justify-content:center;\
             border-radius:2px\" title=\"bit {i}\">{bit}</div>"
        ));
    }
    grid.push_str("</div>");

    // Bit numbers row
    let mut bit_nums = String::from(
        "<div style=\"display:flex;gap:1px;font-family:monospace;font-size:9px;color:#888\">"
    );
    for i in (0..bits).rev() {
        bit_nums.push_str(&format!(
            "<div style=\"width:18px;text-align:center\">{i}</div>"
        ));
    }
    bit_nums.push_str("</div>");

    Some(format!(
        "<div style=\"font-family:monospace;font-size:13px;padding:8px\">\
         <div><b>Address:</b> 0x{address:x}</div>\
         <div><b>Hex:</b> 0x{hex}</div>\
         <div><b>Bin:</b> 0b{bin}</div>\
         <div><b>Dec:</b> {int_val}</div>\
         <div style=\"margin-top:8px\"><b>Bits:</b></div>\
         {grid}\
         {bit_nums}\
         </div>"
    ))
}

#[cfg(feature = "std")]
fn insert_separators(s: &str, group: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len() + s.len() / group);
    for (i, ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % group == 0 {
            result.push('_');
        }
        result.push(*ch);
    }
    result
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_round_trip_u32() {
        let storage = Box::new(0u32);
        let address = &*storage as *const u32 as usize;
        let ds = unsafe { RegisterDataset::<u32>::new(address) };

        ds.save(0xDEAD_BEEF).unwrap();
        assert_eq!(ds.load().unwrap(), 0xDEAD_BEEF);
    }

    #[test]
    fn register_round_trip_u8() {
        let storage = Box::new(0u8);
        let address = &*storage as *const u8 as usize;
        let ds = unsafe { RegisterDataset::<u8>::new(address) };

        ds.save(0xFF).unwrap();
        assert_eq!(ds.load().unwrap(), 0xFF);
    }

    #[test]
    fn register_round_trip_u16() {
        let storage = Box::new(0u16);
        let address = &*storage as *const u16 as usize;
        let ds = unsafe { RegisterDataset::<u16>::new(address) };

        ds.save(0x1234).unwrap();
        assert_eq!(ds.load().unwrap(), 0x1234);
    }

    #[cfg(feature = "std")]
    #[test]
    fn register_html_shows_value() {
        let storage = Box::new(0u32);
        let address = &*storage as *const u32 as usize;
        let ds = unsafe { RegisterDataset::<u32>::new(address) };
        ds.save(0b1010_0101).unwrap();

        let html = ds.html().unwrap();
        assert!(html.contains("00a5"), "should contain hex: {html}");
        assert!(html.contains("165"), "should contain decimal: {html}");
    }
}
