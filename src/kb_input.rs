use std::collections::HashMap;

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
pub struct KbTouchInfo {
    pub start_pos: (f64, f64),
    pub current_pos: (f64, f64),
    pub frame_delta: (f64, f64),
    pub touch_state: KbButtonState,
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

    pub key_shift: KbButtonState,

    pub touch_id_to_info: HashMap<u64, KbTouchInfo>,
}

#[allow(dead_code)]
impl KbInputManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update_touch(
        &mut self,
        phase: winit::event::TouchPhase,
        id: u64,
        location: winit::dpi::PhysicalPosition<f64>,
    ) {
        if phase == winit::event::TouchPhase::Started {
            let touch_info = KbTouchInfo {
                start_pos: location.into(),
                current_pos: location.into(),
                frame_delta: (0.0, 0.0),
                touch_state: KbButtonState::JustPressed,
            };
            self.touch_id_to_info.insert(id, touch_info);
        } else if phase == winit::event::TouchPhase::Cancelled
            || phase == winit::event::TouchPhase::Ended
        {
            self.touch_id_to_info.remove(&id);
        } else if phase == winit::event::TouchPhase::Moved {
            let touch_info = &mut self.touch_id_to_info.get_mut(&id).unwrap();
            touch_info.frame_delta.0 = touch_info.current_pos.0 - location.x;
            touch_info.frame_delta.1 = touch_info.current_pos.1 - location.y;
            touch_info.current_pos.0 = location.x;
            touch_info.current_pos.1 = location.y;
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
            PhysicalKey::Code(KeyCode::ShiftLeft) => {
                if pressed {
                    if self.key_shift == KbButtonState::None {
                        self.key_shift = KbButtonState::JustPressed;
                    }
                } else {
                    self.key_shift = KbButtonState::None;
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
        if self.key_shift == KbButtonState::JustPressed {
            self.key_shift = KbButtonState::Down;
        }

        for touch in &mut self.touch_id_to_info {
            touch.1.frame_delta.0 = 0.0;
            touch.1.frame_delta.1 = 0.0;
            touch.1.touch_state = KbButtonState::Down;
        }
    }

    pub fn get_key_state(&self, key: &str) -> KbButtonState {
        let button_state = match key {
            "v" => self.key_v.clone(),
            "w" => self.key_w.clone(),
            "a" => self.key_a.clone(),
            "s" => self.key_s.clone(),
            "d" => self.key_d.clone(),
            "m" => self.key_m.clone(),
            "i" => self.key_i.clone(),
            "y" => self.key_y.clone(),
            "h" => self.key_h.clone(),
            "left_arrow" => self.key_arrow_left.clone(),
            "right_arrow" => self.key_arrow_right.clone(),
            "up_arrow" => self.key_arrow_up.clone(),
            "down_arrow" => self.key_arrow_down.clone(),
            "space" => self.key_space.clone(),
            "left_shift" => self.key_shift.clone(),
            _ => {
                panic!("Doh!");
            }
        };

        button_state
    }

    pub fn get_touch_map(&self) -> &HashMap<u64, KbTouchInfo> {
        &self.touch_id_to_info
    }
}
