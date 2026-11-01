type Imported = (
    gltf::Document,
    Vec<gltf::buffer::Data>,
    Vec<gltf::image::Data>,
);

/// APK assets are not filesystem paths. Embed only the five runtime GLBs,
/// excluding the original FBX files, Godot metadata and unused models.
pub fn import(path: &str) -> Result<Imported, String> {
    #[cfg(target_os = "android")]
    {
        let bytes: &[u8] = match path {
            "assets/generated/x_bot.glb" => include_bytes!("../assets/generated/x_bot.glb"),
            "assets/generated/y_bot.glb" => include_bytes!("../assets/generated/y_bot.glb"),
            "assets/generated/running.glb" => include_bytes!("../assets/generated/running.glb"),
            "assets/generated/idle.glb" => include_bytes!("../assets/generated/idle.glb"),
            "assets/models/ball.glb" => include_bytes!("../assets/models/ball.glb"),
            _ => return Err(format!("Model nie je pribaleny v APK: {path}")),
        };
        gltf::import_slice(bytes).map_err(|e| e.to_string())
    }
    #[cfg(not(target_os = "android"))]
    {
        gltf::import(path).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn runtime_glbs_are_self_contained_for_apk_loading() {
        let assets: [&[u8]; 5] = [
            include_bytes!("../assets/generated/x_bot.glb"),
            include_bytes!("../assets/generated/y_bot.glb"),
            include_bytes!("../assets/generated/running.glb"),
            include_bytes!("../assets/generated/idle.glb"),
            include_bytes!("../assets/models/ball.glb"),
        ];
        for bytes in assets {
            let (_, buffers, _) = gltf::import_slice(bytes).unwrap();
            assert!(!buffers.is_empty());
        }
    }
}
