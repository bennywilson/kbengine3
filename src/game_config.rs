#[derive(Clone)]
pub struct GameConfig {
    pub enemy_spawn_delay: f32,
    pub enemy_move_speed: f32,
    pub max_render_instances: u32,
	pub window_width: u32,
	pub window_height: u32,
    pub graphics_backend: wgpu::Backends,
    pub graphics_power_pref: wgpu::PowerPreference,
}

impl GameConfig {
    pub fn new() -> Self {
		let config_file_text = include_str!("../game_assets/game_config.txt");

        let json_file = json::parse(&config_file_text).unwrap();
		
		let json_val = json_file["enemy_spawn_delay"].as_f32();
		let mut enemy_spawn_delay = 0.01;
		match json_val {
			Some(val) => { enemy_spawn_delay = val; }
			None => ()
		}

		let json_val = json_file["enemy_move_speed"].as_f32();
		let mut enemy_move_speed = 0.01;
		match json_val {
			Some(val) => { enemy_move_speed = val;}
			None => ()
		}

		let mut max_render_instances = 10000;
		let json_val = json_file["max_instances"].as_u32();
		match json_val {
			Some(val) => { max_render_instances = val; }
			None => ()
		}

		let mut window_width = 1280;
		let json_val = json_file["window_width"].as_u32();
		match json_val {
			Some(val) => { window_width = val; }
			None => ()
		}

		let mut window_height = 720;
		let json_val = json_file["window_height"].as_u32();
		match json_val {
			Some(val) => { window_height = val; }
			None => ()
		}

		let mut graphics_backend = wgpu::Backends::default();
		let json_val = json_file["graphics_back_end"].as_str();
		match json_val {
			Some(val) => {
	            graphics_backend = match val {
                    "dx12" => { wgpu::Backends::DX12 }
                    "webgpu" => { wgpu::Backends::BROWSER_WEBGPU }
                    "vulkan" => { wgpu::Backends::VULKAN }
                    _ => { wgpu::Backends::all() }
                };
			}
			None => ()
		}

		let mut graphics_power_pref = wgpu::PowerPreference::default();
		let json_val = json_file["graphics_power_pref"].as_str();
		match json_val {
			Some(val) => {
	            graphics_power_pref = match val {
                    "high" => { wgpu::PowerPreference::HighPerformance }
                    "low" => { wgpu::PowerPreference::LowPower }
                    _ => { wgpu::PowerPreference::None }
                };
			}
			None => ()

		}

        GameConfig {
            enemy_spawn_delay,
            enemy_move_speed,
            max_render_instances,
			window_width,
			window_height,
            graphics_backend,
            graphics_power_pref,
        }
    }
}
