#![no_std]
#![no_main]

use core::cell::RefCell;
use defmt::*;
use display_interface_spi::SPIInterface;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AnyPin, Level, Output, OutputType, Pin, Pull, Speed};
use embassy_stm32::spi::{self, Spi};
use embassy_stm32::time::hz;
use embassy_stm32::timer::pwm_input::PwmInput;
use embassy_stm32::timer::simple_pwm::{PwmPin, SimplePwm};
use embassy_stm32::timer::{self, Channel};
use embassy_stm32::usart::{self, Uart};
use embassy_stm32::{bind_interrupts, peripherals};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::{Delay, Timer};
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text},
};
use heapless::String;
use ili9341::{DisplaySize240x320, Ili9341, Orientation};
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
    TIM4 => timer::InterruptHandler<peripherals::TIM4>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // RCC config to use HSE at 72 MHz
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

    // USART config
    let uart_rx = p.PA3;
    let uart_tx = p.PA2;
    let config = usart::Config::default();
    let mut usart = Uart::new_blocking(p.USART2, uart_rx, uart_tx, config).unwrap();

    info!("Hello World!");

    unwrap!(spawner.spawn(blinky(p.PC13.degrade())));

    // SPI config
    let clk = p.PA5;
    let mosi = p.PA7;
    let miso = p.PA6;
    let lcd_cs = p.PA4;
    let lcd_reset = p.PA1;
    let lcd_dc = p.PA0;

    let mut lcd_spi_config = spi::Config::default();
    lcd_spi_config.frequency = hz(26_000_000);
    let spi = Spi::new_blocking(p.SPI1, clk, mosi, miso, lcd_spi_config.clone());
    let spi_bus: Mutex<NoopRawMutex, _> = Mutex::new(RefCell::new(spi));

    let lcd_spi = SpiDeviceWithConfig::new(
        &spi_bus,
        Output::new(lcd_cs, Level::High, Speed::Medium),
        lcd_spi_config,
    );

    let mut delay = Delay;
    let lcd_reset = Output::new(lcd_reset, Level::Low, Speed::Medium);
    let lcd_dc = Output::new(lcd_dc, Level::Low, Speed::Medium);
    let spi_iface = SPIInterface::new(lcd_spi, lcd_dc);
    let mut lcd = Ili9341::new(
        spi_iface,
        lcd_reset,
        &mut delay,
        Orientation::Landscape,
        DisplaySize240x320,
    )
    .unwrap();

    unwrap!(lcd.clear(Rgb565::BLACK));

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::BLUE);
    unwrap!(Text::with_alignment(
        "Tank Level Indicator",
        Point::new(320 / 2, 20),
        style,
        Alignment::Center,
    )
    .draw(&mut lcd));

    let style = MonoTextStyle::new(&FONT_10X20, Rgb565::GREEN);
    unwrap!(
        Text::with_alignment("80%", Point::new(260, 120), style, Alignment::Left,).draw(&mut lcd)
    );

    let style = PrimitiveStyleBuilder::new()
        .stroke_color(Rgb565::MAGENTA)
        .stroke_width(3)
        .fill_color(Rgb565::BLACK)
        .build();
    unwrap!(Rectangle::new(Point::new(20, 40), Size::new(200, 180))
        .into_styled(style)
        .draw(&mut lcd));

    let style = PrimitiveStyleBuilder::new()
        .fill_color(Rgb565::GREEN)
        .build();
    unwrap!(
        Rectangle::new(Point::new(20, 100), Size::new(200, 180 - 60))
            .into_styled(style)
            .draw(&mut lcd)
    );

    let capture_freq = 1_000_000;
    let mut pwm_input = PwmInput::new(p.TIM4, p.PB6, Pull::None, Irqs, hz(capture_freq));
    pwm_input.enable();

    // PWM T1C1
    let trig = PwmPin::new_ch1(p.PA8, OutputType::PushPull);
    let mut pwm = SimplePwm::new(
        p.TIM1,
        Some(trig),
        None,
        None,
        None,
        hz(10),
        Default::default(),
    );
    let max_duty = pwm.get_max_duty();
    info!("max duty cycle: {}", max_duty);
    pwm.set_duty(Channel::Ch1, max_duty / 10);
    pwm.enable(Channel::Ch1);

    let samples = 10;

    loop {
        let mut sum = 0;
        for _ in 0..samples {
            info!("wait for pulse...");
            let pulse_width = pwm_input.wait_for_falling_edge().await as u16;
            let pulse_width_us = pulse_width as u32 * (1_000_000 / capture_freq);
            info!("new width = {} ticks -> {} us", pulse_width, pulse_width_us);
            let distance_mm = pulse_width_us as f32 * 0.171;
            info!("Distance = {} mm", distance_mm);

            sum += distance_mm as u32;
        }

        let average_distance_mm = sum / samples;
        let mut s: String<16> = String::new();
        uwrite!(s, "{}\r\n", average_distance_mm as u32).unwrap();
        unwrap!(usart.blocking_write(s.as_bytes()));
    }
}
