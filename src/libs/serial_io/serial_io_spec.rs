
#[cfg(test)]
mod serial_io_spec {
    //use hamcrest2::prelude::*;
    use crate::libs::serial_io::serial_io::{DefaultSerialIO, SerialIO};

    #[ctor::ctor]
    fn before_each() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[ctor::dtor]
    fn after_each() {

    }

    #[test]
    fn timeout() {
        let serial_io = DefaultSerialIO::new("/dev/tty.usbserial-1410".to_string());
        match serial_io {
            Ok(mut serialport) => {
                let mut buf: [u8; 1] = [0];
                for _n in 1..10 {
                    let read = serialport.read(& mut buf);
                    match read {
                        Ok(read_bytes) => {
                            println!("Read {} bytes", read_bytes);
                        }
                        Err(e) => {
                            println!("Error {}", e.to_string());
                        }
                    };
                }
            }
            Err(e) => {
                panic!(e);
            }
        }
    }
}
