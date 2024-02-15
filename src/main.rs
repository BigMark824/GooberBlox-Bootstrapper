use anyhow::anyhow;
use chrono::Local;
use colored::*;
use core::cmp::min;
use crossterm::execute;
use std::env::{self, args};
use std::io::{stdout, Cursor};
use std::path::Path;
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use tokio::fs::copy;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{self, Client};
use sha2::{Digest, Sha256};
use url::Url;
use winreg::enums::*;
use winreg::RegKey;

const BASE_URL: &str = "goober.biz";
const APPDATA_SUB: &str = "GooberBlox";

fn print_advanced(mesg: &str, type_of_msg: i32) {
    match type_of_msg{
        0 /* info */ => println!("{}", format!("[{}] [{}] {}", Local::now(), "main".green(), mesg)),
        1 /* error */ => println!("{}", format!("[{}] [{}] {}", Local::now(), "error".red(), mesg)),
        _ => unimplemented!()
    };
}

pub fn clear_terminal_screen() {
    print!("{}[2J", 27 as char); /* Use ansi */
}

pub async fn http_get(client: &Client, url: &str) -> anyhow::Result<String> {
    let response = client.get(url).send().await;
    if let Err(err) = response {
        println!("Unable to visit {}", url);
        return Err(err.into());
    }

    Ok(response?.text().await?)
}

pub async fn download_file<T: AsRef<str>>(client: &Client, url: T) -> anyhow::Result<Vec<u8>> {
    let response = client.get(url.as_ref()).send().await?;
    let content_length = response.content_length().unwrap_or(0) as usize;

    // Create a progress bar
    let progress_bar = ProgressBar::new(content_length as u64);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Progress bar failed to be made.")
            .progress_chars("##-"),
    );

    let mut file_content = Vec::new();
    let mut byte_stream = response.bytes_stream();
    let mut downloaded = 0;

    while let Some(item) = byte_stream.next().await {
        let chunk = item?;
        //write_with_progress(&mut file_content, chunk.to_vec(), &progress_bar);
        file_content.write_all(&chunk).await?;
        downloaded = min(downloaded + chunk.len(), content_length);
        progress_bar.set_position(downloaded as u64);
    }

    progress_bar.finish_and_clear();
    Ok(file_content)
}

pub async fn calculate_file_sha256(file_path: &Path) -> anyhow::Result<String> {
    let mut sha256 = Sha256::new();
    let mut file = File::open(file_path).await.expect("Hard Error");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer).await?;
    sha256.update(&buffer);

    Ok(format!("{:x}", sha256.finalize()))
}

/*
    Whatever you do dont make this return a result;
    for some reason rust freaks the f**k out
*/
#[tokio::main]
async fn main() {
    clear_terminal_screen();
    execute!(stdout(), crossterm::terminal::SetSize(85, 27)).expect("Failed to set TermSize");

    println!("{}", "Welcome to...".bold().green());
    tokio::time::sleep(Duration::from_secs(2)).await;

    println!(
        "{}{}",
        " ██████╗  ██████╗  ██████╗ ██████╗ ███████╗██████╗ ".green(),
        "██████╗ ██╗      ██████╗ ██╗  ██╗".blue()
    );
    println!(
        "{}{}",
        "██╔════╝ ██╔═══██╗██╔═══██╗██╔══██╗██╔════╝██╔══██╗".green(),
        "██╔══██╗██║     ██╔═══██╗╚██╗██╔╝".blue()
    );
    println!(
        "{}{}",
        "██║  ███╗██║   ██║██║   ██║██████╔╝█████╗  ██████╔╝".green(),
        "██████╔╝██║     ██║   ██║ ╚███╔╝".blue()
    );
    println!(
        "{}{}",
        "██║   ██║██║   ██║██║   ██║██╔══██╗██╔══╝  ██╔══██╗".green(),
        "██╔══██╗██║     ██║   ██║ ██╔██╗ ".blue()
    );
    println!(
        "{}{}",
        "╚██████╔╝╚██████╔╝╚██████╔╝██████╔╝███████╗██║  ██║".green(),
        "██████╔╝███████╗╚██████╔╝██╔╝ ██".blue()
    );
    println!(
        "{}{}",
        "╚═════╝  ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝".green(),
        "╚═════╝ ╚══════╝ ╚═════╝ ╚═╝  ╚═╝".blue()
    );
    println!("Did you know that the current date is {}?\n", Local::now());

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        //play mode
        let mut iter = args.into_iter();
        while let Some(arg) = iter.next() {
            if arg.starts_with("goober-player:///?") {
                //hash_check(http_client.clone()).await;
                // ARGS FOUND!
                let Some(uri) = parse_launch_arguments() else {
                    eprintln!("Invalid Launch arguments");
                    return;
                };

                let playerbeta_path = dirs::data_local_dir()
                    .expect("Err")
                    .join("GooberBlox")
                    .join("Roblox")
                    .join("2016")
                    .join("GooberPlayerBeta.exe");

                let _playerbeta = Command::new(playerbeta_path)
                    .args([r"--authenticationUrl",r"http://goober.biz/login/negotiate.ashx",r"--authenticationTicket",&uri.token,r"--joinScriptUrl",&format!(r"http://www.goober.biz/game/newcl/join.ashx?placeid={}&auth={}&game={}",uri.place,uri.token,uri.game)])
                    .arg(r"--play")
                    .spawn()
                    .expect("Failed to start playerbeta!");
                print_advanced("Launched client!", 0);
            }
        }
    } else {
        install().await.expect("Failed to install");
    };
    print_advanced("Tasks done!", 0);
    sleep(Duration::new(3, 0));
}

