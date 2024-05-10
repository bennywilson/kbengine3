use std::sync::Arc;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

pub mod kb_assets;
pub mod kb_collision;
pub mod kb_config;
pub mod kb_engine;
pub mod kb_input;
pub mod kb_game_object;
pub mod kb_renderer;
pub mod kb_resource;
pub mod kb_utils;
pub mod render_groups {
    pub mod kb_line_group;
    pub mod kb_model_group;
    pub mod kb_postprocess_group;
    pub mod kb_sprite_group;
    pub mod kb_sunbeam_group;
}

use crate::kb_config::*;
use crate::kb_engine::*;
use crate::kb_input::*;
use crate::kb_renderer::*;
use crate::kb_resource::*;

#[cfg(target_arch = "wasm32")]
const WEBAPP_CANVAS_ID: &str = "target";

pub async fn run_game<T>(mut game_config: KbConfig) where T: KbGameEngine + 'static {
    env_logger::init();

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
    let mut input_manager = KbInputManager::new();
    let mut game_renderer = KbRenderer::new(window.clone(), &game_config).await;

    game_engine.initialize_world(&mut game_renderer, &game_config).await;

    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
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
                            game_engine.tick_frame(&mut game_renderer, &mut input_manager, &mut game_config);
                            let render_result = game_renderer.render_frame(&game_engine.get_game_objects(), &game_config);
                            match render_result {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => { let _ = async { game_renderer.resize(&game_config).await; }; },
		                        Err(wgpu::SurfaceError::OutOfMemory) => { control_flow.exit() }
		                        Err(e) => { eprintln!("{:?}", e) },
                            }
                        }
        
                        WindowEvent::CloseRequested => { control_flow.exit() }

                        WindowEvent::Resized(physical_size) => {
                            if physical_size.width > 0 && physical_size.height > 0 {
                                game_config.window_width = physical_size.width;
                                game_config.window_height = physical_size.height;
                                let _ = async { game_renderer.resize(&game_config).await; };
                            }
                        }

                        WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                            input_manager.update(event.physical_key, event.state);

                            if input_manager.key_h() == KbButtonState::JustPressed {
                                game_renderer.enable_help_text();
                            }

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
	    let _ = event_loop.run( |event, control_flow| {

            match event {

                Event::WindowEvent {
                    ref event,
                    window_id,
                } if window_id == game_renderer.window_id() => {

                    match event {
                        WindowEvent::RedrawRequested => {
                            game_engine.tick_frame(&mut game_renderer, &mut input_manager, &mut game_config);
                            let render_result = game_renderer.render_frame(&game_engine.get_game_objects(), &game_config);
                            match render_result {
                                Ok(_) => {}
                                Err(wgpu::SurfaceError::Lost) => { let _ = async { game_renderer.resize(&game_config).await; }; },
		                        Err(wgpu::SurfaceError::OutOfMemory) => { control_flow.exit() }
		                        Err(e) => { eprintln!("{:?}", e) },
                            }
                        }
        
                        WindowEvent::CloseRequested => { control_flow.exit() }

                        WindowEvent::Resized(physical_size) => {
                            if physical_size.width > 0 && physical_size.height > 0 {
                                game_config.window_width = physical_size.width;
                                game_config.window_height = physical_size.height;
                                let _ = async { game_renderer.resize(&game_config).await; };
                            }
                        }

                        WindowEvent::KeyboardInput { device_id: _, event, is_synthetic: _ } => {
                            input_manager.update(event.physical_key, event.state);

                            if input_manager.key_h() == KbButtonState::JustPressed {
                                game_renderer.enable_help_text();
                            }

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
