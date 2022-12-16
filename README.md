# digimorse

## What is this?
Morse code was the first means by which messages were sent by Marconi, as he performed his experiments
in the 1890's. This was achieved using a crude *spark gap* transmitter. In 1913, Edwin Armstrong
and Alexander Meissner used thermionic valves to build a *continuous wave oscillator* which
refined the transmission mechanism. This form of transmission, known by the acronym CW, was widely used
for military, maritime and commercial telegraphy, gradually being replaced by more modern systems of
communication. Its last maritime use was in 1999.

It is still, however, used widely by amateur radio operators, such as myself. Although the electronic
circuits used have been refined as far as they can be, the essential *continuous wave* transmission mechanism
has remained unchanged.

This project is an attempt to modernise that.

## Project Status
Project started September 2020. Currently in its early development / feasibility study phase. I'm working on the channel
encoder, encoding/decoding messages via low-density parity-check (LDPC) codes. 

There's no graphical user interface at the moment - the system is configured and used via a command line interface.

# Overview
This is an EXPERIMENT in taking Morse code, in real-time, encoding it, wrapping
it in modern error-detection/correction codes, and modulating it out to a
connected amateur radio transceiver as a narrow-bandwidth signal. 

The intention is to take our historic mode of communication, and be able to
receive it and decode it (with your brain) whilst removing the effects of
natural or man-made interference and fading. The operator will be able to
visualise all the transmissions in a waterfall display, making it easy to
see and select transmissions of interest. There will be the facility to
filter out some or all stations so you can hone in on the one you want - and
reply to it in digitally encoded Morse. By adding error correction, and
carefully designing the encoding and modulation, it is hoped that the weak
signal characteristic enjoyed by users of FT8, JT9 etc. can be added to Morse.

What digimorse is not:
* Computer-generated or computer-decoded Morse. It doesn't decode the Morse for
  you. That's cheating! It is a beautiful skill to master, like a language or
  musical instrument; mastering Morse is a fine human achievement. digimorse
  seeks to add another dimension to it!
* The death of CW or Amateur Radio As We Know It(tm). You don't have to use it!
* Completely automated, just macro-key-pressing. No, you use your normal Morse
  key or paddle. Macros will come later perhaps.
* Quantized. If you have a unique rhythm to your keying, digimorse won't correct
  it. The timing of your keying goes out verbatim. You can use our keyer with a
  paddle which will give good timing (later).

There are arguments that adding a keyboard entry facility (instead of key/paddle)
and an auto-decoder (instead of human decode) might be warranted: in the case of
disabilities. I'm happy to discuss these and maybe implement them; they're not my
current priority.

What you'll find different to usual CW:
* No noise! QRN, QRM, QSB, gone!
* When you start keying, digimorse encodes your callsign and locator
  automatically, digitally. Receiving stations will see who and where you are on
  the waterfall display. Your encoded Morse is sent along with this - so
  receivers won't hear your keying as soon as you start. There are short delays.
* When you stop keying, digimorse may not have finished encoding and
  transmitting. The screen will show you how much longer your transmission will
  actually take.
* So there's not a rapid-fire switching of conversation.
 
What do I need to try it?
* A fairly modern computer running Windows 10, macOS 10.15ish (High Sierra to
  Big Sur), or Ubuntu 16.xx LTS, 18.xx LTS, 20.xx LTS.
* Sorry no Raspberry Pi yet - definitely considering a build for this.
* An interface between the computer sound system and your amateur radio
  transceiver, e.g. Tigertronics SignaLink USB.
* Monitor, keyboard, mouse.
* An interface between your Morse key or paddle and USB. See the
  https://github.com/devzendo/digimorse-arduino-keyer project for a simple
  Arduino Nano-based Morse key/paddle <-> USB Serial interface and simple keyer.



## Downloads
There aren't any yet. There will be for the first release, but that won't be any
time soon. You'd have to install build tools and build it yourself...


## Roadmap
First release:
* Reads keyer output, measures timings, source-encodes these, and mixes with
  callsign and locator information, adds error detection/correction with a
  low-density parity check code, modulates using ??? and Gaussian Frequency
  Shift Keying to prevent hard phase transitions resulting in splatter.
* Can choose your transmit offset within the 2500Hz bandwidth shown.
* Waterfall shows all signals in received bandwidth, and their decoded
  callsign/locator information.
* Select a signal, or click-drag-release over a portion of the display to apply
  a filter so that only the chosen signals are played.

