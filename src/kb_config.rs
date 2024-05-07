use crate::KbPostProcessMode;

#[derive(Clone)]
pub struct KbConfig {
	// From file
	pub enemy_spawn_delay: f32,
	pub enemy_move_speed: f32,
	pub max_render_instances: u32,
	pub window_width: u32,
	pub window_height: u32,
	pub fov: f32,
	pub foreground_fov: f32,
	pub graphics_backend: wgpu::Backends,
	pub graphics_power_pref: wgpu::PowerPreference,
	pub _vsync: bool,

	// Dynamic
	pub start_time: instant::Instant,
	pub delta_time: f32,
	pub last_frame_time: f32,
	pub postprocess_mode: KbPostProcessMode,
}

impl KbConfig {
    pub fn new(config_file_text: &str) -> Self {

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
			fov: 75.0,
			foreground_fov: 50.0,
            graphics_backend,
            graphics_power_pref,
			_vsync,

			start_time: instant::Instant::now(),
			delta_time: 0.0,
			last_frame_time: 0.0,
			postprocess_mode: KbPostProcessMode::Passthrough,
        }
    }

	pub fn update_frame_times(&mut self) {
		let elapsed_time = self.start_time.elapsed().as_secs_f32();
		self.delta_time = elapsed_time - self.last_frame_time;
		self.last_frame_time = elapsed_time;
	}
}
