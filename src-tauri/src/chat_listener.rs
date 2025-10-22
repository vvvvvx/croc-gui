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

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[tauri::command]
pub async fn start_chat_listener(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    window: tauri::Window,
    code: String,
) -> Result<(), String> {
    // 控制监听运行和退出。true时在running,false时退出监听循环。
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
                    //running.store(false, Ordering::SeqCst);
                }
            }
            // 每 10 秒再检测一次
            sleep(Duration::from_secs(5)).await;
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
