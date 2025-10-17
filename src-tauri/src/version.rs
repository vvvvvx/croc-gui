use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct GiteeRelease {
    tag_name: String,
    body: String,
}
#[tauri::command]
pub async fn check_update() -> Result<GiteeRelease, String> {
    use reqwest;
    let url = "https://gitee.com/api/v5/repos/vvvvvx/croc-gui/releases/latest";
    match reqwest::Client::new().get(url).send().await {
        Ok(response) => {
            if let Ok(release) = response.json::<GiteeRelease>().await {
                println!("Latest release: {release:?}");
                Ok(release)
            } else {
                Err("Failed to parse release info from Gitee".to_string())
            }
        }
        Err(e) => {
            eprintln!("Error getting latest release info from Gitee: {e}");
            Err("Failed to get latest release info from Gitee".to_string())
        }
    }
}
