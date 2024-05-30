# Tank level indicator

```
/*
 * Indicateur de niveau pour la cuve à Fioul
 * Composants :
 *  * STM32F103C8T6 "Blue Pill"
 *  * ecran TFT ILI9341
 *  * US type HC-SR04
 *
 *  Branchement de l'ecran :
 *  ecran        STM32F103
 *  ----------------------
 *  MISO         PA6 (MISO1)
 *  LED          3V3
 *  SCK          PA5 (SCK1)
 *  MOSI         PA7 (MOSI1)
 *  DC/RS        PA0
 *  RESET        PA1
 *  CS           PA4 (NSS1)
 *  GND          GND
 *  VCC          3V3
 *
 *  Branchement Debug Serie
 *  GND          GND
 *  RX           PA2 (TX2)
 *  TX           PA3 (RX2)
 *
 *  Branchement HC-SR04
 *  GND          GND
 *  VCC          5V
 *  TRIGGER      PA8 (T1C1)
 *  ECHO         PB6 (T4C1)
 *
 *  Input capture en mode PWM sur ECHO pour detecter les fronts montants et
 *  descendants pour recuperer la periode et la largeur de l'impulsion
 *  PWM sur Trigger avec une frequence de 1MHz pour calculer en microsecondes
 *  et une periode de 35000 us qui sert de timeout
 *
 */
```
