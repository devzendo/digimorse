use log::{info, warn};
use serialport::{SerialPort, SerialPortSettings, DataBits, FlowControl, Parity, StopBits};
use std::time::Duration;
use std::io;

// This trait is an abstraction over serialport, so that it can be mocked. Read and write are
// blocking.
pub trait SerialIO : Send {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>;
    fn flush(&mut self) -> io::Result<()>;
}

// A SerialIO that uses serialport.
#[readonly::make]
pub struct DefaultSerialIO {
    serial_port: Box<dyn SerialPort>
}

impl DefaultSerialIO {
    pub fn new(port_name: String) -> Result<DefaultSerialIO, String> {
        info!("Opening serial port '{}'", port_name);
        let settings: SerialPortSettings = SerialPortSettings {
            baud_rate: 115200, // Greatest speed of the Arduino serial monitor
            // https://arduino.stackexchange.com/questions/296/how-high-of-a-baud-rate-can-i-go-without-errors
            data_bits: DataBits::Eight,
            flow_control: FlowControl::None,
            parity: Parity::None,
            stop_bits: StopBits::One,
            timeout: Duration::from_millis(20)
        };
        return match serialport::open_with_settings(&port_name, &settings) {
            Ok(port) => {
                info!("Port open");
                Ok(DefaultSerialIO {
                    serial_port: port
                })
            }
            Err(e) => {
                let msg = format!("Failed to open '{}'. Error: {}", port_name, e);
                warn!("{}", msg);
                Err(msg)
            }
        }
    }
}

impl SerialIO for DefaultSerialIO {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        return self.serial_port.read(buf);
    }

    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        return self.serial_port.write(buf);
    }

    fn flush(&mut self) -> io::Result<()> {
        return self.serial_port.flush();
    }
}


#[cfg(test)]
#[path = "./serial_io_spec.rs"]
mod serial_io_spec;
