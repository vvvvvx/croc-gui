// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::env::consts::OS;
use std::io::{self, Read};
use std::process::Stdio;
use std::process::{Child, Command};
use tokio::io::AsyncReadExt;
//use tauri::plugin;
use std::fs::{self};
use tauri::Emitter;
//use std::path::Path;
//use tauri_plugin_shell;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

static RE_CODE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"Code is:\s*([0-9a-zA-Z]+-[0-9a-zA-Z]+(?:-[a-zA-Z0-9]+)*)")
        .expect("Invalid regex")
});

static RE_HASHING: Lazy<Regex> = Lazy::new(|| {
    //Regex::new(r"Hashing\s+([^\s%]+)\s*(\d+)%\s*\|([^|]+)\|\s*\((\d*\.?\d*)\s*MB/s\)\s*\[(\d+)(s|m|month|year):(\d+)\5\]")
    //(Hashing)   (文件名)   (百分比)  (进度条)               (速度)         (时间1 单位 : 时间2 单位)
    //Regex::new(r"(Hashing)\s+([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\(\d+\.?\d*\s+[a-zA-Z]+/s\))\s+(\[\d+\.?\d*[a-zA-Z]+:\d+\.?\d*[a-zA-Z]+\])")
    Regex::new(r"(Hashing)\s+([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\([\d\s\.A-Za-z]+/[a-zA-Z]\))\s+(\[[\da-zA-Z]+:[\da-zA-Z]+\])")
        .expect("Invalid regex for Hashing")
});

static RE_SENDING: Lazy<Regex> = Lazy::new(|| {
    //Regex::new(r"([^\s%]+)\s*(\d+)%\s*\|([^|]+)\|\s*\((\d*\.?\d*)/(\d*\.?\d*)\s*MB,\s*(\d*\.?\d*)\s*MB/s\)\s*\[(\d+)(s|m|month|year):(\d+)\7\]")
    //(文件名)   (百分比)  (进度条)               (已发送/总大小    速度)         (时间1 单位 : 时间2 单位)
    Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\([\d/\.\sA-Za-z]+/[\d/\.\sA-Za-z]+,[\d\s\.A-Za-z]+/[a-zA-Z]\))\s+(\[[\da-zA-Z]+:[\da-zA-Z]+\])")
        .expect("Invalid regex for Sending")
});
static RE_COMPLETED: Lazy<Regex> = Lazy::new(|| {
    //(文件名)   (百分比)  (进度条)               (已发送/总大小    速度)
    // Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\(\d+\.?\d*/\d+\.?\d*\s+[a-zA-Z]+,\s+\d+\.?\d*\s[a-zA-Z]+/s\))")
    //Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\([\d/\.\sA-Za-z]+,[\d/\.\sA-Za-z]+\))")
    Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\([\d/\.\sA-Za-z]+/[\d/\.\sA-Za-z]+,[\d\s\.A-Za-z]+/[a-zA-Z]\))")
        .expect("Invalid regex for Completed")
});
static RE_RECEIVE_MSG: Lazy<Regex> = Lazy::new(|| {
    // Receiving (<-125.70.11.136:50824)
    // 以上行开头，下行开始才是消息内容
    Regex::new(r"(Receiving\s+\(<\-\d+\.\d+\.\d+\.\d+:\d+\)|Sending\s+\(\->\d+\.\d+\.\d+\.\d+:\d+\))[\n\r\s]+([^\s%]+([\n\r\s]*[^\s%])*)")
        .expect("Invalid regex for ReceiveMsg")
});

static RE_STATUS: Lazy<Regex> = Lazy::new(|| {
    //Connecting | connecting | Receiving (<-134.12.34:56789) | Sending (->134.12.34:56789)
    Regex::new(r"(Connecting|connecting|Receiving\s+\(<\-\d+\.\d+\.\d+\.\d+:\d+\)|Sending\s+\(\->\d+\.\d+\.\d+\.\d+:\d+\))")
        .expect("Invalid regex for Status")
});