Second release:
* Who knows?


## Release Notes
0.0.1 First Release (work in progress)
* Created repository! Wrote the above... made notes... learning about
  information theory, error control coding, modulation/demodulation, the Fourier
  transform, the Rust programming language, the FLTK and PortAudio frameworks for
  GUI and audio, built the Arduino keyer.

 
## Configuring and running Digimorse
The GUI is not present yet; all configuration needs to be done via the command line. To run Digimorse, use the
terminal or Command Prompt, and run the 'digimorse' program. There are several options and modes in which you can run
the software. Add the '--help' option to the command line to show the full details.

Before Digimorse can be used, several devices need setting in its configuration.

You will need to configure your audio output device (for speakers or headphone) so you can hear the decoded Morse, and
your sidetone when keying. 

You will need to configure your transceiver audio output and input devices so Digimorse can receive and transmit
encoded Morse.

To discover the names of the audio devices currently available on your system, use the ListAudioDevices mode:

```
$ digimorse ListAudioDevices
[2021-09-17T07:49:11Z INFO  digimorse] Number of audio devices = 4
[2021-09-17T07:49:11Z INFO  digimorse] 0: "Built-in Microphone" / IN:2 OUT:0 @ 96000Hz default; 48000Hz supported
[2021-09-17T07:49:11Z INFO  digimorse] 1: "Built-in Output" / IN:0 OUT:2 @ 44100Hz default; 48000Hz supported
[2021-09-17T07:49:11Z INFO  digimorse] 2: "USB AUDIO  CODEC" / IN:0 OUT:2 @ 48000Hz default; 48000Hz supported
[2021-09-17T07:49:11Z INFO  digimorse] 3: "USB AUDIO  CODEC" / IN:2 OUT:0 @ 48000Hz default; 48000Hz supported
```

Please take care with the device names. Note in the above output, my transceiver is shown as "USB AUDIO  CODEC" - and
has two spaces between AUDIO and CODEC. You must copy-and-paste these names precisely when setting the devices, as
shown below...

Now, set the appropriate devices. This is a one-off operation, you don't need to do it every time Digimorse runs - the
settings you make here are saved in the software's configuration file. The following command should be given on one
line; it is split for display in this guide:

```
$ digimorse --audioout "Built-in Output" --rigaudioout "USB AUDIO  CODEC" --rigaudioin "USB AUDIO  CODEC"
  --keyer /dev/tty.usbserial-1420
[2021-09-17T07:50:13Z INFO  digimorse] Setting audio output device to 'Built-in Output'
[2021-09-17T07:50:13Z INFO  digimorse] Setting rig output device to 'USB AUDIO  CODEC'
[2021-09-17T07:50:13Z INFO  digimorse] Setting audio input device to 'USB AUDIO  CODEC'
[2021-09-17T07:50:13Z INFO  digimorse] Audio output device is 'Built-in Output'
[2021-09-17T07:50:13Z INFO  digimorse] Rig output device is 'USB AUDIO  CODEC'
[2021-09-17T07:50:13Z INFO  digimorse] Rig input device is 'USB AUDIO  CODEC'
[2021-09-17T07:50:13Z WARN  digimorse] No keyer serial port device has been configured; use the -k or --keyer options
```

Ok, now we have the audio devices set, we need to tell Digimorse which device the keyer is connected to.
Currently there's no easy way to show which device this is. Basically, plug it in, and:
* On Windows, look in Device Manager under COM and LPT ports, to see what's new.
* On macOS, in a terminal, ls -l /dev/tty.usbserial* and choose the device file you see there.



## Configuration File
* macOS: /Users/<your username>/Library/ApplicationData/digimorse/digimorse.toml
* Linux: /home/<your username>/.digimorse/digimorse.toml
* Windows: C:\Users\<your username>\AppData\Roaming\digimorse.toml

# Development

## Current activities
For information on current development activities, please see the [TODO file](TODO.md) file.

## Technology
All development is in Rust, which is a new, difficult, but interesting
language. It would be easier in C++, and quicker - but probably less provably
correct, and portability would be painful.

## Source directory structure
The source is split into the following directories:

src/lib.rs - main body of code split into various modules.

src/digimorse.bin/main.rs - the main application.

docs - documentation, rough notes, references.

## Building
You need Rust 1.63.0 or greater.

* cargo test
* cargo build --release

### Building on macOS
Since digimorse uses fltk-sys, which compiles its C++ during build, and this requires the macOS SDK, you may find you
need to create a symbolic link to fix this build issue:

