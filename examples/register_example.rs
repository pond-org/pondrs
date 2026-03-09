//! Example demonstrating RegisterDataset and GpioDataset with pipeline viz.
//!
//! Allocates memory on the heap to simulate hardware registers, then runs
//! a pipeline that reads a "sensor" register, processes the value, and
//! sets GPIO pins based on thresholds.
//!
//! Usage:
//!   cargo run --example register_example -- viz
//!   # open http://localhost:8080, then in another terminal:
//!   cargo run --example register_example -- run

use serde::{Deserialize, Serialize};

use pondrs::datasets::{GpioDataset, Param, RegisterDataset};
use pondrs::Dataset;
use pondrs::error::PondError;
use pondrs::hooks::LoggingHook;
use pondrs::pipeline::{Node, Steps};
use pondrs::viz::VizHook;

// ---------------------------------------------------------------------------
// Simulated hardware: heap-allocated "registers"
// ---------------------------------------------------------------------------

struct SimulatedHardware {
    sensor_reg: Box<u16>,
    status_reg: Box<u32>,
}

impl SimulatedHardware {
    fn new() -> Self {
        Self {
            sensor_reg: Box::new(0),
            status_reg: Box::new(0),
        }
    }

    fn sensor_address(&self) -> usize {
        &*self.sensor_reg as *const u16 as usize
    }

    fn status_address(&self) -> usize {
        &*self.status_reg as *const u32 as usize
    }
}

// ---------------------------------------------------------------------------
// Catalog and params
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Catalog {
    sensor: RegisterDataset<u16>,
    status: RegisterDataset<u32>,
    led_ok: GpioDataset,
    led_warn: GpioDataset,
    led_crit: GpioDataset,
}

#[derive(Serialize, Deserialize)]
struct Params {
    warn_threshold: Param<u16>,
    crit_threshold: Param<u16>,
}

// ---------------------------------------------------------------------------
// Pipeline
// ---------------------------------------------------------------------------

fn register_pipeline<'a>(
    cat: &'a Catalog,
    params: &'a Params,
) -> impl Steps<PondError> + 'a {
    (
        Node {
            name: "read_sensor",
            func: |raw: u16| -> (u32,) {
                println!("  Sensor reading: 0x{raw:04x} ({raw})");
                (raw as u32,)
            },
            input: (&cat.sensor,),
            output: (&cat.status,),
        },
        Node {
            name: "set_ok_led",
            func: |reading: u16, warn: u16| {
                let ok = reading < warn;
                println!("  OK LED: {ok} (reading {reading} < warn {warn})");
                (ok,)
            },
            input: (&cat.sensor, &params.warn_threshold),
            output: (&cat.led_ok,),
        },
        Node {
            name: "set_warn_led",
            func: |reading: u16, warn: u16, crit: u16| {
                let warning = reading >= warn && reading < crit;
                println!("  WARN LED: {warning}");
                (warning,)
            },
            input: (&cat.sensor, &params.warn_threshold, &params.crit_threshold),
            output: (&cat.led_warn,),
        },
        Node {
            name: "set_crit_led",
            func: |reading: u16, crit: u16| {
                let critical = reading >= crit;
                println!("  CRIT LED: {critical}");
                (critical,)
            },
            input: (&cat.sensor, &params.crit_threshold),
            output: (&cat.led_crit,),
        },
    )
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

fn main() -> Result<(), PondError> {
    let hw = SimulatedHardware::new();

    let catalog = Catalog {
        sensor: unsafe { RegisterDataset::new(hw.sensor_address()) },
        status: unsafe { RegisterDataset::new(hw.status_address()) },
        led_ok: unsafe { GpioDataset::new(hw.status_address(), 0, "LED_OK") },
        led_warn: unsafe { GpioDataset::new(hw.status_address(), 1, "LED_WARN") },
        led_crit: unsafe { GpioDataset::new(hw.status_address(), 2, "LED_CRIT") },
    };

    let params = Params {
        warn_threshold: Param(500),
        crit_threshold: Param(900),
    };

    let app = pondrs::app::App::new(catalog, params)
        .with_hooks((
            LoggingHook::new(),
            VizHook::new("http://localhost:8080".to_string()),
        ))
        .with_args(std::env::args_os())?;

    // Pre-load a sensor reading so the pipeline has data
    app.catalog().sensor.save(750)?;

    app.dispatch(register_pipeline)
}
