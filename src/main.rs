use chrono::Local;
use colored::*;
use crossterm::execute;
use std::env;
use std::fs::{create_dir_all, File};
use std::io::{stdout, Cursor, Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;

use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{self, Client};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use url::Url;
use winreg::enums::*;
use winreg::RegKey;

#[derive(Debug, Deserialize)]
struct HashResponse {
    #[serde(rename = "LauncherHash")]
    launcher_hash: String,
    #[serde(rename = "ClientHash")]
    client_hash: String,
}

fn print_advanced(mesg: &str, type_of_msg: i32) {
    match type_of_msg{
        0 /* info */ => println!("{}", format!("[{}] [{}] {}", Local::now(), "main".green(), mesg)),
        1 /* error */ => println!("{}", format!("[{}] [{}] {}", Local::now(), "error".red(), mesg)),
        _ => unimplemented!()
    };
}

pub fn clear_terminal_screen() {
    if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/c", "cls"])
            .spawn()
            .expect("cls command failed to start")
            .wait()
            .expect("failed to wait");
    } else {
        Command::new("clear")
            .spawn()
            .expect("clear command failed to start")
            .wait()
            .expect("failed to wait");
    };
}

pub async fn http_get(client: &Client, url: &str) -> Result<String, reqwest::Error> {
    let response = client.get(url).send().await;
    if let Err(err) = response {
        println!("Unable to visit {}", url);
        return Err(err);
    }
    Ok(response.unwrap().text().await.unwrap())
}

pub async fn download_file(client: &Client, url: &str) -> Result<Vec<u8>, reqwest::Error> {
    let response = client.get(url).send().await?;
    let content_length = response.content_length().unwrap_or(0);

    // Create a progress bar
    let progress_bar = ProgressBar::new(content_length);
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("Progress bar failed to be made.")
            .progress_chars("##-"),
    );

    let mut file_content = Vec::new();
    let mut byte_stream = response.bytes_stream();

    while let Some(item) = byte_stream.next().await {
        let chunk = item?;
        write_with_progress(&mut file_content, chunk.to_vec(), &progress_bar);
    }

    progress_bar.finish_and_clear();
    Ok(file_content)
}

fn write_with_progress(file_content: &mut Vec<u8>, chunk: Vec<u8>, progress_bar: &ProgressBar) {
    #[cfg(target_os = "windows")]
    {
        seek_write_with_progress(file_content, chunk, progress_bar);
    }
    #[cfg(not(target_os = "windows"))]
    {
        write_at_with_progress(file_content, chunk, progress_bar);
    }
}

fn write_file(file_content: &[u8], path: &PathBuf) -> File {
    let mut file = File::create(path).unwrap();
    file.write_all(file_content).unwrap();
    file
}

#[cfg(target_os = "windows")]
fn seek_write_with_progress(
    file_content: &mut Vec<u8>,
    chunk: Vec<u8>,
    progress_bar: &ProgressBar,
) {
    // Seek to the end of the file content and write the chunk
    file_content.extend_from_slice(&chunk);

    // Update progress bar
    progress_bar.inc(chunk.len() as u64);
}

#[cfg(not(target_os = "windows"))]
fn write_at_with_progress(file_content: &mut Vec<u8>, chunk: Vec<u8>, progress_bar: &ProgressBar) {
    // Write the chunk at the end of the file content
    file_content.extend_from_slice(&chunk);

    // Update progress bar
    progress_bar.inc(chunk.len() as u64);
}

pub async fn calculate_file_sha256(file_path: &Path) -> String {
    let mut sha256 = Sha256::new();
    let mut file = File::open(file_path).expect("Hard Error");
    let mut buffer = Vec::new();

    file.read_to_end(&mut buffer);
    sha256.update(&buffer);

    format!("{:x}", sha256.finalize())
}

