Hi and thanks for checking out my project!  This project demonstrates using Rust and wgpu 0.19.3 (latest at the time of this project) to build a simple sprite renderer.  

All sprites are drawn with a single draw_indexed call using instancing.

========== Building and Debugging ==========
Be sure to set your working directory to the root of the kbEngine3 folder when debugging in Visual Studio, running from RenderDoc, etc.
I'm using the following docs and tutorials:

========== Config file ==========
There is a config file at GameAssets\game_config.txt that lets you control several parameters.  An example config is below:

{
    "enemy_spawn_timer": 0.5,
    "enemy_speed": 0.5,
    "max_instances": 10000,

    "_comment": "Valid values 'graphics_back_end' are default, vulkan, and dx12",
    "graphics_back_end": "vulkan",

    "_comment": "Valid values for 'graphics_power_pref' are default, low, and high"
    "graphics_power_pref": "default"
}


========== Resources ==========
https://doc.rust-lang.org/book/ch01-00-getting-started.html

https://sotrh.github.io/learn-wgpu/#what-is-wgpu

https://registry.khronos.org/vulkan/specs/1.3/html/vkspec.html

Benny Wilson
bennywilson@benny-wilson.com