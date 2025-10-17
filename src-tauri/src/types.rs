use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitInfo {
    pub croc_code: String,
    pub info: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitProgress {
    pub croc_code: String,
    pub files: Vec<FileItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressData {
    pub progress_type: String, // 格式类型：Hashing/Sending/Done
    pub filename: String,      // 文件名
    pub percentage: String,    // 百分比
    pub progress_bar: String,  // 进度条
    pub progress: String,      // 进度
    pub time: String,          //已耗时：预估耗时
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileItem {
    pub file: String,   //文件路径
    pub status: String, //发送进度状态信息,由ProgressData组合而成的字符串
    pub is_dir: bool,   //是否为目录
}

#[derive(Default)]
pub struct CrocWorker {
    pub tasks: HashMap<String, Arc<AtomicBool>>, // code -> running flag
}
