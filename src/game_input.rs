//use winit::event::{VirtualKeyCode, ElementState};
use winit::*;

#[derive(Debug, Default)]
pub struct InputManager {
    pub left_pressed: bool,
    pub right_pressed: bool,
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub fire_pressed: bool
}

#[allow(dead_code)] 
impl InputManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn update(&mut self, key: winit::keyboard::PhysicalKey, state: event::ElementState) -> bool {
        let pressed = state == winit::event::ElementState::Pressed;
        match key {
            keyboard::PhysicalKey::Code(keyboard::KeyCode::KeyA) => {
                self.left_pressed = pressed;
                true
            }
            keyboard::PhysicalKey::Code(keyboard::KeyCode::KeyD) => {
                self.right_pressed = pressed;
                true
            }
            keyboard::PhysicalKey::Code(keyboard::KeyCode::KeyW) => {
                self.up_pressed = pressed;
                true
            }
            keyboard::PhysicalKey::Code(keyboard::KeyCode::KeyS) => {
                self.down_pressed = pressed;
                true
            }
            keyboard::PhysicalKey::Code(keyboard::KeyCode::Space) => {
                self.fire_pressed = pressed;
                true
            }
            _ => false
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
}