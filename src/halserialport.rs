// Inline implementation for embedded_hal 0.2.7 serial abstraction towards SerialPort
// Fixed solution from vesc-comm and serial-embedded-hal (https://github.com/thenewwazoo/serial-embedded-hal)
// - Use serialport cargo instead of serial
// - Removed Mutex for single thread use, using Box instead
// - Improved write WouldBlock conditions

extern crate embedded_hal;
extern crate nb;
extern crate serialport;

pub struct HalSerialPort {
    inner: Box<dyn serialport::SerialPort>,
}

impl HalSerialPort {
    pub fn new(inner: Box<dyn serialport::SerialPort>) -> Self {
        HalSerialPort { inner }
    }
}

impl embedded_hal::serial::Read<u8> for HalSerialPort {
    type Error = serialport::Error;

    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        let mut buf: [u8; 1] = [0];
        match (*self).inner.read(&mut buf) {
            Ok(_) => {
                // println!("Read {}", buf[0]);
                Ok(buf[0])
            }
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => Err(nb::Error::WouldBlock),
                std::io::ErrorKind::TimedOut => Err(nb::Error::WouldBlock),
                _ => Err(nb::Error::Other(serialport::Error::new(
                    serialport::ErrorKind::Io(e.kind()),
                    "bad read",
                ))),
            },
        }
    }
}

impl embedded_hal::serial::Write<u8> for HalSerialPort {
    type Error = serialport::Error;

    fn write(&mut self, byte: u8) -> nb::Result<(), Self::Error> {
        match (*self).inner.write(&[byte]) {
            Ok(1) => {
                // println!("Wrote {}", byte);
                Ok(())
            }
            Ok(_) => Err(nb::Error::WouldBlock),
            Err(e) => match e.kind() {
                std::io::ErrorKind::WouldBlock => Err(nb::Error::WouldBlock),
                std::io::ErrorKind::TimedOut => Err(nb::Error::WouldBlock),
                _ => Err(nb::Error::Other(serialport::Error::new(
                    serialport::ErrorKind::Io(e.kind()),
                    "bad write",
                ))),
            },
        }
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        match (*self).inner.flush() {
            Ok(_) => Ok(()),
            Err(e) => Err(nb::Error::Other(serialport::Error::new(
                serialport::ErrorKind::Io(e.kind()),
                "bad flush",
            ))),
        }
    }
}
