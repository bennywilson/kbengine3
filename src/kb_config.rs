#[derive(Clone)]
pub struct KbConfig {
	pub enemy_spawn_delay: f32,
	pub enemy_move_speed: f32,
	pub max_render_instances: u32,
	pub window_width: u32,
	pub window_height: u32,
	pub graphics_backend: wgpu::Backends,
	pub graphics_power_pref: wgpu::PowerPreference,
	pub _vsync: bool,
}

impl KbConfig {
    pub fn new() -> Self {
		let config_file_text = include_str!("../game_assets/game_config.txt");

        let json_file = json::parse(&config_file_text).unwrap();
		
		let json_val = json_file["enemy_spawn_delay"].as_f32();
		let enemy_spawn_delay = match json_val {
			Some(val) => { val }
			None => { 1.0 }
		};

		let json_val = json_file["enemy_move_speed"].as_f32();
		let enemy_move_speed = match json_val {
			Some(val) => { val }
			None => { 0.01 }
		};

		let json_val = json_file["max_instances"].as_u32();
		let max_render_instances = match json_val {
			Some(val) => { val }
			None => { 10000 }
		};

		let json_val = json_file["window_width"].as_u32();
		let window_width = match json_val {
			Some(val) => { val }
			None => { 1280 }
		};

		let json_val = json_file["window_height"].as_u32();
		let		window_height = match json_val {
			Some(val) => { val }
			None => { 720 }
		};

		let json_val = json_file["graphics_back_end"].as_str();
		let graphics_backend = match json_val {
			Some(val) => {
				match val {
                    "dx12" => { wgpu::Backends::DX12 }
                    "webgpu" => { wgpu::Backends::BROWSER_WEBGPU }
                    "vulkan" => { wgpu::Backends::VULKAN }
                    _ => { wgpu::Backends::all() }
                }
			}
			None => { wgpu::Backends::all() }
		};

		let json_val = json_file["graphics_power_pref"].as_str();
		let graphics_power_pref = match json_val {
			Some(val) => {
	            match val {
                    "high" => { wgpu::PowerPreference::HighPerformance }
                    "low" => { wgpu::PowerPreference::LowPower }
                    _ => { wgpu::PowerPreference::None }
                }
			}
			None => { wgpu::PowerPreference::None }
		};

		let json_val = json_file["vsync"].as_bool();
		let _vsync = match json_val {
			Some(val) => { val }
			None => { true }
		};

        KbConfig {
            enemy_spawn_delay,
            enemy_move_speed,
            max_render_instances,
			window_width,
			window_height,
            graphics_backend,
            graphics_power_pref,
			_vsync,
        }
    }
}
