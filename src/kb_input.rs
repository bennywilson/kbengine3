use winit::{event::ElementState, keyboard::{KeyCode, PhysicalKey}};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum KbButtonState {
   #[default]
   None = 0,
   JustPressed = 1,
   Down = 2,
}

#[derive(Debug, Default)]
pub struct KbInputManager {
    pub left_arrow_pressed: bool,
    pub right_arrow_pressed: bool,
    pub up_arrow_pressed: bool,
    pub down_arrow_pressed:bool,
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub fire_pressed: bool,
    pub one_pressed: bool,
    pub two_pressed: bool,
    pub three_pressed: bool,
    pub four_pressed: bool,
    pub space_pressed: bool,

    pub key_i: KbButtonState,
    pub key_y: KbButtonState,
}

#[allow(dead_code)] 
impl KbInputManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self, key: PhysicalKey, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;

        match key {
            PhysicalKey::Code(KeyCode::KeyA) => {
                self.left_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) => {
                self.right_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyW) => {
                self.up_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) => {
                self.down_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::Space) => {
                self.fire_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::Digit1) => {
                self.one_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::Digit2) => {
                self.two_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::Digit3) => {
                self.three_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::Digit4) => {
                self.four_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::ArrowUp) => {
                self.up_arrow_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::ArrowDown) => {
                self.down_arrow_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                self.left_arrow_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                self.right_arrow_pressed = pressed;
                true
            }
            PhysicalKey::Code(KeyCode::KeyI) => {
                if pressed {
                    if self.key_i == KbButtonState::None {
                        self.key_i = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_i = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyY) => {
                if pressed {
                    if self.key_y == KbButtonState::None {
                        self.key_y = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_y = KbButtonState::None;
                }
                true
            }
            _ => false
        }
    }

    pub fn update_key_states(&mut self) {
        if self.key_i == KbButtonState::JustPressed {
            self.key_i = KbButtonState::Down;
        }
        if self.key_y == KbButtonState::JustPressed {
            self.key_y = KbButtonState::Down;
        }
    }

    pub fn up_pressed(&self) -> bool {
        self.up_pressed
    }

    pub fn down_pressed(&self) -> bool {
        self.down_pressed
    }

    pub fn left_pressed(&self) -> bool {
        self.left_pressed
    }

    pub fn right_pressed(&self) -> bool {
        self.right_pressed
    }

    pub fn fire_pressed(&self) -> bool {
        self.fire_pressed
    }

    pub fn key_i(&self) -> KbButtonState {
        self.key_i.clone()
    }

    pub fn key_y(&self) -> KbButtonState {
        self.key_y.clone()
    }
}