static RE_PERCENT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+%").expect("Invalid regex for Percent"));

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgressData {
    progress_type: String, // 格式类型：Hashing/Sending/Done
    filename: String,      // 文件名
    percentage: String,    // 百分比
    progress_bar: String,  // 进度条
    progress: String,      // 进度
    time: String,          //已耗时：预估耗时
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct FileItem {
    file: String,   //文件路径
    status: String, //发送进度状态信息,由ProgressData组合而成的字符串
    is_dir: bool,   //是否为目录
}

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
use tauri::State;
use tokio::{
    process::Command as tCommand,
    sync::Mutex,
    time::{sleep, Duration},
};

#[derive(Default)]
struct CrocWorker {
    tasks: HashMap<String, Arc<AtomicBool>>, // code -> running flag
}

#[tauri::command]
async fn start_chat_listener(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    window: tauri::Window,
    code: String,
) -> Result<(), String> {
    let mut worker = state.lock().await;

    // 如果该 code 已在运行，则返回
    if let Some(flag) = worker.tasks.get(&code) {
        if flag.load(Ordering::SeqCst) {
            println!("Code:{code}:\nListener already running");
            return Ok(());
        }
    }

    let running = Arc::new(AtomicBool::new(true));
    worker.tasks.insert(code.clone(), running.clone());
    //let window_clone = window.clone();
    let state_clone = state.inner().clone();
    let code_clone = code.clone();

    tokio::spawn(async move {
        //tokio::task::spawn_blocking(move || {
        println!("Code:{code_clone}\nlistener started");

        while running.load(Ordering::SeqCst) {
            // 运行 croc receive 命令
            #[cfg(windows)]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .arg(&code_clone)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run croc,maybe croc not found"); //

            #[cfg(not(windows))]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .env("CROC_SECRET", code_clone.clone()) // 设置环境变量
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run croc,maybe croc not found"); //

            // 读取 stdout
            let mut stdout = child.stdout.take().unwrap();
            let mut stdout_buf = Vec::new();
            stdout.read_to_end(&mut stdout_buf).await.unwrap();
            let stdout_str = String::from_utf8_lossy(&stdout_buf);

            // 读取 stderr
            let mut stderr = child.stderr.take().unwrap();
            let mut stderr_buf = Vec::new();
            stderr.read_to_end(&mut stderr_buf).await.unwrap();
            let stderr_str = String::from_utf8_lossy(&stderr_buf);

            println!("Listener stdout:{stdout_str}");
            println!("Listener stderr:{stderr_str}");
            // if let Some(msg) = get_text_msg(&stderr_str) {
            //     println!("Extracted msg: {}", msg);
            //     window
            //         .emit(
            //             "croc-receive-text-msg",
            //             Some(EmitInfo {
            //                 croc_code: code_clone.clone(),
            //                 info: msg,
            //             }),
            //         )
            //         .unwrap();
            //     running.store(false, Ordering::SeqCst);
            // }
            // if let Some(msg) = get_text_msg(&stdout_str) {
            //     println!("Extracted msg: {}", msg);
            //     window_clone
            //         .emit(
            //             "croc-receive-text-msg",
            //             Some(EmitInfo {
            //                 croc_code: code_clone.clone(),
            //                 info: msg,
            //             }),
            //         )
            //         .unwrap();
            //     running.store(false, Ordering::SeqCst);
            // }

            let status = child.wait().await;

            if let Ok(status) = status {
                if status.success() {
                    // if status is success,the msg is the stdout_str
                    window
                        .emit(
                            "croc-receive-text-msg",
                            Some(EmitInfo {
                                croc_code: code_clone.clone(),
                                info: stdout_str.to_string(),
                            }),
                        )
                        .unwrap();
                    running.store(false, Ordering::SeqCst);
                }
            }
            // match status {
            //     Ok(status) => {
            //         if status.success() {
            //             // 传输完成，强制将所有文件状态更新为100%
            //             window
            //                 .emit(
            //                     "croc-receive-text-success",
            //                     Some(EmitInfo {
            //                         croc_code: code_clone.clone(),
            //                         info: stdout_str.to_string() + stderr_str.to_string().as_str(),
            //                     }),
            //                 )
            //                 .unwrap();
            //             running.store(false, Ordering::SeqCst);
            //         }
            //     }
            //     Err(e) => {}
            // }
            // 每 10 秒再检测一次
            sleep(Duration::from_secs(10)).await;
        }

        println!("Code:{code_clone}\nlistener stopped");
        let mut worker = state_clone.lock().await;
        worker.tasks.remove(&code_clone);
    });

    ///////////////////////////////////////////////////
    Ok(())
}

#[tauri::command]
async fn stop_chat_listener(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    code: String,
) -> Result<(), String> {
    let worker = state.lock().await;
    if let Some(flag) = worker.tasks.get(&code) {
        flag.store(false, Ordering::SeqCst);
    }
    Ok(())
}
fn get_code(text: &str) -> Option<String> {
    RE_CODE
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))

    // if let Some(caps) = RE.captures(text) {
    //     if let Some(code_match) = caps.get(1) {
    //         return Some(code_match.as_str().to_string());
    //     }
    // }
    // None
}