#[tokio::main]
async fn main() {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::new(18446744073709551614, 0))
        .build()
        .expect("Hard Error");

    clear_terminal_screen();
    execute!(stdout(), crossterm::terminal::SetSize(85, 27)).unwrap();

    println!("{}", "Welcome to...".bold().green());
    let line1 = format!(
        "{}{}",
        " ██████╗  ██████╗  ██████╗ ██████╗ ███████╗██████╗ ".green(),
        "██████╗ ██╗      ██████╗ ██╗  ██╗".blue()
    );
    let line2 = format!(
        "{}{}",
        "██╔════╝ ██╔═══██╗██╔═══██╗██╔══██╗██╔════╝██╔══██╗".green(),
        "██╔══██╗██║     ██╔═══██╗╚██╗██╔╝".blue()
    );
    let line3 = format!(
        "{}{}",
        "██║  ███╗██║   ██║██║   ██║██████╔╝█████╗  ██████╔╝".green(),
        "██████╔╝██║     ██║   ██║ ╚███╔╝".blue()
    );
    let line4 = format!(
        "{}{}",
        "██║   ██║██║   ██║██║   ██║██╔══██╗██╔══╝  ██╔══██╗".green(),
        "██╔══██╗██║     ██║   ██║ ██╔██╗ ".blue()
    );
    let line5 = format!(
        "{}{}",
        "╚██████╔╝╚██████╔╝╚██████╔╝██████╔╝███████╗██║  ██║".green(),
        "██████╔╝███████╗╚██████╔╝██╔╝ ██".blue()
    );
    let line6 = format!(
        "{}{}",
        "╚═════╝  ╚═════╝  ╚═════╝ ╚═════╝ ╚══════╝╚═╝  ╚═╝".green(),
        "╚═════╝ ╚══════╝ ╚═════╝ ╚═╝  ╚═╝".blue()
    );
    tokio::time::sleep(Duration::from_secs(2)).await;
    println!(
        "{}\n{}\n{}\n{}\n{}\n{}",
        line1, line2, line3, line4, line5, line6
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
                let uri_str = arg;
                if let Ok(url) = Url::parse(&uri_str) {
                    let placeid = get_query_param(&url, "placeid");
                    let player_token = get_query_param(&url, "auth");
                    let game = get_query_param(&url, "game");
                    let mut playerbeta_path = dirs::data_local_dir().expect("Err");
                    playerbeta_path.push("GooberBlox");
                    playerbeta_path.push("Roblox");
                    playerbeta_path.push("2016");
                    playerbeta_path.push("GooberPlayerBeta.exe");

                    let _playerbeta = Command::new(playerbeta_path)
                    .args([r"--authenticationUrl",r"http://goober.biz/login/negotiate.ashx",r"--authenticationTicket",&player_token,r"--joinScriptUrl",&format!(r"http://www.goober.biz/game/newcl/join.ashx?placeid={placeid}&auth={player_token}&game={game}")])
                    .arg(r"--play")
                    .spawn()
                    .expect("Failed to start playerbeta!");
                    print_advanced("Launched client!", 0)
                } else {
                    eprintln!("Invalid URI: {}", uri_str);
                }
            }
        }
    } else {
        //install mode
        install().await;
    };
    print_advanced("Tasks done!", 0);
    sleep(Duration::new(3, 0));
}

async fn hash_check(http_client: Client) {
    let base_url: &str = "goober.biz";
    let setup_url: &str = &format!("setup.{}", base_url);
    let mut exec_pathbuf = dirs::data_local_dir()
        .expect("Hard Error")
        .join("GooberBlox");
    let mut playerbeta_path = exec_pathbuf.join("Roblox").join("2016");
    playerbeta_path.push("GooberPlayerBeta.exe");
    let playerbeta_hash_on_disk: &str = "none";
    let launcher_hash_on_appdata: &str = "none";
    let _ = create_dir_all(&exec_pathbuf.join("Roblox"));
    if !playerbeta_path.exists() {
        let playerbeta_hash_on_disk = "TEST";
    } else {
        let playerbeta_hash_on_disk: String = calculate_file_sha256(&playerbeta_path).await;
    }
    if !exec_pathbuf.join("GooberLauncher.exe").exists() {
        let launcher_hash_on_appdata: String =
            calculate_file_sha256(&env::current_exe().expect("Hard Error")).await;
    } else {
        let launcher_hash_on_appdata: String =
            calculate_file_sha256(&exec_pathbuf.join("GooberLauncher.exe")).await;
    }
    let launcher_hash_on_disk: String =
        calculate_file_sha256(&env::current_exe().expect("Hard Error")).await;
    let hash_on_web = http_get(
        &http_client,
        &format!("http://{}/game/game-version", &base_url),
    )
    .await
    .expect("Hard Error");

    match serde_json::from_str::<HashResponse>(&hash_on_web) {
        Ok(hash_response) => {
            let client_hash = hash_response.client_hash;
            let launcher_hash = hash_response.launcher_hash;
            if client_hash == playerbeta_hash_on_disk || playerbeta_hash_on_disk != "TEST" {
                print_advanced("Client is up-to-date!", 0)
            } else {
                print_advanced("Client is out-of-date or corrupted, redownloading..", 0);
                let file_content = Cursor::new(
                    download_file(
                        &http_client,
                        &format!("http://{}/GooberClientE.zip", &setup_url),
                    )
                    .await
                    .unwrap(),
                );

                let _ = std::fs::remove_dir_all(&exec_pathbuf.join("Roblox").join("2016"));
                print_advanced("Extracting...", 0);
                match zip_extract::extract(
                    file_content,
                    &exec_pathbuf.join("Roblox").join("2016"),
                    true,
                ) {
                    Ok(_) => {
                        print_advanced("Extraction finished!", 0);
                        let file_content: Vec<u8> = download_file(
                            &http_client,
                            &format!("http://{}/2016-RobloxAppE.exe", &base_url),
                        )
                        .await
                        .unwrap();
                        write_file(
                            &file_content,
                            &exec_pathbuf
                                .join("Roblox")
                                .join("2016")
                                .join("GooberPlayerBeta.exe"),
                        );
                        print_advanced("Installation finished!", 0);
                    }
                    Err(err) => {
                        let _ = std::fs::remove_dir_all(&exec_pathbuf.join("Roblox").join("2016"));
                        eprintln!("Error during extraction: {:?}", err)
                    }
                }
            }
            if launcher_hash == launcher_hash_on_disk {
                print_advanced("Launcher is up-to-date.", 0)
            } else {
                print_advanced("Please re-download the launcher from the site.", 1)
            }
            if launcher_hash_on_disk != launcher_hash_on_appdata {
                if let Ok(executable_path) = &env::current_exe() {
                    if let Ok(_executable_file) = std::fs::File::open(&executable_path) {
                        sleep(Duration::from_secs(2));
                        copy_executable(&executable_path, &exec_pathbuf.join("GooberLauncher.exe"));
                        //async { install_further("2016").await }.await;
                    }
                }
            }
        }
        Err(err) => eprintln!("Error deserializing JSON: {}", err),
    }
}

