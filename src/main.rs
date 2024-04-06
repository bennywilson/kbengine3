use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
  //  window::*,
};

mod game_texture;
mod game_object;
mod game_renderer;
mod game_engine;
mod game_input;

use crate::game_engine::GameEngine;

pub async fn run() {

    env_logger::init();
    let event_loop: EventLoop<()> = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window_size: winit::dpi::Size = winit::dpi::Size::new(winit::dpi::PhysicalSize::new(1920.0, 1080.0));
    if window.request_inner_size(window_size) != None {
        println!("Display will return window size later");
    }
    
    let mut game_engine = GameEngine::new(window).await;
    game_engine.initialize_world();
    
	let _ = event_loop.run(move |event, control_flow| {
        match event {

            Event::AboutToWait => {
                game_engine.tick_frame();
            }

            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == game_engine.window_id() => {

                match event {
                    WindowEvent::RedrawRequested => { game_engine.render_frame(); }
                    WindowEvent::CloseRequested => { control_flow.exit() }
                    WindowEvent::Resized(physical_size) => {
                        game_engine.resize(*physical_size);
                    }
                    /*WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                        game_engine.resize(winit::dpi::PhysicalSize::<u32>::new(scale_factor));
                    }*/
                    WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                        game_engine.input_manager.update(event.physical_key, event.state);
                    }
                    _ => { }
                }
            }
            _ => {game_engine.tick_frame();}
        }
    });
}

fn main() {
   pollster::block_on(run());
}