fn get_status(text: &str) -> Option<String> {
    RE_STATUS
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
fn get_text_msg(text: &str) -> Option<String> {
    RE_RECEIVE_MSG
        .captures(text)
        .and_then(|caps| caps.get(2).map(|m| m.as_str().to_string()))
}
fn get_progress_data(text: &str, receive_or_send: &str) -> Option<ProgressData> {
    // 尝试匹配Hashing格式1
    if let Some(caps) = RE_HASHING.captures(text) {
        return Some(ProgressData {
            progress_type: "Hashing".to_string(),
            filename: caps[2].to_string(),
            percentage: caps[3].to_string(),
            progress_bar: caps[4].to_string(),
            progress: caps[5].to_string(),
            time: caps[6].to_string(),
        });
    }

    // 尝试匹配Sending格式2
    if let Some(caps) = RE_SENDING.captures(text) {
        return Some(ProgressData {
            progress_type: receive_or_send.to_string(),
            filename: caps[1].to_string(),
            percentage: caps[2].to_string(),
            progress_bar: caps[3].to_string(),
            progress: caps[4].to_string(),
            time: caps[5].to_string(),
        });
    }
    // 尝试匹配Completed格式3
    if let Some(caps) = RE_COMPLETED.captures(text) {
        return Some(ProgressData {
            progress_type: "Done".to_string(),
            filename: caps[1].to_string(),
            percentage: caps[2].to_string(),
            progress_bar: caps[3].to_string(),
            progress: caps[4].to_string(),
            time: "".to_string(),
        });
    }
    None
}

fn is_dir(path: &str) -> bool {
    let metadata = fs::metadata(path);
    if let Ok(meta) = metadata {
        return meta.is_dir();
    }
    false
}

fn get_direct_files(path: &str) -> Result<Vec<FileItem>, std::io::Error> {
    if !is_dir(path) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Not a directory",
        ));
    }
    let mut files: Vec<FileItem> = Vec::new();
    //let parent_path=path.clone();
    let entries = fs::read_dir(path)?;
    for entry in entries {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            let file_path = entry.path();
            let file_str = file_path.to_string_lossy().to_string();
            files.push(FileItem {
                file: file_str,
                status: "待发送/Pending".to_string(),
                is_dir: false,
            });
        } else if file_type.is_dir() {
            let dir_path = entry.path();
            let dir_str = dir_path.to_string_lossy().to_string();
            let mut sub_files = get_direct_files(&dir_str)?;
            files.append(&mut sub_files);
        }
    }
    Ok(files)
}
//Extract dir to file list
fn insert_files_after_dir(files: Vec<FileItem>) -> Vec<FileItem> {
    let mut result: Vec<FileItem> = Vec::new();
    for file in files.iter() {
        if is_dir(&file.file) {
            result.push(FileItem {
                file: file.file.clone(),
                status: "Pending".to_string(),
                is_dir: true,
            });
            if let Ok(mut dir_files) = get_direct_files(&file.file) {
                result.append(&mut dir_files);
            }
        } else {
            //result.push(file.clone());
        }
    }
    result
}
// 更新文件状态
fn update_file_status(files: &mut [FileItem], filename: &str, status: &str) {
    let cleaned_filename = filename.trim_end_matches("...").trim_end_matches("�");

    if let Some(file_item) = files.iter_mut().find(|f| {
        // 模糊匹配：检查文件名前缀是否匹配
        f.file.contains(cleaned_filename)
    }) {
        file_item.status = status.to_string();
        // println!("Updated file item data: {:?}", file_item);
        // println!("Updated file data: {:?}", files);
    } else {
        println!("File item with filename '{filename}' not found");
    }
}
fn is_contains_file(files: &[FileItem], filename: &str) -> bool {
    let cleaned_filename = filename.trim_end_matches("...").trim_end_matches("�");
    for file in files.iter() {
        if file.file.contains(cleaned_filename) {
            return true;
        }
    }
    false
}
fn replace_hash_percent(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        //file.status = RE_PERCENT.replace_all(&file.status,"100%").to_string()
        file.status = "Hashed: 100%".to_string();
    }
}
fn replace_completed_percent(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        file.status = RE_PERCENT.replace_all(&file.status, "100%").to_string()
    }
}
fn update_folder_status_after_completed(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        if file.is_dir {
            file.status = "Done: 100%".to_string();
        }
    }
}