```
 fatal: not a git repository (or any of the parent directories): .git
  CMake Warning at /opt/local/share/cmake-3.22/Modules/Platform/Darwin-Initialize.cmake:303 (message):
    Ignoring CMAKE_OSX_SYSROOT value:

     /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX12.0.sdk

    because the directory does not exist.
```

In /Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/, there's a directory called
MacOSX.sdk. After upgrading XCode, I had to create a link in the SDKs directory pointing to it:

```
bash-3.2# ls -l
total 0
drwxr-xr-x  7 root  wheel  224 Nov 19 22:25 MacOSX.sdk
lrwxr-xr-x  1 root  wheel   10 Mar  8 07:44 MacOSX12.0.sdk -> MacOSX.sdk
lrwxr-xr-x  1 root  wheel   10 Mar  7 10:03 MacOSX12.1.sdk -> MacOSX.sdk
```

You also need portaudio (which needs pkg-config). Install from macports. There are brew variants, but I don't know brew.

### Building on Windows
.. not yet ..

### Building on Linux
e.g. on Ubuntu (I'm using Ubuntu 22.04 Budgie)
apt install build-essential cmake
apt install libudev1 libudev0 libudev-dev librust-libudev-dev 
apt install libportaudio2
apt install libxft-dev libxext-dev libxinerama-dev libxcursor-dev libpango1.0-dev
apt install libfltk1.1 libfltk1.3 libfltk1.1-dev libfltk1.3-dev
Install rustup.


# Documentation
Please see the 'docs' directory. The main document is ['The digimorse Communications Protocol'](docs/the-digimorse-communications-protocol.pdf).

There are various other rough notes in ASCII, RTF and OmmWriter format.

The documentation is built using LaTeX. I use MacPorts, with the `texlive` and
`texlive-latex-extra` packages. The main styles used are from the Tufte-LaTeX package, which may be found at
https://github.com/Tufte-LaTeX/tufte-latex. I use the `tufte-handout` class.

To build the documentation, a simple 'make' should suffice - this produces the relevant PDFs.




# Acknowledgements
Bart Massey's PortAudio examples at https://github.com/BartMassey/portaudio-rs-demos

Shepmaster's panic_after test helper routine at https://github.com/rust-lang/rfcs/issues/2798

The authors of the Tufte-LaTeX package, from https://github.com/Tufte-LaTeX/tufte-latex 

Wim Looman provided the initial LaTeX Makefile, from https://gist.github.com/Nemo157/539229

Martin Nawrath for the Direct Digital Synthesis sine wave generator ported to Rust in ToneGenerator.rs.
See http://interface.khm.de/index.php/lab/interfaces-advanced/arduino-dds-sinewave-generator/
Used in earlier keying generation code in ToneGenerator.rs.

Prof. Sarah J. Johnson for writing "Iterative Error Correction: Turbo, Low-Density Parity-Check and Repeat-
Accumulate Codes".

Adam Greig for his labrador-ldpc LDPC encoder/decoder at
https://github.com/adamgreig/labrador-ldpc
Used in the digimorse channel codec. Thanks also to Ed W8EMV for suggesting it on the mastodon.radio instance. 

Radford M. Neal for his LDPC-Codes LDPC parity check matrix generation code at
https://github.com/radfordneal/LDPC-codes
Used in early experiments in generating the LDPC parity check and generator matrices.

Maxime Tremblay for the LDPC and Sparse Binary Matrix crates at https://github.com/maxtremblay/ldpc and
https://github.com/maxtremblay/sparse-binary-matrix
Used in early experiments encoding and decoding in the digimorse channel codec.

Daniel Estévez's LDPC Toolbox crate at https://github.com/daniestevez/ldpc-toolbox
Used in initial experiments encoding and decoding in the digimorse channel codec.

Kārlis Goba YL3JG's ft8_lib at https://github.com/kgoba/ft8_lib.
In particular, the Gaussian Frequency Shift Keying code at https://github.com/kgoba/ft8_lib/blob/master/gen_ft8.c which
is adapted to the slightly wider bandwidth modulation used by digimorse in the transmitter.

# Bibliography
TBC



# License, Copyright & Contact info
This code is released under the Apache 2.0 License: http://www.apache.org/licenses/LICENSE-2.0.html.

(C) 2020-2022 Matt J. Gumbley

matt.gumbley@devzendo.org

@mattgumbley @devzendo

http://devzendo.github.io/digimorse