async fn install_further(year: &str) {
    let http_client = reqwest::Client::builder()
        .timeout(Duration::new(18446744073709551614, 0))
        .build()
        .expect("Hard Error");

    let base_url: &str = "goober.biz";
    let setup_url: &str = &format!("setup.{}", base_url);
    let mut exec_pathbuf = dirs::data_local_dir().expect("Hard Error");
    let appdata_sub = "GooberBlox";
    //let roblox_sub = "Roblox";

    exec_pathbuf.push(appdata_sub);
    if !exec_pathbuf.join("Roblox").join("2016").exists()
        || !exec_pathbuf.join("GooberLauncher.exe").exists()
    {
        let _ = create_dir_all(&exec_pathbuf.join("Roblox"));
        let file_content = download_file(
            &http_client,
            &format!("http://{}/GooberClient.zip", &setup_url),
        )
        .await
        .unwrap();

        match zip_extract::extract(
            Cursor::new(file_content),
            &exec_pathbuf.join("Roblox").join("2016"),
            true,
        ) {
            Ok(_) => print_advanced("Installation finished..", 0),
            Err(err) => {
                let _ = std::fs::remove_dir_all(&exec_pathbuf.join("Roblox").join("2016"));
                eprintln!("Error during extraction: {:?}", err)
            }
        }
    } else {
        //hash_check(http_client).await;
    }
}

async fn install() -> Result<String, reqwest::Error> {
    let appdata_sub: &str = "GooberBlox";
    let bootstrapper_filename: &str = "GooberLauncher.exe";
    let uri_scheme: &str = "goober-player";

    let hkcu_classes_key: RegKey = RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey_with_flags("Software\\Classes", KEY_WRITE)
        .unwrap();
    let mut exec_pathbuf = dirs::data_local_dir()
        .expect("Hard Error")
        .join(&appdata_sub);
    if !exec_pathbuf.exists() {
        let _ = create_dir_all(&exec_pathbuf);
    };
    if !exec_pathbuf.join("Roblox").join("2016").exists()
        || !exec_pathbuf.join(&bootstrapper_filename).exists()
    {
        if let Ok(executable_path) = &env::current_exe() {
            if let Ok(_executable_file) = std::fs::File::open(&executable_path) {
                if copy_executable(&executable_path, &exec_pathbuf.join(&bootstrapper_filename)) {
                    print_advanced("Starting installation..", 0);
                    install_further("2016").await;
                } else {
                    panic!("Unable to install, make a ticket for help.");
                }
            } else {
                eprintln!("executable path couldnt be grabbed");
            }
        }
    }
    let exec_keypath: String = format!(
        "\"{}\" \"%1\"",
        &exec_pathbuf.join(&bootstrapper_filename).display()
    );

    let scheme_key_result: Result<(RegKey, RegDisposition), _> =
        hkcu_classes_key.create_subkey_with_flags(uri_scheme, KEY_WRITE);
    match scheme_key_result {
        Ok((scheme_key, _)) => {
            scheme_key
                .set_value("", &format!("URL {} Protocol", uri_scheme))
                .unwrap();
            scheme_key.set_value("URL Protocol", &"").unwrap();

            let (command_key, _) = scheme_key
                .create_subkey_with_flags("shell\\open\\command", KEY_WRITE)
                .unwrap();
            command_key.set_value("", &exec_keypath).unwrap();

            let _icon_key = scheme_key
                .create_subkey_with_flags("DefaultIcon", KEY_WRITE)
                .unwrap();
        }
        Err(err) => {
            eprintln!(
                "An error has occurred, please report it to Gooberblox via tickets: {}",
                err
            )
        }
    }
    Ok("Hi".to_string())
}

fn get_query_param(url: &Url, key: &str) -> String {
    url.query_pairs()
        .find_map(|(k, v)| if k == key { Some(v.to_string()) } else { None })
        .unwrap_or_default()
}

fn copy_executable(source: &Path, target: &Path) -> bool {
    if let Err(err) = std::fs::copy(source, target) {
        eprintln!("Error copying executable: {:?}", err);
        false
    } else {
        true
    }
}
