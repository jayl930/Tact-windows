use crate::settings;
use std::path::Path;

/// Add a folder to recent folders list (max 5).
pub fn add_recent_folder(folder: &str) {
    let mut settings = settings::load_settings();

    // Remove if already present, then add to front
    settings.recent_folders.retain(|f| f != folder);
    settings.recent_folders.insert(0, folder.to_string());
    settings.recent_folders.truncate(5);

    let _ = settings::save_settings_to_disk(&settings);
}

/// Add a folder to favorites.
pub fn add_favorite_folder(folder: &str) -> Result<(), String> {
    let path = Path::new(folder);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    let mut settings = settings::load_settings();
    if !settings.favorite_folders.contains(&folder.to_string()) {
        settings.favorite_folders.push(folder.to_string());
        settings::save_settings_to_disk(&settings)?;
    }
    Ok(())
}

/// Remove a folder from favorites.
pub fn remove_favorite_folder(folder: &str) -> Result<(), String> {
    let mut settings = settings::load_settings();
    settings.favorite_folders.retain(|f| f != folder);
    settings::save_settings_to_disk(&settings)
}

/// Set the output folder.
pub fn set_output_folder(folder: &str) -> Result<(), String> {
    let path = Path::new(folder);
    if !path.exists() {
        return Err("Folder does not exist".to_string());
    }

    let mut settings = settings::load_settings();
    settings.output_folder = Some(folder.to_string());
    settings::save_settings_to_disk(&settings)?;
    add_recent_folder(folder);
    Ok(())
}
