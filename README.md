# Tank level indicator

## Components

* STM32F103C8T6 "Blue Pill"
* TFT ILI9341 screen
* Ultrasonic sensor HC-SR04

## Wiring

```
 screen        STM32F103
 ----------------------
 MISO         PA6 (MISO1)
 LED          3V3
 SCK          PA5 (SCK1)
 MOSI         PA7 (MOSI1)
 DC/RS        PA0
 RESET        PA1
 CS           PA4 (NSS1)
 GND          GND
 VCC          3V3

 Serial Debug
 GND          GND
 RX           PA2 (TX2)
 TX           PA3 (RX2)

 US HC-SR04
 GND          GND
 VCC          5V
 TRIGGER      PA8 (T1C1)
 ECHO         PB6 (T4C1)
```

## Description

A Timer PWM signal on the TRIGGER pin generates a pulse regularly
while the ECHO pin is connected to another Timer in Input Capture mode.

The Input Capture timer has a frequency of 1 MHz to get capture values in
microseconds. The difference of capture values between the rising and falling
edges gives the pulse width.
