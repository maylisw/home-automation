use embedded_hal::delay::DelayNs;

use embedded_hal::digital::PinState::{High, Low};
use embedded_hal::digital::{OutputPin, PinState};

/// different positions of the motor.
/// Depending on the state different pins have to be high
/// |wire | 0 | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 |
/// | --- | - | - | - | - | - | - | - | - | - |
/// |  1  |   |   |   |   |   |   | x | x | x |
/// |  2  |   |   |   |   | x | x | x |   |   |
/// |  3  |   |   | x | x | x |   |   |   |   |
/// |  4  |   | x | x |   |   |   |   |   | x |
#[derive(Copy, Clone, Debug)]
enum State {
    State0,
    State1,
    State2,
    State3,
    State4,
}

const fn get_pin_states(s: State) -> [PinState; 4] {
    match s {
        State::State0 => [Low, Low, Low, Low],
        State::State1 => [Low, Low, Low, High],
        State::State2 => [Low, Low, High, Low],
        State::State3 => [Low, High, Low, Low],
        State::State4 => [High, Low, Low, Low],
    }
}

const fn get_next_state(s: State) -> State {
    match s {
        State::State1 => State::State2,
        State::State2 => State::State3,
        State::State3 => State::State4,
        State::State4 | State::State0 => State::State1,
    }
}

const fn get_prev_state(s: State) -> State {
    match s {
        State::State0 | State::State1 => State::State4,
        State::State2 => State::State1,
        State::State3 => State::State2,
        State::State4 => State::State3,
    }
}

/// Struct representing a Stepper motor with the 4 driver pins
pub struct ULN2003<P1, P2, P3, P4, D>
where
    P1: OutputPin,
    P2: OutputPin,
    P3: OutputPin,
    P4: OutputPin,
    D: DelayNs,
{
    in1: P1,
    in2: P2,
    in3: P3,
    in4: P4,
    state: State,
    dir: Direction,
    delay: Option<D>,
}

impl<P1: OutputPin, P2: OutputPin, P3: OutputPin, P4: OutputPin, D: DelayNs>
    ULN2003<P1, P2, P3, P4, D>
{
    /// Create a new `StepperMotor` from the 4 pins connected to te uln2003 driver.
    /// The delay parameter is needed if you want to use the `step_for` function.
    pub const fn new(in1: P1, in2: P2, in3: P3, in4: P4, delay: Option<D>) -> Self {
        Self {
            in1,
            in2,
            in3,
            in4,
            state: State::State0,
            dir: Direction::Normal,
            delay,
        }
    }

    fn apply_state(&mut self) -> Result<(), StepError> {
        let states = get_pin_states(self.state);
        set_state(&mut self.in1, states[0])?;
        set_state(&mut self.in2, states[1])?;
        set_state(&mut self.in3, states[2])?;
        set_state(&mut self.in4, states[3])?;
        Ok(())
    }
}

/// gets returned if an Error happens while stepping
#[derive(Debug)]
pub struct StepError;

impl<P1: OutputPin, P2: OutputPin, P3: OutputPin, P4: OutputPin, D: DelayNs> StepperMotor
    for ULN2003<P1, P2, P3, P4, D>
{
    fn step(&mut self) -> Result<(), StepError> {
        match self.dir {
            Direction::Normal => self.state = get_next_state(self.state),
            Direction::Reverse => self.state = get_prev_state(self.state),
        }
        self.apply_state()?;
        Ok(())
    }

    fn step_for(&mut self, steps: i32, ms: u32) -> Result<(), StepError> {
        if self.delay.is_none() {
            return Err(StepError);
        }
        for _ in 0..steps {
            self.step()?;
            self.delay.as_mut().unwrap().delay_ms(ms);
        }
        Ok(())
    }

    fn set_direction(&mut self, dir: Direction) {
        self.dir = dir;
    }

    fn stop(&mut self) -> Result<(), StepError> {
        self.state = State::State0;
        self.apply_state()?;
        Ok(())
    }
}

fn set_state<P: OutputPin>(pin: &mut P, state: PinState) -> Result<(), StepError> {
    match pin.set_state(state) {
        Ok(()) => Ok(()),
        Err(_) => Err(StepError),
    }
}

/// trait to prevent having to pass around the struct with all the generic arguments
pub trait StepperMotor {
    /// Do a single step
    fn step(&mut self) -> Result<(), StepError>;
    /// Do multiple steps with a given delay in ms
    fn step_for(&mut self, steps: i32, delay: u32) -> Result<(), StepError>;
    /// Set the stepping direction
    fn set_direction(&mut self, dir: Direction);
    /// Stopping sets all pins low
    fn stop(&mut self) -> Result<(), StepError>;
}

/// Direction the motor turns in. Just reverses the order of the internal states.
pub enum Direction {
    Normal,
    Reverse,
}
