use std::{fs, collections::HashMap};
use serde::Deserialize;
use lazy_static::lazy_static;
use regex::Regex;
use dialoguer::{
    Input,
    theme::ColorfulTheme
};
use clap::{Parser};
use std::path::PathBuf;

#[derive(Parser)]
struct Cli {
    /// Sets a custom folder path
    #[clap(short, long)]
    output: Option<PathBuf>,
}


#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Thread {
    posts: Vec<Post>
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Post {
    filename: Option<String>,
    tim: Option<u64>,
    ext: Option<String>,
}

async fn get_thread(client: &reqwest::Client, board: &String, op_id: &String) -> Result<Thread, reqwest::Error> {
    let response = client
        .get(format!("https://a.4cdn.org/{board}/thread/{op_id}.json"))
        .send()
        .await?;

    let users: Thread = response
        .json()
        .await?;

    Ok(users)
}

lazy_static! {
    static ref RE: Regex = Regex::new(r"^https://boards.4chan(nel)?.org/([0-9A-Za-z]+)/thread/([0-9]+)").unwrap();
}

fn get_url() -> Result<(String, String), Box<dyn std::error::Error>> {
    let url : String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Paste thread link:")
        .validate_with(|input: &String| -> Result<(), &str> {
            if RE.is_match(input) {
                Ok(())
            } else {
                Err("Not a valid thread URL.")
            }
        })
        .interact_text()?;
    
    let caps = RE.captures(&url).unwrap();
    let board = caps
        .get(2)
        .unwrap()
        .as_str()
        .to_string();
        
    let op_id = caps
        .get(3)
        .unwrap()
        .as_str()
        .to_string();
    
    Ok((board, op_id))
}

async fn download_img(client: &reqwest::Client, fpath: &PathBuf, img_api_url: &String) -> Result<(), Box<dyn std::error::Error>> {
    // If file exists, we already downloaded it 
    if std::fs::metadata(fpath).is_ok() {
        return Ok(());
    }

    let mut out = std::fs::File::create(fpath)?;

    let img_bytes = client
    .get(img_api_url)
    .send()
    .await?
    .bytes()
    .await?;
    
    std::io::copy(&mut img_bytes.as_ref(), &mut out)?;

    Ok(())
}

fn get_folder_path(output: &Option<PathBuf>) -> Option<PathBuf> {
    match output {
        Some(input_path) => {
            if !std::fs::metadata(&input_path).unwrap().is_dir() {
                None
            } else {
                Some(input_path)
            }
        }, 
        None => Some(std::env::current_dir().unwrap())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let folder_path = get_folder_path(&cli.output);
    if folder_path.is_none() {
        return Err(format!("Output filepath {:?} does not exist.", cli.output));
    }

    let (board, op_id) = get_url()?;

    folder_path.push(board);
    folder_path.push(op_id);

    fs::create_dir_all(folder_path)?;

    let client = reqwest::Client::new();

    let thread_obj = get_thread(&client, &board, &op_id).await?;

    for post in thread_obj.posts {
        let mut filenames: HashMap<&String, u32> = HashMap::new();
        match post {
            Post {
                filename : Some(filename),
                tim : Some(tim),
                ext : Some(ext),
            } => {
                let count = filenames.entry(&filename)
                    .and_modify(|count| {*count += 1})
                    .or_insert(0);
                let filename_num = match count {
                    0 => "".to_string(),
                    _ => count.to_string()
                };
                let fpath = folder_path.join(format!("{board}/{op_id}/{filename}{filename_num}{ext}"));
                let img_api_url = format!("https://i.4cdn.org/{board}/{tim}{ext}");
                download_img(&client, &fpath, &img_api_url).await?
            },
            _ => ()
        }
    }

    Ok(())
}