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
    let state_clone = state.inner().clone();

    //提前释放锁
    drop(worker);
    let code_clone = code.clone();

    tokio::spawn(async move {
        //tokio::task::spawn_blocking(move || {
        // let last_code_parts = code_clone.split('-').last().unwrap();
        let last_code_parts = code_clone.split('-').next_back().unwrap();
        let f2 = &code_clone[0..2];
        println!("Code:[ {f2}..{last_code_parts} ] listener started");
        while running.load(Ordering::SeqCst) {
            // {
            //     let _permit = semaphore.clone().acquire_owned().await.unwrap();
            //     println!("⏳ [ {f2}..{last_code_parts} ] got semaphore, running croc...");

            // use chrono::Local;
            // use chrono::Timelike;
            // let sec = Local::now().time().second();
            // println!("Second : {sec}");
            //
            // // 每个10秒的前4秒:0123用于发送，第8、9秒用于接收消息，避免同时发送和接收端口冲突。
            // let d = sec % 10;
            // if d != 8 {
            //     sleep(Duration::from_secs(1)).await;
            //     continue;
            // }
            //
            // println!("Receiving message...");
            // 运行 croc receive 命令
            #[cfg(windows)]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .arg(&code_clone)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .env("LANG", "C.UTF-8")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run croc,maybe croc not found"); //

            #[cfg(not(windows))]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .env("CROC_SECRET", code_clone.clone()) // 设置环境变量
                .env("LANG", "C.UTF-8")
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

            println!("Listener stdout[{f2}..{last_code_parts}]:{stdout_str}");
            println!("Listener stderr[{f2}..{last_code_parts}]:{stderr_str}");

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
            // }
            // 每 5 秒再检测一次
            sleep(Duration::from_secs(7)).await;
        }

        println!("Code:[ {f2}..{last_code_parts} ] listener stopped");
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
/*
#[tauri::command]
pub async fn add_code_task(
    state: State<'_, Arc<tokio::sync::Mutex<CrocWorker>>>,
    code: String,
) -> Result<(), String> {
    let mut worker = state.lock().await;

    if worker.tasks.contains_key(&code) {
        println!("Code {code} already in task list");
        return Ok(());
    }

    worker
        .tasks
        .insert(code.clone(), Arc::new(AtomicBool::new(true)));
    println!("Added code task: {}", code);
    Ok(())
}

#[tauri::command]
pub async fn start_global_listener(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    window: tauri::Window,
) -> Result<(), String> {
    let worker = state.lock().await;

    // 已有全局监听在运行就不重复启动
    if worker.running.load(Ordering::SeqCst) {
        println!("Global listener already running");
        return Ok(());
    }

    worker.running.store(true, Ordering::SeqCst);
    let running = worker.running.clone();
    let state_clone = state.inner().clone();

    tokio::spawn(async move {
        println!("✅ Global croc listener started");

        while running.load(Ordering::SeqCst) {
            let codes: Vec<String> = {
                let worker = state_clone.lock().await;
                worker.tasks.keys().cloned().collect()
            };

            for code in codes {
                let should_run = {
                    let worker = state_clone.lock().await;
                    worker
                        .tasks
                        .get(&code)
                        .map(|f| f.load(Ordering::SeqCst))
                        .unwrap_or(false)
                };

                if !should_run {
                    continue;
                }

                let code_clone = code.clone();
                println!("🔍 Checking croc code: {}", code_clone);

                let last_code_parts = code_clone.split('-').last().unwrap();
                let f2 = &code_clone[0..2];

                // 调用 croc receive
                #[cfg(windows)]
                let mut child = tCommand::new("croc")
                    .arg("--yes")
                    .arg(&code_clone)
                    .creation_flags(0x08000000)
                    .env("LANG", "C.UTF-8")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to run croc,maybe croc not found"); //

                #[cfg(not(windows))]
                let mut child = tCommand::new("croc")
                    .arg("--yes")
                    .env("CROC_SECRET", code_clone.clone())
                    .env("LANG", "C.UTF-8")
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .expect("Failed to run croc,maybe croc not found"); //
                                                                        /*
                                                                        if let Ok(mut child) = child {
                                                                            let mut stdout = child.stdout.take().unwrap();
                                                                            let mut buf = Vec::new();
                                                                            let _ = stdout.read_to_end(&mut buf).await;
                                                                            let output = String::from_utf8_lossy(&buf);

                                                                            if !output.trim().is_empty() {
                                                                                println!("📨 Received from [{code_clone}]: {output}");

                                                                                let _ = window.emit(
                                                                                    "croc-receive-text-msg",
                                                                                    Some(EmitInfo {
                                                                                        croc_code: code_clone.clone(),
                                                                                        info: output.to_string(),
                                                                                    }),
                                                                                );
                                                                            }
                                                                        } else {
                                                                            println!("⚠️ Failed to spawn croc for code {code_clone}");
                                                                        }
                                                                        */

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

                println!("Listener stdout[{f2}..{last_code_parts}]:{stdout_str}");
                println!("Listener stderr[{f2}..{last_code_parts}]:{stderr_str}");

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
                // 每 2 秒再检测一次
                sleep(Duration::from_secs(5)).await; // 每个 code 间隔 2 秒
            }

            // 所有 code 检测完后再休息几秒
            sleep(Duration::from_secs(1)).await;
        }

        println!("🛑 Global croc listener stopped");
    });

    Ok(())
}

#[tauri::command]
pub async fn stop_global_listener(state: State<'_, Arc<Mutex<CrocWorker>>>) -> Result<(), String> {
    let worker = state.lock().await;
    worker.running.store(false, Ordering::SeqCst);
    println!("Global listener stopping...");
    Ok(())
}

#[tauri::command]
pub async fn stop_code_task(
    state: State<'_, Arc<Mutex<CrocWorker>>>,
    code: String,
) -> Result<(), String> {
    let mut worker = state.lock().await;

    if let Some(flag) = worker.tasks.remove(&code) {
        flag.store(false, Ordering::SeqCst);
        println!("🛑 Stopped code listener for: {}", code);
        Ok(())
    } else {
        Err(format!("Code task not found: {}", code))
    }
}
*/

/*
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
        let last_code_parts = code_clone.split('-').last().unwrap();
        let f2 = &code_clone[0..2];
        println!("Code:[ {f2}..{last_code_parts} ] listener started");

        while running.load(Ordering::SeqCst) {
            // 运行 croc receive 命令
            #[cfg(windows)]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .arg(&code_clone)
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .env("LANG", "C.UTF-8")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .expect("Failed to run croc,maybe croc not found"); //

            #[cfg(not(windows))]
            let mut child = tCommand::new("croc")
                .arg("--yes")
                .env("CROC_SECRET", code_clone.clone()) // 设置环境变量
                .env("LANG", "C.UTF-8")
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

            println!("Listener stdout[{f2}..{last_code_parts}]:{stdout_str}");
            println!("Listener stderr[{f2}..{last_code_parts}]:{stderr_str}");

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
            sleep(Duration::from_secs(7)).await;
        }

        println!("Code:[ {f2}..{last_code_parts} ] listener stopped");
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
*/
