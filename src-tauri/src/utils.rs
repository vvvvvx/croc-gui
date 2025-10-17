use crate::types::*;
use std::fs::{self};
use tauri::Emitter;

pub fn is_dir(path: &str) -> bool {
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
pub fn insert_files_after_dir(files: Vec<FileItem>) -> Vec<FileItem> {
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
pub fn update_file_status(files: &mut [FileItem], filename: &str, status: &str) {
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
pub fn is_contains_file(files: &[FileItem], filename: &str) -> bool {
    let cleaned_filename = filename.trim_end_matches("...").trim_end_matches("�");
    for file in files.iter() {
        if file.file.contains(cleaned_filename) {
            return true;
        }
    }
    false
}

pub fn emit_exit_info(window: tauri::Window, oper_type: &str, code: String, status_code: i32) {
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
                    "网络连接错误，请重试。重试多次无效，请更换Code。\nNetwork error,retry. If trying again and not, change the Code. \n\n",
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
