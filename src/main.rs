use clap::Parser;
use colored::Colorize;
use reqwest::{multipart, Client};
use serde::Deserialize;
use std::env;
use std::error::Error;
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::sleep;

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    id: Option<String>,
    // #[serde(default)]
    // success: bool,
    skin: Option<SkinData>,
}

#[derive(Debug, Deserialize)]
struct SkinData {
    texture: TextureData,
}

#[derive(Debug, Deserialize)]
struct TextureData {
    data: TextureValue,
}

#[derive(Debug, Deserialize)]
struct TextureValue {
    value: String,
    signature: String,
}

// #[derive(Debug, Serialize)]
// struct GenerateRequest {
//     visibility: String,
//     variant: String,
//     name: Option<String>,
// }

const API_BASE: &str = "https://api.mineskin.org/v2";
const USER_AGENT: &str = "MineSkinUploader/1.0";

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the skin PNG file to upload
    #[arg(value_name = "PATH")]
    skin_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Get API key from environment variable
    let api_key = env::var("MINESKIN_API_KEY").expect("MINESKIN_API_KEY must be set");

    // Parse command line arguments
    let Args { skin_path } = Args::parse();

    if !skin_path.exists() {
        eprintln!(
            "{}",
            format!("File not found: {}", skin_path.display()).red()
        );
        std::process::exit(1);
    }

    // Create HTTP client
    let client = Client::new();

    // Read file
    let file_part = multipart::Part::file(skin_path)
        .await?
        .mime_str("image/png")?;

    // Create form
    let form = multipart::Form::new()
        .part("file", file_part)
        .text("visibility", "public")
        .text("variant", "classic");

    // Submit initial request
    println!("{}", "Uploading skin...".blue());
    let response = client
        .post(format!("{API_BASE}/generate"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("User-Agent", USER_AGENT)
        .multipart(form)
        .send()
        .await?;

    let generate_response: GenerateResponse = response.json().await?;

    // If we get a direct success response
    if let Some(skin) = generate_response.skin {
        println!("{}", "Upload successful!".green());
        println!("{} {}", "Texture:".yellow(), skin.texture.data.value);
        println!("{} {}", "Signature:".yellow(), skin.texture.data.signature);
        return Ok(());
    }

    // If we get a job ID, poll for completion
    if let Some(job_id) = generate_response.id {
        println!("{} {}", "Job queued with ID:".blue(), job_id);

        // Poll every second until completion
        loop {
            sleep(Duration::from_secs(1)).await;

            let status_response = client
                .get(format!("{API_BASE}/queue/{job_id}"))
                .header("Authorization", format!("Bearer {api_key}"))
                .header("User-Agent", USER_AGENT)
                .send()
                .await?;

            let status: GenerateResponse = status_response.json().await?;

            if let Some(skin) = status.skin {
                println!("{}", "Upload completed successfully!".green());
                println!("{} {}", "Texture:".yellow(), skin.texture.data.value);
                println!("{} {}", "Signature:".yellow(), skin.texture.data.signature);
                break;
            }

            println!("{}", "Job still processing...".blue());
        }
    }

    Ok(())
}
