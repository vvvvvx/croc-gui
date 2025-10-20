use crate::types::{FileItem, ProgressData};
use once_cell::sync::Lazy;
use regex::Regex;

static RE_CODE: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"Code is:\s*([0-9a-zA-Z]+-[0-9a-zA-Z]+(?:-[a-zA-Z0-9]+)*)")
        .expect("Invalid regex")
});

static RE_HASHING: Lazy<Regex> = Lazy::new(|| {
    //Regex::new(r"Hashing\s+([^\s%]+)\s*(\d+)%\s*\|([^|]+)\|\s*\((\d*\.?\d*)\s*MB/s\)\s*\[(\d+)(s|m|month|year):(\d+)\5\]")
    //(Hashing)   (文件名)   (百分比)  (进度条)               (速度)         (时间1 单位 : 时间2 单位)
    //Regex::new(r"(Hashing)\s+([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\(\d+\.?\d*\s+[a-zA-Z]+/s\))\s+(\[\d+\.?\d*[a-zA-Z]+:\d+\.?\d*[a-zA-Z]+\])")
    Regex::new(r"(Hashing)\s+([^\s%]+)\s+(\d+%)\s*\|([^|]*)\|\s*(\([\d\s\.A-Za-z]+/[a-zA-Z]\))\s+(\[[\da-zA-Z]+:[\da-zA-Z]+\])")
        .expect("Invalid regex for Hashing")
});

static RE_SEND_OR_RECEIVE: Lazy<Regex> = Lazy::new(|| {
    //Regex::new(r"([^\s%]+)\s*(\d+)%\s*\|([^|]+)\|\s*\((\d*\.?\d*)/(\d*\.?\d*)\s*MB,\s*(\d*\.?\d*)\s*MB/s\)\s*\[(\d+)(s|m|month|year):(\d+)\7\]")
    //(文件名)   (百分比)  (进度条)               (已发送/总大小    速度)         (时间1 单位 : 时间2 单位)
    Regex::new(r"([^\s%]+(\s+[^\s%]+)*)\s+(\d+%)\s+\|([^|]*)\|\s+(\([\d/\.\sA-Za-z]+/[\d/\.\sA-Za-z]+,[\d\s\.A-Za-z]+/[a-zA-Z]\))\s+(\[[\da-zA-Z]+:[\da-zA-Z]+\])")
        .expect("Invalid regex for Sending")
});
static RE_COMPLETED: Lazy<Regex> = Lazy::new(|| {
    //(文件名)   (百分比)  (进度条)               (已发送/总大小    速度)
    // Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\(\d+\.?\d*/\d+\.?\d*\s+[a-zA-Z]+,\s+\d+\.?\d*\s[a-zA-Z]+/s\))")
    //Regex::new(r"([^\s%]+)\s*(\d+%)\s*\|([^|]*)\|\s*(\([\d/\.\sA-Za-z]+,[\d/\.\sA-Za-z]+\))")
    Regex::new(r"([^\s%]+(\s+[^\s%]+)*)\s+(\d+%)\s*\|([^|]*)\|\s*(\([\d/\.\sA-Za-z]+/[\d/\.\sA-Za-z]+,[\d\s\.A-Za-z]+/[a-zA-Z]\))")
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
    Regex::new(r"(Zipping|Unzipping file|securing channel...|Connecting|connecting|Receiving\s+\(<\-\d+\.\d+\.\d+\.\d+:\d+\)|Sending\s+\(\->\d+\.\d+\.\d+\.\d+:\d+\))")
        .expect("Invalid regex for Status")
});
static RE_ZIP_FILENAME: Lazy<Regex> = Lazy::new(|| {
    //Regex::new(r"Sending\s'([^\s%]+\.zip)'\sand\s\d+\sfolders")
    Regex::new(r"Sending\s'([^\s%]+\.zip)'").expect("Invalid regex for zip filename")
});

static RE_PERCENT: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\d+%").expect("Invalid regex for Percent"));

pub fn get_code(text: &str) -> Option<String> {
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
pub fn get_zip_filename(text: &str) -> Option<String> {
    RE_ZIP_FILENAME
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
pub fn get_status(text: &str) -> Option<String> {
    RE_STATUS
        .captures(text)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
pub fn get_text_msg(text: &str) -> Option<String> {
    RE_RECEIVE_MSG
        .captures(text)
        .and_then(|caps| caps.get(2).map(|m| m.as_str().to_string()))
}
pub fn get_progress_data(text: &str, receive_or_send: &str) -> Option<ProgressData> {
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
    if let Some(caps) = RE_SEND_OR_RECEIVE.captures(text) {
        // println!("SendOrReceive:\ncaps[0]:{}\ncaps[1]:{}\ncaps[2]:{}\ncaps[3]:{}\ncaps[4]:{}\ncaps[5]:{}",
        //     caps[0].to_string(),caps[1].to_string(),caps[2].to_string(),caps[3].to_string(),caps[4].to_string(),caps[5].to_string());
        return Some(ProgressData {
            progress_type: receive_or_send.to_string(),
            filename: caps[1].to_string(),
            percentage: caps[3].to_string(),
            progress_bar: caps[4].to_string(),
            progress: caps[5].to_string(),
            time: caps[6].to_string(),
        });
    }
    // 尝试匹配Completed格式3
    if let Some(caps) = RE_COMPLETED.captures(text) {
        // println!(
        //     "Completed：\ncaps[0]:{}\ncaps[1]:{}\ncaps[2]:{}\ncaps[3]:{}\ncaps[4]:{}\ncaps[5]:{}",
        //     caps[0].to_string(),
        //     caps[1].to_string(),
        //     caps[2].to_string(),
        //     caps[3].to_string(),
        //     caps[4].to_string(),
        //     caps[5].to_string()
        // );
        return Some(ProgressData {
            progress_type: "Done".to_string(),
            filename: caps[1].to_string(),
            percentage: caps[3].to_string(),
            progress_bar: caps[4].to_string(),
            progress: caps[5].to_string(),
            time: "".to_string(),
        });
    }
    None
}

pub fn replace_hash_percent(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        //file.status = RE_PERCENT.replace_all(&file.status,"100%").to_string()
        file.status = "Hashed: 100%".to_string();
    }
}
pub fn replace_completed_percent(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        file.status = RE_PERCENT.replace_all(&file.status, "100%").to_string()
    }
}
pub fn update_folder_status_after_completed(files: &mut [FileItem]) {
    for file in files.iter_mut() {
        if file.is_dir {
            file.status = "Done: 100%".to_string();
        }
    }
}
