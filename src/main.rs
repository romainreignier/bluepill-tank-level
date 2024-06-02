#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AnyPin, Level, Output, OutputType, Pin, Pull, Speed};
use embassy_stm32::time::hz;
use embassy_stm32::timer::input_capture::{CapturePin, InputCapture};
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{self, Channel};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_time::Timer;
use heapless::String;
use ufmt::uwrite;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::task]
async fn blinky(led: AnyPin) {
    let mut led = Output::new(led, Level::High, Speed::Low);

    loop {
        // Heartbeat like pattern
        led.set_low();
        Timer::after_millis(10).await;

        led.set_high();
        Timer::after_millis(300).await;

        led.set_low();
        Timer::after_millis(10).await;

        led.set_high();
        Timer::after_millis(1000).await;
    }
}

bind_interrupts!(struct Irqs {
    TIM4 => timer::CaptureCompareInterruptHandler<peripherals::TIM4>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = embassy_stm32::Config::default();
    {
        use embassy_stm32::rcc::*;
        config.rcc.hse = Some(Hse {
            freq: hz(8_000_000),
            mode: HseMode::Oscillator,
        });
        config.rcc.pll = Some(Pll {
            src: PllSource::HSE,
            prediv: PllPreDiv::DIV1,
            mul: PllMul::MUL9,
        });
        config.rcc.sys = Sysclk::PLL1_P;
        config.rcc.ahb_pre = AHBPrescaler::DIV1;
        config.rcc.apb1_pre = APBPrescaler::DIV2;
        config.rcc.apb2_pre = APBPrescaler::DIV1;
    }
    let p = embassy_stm32::init(config);

    let config = usart::Config::default();
    let mut usart = Uart::new_blocking(p.USART2, p.PA3, p.PA2, config).unwrap();

    info!("Hello World!");

    unwrap!(spawner.spawn(blinky(p.PC13.degrade())));

    // Input Capture PB6 T4C1
    let capture_freq = 10_000;
    let echo = CapturePin::new_ch1(p.PB6, Pull::Down);
    let mut ic = InputCapture::new(
        p.TIM4,
        Some(echo),
        None,
        None,
        None,
        Irqs,
        hz(capture_freq),
        Default::default(),
    );

    // PWM T1C1
    // Period of 1 Hz for a pulse per second
    let trig = PwmPin::new_ch1(p.PA8, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(trig),
        None,
        None,
        None,
        hz(1),
        Default::default(),
    );
    let max_duty = pwm.get_max_duty();
    info!("max duty cycle: {}", max_duty);
    pwm.set_duty(Channel::Ch1, max_duty / 50); // 20 ms pulse every second
    pwm.enable(Channel::Ch1);

    loop {
        info!("wait for pulse...");
        ic.wait_for_rising_edge(Channel::Ch1).await;
        let start_pulse_value = ic.get_capture_value(Channel::Ch1) as u16;
        ic.wait_for_falling_edge(Channel::Ch1).await;
        let end_pulse_value = ic.get_capture_value(Channel::Ch1) as u16;
        let pulse_width = end_pulse_value - start_pulse_value;
        let pulse_width_us = pulse_width as u32 * 1_000_000 / capture_freq;
        info!("new width = {} ticks -> {} us", pulse_width, pulse_width_us);
        let distance_mm = pulse_width_us as f32 * 0.171;
        info!("Distance = {} mm", distance_mm);

        let mut s: String<16> = String::new();
        uwrite!(s, "{}\r\n", distance_mm as u32).unwrap();
        unwrap!(usart.blocking_write(s.as_bytes()));
    }
}
