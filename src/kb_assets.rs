use std::{collections::HashMap, path::Path};
use wgpu::ShaderModule;

use crate::log;
use crate::kb_resource::*;

#[cfg(target_arch = "wasm32")]
fn format_url(file_name: &str) -> reqwest::Url {
    let window = web_sys::window().unwrap();
    let location = window.location();
    let  origin = location.origin().unwrap();
    let base = reqwest::Url::parse(&format!("{}/", origin,)).unwrap();
    base.join(file_name).unwrap()
}

pub async fn load_binary(file_name: &str) -> anyhow::Result<Vec<u8>> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
			let path = Path::new(file_name);
			let file_name = format!("/rust_assets/{}", path.file_name().unwrap().to_str().unwrap());

            let url = format_url(&file_name);
            let data = reqwest::get(url)
                .await?
                .bytes()
                .await?
                .to_vec();
        } else {
            let data = std::fs::read(file_name)?;
        }
    }
    Ok(data)
}

pub async fn load_string(file_name: &str) -> anyhow::Result<String> {
    cfg_if::cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
			let path = Path::new(file_name);
			let file_name = format!("/rust_assets/{}", path.file_name().unwrap().to_str().unwrap());

            let url = format_url(&file_name);
            let txt = reqwest::get(url)
                .await?
                .text()
                .await?;
        } else {
            let txt = std::fs::read_to_string(file_name)?;
        }
    }

    Ok(txt)
}