fn emit_exit_info(window: tauri::Window, oper_type: &str, code: String, status_code: i32) {
    match status_code {
        0 => {} // sucess
        1 => {
            //常规错误
            if oper_type == "send" {
                emit_send_signal(
                    window,
                    "croc-send-error",
                    EmitInfo {
                        croc_code: code,
                        info: "普通错误，请重发。\nCommon error,please retry.\n\n".to_string(),
                    },
                );
            } else {
                emit_receive_signal(
                    window,
                    "croc-receive-error",
                    "无接收内容\nNo content to receive \n\n",
                );
            }
        }
        2 => {
            // 网络连接或中断

            if oper_type == "send" {
                emit_send_signal(
                    window,
                    "croc-send-error",
                    EmitInfo {
                        croc_code: code,
                        info: "网络连接错误，请重试。重试多次无效，请更换Code。\nNetwork error,retry. If trying again and not, change the Code if  \n\n".to_string(),
                    },
                );
            } else {
                emit_receive_signal(
                    window,
                    "croc-receive-error",
                    "网络连接错误，请重试。重试多次无效，请更换Code。\nNetwork error,retry. If trying again and not, change the Code if  \n\n",
                );
            }
        }
        3 => {
            // 验证失败
            if oper_type == "send" {
                emit_send_signal(
                    window,
                    "croc-send-error",
                    EmitInfo {
                        croc_code: code,
                        info: "验证失败，请更换Code。\nVerification failed,change the Code.  \n\n"
                            .to_string(),
                    },
                );
            } else {
                emit_receive_signal(
                    window,
                    "croc-receive-error",
                    "验证失败，请更换Code。\nVerification failed,change the Code.  \n\n",
                );
            }
        }
        4 => {
            // 传输异常终止
            if oper_type == "send" {
                emit_send_signal(
                    window,
                    "croc-send-error",
                    EmitInfo {
                        croc_code: code,
                        info: "传输异常终止，请重新发送，若依然异常，请更换Code\nThe transmission is terminated abnormally,resend.If still abnormal，change the Code. \n\n"
                            .to_string(),
                    },
                );
            } else {
                emit_receive_signal(
                    window,
                    "croc-receive-error",
                    "传输异常终止，请重新发送，若依然异常，请更换Code\nThe transmission is terminated abnormally,resend.If still abnormal，change the Code. \n\n",
                );
            }
        }
        _ => {}
    }
}

