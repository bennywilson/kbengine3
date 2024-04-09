use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod game_texture;
mod game_object;
mod game_renderer;
mod game_engine;
mod game_input;
mod game_config;

use crate::game_engine::GameEngine;
use crate::game_renderer::GameRenderer;
use crate::game_config::GameConfig;

pub async fn run_game() {
    env_logger::init();
 
    let game_config = GameConfig::new();

    let event_loop: EventLoop<()> = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let window_size: winit::dpi::Size = winit::dpi::Size::new(winit::dpi::PhysicalSize::new(game_config.window_width, game_config.window_height));
    if window.request_inner_size(window_size) != None {
        println!("Display will return window size later");
    }

    let mut game_engine = GameEngine::new(&game_config).await;
    game_engine.initialize_world();
    
    let mut game_renderer = GameRenderer::new(&window, game_config).await;

	let _ = event_loop.run(move |event, control_flow| {
        match event {

            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == game_renderer.window_id() => {

                match event {
                    WindowEvent::RedrawRequested => {
                        game_engine.tick_frame();
                        let render_result = game_renderer.render_frame(&game_engine.game_objects);
                        match render_result {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => { game_renderer.resize(game_renderer.size); },
		                    Err(wgpu::SurfaceError::OutOfMemory) => { control_flow.exit() }
		                    Err(e) => { eprintln!("{:?}", e) },
                        }
                    }
        
                    WindowEvent::CloseRequested => { control_flow.exit() }

                    WindowEvent::Resized(physical_size) => { game_renderer.resize(*physical_size); }

                    WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                        game_engine.input_manager.update(event.physical_key, event.state);
                    }
                    _ => { }
                }
            }

            _ => {
                game_renderer.request_redraw();
            }
        }
    }); 
}
