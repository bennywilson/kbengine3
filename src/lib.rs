use std::sync::Arc;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod kb_resource;
mod kb_object;
mod kb_renderer;
mod kb_engine;
mod kb_input;
mod kb_config;
mod kb_log;
mod kb_utils;

use crate::kb_engine::KbEngine;
use crate::kb_renderer::KbRenderer;
use crate::kb_config::KbConfig;

#[cfg(target_arch = "wasm32")]
const WEBAPP_CANVAS_ID: &str = "target";

pub async fn run_game() {
    env_logger::init();

    let game_config = KbConfig::new();

    let event_loop: EventLoop<()> = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    #[cfg(target_arch = "wasm32")]
    let window = Arc::new({
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let dom_window = web_sys::window().unwrap();
        let dom_document = dom_window.document().unwrap();
        let dom_canvas = dom_document.get_element_by_id(WEBAPP_CANVAS_ID).unwrap();
        let canvas = dom_canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok();
        WindowBuilder::default()
            .with_canvas(canvas)
            .build(&event_loop).unwrap()
    });

    #[cfg(not(target_arch = "wasm32"))]
    let window = Arc::new({
        let window_size: winit::dpi::Size = winit::dpi::Size::new(winit::dpi::PhysicalSize::new(game_config.window_width, game_config.window_height));
        WindowBuilder::new().with_inner_size(window_size).build(&event_loop).unwrap()
    });

    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(game_config.window_width, game_config.window_height));

    let mut game_engine = KbEngine::new(&game_config);
    game_engine.initialize_world();
    
    let mut game_renderer = KbRenderer::new(window.clone(), game_config.clone());

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        game_renderer.init_renderer(window.clone()).await;
	    let _ = event_loop.spawn(move |event, control_flow| {
            let _ = &mut game_renderer;
            let _ = &game_config;
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
                            if game_engine.input_manager.one_pressed { game_renderer.set_postprocess_mode(game_renderer::PostProcessMode::Passthrough); }
                            if game_engine.input_manager.two_pressed { game_renderer.set_postprocess_mode(game_renderer::PostProcessMode::Desaturation); }
                            if game_engine.input_manager.three_pressed { game_renderer.set_postprocess_mode(game_renderer::PostProcessMode::ScanLines); }
                            if game_engine.input_manager.four_pressed { game_renderer.set_postprocess_mode(game_renderer::PostProcessMode::Warp); }
                        }
                        _ => { }
                    }
                }

                _ => {
                    window.request_redraw();
                }
            }
        });
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(1920, 1080));

        game_renderer.init_renderer(window.clone()).await;
	    let _ = event_loop.run( |event, control_flow| {

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

                            if game_engine.input_manager.one_pressed { game_renderer.set_postprocess_mode(kb_renderer::KbPostProcessMode::Passthrough); }
                            if game_engine.input_manager.two_pressed { game_renderer.set_postprocess_mode(kb_renderer::KbPostProcessMode::Desaturation); }
                            if game_engine.input_manager.three_pressed { game_renderer.set_postprocess_mode(kb_renderer::KbPostProcessMode::ScanLines); }
                            if game_engine.input_manager.four_pressed { game_renderer.set_postprocess_mode(kb_renderer::KbPostProcessMode::Warp); }
                        }
                        _ => { }
                    }
                }

                _ => {
                    window.request_redraw();
                }
            }
        });
    }
}