fn emit_receive_signal(window: tauri::Window, signal: &str, message: &str) {
    window
        .emit(signal, Some(message.to_string()))
        .unwrap_or_else(|_| panic!("Failed to send {signal} message"));
}
fn emit_send_signal(window: tauri::Window, signal: &str, message: EmitInfo) {
    window
        .emit(signal, Some(message))
        .unwrap_or_else(|_| panic!("Failed to send {signal} message"));
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmitInfo {
    croc_code: String,
    info: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmitProgress {
    croc_code: String,
    files: Vec<FileItem>,
}
// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct EmitStatus {
//     croc_code: String,
//     status: String,
// }

#[tauri::command]
async fn send_files(
    window: tauri::Window,
    mut files: Vec<FileItem>, // 需要发送的文件列表或目录
    code: String,             // 传输代码Code
    is_folder: bool,          // 是否为目录
) -> Result<(), String> {
    let mut croc_args = vec![
        "--yes".to_string(),
        "send".to_string(),
        "--transfers".to_string(),
        "8".to_string(),
    ];
    // 构建 croc 命令参数
    // croc_args.push("--yes".to_string());
    // croc_args.push("send".to_string());
    // croc_args.push("--transfers".to_string());
    // croc_args.push("8".to_string()); // 设置并发传输数为

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
    if is_folder {
        files = insert_files_after_dir(files);
    }
    // 启动 croc 进程
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
            // for line in reader.lines() {
            //     match line {
            //         Ok(line) => {
            //             println!("croc output: {}", line);
            //         }
            //         Err(err) => eprintln!("Error reading line: {}", err),
            //     }
            // }
        }

        if let Some(stdout) = child.stdout.take() {
            let mut reader = io::BufReader::new(stdout);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        println!("croc output: {output}");
                        if let Some(code) = get_code(&output) {
                            // println!("Extracted code: {}", code);
                            println!("Output from [stdout].");
                            window.emit("croc-code", Some(code.to_string())).unwrap();
                        }
                        if output.contains("Code is:") {
                            // 传输完成，强制将所有文件状态更新为100%
                            replace_hash_percent(&mut files);
                            window
                                .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();
                            window
                                .emit("croc-send-file-ready", Some(EmitInfo{croc_code:code_str.clone(),info: "文件已准备好，请把Code给对方以开始接收。\nFiles ready,provide the Code to recipient to receive.".to_string()}))
                                .unwrap();

                        }
                        if let Some(status) = get_status(&output) {
                            // println!("Extracted status: {}", status);
                            window
                                .emit("croc-send-file-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
                                .unwrap();
                        }
                        if let Some(progress_data) = get_progress_data(&output,"Sending") {
                            // println!("Extracted progress data: {:?}", progress_data);
                            let status_str = format!(
                                "{}: {} {} {}",
                                progress_data.progress_type,
                                progress_data.percentage,
                                progress_data.progress,
                                progress_data.time
                            );
                            update_file_status(&mut files, &progress_data.filename, &status_str);
                            // 发送更新后的文件状态列表到前端
                            window
                                .emit("croc-send-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                                .unwrap();
                        }
                        if output.contains("not enough open ports") {
                            window
                                .emit("croc-send-error", Some(EmitInfo{croc_code:code_str.clone(),info:"重复发送。\nRepeated sending.".to_string()}))
                                .unwrap();

                        }
                        // window
                        //     .emit("croc-output", Some(output.to_string()))
                        //     .unwrap();
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}");
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
            // window
            //     .emit(
            //         "croc-send -error",
            //         Some(EmitInfo{croc_code:code_str.clone().to_string(),info: format!("Croc command failed with status: {status}")}),
            //     )
            //     .unwrap();
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
async fn receive_files(
    window: tauri::Window,
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
    let mut croc_args: Vec<String> = vec![];
    // 构建 croc 命令参数
    croc_args.push("--yes".to_string());
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
    println!("Running croc with args: {croc_args:?}");
    let code2 = code.clone();
    let code_str = code2.clone();
    let mut files: Vec<FileItem> = vec![];
    tokio::task::spawn_blocking(move || {

        #[cfg(not(windows))]
        let mut child: Child = Command::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2) // 设置环境变量
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command");

        #[cfg(windows)]
        let mut child = Command::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        // 处理 croc 输出

        if let Some(stderr) = child.stderr.take() {
            let mut reader = io::BufReader::new(stderr);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        println!("croc output: {output}");

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
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}");
                        break;
                    }
                }
            }
        }
        if let Some(stdout) = child.stdout.take() {
            let mut reader = io::BufReader::new(stdout);
            let mut buffer = [0u8; 4096];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break, // EOF
                    Ok(n) => {
                        let output = String::from_utf8_lossy(&buffer[..n]);
                        println!("croc output: {output}");

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
                    }
                    Err(err) => {
                        eprintln!("Error reading stdout: {err}");
                        break;
                    }
                }
            }
        }
        let status = child.wait().expect("Command wasn't running");

        if status.success() {
            // 传输完成，强制将所有文件状态更新为100%
            replace_completed_percent(&mut files);
            window.emit("croc-receive-file-progress", Some(EmitProgress{croc_code:code_str.clone(),files: files.clone()}))
                .unwrap();

            window
                .emit("croc-receive-file-success", Some(EmitInfo{croc_code:code_str.clone(),info: "所有文件已成功接收\nAll files received successfully".to_string()}))
                .unwrap();
        } else {
            emit_exit_info(window.clone(), "receive", code_str.clone(), status.code().unwrap()); 
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
    let mut croc_args: Vec<String> = vec![];
    // 构建 croc 命令参数
    croc_args.push("--yes".to_string());
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
        if let Some(stdout) = child.stdout.take() {
            let mut reader = io::BufReader::new(stdout);
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
                                .emit("croc-send-error", Some(EmitInfo{croc_code:code_str.clone(),info: "太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()}))
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
            // 传输完成，强制将所有文件状态更新为100%
            window
                .emit("croc-send-text-success", Some(EmitInfo{croc_code:code_str.clone().to_string(),info: msg}))
                .unwrap();
        } else {
            emit_exit_info(window.clone(), "send", code_str.clone(), status.code().unwrap()); 
            // window
            //     .emit(
            //         "croc-send-error",
            //         Some(EmitInfo{croc_code:code_str.clone().to_string(),info: format!("Croc command failed with status: {status}")}),
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
async fn receive_text(
    window: tauri::Window,
    code: String, // 传输代码Code
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
    let mut croc_args: Vec<String> = vec![];
    // 构建 croc 命令参数
    croc_args.push("--yes".to_string());
    if OS == "windows" {
        croc_args.push(code.clone())
    }

    println!("Running croc with args: {croc_args:?}");
    let code2 = code.clone();
    tokio::task::spawn_blocking(move || {
        #[cfg(not(windows))]
        let mut child: Child = Command::new("croc")
                .args(croc_args)
                .env("CROC_SECRET", code2.clone()) // 设置环境变量
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .expect("Failed to start croc command");

        #[cfg(windows)]
        let mut child = Command::new("croc")
            .args(croc_args)
            // windows下需要设置不显示命令行窗口
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to start croc command");

        let code_str = code2.trim().to_string();
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
                        full_output+=output.as_str();
                    }
                    Err(err) => {
                        eprintln!("Error reading stderr: {err}");
                        break;
                    }
                }
            }
        }
        // if let Some(stdout) = child.stdout.take() {
        //     let mut reader = io::BufReader::new(stdout);
        //     let mut buffer = [0u8; 4096];
        //     loop {
        //         match reader.read(&mut buffer) {
        //             Ok(0) => break, // EOF
        //             Ok(n) => {
        //                 let output = String::from_utf8_lossy(&buffer[..n]).to_string();
        //                 println!("croc output: {output}");
        //
        //
        //                 if let Some(status) = get_status(&output) {
        //                     // println!("Extracted status: {}", status);
        //                     window
        //                         .emit("croc-receive-text-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
        //                         .unwrap();
        //                 }
        //                 if let Some(msg) = get_text_msg(&output) {
        //                     // println!("Extracted msg: {}", msg);
        //                     window
        //                         .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: msg}))
        //                         .unwrap();
        //                 }
        //
        //                 if output.contains("not enough open ports") {
        //                     window
        //                         .emit("croc-receive-error", Some("太多发送进程未接收，通道池已满，关闭程序以清理。\nToo many sending process have not been received,close this program to kill.".to_string()))
        //                         .unwrap();
        //                     // return "Repeated sending.".to_string();
        //                 }
        //                 if output.contains("(secure channel) not ready") {
        //                     window
        //                         .emit("croc-receive-error", Some("无内容可接收或Code冲突,等会儿再试或换个Code重试。\nNo content to receive or Code conflict,waiting or change the Code.".to_string()))
        //                         .unwrap();
        //                 }
        //                 full_output+=output.as_str();
        //             }
        //             Err(err) => {
        //                 eprintln!("Error reading stderr: {err}");
        //                 break;
        //             }
        //         }
        //     }
        // }

        let mut stdout = child.stdout.take().unwrap();
        let mut stdout_buf = Vec::new();
        stdout.read_to_end(&mut stdout_buf).unwrap();
        let stdout_str = String::from_utf8_lossy(&stdout_buf);

        // if let Some(msg) = get_text_msg(&full_output) {
        //     // println!("Extracted msg: {}", msg);
        //     window
        //         .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: msg}))
        //         .unwrap();
        // }
        // if let Some(status) = get_status(&full_output) {
        //     // println!("Extracted status: {}", status);
        //     window
        //         .emit("croc-send-text-status", Some(EmitInfo{croc_code:code_str.clone(),info: status.to_string()}))
        //         .unwrap();
        // }
        let status = child.wait().expect("Command wasn't running");

        if status.success() {
            // 传输成功，则消息正文就是stdout_str
            window
                .emit("croc-receive-text-msg", Some(EmitInfo{croc_code:code_str.clone(),info: stdout_str.to_string()}))
                .unwrap();
            // window
            //     .emit("croc-receive-text-success", Some(EmitInfo{croc_code:code_str.clone(),info: "receive success".to_string()}))
            //     .unwrap();
        } else {
            emit_exit_info(window.clone(), "receive", code_str.clone(), status.code().unwrap()); 
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

#[derive(Debug, Deserialize, Serialize)]
struct GiteeRelease {
    tag_name: String,
    body: String,
}
#[tauri::command]
async fn check_update() -> Result<GiteeRelease, String> {
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

// use tauri::App;
// use tauri::RunEvent;
// use tauri::AppHandle;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(Arc::new(Mutex::new(CrocWorker::default())))
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
            stop_chat_listener
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
