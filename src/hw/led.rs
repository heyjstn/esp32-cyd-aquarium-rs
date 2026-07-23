use esp_idf_hal::gpio::{Output, OutputPin, PinDriver};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;
use esp_idf_hal::ledc::LedcDriver;

/// Color is constructed by mixing of RGB
///
/// Status = 1 means that the LED color is enabled
#[derive(PartialEq, Eq)]
pub enum Color {
    RED,
    GREEN,
    BLUE,
    YELLOW,
}

pub enum LedMode {
    Off,
    Solid(Color),
    Blink { color: Color, interval: Duration },
}

pub struct Led<'d> {
    red: PinDriver<'d, Output>,
    green: PinDriver<'d, Output>,
    blue: PinDriver<'d, Output>,
}

pub fn new(
    red: impl OutputPin + 'static,
    green: impl OutputPin + 'static,
    blue: impl OutputPin + 'static,
) -> Led<'static> {
    Led {
        red: PinDriver::output(red).unwrap(),
        green: PinDriver::output(green).unwrap(),
        blue: PinDriver::output(blue).unwrap(),
    }
}

impl Led<'static> {
    fn on(&mut self, color: &Color) -> anyhow::Result<()> {
        if *color == Color::YELLOW {
            self.red.set_low()?;
            self.green.set_low()?;
            self.blue.set_high()?;
            return Ok(());
        }

        if *color == Color::RED {
            self.red.set_low()?;
        } else {
            self.red.set_high()?;
        };

        if *color == Color::GREEN {
            self.green.set_low()?;
        } else {
            self.green.set_high()?;
        };

        if *color == Color::BLUE {
            self.blue.set_low()?;
        } else {
            self.blue.set_high()?;
        };
        Ok(())
    }

    pub fn off(&mut self) -> anyhow::Result<()> {
        self.red.set_high()?;
        self.green.set_high()?;
        self.blue.set_high()?;
        Ok(())
    }

    pub fn run(&mut self, receiver: Receiver<LedMode>) -> anyhow::Result<()> {
        let mut mode = receiver.recv()?;

        loop {
            mode = match mode {
                LedMode::Off => {
                    self.off()?;
                    receiver.recv()?
                }
                LedMode::Solid(color) => {
                    self.on(&color)?;
                    receiver.recv()?
                }
                LedMode::Blink { color, interval } => {
                    self.next_blink_mode(&receiver, color, interval)?
                }
            };
        }
    }

    fn next_blink_mode(
        &mut self,
        receiver: &Receiver<LedMode>,
        color: Color,
        interval: Duration,
    ) -> anyhow::Result<LedMode> {
        self.on(&color)?;
        if let Some(mode) = receive_mode(receiver, interval)? {
            return Ok(mode);
        }

        self.off()?;
        Ok(receive_mode(receiver, interval)?.unwrap_or(LedMode::Blink { color, interval }))
    }
}

fn receive_mode(
    receiver: &Receiver<LedMode>,
    timeout: Duration,
) -> anyhow::Result<Option<LedMode>> {
    match receiver.recv_timeout(timeout) {
        Ok(mode) => Ok(Some(mode)),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(err @ RecvTimeoutError::Disconnected) => Err(err.into()),
    }
}
