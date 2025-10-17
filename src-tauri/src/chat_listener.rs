use crate::types::{CrocWorker, EmitInfo};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::Emitter;
use tauri::State;
use tokio::io::AsyncReadExt;
use tokio::{
    process::Command as tCommand,
    sync::Mutex,
    time::{sleep, Duration},
};

// #[derive(Debug, Clone, Serialize, Deserialize)]
// struct EmitInfo {
//     croc_code: String,
//     info: String,
// }

#[tauri::command]
pub async fn start_chat_listener(
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
pub async fn stop_chat_listener(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    code: String,
) -> Result<(), String> {
    let worker = state.lock().await;
    if let Some(flag) = worker.tasks.get(&code) {
        flag.store(false, Ordering::SeqCst);
    }
    Ok(())
}
