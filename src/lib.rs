use std::sync::Arc;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod kb_config;
pub mod kb_engine;
pub mod kb_input;
pub mod kb_object;
pub mod kb_pipeline;
pub mod kb_renderer;
pub mod kb_resource;
pub mod kb_utils;

use crate::kb_config::KbConfig;
use crate::kb_engine::KbGameEngine;
use crate::kb_input::InputManager;
use crate::kb_renderer::KbRenderer;
use crate::kb_resource::KbPostProcessMode;

#[cfg(target_arch = "wasm32")]
const WEBAPP_CANVAS_ID: &str = "target";

pub async fn run_game<T>() where T: KbGameEngine {
    env_logger::init();

    let mut game_config = KbConfig::new();

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

    let mut game_engine = T::new(&game_config);
    game_engine.initialize_world();

    let mut input_manager = InputManager::new();
    let mut game_renderer = KbRenderer::new(window.clone(), &game_config).await;

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
      //  game_renderer.init_renderer(window.clone()).await;
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
                            let render_result = game_renderer.render_frame(&game_engine.game_objects, &game_config);
                            match render_result {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => { game_renderer.resize(&game_config); },
		                        Err(wgpu::SurfaceError::OutOfMemory) => { control_flow.exit() }
		                        Err(e) => { eprintln!("{:?}", e) },
                            }
                        }
        
                        WindowEvent::CloseRequested => { control_flow.exit() }

                        WindowEvent::Resized(physical_size) => {
                            if physical_size.width > 0 && physical_size.height > 0 {
                                game_config.window_width = physical_size.width;
                                game_config.window_height = physical_size.height;
                                game_renderer.resize(&game_config);
                            }
                        }

                        WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                            input_manager.update(event.physical_key, event.state);

                            game_config.postprocess_mode = {
                                if input_manager.one_pressed { KbPostProcessMode::Passthrough } else
                                if input_manager.two_pressed { KbPostProcessMode::Desaturation } else
                                if input_manager.three_pressed { KbPostProcessMode::ScanLines } else
                                if input_manager.four_pressed { KbPostProcessMode::Warp } else 
                                { game_config.postprocess_mode.clone() }
                            }
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

//        game_renderer.init_renderer(window.clone(), &game_config).await;
	    let _ = event_loop.run( |event, control_flow| {

            match event {

                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == game_renderer.window_id() => {

                    match event {
                        WindowEvent::RedrawRequested => {
                            game_engine.tick_frame(&input_manager);
                            let render_result = game_renderer.render_frame(&game_engine.get_game_objects(), &game_config);
                            match render_result {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => { game_renderer.resize(&game_config); },
		                        Err(wgpu::SurfaceError::OutOfMemory) => { control_flow.exit() }
		                        Err(e) => { eprintln!("{:?}", e) },
                            }
                        }
        
                        WindowEvent::CloseRequested => { control_flow.exit() }

                        WindowEvent::Resized(physical_size) => {
                            if physical_size.width > 0 && physical_size.height > 0 {
                                game_config.window_width = physical_size.width;
                                game_config.window_height = physical_size.height;
                                game_renderer.resize(&game_config);
                            }
                        }

                        WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                            input_manager.update(event.physical_key, event.state);

                            game_config.postprocess_mode = {
                                if input_manager.one_pressed { KbPostProcessMode::Passthrough } else
                                if input_manager.two_pressed { KbPostProcessMode::Desaturation } else
                                if input_manager.three_pressed { KbPostProcessMode::ScanLines } else
                                if input_manager.four_pressed { KbPostProcessMode::Warp } else 
                                { game_config.postprocess_mode.clone() }
                            }
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
