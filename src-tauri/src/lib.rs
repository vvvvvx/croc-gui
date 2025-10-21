// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
pub mod chat_listener;
pub mod setting;
pub mod str_oper;
pub mod types;
pub mod utils;
pub mod version;

use std::collections::HashMap;
use std::env::consts::OS;
use std::io::{self, Read};
use std::process::Stdio;
use std::process::{Child, Command};
use tokio::process::{Child as tChild, Command as tCommand};
//use tauri::plugin;
use crate::types::{CrocWorker, EmitInfo, EmitProgress, FileItem};
use crate::utils::*;
use crate::version::check_update;
use chat_listener::{start_chat_listener, stop_chat_listener};
use setting::{load_config, save_config, ConfigState};
//use std::fs::{self};
use once_cell::sync::Lazy;
use std::sync::Arc;
use str_oper::*;
use tauri::Emitter;
use tauri::State;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
//use std::path::Path;
//use tauri_plugin_shell;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// 用于处理croc进程运行中的实时交互。回答Y/N
pub static GLOBAL_STDINS: Lazy<
    Arc<Mutex<HashMap<String, Arc<Mutex<tokio::process::ChildStdin>>>>>,
> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

// static CONFIRM_STATE: Lazy<Mutex<HashMap<String, Option<String>>>> =
//     Lazy::new(|| Mutex::new(HashMap::new()));

