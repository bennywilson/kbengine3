#![feature(async_closure)]

use winit::{
  //  event::*,
    event_loop::{ControlFlow, EventLoop},
  //  window::WindowBuilder,
  //  window::*,
};

mod game_texture;
mod game_object;
mod game_renderer;
mod game_engine;
mod game_input;
 use winit::platform::web::WindowExtWebSys;

 use winit::window::*;


use crate::game_engine::GameEngine;
use crate::game_engine::InitSystem;
use winit::dpi::*;

const WEBAPP_CANVAS_ID: &str = "target";

macro_rules! log {
    ( $( $t:tt )* ) => {
        web_sys::console::log_1(&format!( $( $t )* ).into());
    }
}

pub struct SomeRand {
}

pub async fn run_game() {
    env_logger::init();

    /*
    let event_loop: EventLoop<()> = EventLoop::new().unwrap();
    let window = winit::window::Window::new(&event_loop).unwrap();//std::sync::Arc::new();
    let _ = window.request_inner_size(PhysicalSize::new(1920, 1080));
    let window_arc = std::sync::Arc::new(window);//
    */

  //  let mut game_engine = GameEngine::new(window).await;

  //  pollster::block_on(painter::run_playground(event_loop, window));
    let event_loop: EventLoop<()> = EventLoop::new().unwrap();

    event_loop.set_control_flow(ControlFlow::Poll);
        use wasm_bindgen::JsCast;
        use winit::platform::web::WindowBuilderExtWebSys;
        let dom_window = web_sys::window().unwrap();
        let dom_document = dom_window.document().unwrap();
        let dom_canvas = dom_document.get_element_by_id(WEBAPP_CANVAS_ID).unwrap();
        let canvas = dom_canvas.dyn_into::<web_sys::HtmlCanvasElement>().ok();

        let win = WindowBuilder::default()
            .with_canvas(canvas)
            .build(&event_loop);
       let window =             match win {
                Ok(ref b) => { b.inner_size().width;
                
                   log!("{} {}",b.inner_size().width, b.inner_size().height);
                   win
                }
                _ => { win }
            };
            

//let mut game_engine = GameEngine::new(window.unwrap());

      // {

          

        log!("aiiight!");
             let window_arc = std::sync::Arc::new(window.unwrap());
    let mut game_engine = GameEngine::<'a>::new(window_arc).await;
    game_engine.initialize_world();

	let _ = event_loop.run(move |event, control_flow| {
           game_engine = GameEngine::<'a>::new(window_arc).await;
        match event {
         
           /* Event::AboutToWait => {
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
                    _ => {log!("Window dim is {} {}", window.inner_size().width, window.inner_size().height); }
                }
            }*/
            _ => {log!("Window dim is {} {}", window.inner_size().width, window.inner_size().height);}//game_engine.tick_frame();}
        }
    });
}