struct LaunchArguments {
    pub place: String,
    pub token: String,
    pub game: String,
}

fn parse_launch_arguments() -> Option<LaunchArguments> {
    let argument_url = args()
        .filter_map(|v| Url::parse(&v).ok())
        .collect::<Vec<Url>>()
        .pop()?;

    let mut argument = argument_url.query_pairs();

    let mut place = None;
    let mut token = None;
    let mut game = None;

    while let Some((name, value)) = argument.next() {
        let name = name.to_string();
        let value = value.to_string();

        match name.as_str() {
            "placeid" => place = Some(value),
            "auth" => token = Some(value),
            "game" => game = Some(value),
            _ => continue,
        }
    }

    Some(LaunchArguments {
        place: place?,
        token: token?,
        game: game?,
    })
}

async fn install_further() -> anyhow::Result<()> {
    let setup_url: &str = &format!("setup.{}", BASE_URL);

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("Hard Error");

    let exec_pathbuf = dirs::data_local_dir()
        .expect("Hard Error")
        .join(APPDATA_SUB);

    let install_folder = exec_pathbuf.join("Roblox").join("2016");

    if !install_folder.exists() || !exec_pathbuf.join("GooberLauncher.exe").exists() {
        create_dir_all(exec_pathbuf.join("Roblox")).await?;
        let file_content = download_file(
            &http_client,
            format!("http://{}/GooberClient.zip", &setup_url),
        )
        .await?;

        if let Err(err) = zip_extract::extract(
            Cursor::new(file_content),
            &exec_pathbuf.join("Roblox").join("2016"),
            true,
        ) {
            std::fs::remove_dir_all(&exec_pathbuf.join("Roblox").join("2016"))?;
            eprintln!("Error during extraction {:?}", err);
            return Err(anyhow!("Error during extraction {:?}", err));
        }

        print_advanced("Client installed", 0)
    }

    Ok(())
}

async fn install() -> anyhow::Result<()> {
    let bootstrapper_filename: &str = "GooberLauncher.exe";
    let uri_scheme: &str = "goober-player";

    let hkcu_classes_key: RegKey =
        RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags("Software\\Classes", KEY_WRITE)?;
    let exec_pathbuf = dirs::data_local_dir()
        .expect("Hard Error")
        .join(APPDATA_SUB);
    if !exec_pathbuf.exists() {
        let _ = create_dir_all(&exec_pathbuf);
    };
    if exec_pathbuf.join("Roblox").join("2016").exists()
        || exec_pathbuf.join(&bootstrapper_filename).exists()
    {
        return Ok(());
    }

    let executable_path = env::current_exe()?;

    if let Err(err) = copy(executable_path, exec_pathbuf.join(bootstrapper_filename)).await {
        eprintln!("Error copying executable: {:?}", err);
        panic!("Unable to install, make a ticket for help.");
    }

    print_advanced("Starting installation..", 0);
    install_further().await?;

    let exec_keypath: String = format!(
        "\"{}\" \"%1\"",
        &exec_pathbuf.join(&bootstrapper_filename).display()
    );

    let scheme_key_result: Result<(RegKey, RegDisposition), _> =
        hkcu_classes_key.create_subkey_with_flags(uri_scheme, KEY_WRITE);
    match scheme_key_result {
        Ok((scheme_key, _)) => {
            scheme_key.set_value("", &format!("URL {} Protocol", uri_scheme))?;
            scheme_key.set_value("URL Protocol", &"")?;

            let (command_key, _) =
                scheme_key.create_subkey_with_flags("shell\\open\\command", KEY_WRITE)?;
            command_key.set_value("", &exec_keypath)?;

            let _icon_key = scheme_key.create_subkey_with_flags("DefaultIcon", KEY_WRITE)?;
        }
        Err(err) => {
            eprintln!(
                "An error has occurred, please report it to Gooberblox via tickets: {}",
                err
            )
        }
    }
    Ok(())
}
