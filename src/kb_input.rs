use winit::{
    event::ElementState,
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum KbButtonState {
    #[default]
    None = 0,
    JustPressed = 1,
    Down = 2,
}

impl KbButtonState {
    pub fn is_down(&self) -> bool {
        *self == KbButtonState::Down
    }

    pub fn just_pressed(&self) -> bool {
        *self == KbButtonState::JustPressed
    }
}

#[derive(Debug, Default)]
pub struct KbInputManager {
    pub key_space: KbButtonState,

    pub key_arrow_left: KbButtonState,
    pub key_arrow_up: KbButtonState,
    pub key_arrow_down: KbButtonState,
    pub key_arrow_right: KbButtonState,

    pub key_w: KbButtonState,
    pub key_a: KbButtonState,
    pub key_s: KbButtonState,
    pub key_d: KbButtonState,

    pub key_i: KbButtonState,
    pub key_y: KbButtonState,
    pub key_m: KbButtonState,
    pub key_h: KbButtonState,
    pub key_v: KbButtonState,

    pub touch: KbButtonState,
}

#[allow(dead_code)]
impl KbInputManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update_touch(&mut self, phase: winit::event::TouchPhase) {
        if phase == winit::event::TouchPhase::Started {
            self.touch = KbButtonState::JustPressed
        } else if phase == winit::event::TouchPhase::Cancelled || phase == winit::event::TouchPhase::Ended {
            self.touch = KbButtonState::None
        }
    }

    pub fn update(&mut self, key: PhysicalKey, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;

        match key {
            PhysicalKey::Code(KeyCode::KeyA) => {
                if pressed {
                    if self.key_a == KbButtonState::None {
                        self.key_a = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_a = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyD) => {
                if pressed {
                    if self.key_d == KbButtonState::None {
                        self.key_d = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_d = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyW) => {
                if pressed {
                    if self.key_w == KbButtonState::None {
                        self.key_w = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_w = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyS) => {
                if pressed {
                    if self.key_s == KbButtonState::None {
                        self.key_s = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_s = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::Space) => {
                if pressed {
                    if self.key_space == KbButtonState::None {
                        self.key_space = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_space = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::ArrowUp) => {
                if pressed {
                    if self.key_arrow_up == KbButtonState::None {
                        self.key_arrow_up = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_arrow_up = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::ArrowDown) => {
                if pressed {
                    if self.key_arrow_down == KbButtonState::None {
                        self.key_arrow_down = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_arrow_down = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::ArrowLeft) => {
                if pressed {
                    if self.key_arrow_left == KbButtonState::None {
                        self.key_arrow_left = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_arrow_left = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::ArrowRight) => {
                if pressed {
                    if self.key_arrow_right == KbButtonState::None {
                        self.key_arrow_right = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_arrow_right = KbButtonState::None;
                }
                true
            }
            PhysicalKey::Code(KeyCode::KeyH) => {
                if pressed {
                    if self.key_h == KbButtonState::None {
                        self.key_h = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_h = KbButtonState::None;
                }
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
            PhysicalKey::Code(KeyCode::KeyM) => {
                if pressed {
                    if self.key_m == KbButtonState::None {
                        self.key_m = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_m = KbButtonState::None;
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
            PhysicalKey::Code(KeyCode::KeyV) => {
                if pressed {
                    if self.key_v == KbButtonState::None {
                        self.key_v = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_v = KbButtonState::None;
                }
                true
            }
            _ => false,
        }
    }

    pub fn update_key_states(&mut self) {
        if self.key_i == KbButtonState::JustPressed {
            self.key_i = KbButtonState::Down;
        }
        if self.key_y == KbButtonState::JustPressed {
            self.key_y = KbButtonState::Down;
        }
        if self.key_m == KbButtonState::JustPressed {
            self.key_m = KbButtonState::Down;
        }
        if self.key_h == KbButtonState::JustPressed {
            self.key_h = KbButtonState::Down;
        }
        if self.key_v == KbButtonState::JustPressed {
            self.key_v = KbButtonState::Down;
        }
        if self.key_w == KbButtonState::JustPressed {
            self.key_w = KbButtonState::Down;
        }
        if self.key_a == KbButtonState::JustPressed {
            self.key_a = KbButtonState::Down;
        }
        if self.key_s == KbButtonState::JustPressed {
            self.key_s = KbButtonState::Down;
        }
        if self.key_d == KbButtonState::JustPressed {
            self.key_d = KbButtonState::Down;
        }
        if self.key_arrow_left == KbButtonState::JustPressed {
            self.key_arrow_left = KbButtonState::Down;
        }
        if self.key_arrow_up == KbButtonState::JustPressed {
            self.key_arrow_up = KbButtonState::Down;
        }
        if self.key_arrow_right == KbButtonState::JustPressed {
            self.key_arrow_right = KbButtonState::Down;
        }
        if self.key_arrow_down == KbButtonState::JustPressed {
            self.key_arrow_down = KbButtonState::Down;
        }
        if self.key_space == KbButtonState::JustPressed {
            self.key_space = KbButtonState::Down;
        }

        if self.touch == KbButtonState::JustPressed {
            self.touch = KbButtonState::Down;
        }
    }

    pub fn key_h(&self) -> KbButtonState {
        self.key_h.clone()
    }

    pub fn key_i(&self) -> KbButtonState {
        self.key_i.clone()
    }

    pub fn key_m(&self) -> KbButtonState {
        self.key_m.clone()
    }

    pub fn key_y(&self) -> KbButtonState {
        self.key_y.clone()
    }

    pub fn key_v(&self) -> KbButtonState {
        self.key_v.clone()
    }

    pub fn key_arrow_up(&self) -> KbButtonState {
        self.key_arrow_up.clone()
    }

    pub fn key_arrow_down(&self) -> KbButtonState {
        self.key_arrow_down.clone()
    }

    pub fn key_arrow_left(&self) -> KbButtonState {
        self.key_arrow_left.clone()
    }

    pub fn key_arrow_right(&self) -> KbButtonState {
        self.key_arrow_right.clone()
    }

    pub fn get_key_state(&self, key: &str) -> KbButtonState {
        let button_state = match key {
            "w" => self.key_w.clone(),
            "a" => self.key_a.clone(),
            "s" => self.key_s.clone(),
            "d" => self.key_d.clone(),
            "left_arrow" => self.key_arrow_left.clone(),
            "right_arrow" => self.key_arrow_right.clone(),
            "up_arrow" => self.key_arrow_up.clone(),
            "down_arrow" => self.key_arrow_down.clone(),
            "space" => self.key_space.clone(),
            "touch" => self.touch.clone(),
            _ => KbButtonState::None,
        };

        button_state
    }
}
