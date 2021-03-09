digimorse
=========
There's currently not much to see here, except the idea... perhaps come back
later?


This is an EXPERIMENT in taking Morse code, in real-time, encoding it, wrapping
it in modern error-detection/correction codes, and modulating it out to a
connected amateur radio transceiver as a narrow-bandwidth signal. 

The intention is to take our historic mode of communication, and be able to
receive it and decode it (with your brain) whilst removing the effects of
natural or man-made interference and fading. Visualise all the transmissions in
a waterfall display, making it easy to see and select transmissions of interest.
Filter out some or all stations so you can hone in on the one you want - and
reply to it in digitally encoded Morse. By adding error correction, and
carefully designing the encoding and modulation, it is hoped that the weak
signal characteristic enjoyed by users of FT8, JT9 etc. can be added to Morse.

What digimorse is not:
* It doesn't decode the Morse for you. That's cheating! It is a beautiful skill
  to master, like a language or musical instrument; mastering Morse is a fine
  human achievement. digimorse seeks to add another dimension to it!
* The death of CW or Amateur Radio As We Know It(tm). You don't have to use it!
* Completely automated, just macro-key-pressing. No, you use your normal Morse
  key or paddle. Macros will come later perhaps.
* Quantized. If you have a unique rhythm to your keying, digimorse won't correct
  it. The timing of your keying goes out verbatim. You can use our keyer with a
  paddle which will give good timing.

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
* A fairly modern computer running Windows 10, macOS 10.15ish (Catalina,
  possibly Big Sur), or Ubuntu 16.xx LTS, 18.xx LTS, 20.xx LTS.
* Sorry no Raspberry Pi yet - definitely considering a build for this.
* An interface between the computer sound system and your amateur radio
  transceiver, e.g. Tigertronics SignaLink USB.
* Monitor, keyboard, mouse.
* An interface between your Morse key or paddle and USB. See the
  https://github.com/devzendo/digimorse-arduino-keyer project for a simple
  Arduino Nano-based Morse key/paddle <-> USB Serial interface and simple keyer.


(C) 2020 Matt J. Gumbley
matt.gumbley@devzendo.org
@mattgumbley @devzendo
http://devzendo.github.io/digimorse


Status
------
Project started September 2020. Status: initial investigations, feasiblilty,
thoughts.

In active development:
* Building the keyer.
* Reading the keyer output via the USB Serial link, and determining duration of
  on-off key timing from the USB stream. Feasibility investigation.
* All development is in Rust, which is a new, difficult, but interesting
  language. It would be easier in C++, and quicker - but probably less provably
  correct, and portability would be painful.


Roadmap
-------
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


Release Notes
-------------
0.0.1 First Release (work in progress)
* Created repository! Wrote the above... made notes... learning about
  information theory, error control coding, modulation/demodulation, the Fourier
  transform, the Rust programming language, the FLTK and PortAudio frameworks for
  GUI and audio, built the Arduino keyer.

 
Source directory structure
--------------------------
The source (in the Rust programming language) is split into the following directories:

src/lib.rs - main body of code split into various modules.

src/digimorse.bin/main.rs - the main application.

docs - documentation, rough notes, references.


Building
--------
You need Rust 1.47.0 or greater.

* cargo test
* cargo build --release


License
-------
This code is released under the Apache 2.0 License: http://www.apache.org/licenses/LICENSE-2.0.html.
(C) 2020 Matt Gumbley, DevZendo.org



Bibliography
------------
TBC



