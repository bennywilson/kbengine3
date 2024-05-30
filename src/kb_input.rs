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

    pub key_1: KbButtonState,
    pub key_2: KbButtonState,
    pub key_3: KbButtonState,
    pub key_4: KbButtonState,
    pub key_plus: KbButtonState,
    pub key_minus: KbButtonState,

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

    key_map: HashMap<&'static str, KbButtonState>,
}

#[allow(dead_code)]
impl KbInputManager {
    pub fn new() -> Self {
        let key_map = HashMap::<&str, KbButtonState>::new();
        KbInputManager {
            key_map,
            ..Default::default()
        }
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

        let key_name = match key {
            PhysicalKey::Code(KeyCode::ArrowUp) => { "up_arrow" }
            PhysicalKey::Code(KeyCode::ArrowDown) => { "down_arrow" }
            PhysicalKey::Code(KeyCode::ArrowLeft) => { "left_arrow" }
            PhysicalKey::Code(KeyCode::ArrowRight) => { "right_arrow" } 
            PhysicalKey::Code(KeyCode::Digit1) => { "1" }
            PhysicalKey::Code(KeyCode::Digit2) => { "2" }
            PhysicalKey::Code(KeyCode::Digit3) => { "3" }
            PhysicalKey::Code(KeyCode::Digit4) => { "4" }
            PhysicalKey::Code(KeyCode::Digit5) => { "5" }
            PhysicalKey::Code(KeyCode::Digit6) => { "6" }
            PhysicalKey::Code(KeyCode::Digit7) => { "7" }
            PhysicalKey::Code(KeyCode::Digit8) => { "8" }
            PhysicalKey::Code(KeyCode::Digit9) => { "9" }
            PhysicalKey::Code(KeyCode::Digit0) => { "0" }
            PhysicalKey::Code(KeyCode::Equal) => { "+" },
            PhysicalKey::Code(KeyCode::Minus) => { "-" }
            PhysicalKey::Code(KeyCode::KeyW) => { "w" }
            PhysicalKey::Code(KeyCode::KeyA) => { "a" }
            PhysicalKey::Code(KeyCode::KeyS) => { "s" }
            PhysicalKey::Code(KeyCode::KeyD) => { "d" }
            PhysicalKey::Code(KeyCode::Space) => { "space" } 
            PhysicalKey::Code(KeyCode::KeyH) => { "h" }
            PhysicalKey::Code(KeyCode::KeyM) => { "m" }
            PhysicalKey::Code(KeyCode::KeyY) => { "y" }
            PhysicalKey::Code(KeyCode::KeyV) => { "v" }
            PhysicalKey::Code(KeyCode::ShiftLeft) => { "left_shift" }
            _ => { "none" }
        };
        if key_name == "none" {
            return false;
        }

        let key_pair = self.key_map.get_mut(&key_name);
        if key_pair.is_some() {
            let key_pair = key_pair.unwrap();x`x`
            if pressed {
                if *key_pair == KbButtonState::None {
                    *key_pair = KbButtonState::JustPressed;
                }
            } else {
                *key_pair = KbButtonState::None;    
            };
        } else {
            self.key_map.insert(key_name, KbButtonState::JustPressed);
        }
        return true;
    }

    pub fn update_key_states(&mut self) {
        for button_pair in &mut self.key_map {
            if *button_pair.1 == KbButtonState::JustPressed {
                *button_pair.1 = KbButtonState::Down;
            }
        }

        for touch in &mut self.touch_id_to_info {
            touch.1.frame_delta.0 = 0.0;
            touch.1.frame_delta.1 = 0.0;
            touch.1.touch_state = KbButtonState::Down;
        }
    }

    pub fn get_key_state(&self, key: &str) -> KbButtonState {
        let key_state = self.key_map.get(key);
        if key_state.is_some() {
            return key_state.unwrap().clone();
        }

        KbButtonState::None
    }

    pub fn get_touch_map(&self) -> &HashMap<u64, KbTouchInfo> {
        &self.touch_id_to_info
    }
}
