# Rust Arduino OTP

An Arduino HOTP/TOTP generator implemented in Rust.

```text
PS > python3 -m serial.tools.miniterm

--- Available ports:
---  1: COM1                 'Communications Port (COM1)'
---  2: COM3                 'Standard Serial over Bluetooth link (COM3)'
---  3: COM4                 'Standard Serial over Bluetooth link (COM4)'
---  4: COM5                 'Arduino Uno (COM5)'
--- Enter port index or full name: 4
--- Miniterm on COM5  9600,8,N,1 ---
--- Quit: Ctrl+] | Menu: Ctrl+T | Help: Ctrl+T followed by Ctrl+H ---
Loaded key of length 10 from RTC EEPROM

$ help
key <OTP Key> - Set OTP key.
key - Show current OTP key.
digit <OTP Digits> - Set digits of OTP. (default is 6)
digit - Show OTP digits setting.
hotp <HOTP Counter> - Calculate OTP for a given counter value.
totp - Calculate OTP for the current time. (step of 30)
time <UNIX timestamp> - Set date and time.
time - Show current date and time.
temp - Show current temperature in Celsius.
utemp - Force the RTC to update its temperature reading.
read <addr> - Read a byte from RTC EEPROM at the given 2-byte address. Must provide four hex digits.
readp <addr> - Read a 32-byte page from the RTC EEPROM at the given 2-byte address. Must provide four hex digits.
write <addr> <data> - Write a byte to the RTC EEPROM at the given 2-byte address. Must provide four and two hex digits.
save - Save the current key into RTC EEPROM.
load - Load the saved key from RTC EEPROM.
help - Show this help menu.

$ key
Hello!Þ­¾ï
$ time
Date: 1900/1/1 - 0:1:34
Timestamp: 86494

$ time 1669714637
Date: 2022/11/29 - 9:37:17
Timestamp: 1669714637

$ totp
Timestamp: 1669714659
Counter: 55657155
169658
$ readp 0000
Page: [10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255]

$ readp 0020
Page: [72, 101, 108, 108, 111, 33, 222, 173, 190, 239, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]

$ temp
Current Temperature: 25.75 °C

$
```
