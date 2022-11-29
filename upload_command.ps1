wsl /home/kali/.cargo/bin/cargo build

& 'C:\Program Files (x86)\Arduino\hardware\tools\avr\bin\avrdude.exe' '-CC:\Program Files (x86)\Arduino\hardware\tools\avr\etc\avrdude.conf' -patmega328p -v -carduino -PCOM5 -b115200 -D "-Uflash:w:C:\Users\berna\OneDrive\Projects\Rust\arduino-otp\target\avr-atmega328p\debug\arduino-otp.elf:e"

python3 -m serial.tools.miniterm