#[tauri::command]
async fn send_files(
    window: tauri::Window,
    state: State<'_, ConfigState>,
    mut files: Vec<FileItem>, // 需要发送的文件列表或目录
    code: String,             // 传输代码Code
    is_folder: bool,          // 是否为目录
    zip: bool,                // if zip folder before sendding
    exclude: String,          // exclude patterns when send folders
) -> Result<(), String> {
    let cfg = state.0.read().unwrap().clone();

    println!("send_files cfg:{cfg:?}");
    // 构建 croc 命令参数
    // global parameters
    let mut croc_args = global_args(cfg.clone());
    // send parameters
    croc_args.push("send".to_string());

    if let Some(transfers) = cfg.transfers {
        croc_args.push("--transfers".to_string());
        croc_args.push(transfers.to_string());
    }
    if !exclude.trim().is_empty() {
        croc_args.push("--exclude".to_string());
        croc_args.push(exclude.clone());
    }
    if zip {
        croc_args.push("--zip".to_string());
    }
    if !code.trim().is_empty() && OS == "windows" {
        croc_args.push("--code".to_string());
        croc_args.push(code.clone());
    }
    if files.is_empty() {
        window
            .emit(
                "croc-send-error",
                Some(EmitInfo {
                    croc_code: code.clone().trim().to_string(),
                    info: "No files to send".to_string(),
                }),
            )
            .unwrap();
        return Ok(());
    }

    for file in files.iter() {
        croc_args.push(file.file.clone());
    }

    // 处理目录，插入目录下的文件
    if is_folder && !zip {
        files = insert_files_after_dir(files);
    }
    // 启动 croc 进程
    println!("Send files croc with args: {croc_args:?}");

    let code2 = code.clone();
    tokio::task::spawn_blocking(move || {
        #[cfg(not(windows))]
        let mut child: Child = if !code2.trim().is_empty() {
            Command::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2.clone()) // 设置环境变量
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command,maybe croc not found")
        } else {
            Command::new("croc")
                .args(croc_args)
                .env("CROC_NOUI", true.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command,maybe croc not found")
        };

        #[cfg(windows)]
        let mut child = Command::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        let mut code_str = "".to_string();
        if !code2.trim().is_empty() {
            code_str=code2.trim().to_string();
        }
        // 处理 croc 输出

        if let Some(stderr) = child.stderr.take() {
            let mut reader = io::BufReader::new(stderr);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("croc output: {output}");

                        if let Some(code) = get_code(&output) {
                            // println!("Extracted code: {}", code);
                            println!("Output from [stderr].");
                            window.emit("croc-code", Some(code.to_string())).unwrap();
                            code_str=code.to_string();
                            replace_hash_percent(&mut files);
                        }
                        if output.contains("Code is:") {
                            // 传输完成，强制将所有文件状态更新为100%
                            replace_hash_percent(&mut files);

                            window
                                .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();
                            window
                                .emit("croc-send-file-ready", Some(EmitInfo{croc_code:code_str.clone(),info: "文件已准备好，请把Code给对方以开始接收。\n【Code已复制，直接粘贴】\nFiles ready,provide the Code to recipient to receive.\n【Code copied to clipboard】".to_string()}))
                                .unwrap();
                        }

                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-send-file-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }
                        if zip && is_folder{
                            if let Some(zip_file)=get_zip_filename(&output){
                                // insert zip filename to top
                                //files.insert(0,FileItem { file: zip.to_string(), status: "Pending".to_string(), is_dir: false });
                                let fname=zip_file.to_string();
                                //let fname="Zip file  =>   ".to_string()+zip_file.as_str();
                                files.push(FileItem { file: fname, status: "Pending".to_string(), is_dir: false });
                                window
                                    .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone().to_string(),files: files.clone()}))
                                    .unwrap();
                            }
                        }
                        if let Some(progress_data) = get_progress_data(&output,"Sending") {
                            // println!("Extracted progress data: {:?}", progress_data);
                            let status_str = if progress_data.progress_type=="Hashing"{
                                format!(
                                    "{}: {} ",
                                    progress_data.progress_type,
                                    progress_data.percentage
                                )
                            } else {format!(
                                "{}: {} {} {}",
                                progress_data.progress_type,
                                progress_data.percentage,
                                progress_data.progress,
                                progress_data.time
                            )};
                            // println!("files: {:?}", files);
                            update_file_status(&mut files, &progress_data.filename, &status_str);
                            // 发送更新后的文件状态列表到前端
                            window
                                .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();
                        }
                        if output.contains("not enough open ports") {
                            window
                                .emit("croc-send-error", Some(EmitInfo{croc_code:code_str.clone(),info: "太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()}))
                                .unwrap();
                            // return "Repeated sending.".to_string();
                        }
                        // window
                        //     .emit("croc-output", Some(output.to_string()))
                        //     .unwrap();
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}" );
                        break;
                    }
                }
            }
        }
        let status = child.wait().expect("Command wasn't running");

        if status.success() {
            // 传输完成，强制将所有文件状态更新为100%
            update_folder_status_after_completed(&mut files);
            window
                .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone().to_string(),files: files.clone()}))
                .unwrap();

            window
                .emit("croc-send-file-success", Some(EmitInfo{croc_code:code_str.clone().to_string(),info: "所有文件已成功发送\nFiles sent successfully".to_string()}))
                .unwrap();
        } else {
            emit_exit_info(window.clone(), "send", code_str.clone(), status.code().unwrap()); 
        }
        window.emit("croc-send-file-done", Some(EmitInfo{croc_code:code_str.clone().to_string(),info: "File send done.".to_string()}))
            .unwrap();

        status
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn write_stdin(code: String, input: String) -> Result<(), String> {
    // let mut state = CONFIRM_STATE.lock().await;
    // state.insert(code, Some(input));
    let map = GLOBAL_STDINS.lock().await;

    if let Some(stdin_arc) = map.get(&code) {
        let mut stdin = stdin_arc.lock().await;
        stdin
            .write_all(format!("{input}\n").as_bytes())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn receive_files(
    window: tauri::Window,
    state: State<'_, ConfigState>,
    save_path: String, // 保存路径
    code: String,      // 传输代码Code
) -> Result<(), String> {
    if code.trim().is_empty() {
        window
            .emit(
                "croc-receive-error",
                Some("须输入Code,请输入Code\nCode is empty".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    if save_path.trim().is_empty() {
        window
            .emit(
                "croc-receive-error",
                Some("须输入保存路径,请输入保存路径\nSave path is empty".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    // read config
    let cfg = state.0.read().unwrap().clone();

    // 构建 croc 命令参数
    // global parameters first
    let mut croc_args = global_args(cfg.clone());
    // receive parameters
    croc_args.push("--out".to_string());

    if !is_dir(save_path.as_str()) {
        window
            .emit(
                "croc-receive-error",
                Some("错误的保存路径。\nWrong save path.".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    croc_args.push(save_path.clone());

    if OS == "windows" {
        croc_args.push(code.clone());
    }

    // 启动 croc 进程
    println!("Receive files croc with args: {croc_args:?}");
    let code2 = code.clone();
    let code_str = code2.clone();
    let mut files: Vec<FileItem> = vec![];
    //tokio::task::spawn_blocking(move || {
    tokio::spawn(async move {

        #[cfg(not(windows))]
        let mut child: tChild = tCommand::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2) // 设置环境变量
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command");

        #[cfg(windows)]
        let mut child:tChild = tCommand::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        // 保存stdin,用于初一异常交互Y/N
        let stdin=child.stdin.take().unwrap();
        let state=GLOBAL_STDINS.lock().await.insert(code_str.clone(),Arc::new(Mutex::new(stdin)));
        //let mut state=GLOBAL_STDINS.lock().await.insert(code_str.clone(),Arc::new(Mutex::new(None)));
        drop(state);
        // 处理 croc 输出
        let mut full_out="".to_string(); //for stdout
        let mut full_err="".to_string(); //for stderr


        if let Some(stderr) = child.stderr.take() {
            // let mut reader = io::BufReader::new(stderr);
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("croc receive err-output: {output}");

                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-receive-file-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }
                        if output.contains("(secure channel) not ready") {
                            window
                                .emit("croc-receive-error", Some("无内容可接收或Code冲突,等会儿再试或换个Code重试。\nNo content to receive or Code conflict,waiting or change the Code.".to_string()))
                                .unwrap();
                        }
                        if output.contains("room full") {
                            window
                                .emit("croc-receive-error", Some("远程响应错误，可能是对方连续用相似的自定义Code，\n建议让对方用自动生成Code的方式重发。\n\nRemote response error, possibly because the sender\nrepeatedly used similar custom Codes. \nIt is recommended to have them resend using \nautomatically generated Codes.".to_string()))
                                .unwrap();
                        }
                        if let Some(progress_data) = get_progress_data(&output,"Receiving") {
                            // println!("Extracted progress data: {:?}", progress_data);
                            let status_str = format!(
                                "{}: {} {} {}",
                                progress_data.progress_type,
                                progress_data.percentage,
                                progress_data.progress,
                                progress_data.time
                            );
                            if !is_contains_file(&files,&progress_data.filename){
                                files.push(FileItem{
                                    file:progress_data.filename.clone(),
                                    status:status_str.clone(),
                                    is_dir:false,
                                });
                            } else{
                                update_file_status(&mut files, &progress_data.filename, &status_str);
                            }

                            // println!("files: {:?}", files);
                            // 发送更新后的文件状态列表到前端
                            window
                                .emit("croc-receive-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();
                        }
                        if output.contains("not enough open ports") {
                            window
                                .emit("croc-receive-error", Some("太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()))
                                .unwrap();
                            // return "Repeated sending.".to_string();
                        }
                        if let Some(confirm)=get_confirm(&output) {
                            window.emit("croc_confirm", EmitInfo{croc_code:code_str.clone(),info:confirm}) .unwrap();
                        }
                        full_err+=output.as_str();
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}");
                        break;
                    }
                }
            }
        }
        if let Some(stdout) = child.stdout.take() {
            let mut reader = BufReader::new(stdout);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("croc receive stdout: {output}");

                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-receive-file-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }

                        full_out+=output.as_str();
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}");
                        break;
                    }
                }
            }
        }

        println!("receive file full_stderr:{}",full_err);
        println!("receive file full_stdout:{}",full_out);
        // let mut stdout = child.stdout.take().unwrap();
        // let mut stdout_buf = Vec::new();
        // stdout.read_to_end(&mut stdout_buf).unwrap();
        // let full_out = String::from_utf8_lossy(&stdout_buf).to_string();

        let status = child.wait().await.expect("Command wasn't running");

        if status.success() {
            // 如果full_out是空，其实是接收的文件。
            if full_out.is_empty(){
                // 传输完成，强制将所有文件状态更新为100%
                replace_completed_percent(&mut files);
                window.emit("croc-receive-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                    .unwrap();

                window
                    .emit("croc-receive-file-success", Some(EmitInfo{croc_code:code_str.clone(),info: "所有文件已成功接收\nAll files received successfully".to_string()}))
                    .unwrap();
            }else {
                //如果full_out不是空，实际是接收的text,不是file
                window
                    .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: full_out.to_string()}))
                    .unwrap();

            }
            GLOBAL_STDINS.lock().await.remove(&code_str);
        } else {
            emit_exit_info(window.clone(), "receive", code_str.clone(), status.code().unwrap()); 
            GLOBAL_STDINS.lock().await.remove(&code_str);
            // window
            //     .emit(
            //         "croc-receive-error",
            //         Some(format!("Croc command failed with status: {status}")),
            //     )
            //     .unwrap();
        }
        // window.emit("croc-receive-file-done", Some("File receiving finished.".to_string()))
        //     .unwrap();

    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn send_text(
    window: tauri::Window,
    state: State<'_, ConfigState>,
    msg: String,  // 要发送的信息/ Msg to send
    code: String, // 传输代码Code
) -> Result<(), String> {
    if msg.trim().is_empty() {
        window
            .emit(
                "croc-send-error",
                Some(EmitInfo {
                    croc_code: code.clone().trim().to_string(),
                    info: "消息不能为空".to_string(),
                }),
            )
            .unwrap();
        return Ok(());
    }
    // 读取内存配置
    let cfg = state.0.read().unwrap().clone();

    // 构建 croc 命令参数
    // global parameters first
    let mut croc_args: Vec<String> = global_args(cfg.clone());
    // send parameters
    croc_args.push("send".to_string());
    if !code.trim().is_empty() && OS == "windows" {
        croc_args.push("--code".to_string());
        croc_args.push(code.clone());
    }

    croc_args.push("--text".to_string());
    croc_args.push(msg.clone());

    println!("Running croc with args: {croc_args:?}");
    let code2 = code.clone();
    tokio::task::spawn_blocking(move || {
        #[cfg(not(windows))]
        let mut child: Child = if !code2.trim().is_empty() {
            Command::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2.clone()) // 设置环境变量
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command")
        } else {
            Command::new("croc")
                .args(croc_args)
                .env("CROC_NOUI", true.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command")
        };

        #[cfg(windows)]
        let mut child = Command::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        let mut code_str = "".to_string();
        if !code2.trim().is_empty() {
            code_str=code2.trim().to_string();
        }
        // 处理 croc 输出
        let mut full_output="".to_string();

        if let Some(stderr) = child.stderr.take() {
            let mut reader = io::BufReader::new(stderr);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("croc output: {output}");

                        if let Some(code) = get_code(&output) {
                            // println!("Extracted code: {}", code);
                            println!("Output from [stderr].");
                            window.emit("croc-send-text-code", Some(code.to_string())).unwrap();
                            code_str=code.to_string();
                        }

                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-send-text-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }
                        if output.contains("not enough open ports") {
                            window
                                .emit("croc-send-error", Some(EmitInfo{croc_code:code_str.clone(),info:"太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()}))
                                .unwrap();
                            // return "Repeated sending.".to_string();
                        }
                        full_output+=output.as_str();
                        // window
                        //     .emit("croc-output", Some(output.to_string()))
                        //     .unwrap();
                    }
                    Err(err) => {
                        eprintln!("Error reading stderr: {err}");
                        break;
                    }
                }
            }
        }
        // if let Some(status) = get_status(&full_output) {
        //     // println!("Extracted status: {}", status);
        //     window
        //         .emit("croc-send-text-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
        //         .unwrap();
        // }
        let status = child.wait().expect("Command wasn't running");

        if status.success() {
            window
                .emit("croc-send-text-success", Some(EmitInfo{croc_code:code_str.clone().to_string(),info: msg}))
                .unwrap();
        } else {
            emit_exit_info(window.clone(), "send", code_str.clone(), status.code().unwrap()); 
        }
        // window.emit("croc-receive-file-done", Some("File receiving finished.".to_string()))
        //     .unwrap();

    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
async fn receive_text(
    window: tauri::Window,
    state: State<'_, ConfigState>,
    code: String,      // 传输代码Code
    save_path: String, // 用于处理误在聊天界面接收文件
) -> Result<(), String> {
    if code.trim().is_empty() {
        window
            .emit(
                "croc-receive-error",
                Some("Code can't be empty.".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    if save_path.trim().is_empty() {
        window
            .emit(
                "croc-receive-error",
                Some("须输入保存路径,请输入保存路径\nSave path is empty".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    let cfg = state.0.read().unwrap().clone();
    // 构建 croc 命令参数
    // global parameters first
    let mut croc_args: Vec<String> = global_args(cfg.clone());
    // receive parameters
    croc_args.push("--out".to_string());

    if !is_dir(save_path.as_str()) {
        window
            .emit(
                "croc-receive-error",
                Some("错误的保存路径。\nWrong save path.".to_string()),
            )
            .unwrap();
        return Ok(());
    }
    croc_args.push(save_path);

    if OS == "windows" {
        croc_args.push(code.clone())
    }

    println!("Running croc with args: {croc_args:?}");
    let code2 = code.clone();
    let mut files: Vec<FileItem> = vec![];
    //tokio::task::spawn_blocking(move || {
    tokio::spawn(async move {
        #[cfg(not(windows))]
        let mut child: tChild = tCommand::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2.clone()) // 设置环境变量
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command");

        #[cfg(windows)]
        let mut child: tChild = tCommand::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        let code_str = code2.trim().to_string();

        // 保存stdin,用于初一异常交互Y/N
        let stdin=child.stdin.take().unwrap();
        let state=GLOBAL_STDINS.lock().await.insert(code_str.clone(),Arc::new(Mutex::new(stdin)));

        // 处理 croc 输出
        let mut full_output="".to_string();
        if let Some(stderr) = child.stderr.take() {
            let mut reader = BufReader::new(stderr);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer).await {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]).to_string();
                        println!("croc output: {output}");

                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-receive-text-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }
                        if let Some(msg) = get_text_msg(&output) {
                            // println!("Extracted msg: {}", msg);
                            window
                                .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: msg}))
                                .unwrap();
                        }
                        if output.contains("not enough open ports") {
                            window
                                .emit("croc-receive-error", Some("太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()))
                                .unwrap();
                            // return "Repeated sending.".to_string();
                        }
                        if output.contains("(secure channel) not ready") {
                            window
                                .emit("croc-receive-error", Some("无内容可接收或Code冲突,等会儿再试或换个Code重试。\nNo content to receive or Code conflict,waiting or change the Code.".to_string()))
                                .unwrap();
                        }
                        // 如果错误的在Receive Text界面接收文件
                        if let Some(progress_data) = get_progress_data(&output,"Receiving") {
                            // println!("Extracted progress data: {:?}", progress_data);
                            let status_str = format!(
                                "{}: {} {} {}",
                                progress_data.progress_type,
                                progress_data.percentage,
                                progress_data.progress,
                                progress_data.time
                            );
                            if !is_contains_file(&files,&progress_data.filename){
                                files.push(FileItem{
                                    file:progress_data.filename.clone(),
                                    status:status_str.clone(),
                                    is_dir:false,
                                });
                            } else{
                                update_file_status(&mut files, &progress_data.filename, &status_str);
                            }

                            // println!("files: {:?}", files);
                            // 发送更新后的文件状态列表到前端
                            window
                                .emit("croc-receive-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();

                        }
                        if let Some(confirm)=get_confirm(&output) {
                            window.emit("croc_confirm", EmitInfo{croc_code:code_str.clone(),info:confirm}) .unwrap();
                        }
                        full_output+=output.as_str();
                    }
                    Err(err) => {
                        eprintln!("Error reading stderr: {err}");
                        break;
                    }
                }
            }
        }

        let mut stdout = child.stdout.take().unwrap();
        let mut stdout_buf = Vec::new();
        stdout.read_to_end(&mut stdout_buf).await.unwrap();
        // croc send --text 时，正文在stdout
        let full_out= String::from_utf8_lossy(&stdout_buf).to_string();

        let status = child.wait().await.expect("Command wasn't running");

        if status.success() {
            //如果full_out是空，实际是接收的file,而非text
            if full_out.is_empty(){
                // 传输完成，强制将所有文件状态更新为100%
                replace_completed_percent(&mut files);
                window.emit("croc-receive-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                    .unwrap();

                window
                    .emit("croc-receive-file-success", Some(EmitInfo{croc_code:code_str.clone(),info: "所有文件已成功接收\nAll files received successfully".to_string()}))
                    .unwrap();
            }else {
                //如果full_out不是空，实际是接收的text,不是file ,full_out就是接收正文
                window
                    .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: full_out.to_string()}))
                    .unwrap();

            }
            GLOBAL_STDINS.lock().await.remove(&code_str);
            // window
            //     .emit("croc-receive-text-success", Some(EmitInfo{croc_code:code_str.clone(),info: "receive success".to_string()}))
            //     .unwrap();
        } else {
            emit_exit_info(window.clone(), "receive", code_str.clone(), status.code().unwrap()); 
            GLOBAL_STDINS.lock().await.remove(&code_str);
            // window
            //     .emit(
            //         "croc-receive-error",
            //         Some(format!("Croc command failed with status: {status}")),
            //     )
            //     .unwrap();
        }

    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(())
}
fn kill_croc_process_fn() {
    println!("Searching process cleanning up");
    #[cfg(windows)]
    let mut child = Command::new("taskkill")
        .arg("/IM")
        .arg("croc*")
        .arg("/F")
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()
        .expect("Failed to execute taskkill process");
    #[cfg(not(windows))]
    let mut child = Command::new("pkill")
        .arg("-9")
        .arg("-f")
        .arg("^croc")
        .spawn()
        .expect("Failed to execute pkill process");

    let status = child.wait().expect("Failed to execute taskkill process");
    if !status.success() {
        if let Some(stderr) = child.stderr.take() {
            let mut reader = io::BufReader::new(stderr);
            let mut lines = String::new();
            let _ = reader.read_to_string(&mut lines).unwrap();
            if !lines.trim().is_empty() {
                eprintln!("Error killing croc process: {}", &lines);
            }
        }
        eprintln!("Error killing croc process: {status}");
    } else {
        println!("Croc process cleanned up");
    }
}

// use tauri::App;
// use tauri::RunEvent;
// use tauri::AppHandle;
use crate::setting::{global_args, load_config_internal};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    //读取程序配置信息
    let _cfg = load_config_internal();
    let app = tauri::Builder::default()
        // chat_listener监听进程管理
        .manage(Arc::new(Mutex::new(CrocWorker::default())))
        //加入全局配置信息
        .manage(ConfigState(std::sync::RwLock::new(_cfg)))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        //.plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            send_files,
            receive_files,
            send_text,
            receive_text,
            check_update,
            start_chat_listener,
            stop_chat_listener,
            load_config,
            save_config,
            write_stdin
        ])
        // .run(tauri::generate_context!())
        .build(tauri::generate_context!())
        .expect("error while running tauri application");

    app.run(|_app_handle, event| {
        if let tauri::RunEvent::ExitRequested { .. } = event {
            kill_croc_process_fn();
        }
    });
}
