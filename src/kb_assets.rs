use load_file::*;
use std::collections::HashMap;
use wgpu::ShaderModule;

use crate::log;
use crate::kb_resource::*;


macro_rules! make_kb_handle {
	($asset_type:ident, $handle_type:ident, $mapping_type:ident) => {
		#[derive(Clone, Hash)]
		pub struct $handle_type { index: u32 }
		impl $handle_type {
			fn is_valid(&self) -> bool {
				self.index != u32::MAX 
			}
		}
		impl PartialEq for $handle_type { fn eq(&self, other: &Self) -> bool { self.index == other.index } }
		impl Eq for $handle_type{}

		pub struct $mapping_type {
			names_to_handles: HashMap<String, $handle_type>,
			handles_to_assets: HashMap<$handle_type, $asset_type>,
			next_handle: $handle_type,
		}

		impl $mapping_type {
			pub fn new() -> Self {
				let names_to_handles = HashMap::<String, $handle_type>::new();
				let handles_to_assets = HashMap::<$handle_type, $asset_type>::new();
				let next_handle = $handle_type { index: u32::MAX };
				$mapping_type {
					names_to_handles,
					handles_to_assets,
					next_handle
				}
			}
		}
	}
}
make_kb_handle!(KbTexture, KbTextureHandle, KbTextureAssetMappings);
make_kb_handle!(ShaderModule, KbShaderHandle, KbShaderAssetMappings);

pub struct KbAssetManager {
	texture_mappings: KbTextureAssetMappings,
	shader_mappings: KbShaderAssetMappings,
}

impl KbAssetManager {
	pub fn new() -> Self {
		KbAssetManager {
			texture_mappings: KbTextureAssetMappings::new(),
			shader_mappings: KbShaderAssetMappings::new(),
		}
	}

	pub fn load_texture(&mut self, file_path: &str, device_resource: &KbDeviceResources) -> KbTextureHandle {
		let mappings = &mut self.texture_mappings;
		match mappings.names_to_handles.get(file_path) {
			Some(handle) => {
				return handle.clone()
			}
			_ => {}
		}

		log!("KbAssetManager loading texture {file_path}");
		let new_handle = {
			if mappings.next_handle.is_valid() == false { mappings.next_handle.index = 0; }
			let new_handle = mappings.next_handle.clone();
			mappings.next_handle.index = mappings.next_handle.index + 1;
			new_handle
		};

		let mut cwd: String = "".to_string();
		match std::env::current_dir() {
            Ok(dir) => { cwd = format!("{}", dir.display()); }
            _ => { /* todo use default texture*/ }
        };

		let new_texture = {
			if file_path.chars().nth(1).unwrap() == ':' {
				KbTexture::from_file(file_path, device_resource).unwrap()
			} else if file_path.contains("engine_assets") {
				match std::path::Path::new("./engine_assets").exists() {
					true => { KbTexture::from_file(&format!("./{file_path}"), device_resource).unwrap() }
					false => { KbTexture::from_file(&format!("../{file_path}"), device_resource).unwrap() }
				}
			} else if file_path.contains("game_assets") {
				KbTexture::from_file(&format!("{cwd}/{file_path}"), device_resource).unwrap()
			} else {
				KbTexture::from_file(file_path, device_resource).unwrap()
			}
        };
		mappings.handles_to_assets.insert(new_handle.clone(), new_texture);
		mappings.names_to_handles.insert(file_path.to_string(), new_handle.clone());

		new_handle.clone()
	}

	pub fn get_texture(&self, texture_handle: &KbTextureHandle) -> &KbTexture {
		&self.texture_mappings.handles_to_assets[texture_handle]
	}

	pub fn load_shader(&mut self, file_path: &str, device_resources: &KbDeviceResources) -> KbShaderHandle {
		let mappings = &mut self.shader_mappings;
		match mappings.names_to_handles.get(file_path) {
			Some(handle) => {
				return handle.clone()
			}
			_ => {}
		}

		log!("KbAssetManager loading shader {file_path}");
		let new_handle = {
			if mappings.next_handle.is_valid() == false { mappings.next_handle.index = 0; }
			let new_handle = mappings.next_handle.clone();
			mappings.next_handle.index = mappings.next_handle.index + 1;
			new_handle
		};

		let shader_str = {
			if file_path.contains("engine_assets") {
				match std::path::Path::new("./engine_assets").exists() {
					true => { load_str!(&format!("./{file_path}")) }
					false => { load_str!(&format!("../{file_path}")) }
				}
			} else {
				load_str!(file_path)
			}
        };
        let new_shader = device_resources.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(file_path),
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });

		mappings.handles_to_assets.insert(new_handle.clone(), new_shader);
		mappings.names_to_handles.insert(file_path.to_string(), new_handle.clone());
		new_handle.clone()
	}

	pub fn get_shader(&self, shader_handle: &KbShaderHandle) -> &ShaderModule {
		&self.shader_mappings.handles_to_assets[shader_handle]
	}
}