macro_rules! make_kb_handle {
	($asset_type:ident, $handle_type:ident, $mapping_type:ident) => {
		#[derive(Clone, Hash)]
		pub struct $handle_type { index: u32 }
		impl $handle_type {
			fn is_valid(&self) -> bool {
				self.index != u32::MAX 
			}
			pub fn make_invalid() -> $handle_type {
				$handle_type { index: u32::MAX }
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
type KbByteVec = Vec<u8>;
make_kb_handle!(KbByteVec, KbByteFileHandle, KbByteMappings);
make_kb_handle!(KbModel, KbModelFileHandle, KbModelMappings);

#[allow(dead_code)]
pub struct KbAssetManager {
	texture_mappings: KbTextureAssetMappings,
	shader_mappings: KbShaderAssetMappings,
	model_mappings: KbModelMappings,

	file_to_string_buffer: HashMap<String, String>,
	file_to_byte_buffer: HashMap<String, KbByteVec>,
}

impl KbAssetManager {
	pub fn new() -> Self {
		let mut file_to_byte_buffer =  HashMap::<String, KbByteVec>:: new();
		file_to_byte_buffer.insert("smoke_t.png".to_string(), include_bytes!("./../Examples/3D/game_assets/smoke_t.png").to_vec());
		file_to_byte_buffer.insert("ember_t.png".to_string(), include_bytes!("./../Examples/3D/game_assets/ember_t.png").to_vec());
		file_to_byte_buffer.insert("SpriteSheet.png".to_string(), include_bytes!("../engine_assets/textures/SpriteSheet.png").to_vec());
		file_to_byte_buffer.insert("PostProcessFilter.png".to_string(), include_bytes!("../engine_assets/textures/PostProcessFilter.png").to_vec());
		file_to_byte_buffer.insert("pinky.glb".to_string(), include_bytes!("./../Examples/3D/game_assets/pinky.glb").to_vec());
		file_to_byte_buffer.insert("barrel.glb".to_string(), include_bytes!("./../Examples/3D/game_assets/barrel.glb").to_vec());
		file_to_byte_buffer.insert("shotgun.glb".to_string(), include_bytes!("./../Examples/3D/game_assets/shotgun.glb").to_vec());
		file_to_byte_buffer.insert("floor.glb".to_string(), include_bytes!("./../Examples/3D/game_assets/floor.glb").to_vec());

		let mut file_to_string_buffer =  HashMap::<String, String>:: new();
		file_to_string_buffer.insert("BasicSprite.wgsl".to_string(), include_str!("../engine_assets/shaders/BasicSprite.wgsl").to_string());
		file_to_string_buffer.insert("Model.wgsl".to_string(), include_str!("../engine_assets/shaders/Model.wgsl").to_string());
		file_to_string_buffer.insert("CloudSprite.wgsl".to_string(), include_str!("../engine_assets/shaders/CloudSprite.wgsl").to_string());
		file_to_string_buffer.insert("particle.wgsl".to_string(), include_str!("../engine_assets/shaders/Particle.wgsl").to_string());
		file_to_string_buffer.insert("postprocess_uber.wgsl".to_string(), include_str!("../engine_assets/shaders/postprocess_uber.wgsl").to_string());

		KbAssetManager {
			texture_mappings: KbTextureAssetMappings::new(),
			shader_mappings: KbShaderAssetMappings::new(),
			model_mappings: KbModelMappings::new(),

			file_to_string_buffer,
			file_to_byte_buffer
		}
	}

	pub async fn load_texture(&mut self, file_path: &str, device_resource: &KbDeviceResources<'_>) -> KbTextureHandle {
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
			#[cfg(not(target_arch = "wasm32"))]
			{
				let final_file_path = {
					if file_path.chars().nth(1).unwrap() == ':' {
						file_path.to_string()
					} else if file_path.contains("engine_assets") {
						if Path::new("/./engine_assets").exists() {
							format!("{cwd}/./{file_path}")
						} else {
							format!("{cwd}/../../{file_path}")
						}
					} else if file_path.contains("game_assets") {
						format!("{cwd}/./{file_path}")
					} else {
						file_path.to_string()
					}
				};
				KbTexture::from_file(&final_file_path, device_resource).await.unwrap()

				/*let current_exe = std::env::current_exe();
				let exe_path = current_exe.as_ref().unwrap().parent().unwrap();
				let final_file_path = format!("{}", exe_path.to_string_lossy());
				let final_file_path = format!("{final_file_path}/{file_path}");
				KbTexture::from_file(&final_file_path, device_resource).await.unwrap()*/
			}
			#[cfg(target_arch = "wasm32")]
			{
				let path = Path::new(&file_path);
				let file_name = path.file_name().unwrap().to_str().unwrap();
				log!("Path returned {} ", file_name);

				let byte_buffer = self.file_to_byte_buffer.get(file_name).unwrap();
				KbTexture::from_bytes(&device_resource.device, &device_resource.queue, byte_buffer, file_name).unwrap()
			}
        };

		mappings.handles_to_assets.insert(new_handle.clone(), new_texture);
		mappings.names_to_handles.insert(file_path.to_string(), new_handle.clone());

		new_handle.clone()
	}

	pub fn get_texture(&self, texture_handle: &KbTextureHandle) -> &KbTexture {
		&self.texture_mappings.handles_to_assets[texture_handle]
	}

	pub async fn load_shader(&mut self, file_path: &str, device_resources: &KbDeviceResources<'_>) -> KbShaderHandle {
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

		////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////
		let shader_str = {
			#[cfg(not(target_arch = "wasm32"))]
			{
				let mut cwd: String = "".to_string();
				match std::env::current_dir() {
					Ok(dir) => { cwd = format!("{}", dir.display()); }
					_ => { /* todo use default texture*/ }
				};
				let final_file_path = {
					if file_path.chars().nth(1).unwrap() == ':' {
						file_path.to_string()
					} else if file_path.contains("engine_assets") {
						if Path::new("/./engine_assets").exists() {
							format!("{cwd}/./{file_path}")
						} else {
							format!("{cwd}/../../{file_path}")
						}
					} else {
						file_path.to_string()
					}
				};
				load_string(&final_file_path).await.unwrap()
			}
			#[cfg(target_arch = "wasm32")]
			{
				let path = Path::new(&file_path);
				let file_name = path.file_name().unwrap().to_str().unwrap();
				log!("Path returned {} ", file_name);
				self.file_to_string_buffer.get(file_name).unwrap()
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

	pub async fn load_model(&mut self, file_path: &str, mut device_resource: &mut KbDeviceResources<'_>) -> KbModelFileHandle {
		let mut cwd: String = "".to_string();
		match std::env::current_dir() {
            Ok(dir) => { cwd = format!("{}", dir.display()); }
            _ => { /* todo use default texture*/ }
        };
		
		let new_model = {
			#[cfg(not(target_arch = "wasm32"))]
			{
				let final_file_path = {
					if file_path.chars().nth(1).unwrap() == ':' {
						file_path.to_string()
					} else if file_path.contains("engine_assets") {
						if Path::new("/./engine_assets").exists() {
							format!("{cwd}/./{file_path}")
						} else {
							format!("{cwd}/../../{file_path}")
						}
					} else if file_path.contains("game_assets") {
						format!("{cwd}/./{file_path}")
					} else {
						file_path.to_string()
					}
				};
				let bytes = load_binary(&final_file_path).await.unwrap();
				KbModel::from_bytes(&bytes, &mut device_resource, self).await
			}
			#[cfg(target_arch = "wasm32")]
			{
				let path = Path::new(&file_path);
				let file_name = path.file_name().unwrap().to_str().unwrap();
				log!("Path returned {} {}", file_name, self.file_to_byte_buffer.len());
				let byte_buffer = self.file_to_byte_buffer.get(file_name).unwrap().clone();	// cloning here.
				KbModel::from_bytes(&byte_buffer, &mut device_resource, self).await
			}
        };
				log!("Model loaded");

		let mappings = &mut self.model_mappings;
		match mappings.names_to_handles.get(file_path) {
			Some(handle) => {
				return handle.clone()
			}
			_ => {}
		}

		log!("KbAssetManager loading model {file_path}");
		let new_handle = {
			if mappings.next_handle.is_valid() == false { mappings.next_handle.index = 0; }
			let new_handle = mappings.next_handle.clone();
			mappings.next_handle.index = mappings.next_handle.index + 1;
			new_handle
		};
		mappings.handles_to_assets.insert(new_handle.clone(), new_model);
		mappings.names_to_handles.insert(file_path.to_string(), new_handle.clone());

		new_handle.clone()
	}

	pub fn get_model(&mut self, model_handle: &KbModelFileHandle) -> Option<&mut KbModel> {
		self.model_mappings.handles_to_assets.get_mut(model_handle)
	}

	pub fn get_model_mappigns(&mut self) -> &mut HashMap<KbModelFileHandle, KbModel>  {
		&mut self.model_mappings.handles_to_assets
	}
}
