#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AnyPin, Level, Output, Pull, Pin, Speed};
use embassy_stm32::time::mhz;
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::{self, Channel};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
async fn blinky(led: AnyPin) {
    let mut led = Output::new(led, Level::High, Speed::Low);

    loop {
        info!("high");
        led.set_high();
        Timer::after_millis(300).await;

        info!("low");
        led.set_low();
        Timer::after_millis(300).await;
    }
}

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("Hello World!");

    unwrap!(spawner.spawn(blinky(p.PC13.degrade())));

    // PB6 T4C1
    let ch1 = CapturePin::new_ch1(p.PB6, Pull::None);
    let mut ic = InputCapture::new(p.TIM4, Some(ch1), None, None, None, Irqs, mhz(1), Default::default());

    loop {
        info!("wait for risign edge");
        ic.wait_for_rising_edge(Channel::Ch3).await;

        let capture_value = ic.get_capture_value(Channel::Ch3);
        info!("new capture! {}", capture_value);
    }
}