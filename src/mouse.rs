use uinput::event::controller::Controller::Mouse;
use uinput::event::controller::Mouse::Left;
use uinput::event::relative::Position::{X, Y};
use uinput::event::relative::Relative::Position;
use uinput::event::Event::{Controller, Relative};
use uinput::{Device, Error};

pub fn init() -> Result<Device, Error> {
    uinput::default()?
        .name("test")?
        .event(Controller(Mouse(Left)))?
        .event(Relative(Position(X)))?
        .event(Relative(Position(Y)))?
        .create()
}

pub fn update(
    device: &mut Device,
    x_delta: f32,
    y_delta: f32,
) -> Result<(), Error> {
    device.send(X, x_delta as i32)?;
    device.send(Y, y_delta as i32)?;
    device.synchronize()
}
