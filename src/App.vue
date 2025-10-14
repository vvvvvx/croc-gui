<script setup lang="ts">
import { onMounted,onBeforeUnmount,ref,nextTick,watch,computed} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { listen,UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from '@tauri-apps/api/app';
//import { ElMessageBox } from "element-plus";
import { darkAlert } from "./utils/dialog";
//import { Command } from "@tauri-apps/plugin-shell";
//import path from "@tauri-apps/api/path";
//import fileIcon from "./assets/file.svg";
//import folderIcon from "./assets/folder.svg";


interface fileItem {
  file: string;
  status: string;
  is_dir: boolean;
}
interface emitInfo  {
  croc_code:string,
  info:string
}
interface emitProgress{
  croc_code:string,
  files:fileItem[]
}
interface txtMsg{
  croc_code:string,
  type: string,// To or From
  timestamp: string,
  msg:string
}
// 文件传输类
interface fileProcess{
  croc_code:string,
  type: string,// TextChat or FileSend or FileReceive
  memo:string, // mark the code to who
  status:string, // transfer result
  files:fileItem[],
}
// 文本发送类
interface chatProcess{
  croc_code:string,
  type:string, //TextChat or FileSend or FileReceive
  memo:string, // mark the code to who
  isListening:boolean, // if true ,must wait the back msg first.
  isChatEstablished:boolean,
  msgList:txtMsg[],
}
interface Process {
  croc_code :string,
  type:string,
  memo:string,
  status:string
}

const curVersion=ref(''); //当前版本号
const latestVersion=ref(''); //最新版本号
const latestVersionDesc=ref(''); //最新版本描述
const versionText=computed(()=>{return ((curVersion.value.toLowerCase() < latestVersion.value.toLowerCase()) && latestVersion.value!='') ?  `<a href="https://gitee.com/vvvvvx/croc-gui/releases" target="_blank" style="text-decoration:none;color:green;">有新版本/New version available.</a>`:`<a href="https://gitee.com/vvvvvx/croc-gui/releases" target="_blank" style="text-decoration:none;color:white;">Version: ${curVersion.value}</a>` ;}); //版本号显示文本
const versionTitle=computed(()=>{return ((curVersion.value.toLowerCase() < latestVersion.value.toLowerCase()) && latestVersion.value!='') ? `当前版本：${curVersion.value}  最新版本：${latestVersion.value} \n\n新版本更新：\n${latestVersionDesc.value}`:"点击我查看版本更新信息。"}); //版本号鼠标悬停提示

const typeSend=ref("FileSend");
const typeReceive=ref("FileReceive");
const typeTextChat=ref("TextChat");
const isFolder = ref(false); // File or Folder mode
const hasWarned = ref(false);// custom Code warning,just once
//const isFileTransfer= ref(true); //FileTransfer or TextChat
const crocCode = ref<string>(""); // Croc code for transfer
const memo = ref<string>("");// Code memo
const transferType = ref<string>("FileSend");// FileSend | FileReceive | TextChat
const waitingCodesList = ref<string[]>([]); //Croc codes which are waiting for receiving
const savePath=ref<string>(""); // Application directory path
const isSending=ref<boolean>(false); // Whether currently sending files
const inputText=ref<string>(""); // text to send
const fileProcessList = ref<fileProcess[]>([]); // for multi sending
const chatProcessList = ref<chatProcess[]>([]); // for multi chatting
const chatArea=ref<HTMLTextAreaElement | null>(null); //聊天窗口TextArea
const tempPaths = ref<fileItem[]>([]); //在发送文件时，在生成code之前，临时保存文件列表。
const dropdownCodesListOpen = ref(false);
const wrapperRef = ref<HTMLElement | null>(null);
const Tab = (window as any).bootstrap?.Tab;
//const isAskingMemo = ref<boolean>(false); // 是否在等待用户输入memo,防止在progress事件中重复弹窗。
//const fileSendCount = computed (()=>{
//  fileProcessList.value.filter(fp => fp.type== "FileSend").length
//});

const sendStatus= computed (()=>{   // current send status
  const fp=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == typeSend.value
  );
  return fp ? fp.status: "";
});
const receiveStatus= computed (()=>{   // current receive status
  const fp=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == typeReceive.value
  );
  return fp ? fp.status: "";
});
const fileReceiveCount = computed (()=>{
  return fileProcessList.value.filter(fp => fp.type== typeReceive.value).length;
});

const fileSendCount = computed (()=>{
  return fileProcessList.value.filter(fp => fp.type== typeSend.value).length;
});

const textChatCount = computed (()=>{
  return chatProcessList.value.length;
});

const chatText= computed<string>(()=> { // chatting records text
  return chatGetRecords(crocCode.value); 
});
const codesList = computed<Process[]>(()=> { //下拉框显示内容
  // 先把 fileProcessList 转换成 Process
  const fileProcesses: Process[] = fileProcessList.value.map(fp => ({
    croc_code: fp.croc_code,
    type: fp.type,  // TextChat / FileSend / FileReceive
    memo: fp.memo,
    status:fp.status
  }));

  // 再把 chatProcessList 转换成 Process
  const chatProcesses: Process[] = chatProcessList.value.map(cp => ({
    croc_code: cp.croc_code,
    type: cp.type,  // TextChat / FileSend / FileReceive
    memo: cp.memo,
    status:""
  }));

  // 合并数组
  return [...fileProcesses, ...chatProcesses];

}); 
// Selected file or folder paths to send
const sendPaths= computed<fileItem[]>(()=> {
  console.log("fileProcessList:",fileProcessList);
  const progress=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == typeSend.value
  );
  console.log("sendPaths:",progress ? progress.files : tempPaths.value);
  console.log("tempPaths:",tempPaths.value);
  return progress ? progress.files : tempPaths.value;
});

const receivePaths= computed<fileItem[]>(() => {
  const progress=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == typeReceive.value
  );
  //如果找不到进程记录，就显示临时内容。
  return progress ? progress.files : [];

}); // received file or folder paths 


let listenSendError: UnlistenFn | null = null;
let listenReceiveError: UnlistenFn | null = null;
let listenSendProgress: UnlistenFn | null = null;
let listenStatus: UnlistenFn | null = null;
let listenCode: UnlistenFn | null = null;
let listenReady: UnlistenFn | null = null;
let listenSendFileDone: UnlistenFn | null = null;
let listenReceiveFileProgress: UnlistenFn | null = null;
//let listenReceiveFileDone: UnlistenFn | null = null;
let listenSendFileSuccess: UnlistenFn | null = null;
let listenReceiveFileSuccess: UnlistenFn | null = null;
let listenReceiveStatus: UnlistenFn | null = null;
let listenSendTextSuccess: UnlistenFn | null = null;
let listenSendTextStatus: UnlistenFn | null = null;
let listenSendTextCode: UnlistenFn | null = null;
let listenReceiveTextMsg :UnlistenFn | null = null;
let listenReceiveTextStatus :UnlistenFn | null = null;

async function selectFile() {
  //tempPaths.value=[];
  const selected = await open({
    multiple: true,
    directory: isFolder.value,
    title: isFolder.value ? "选择目录/Select Folders":"选择文件/Select files",
  });
  if (Array.isArray(selected)) {
    //sendPaths.value = selected.map((path) => ({ file: path, status: "待发送/Pending" ,is_dir: isFolder.value }));
    tempPaths.value = selected.map((path) => ({ file: path, status: "待发送/Pending" ,is_dir: isFolder.value }));
  } else if (typeof selected === "string") {
    //sendPaths.value = [{ file: selected, status: "待发送/Pending" ,is_dir: isFolder.value }];
    tempPaths.value = [{ file: selected, status: "待发送/Pending" ,is_dir: isFolder.value }];
  } else {
    //sendPaths.value = [];
    tempPaths.value = [];
  }
  if(tempPaths.value.length!==0){
    newProcess();
  }
  //console.log(sendPaths);
  console.log("selectFile(),tempPaths:",tempPaths.value);
}


function selectCode(li: HTMLElement) {

  
  crocCode.value = li.dataset.code as string;
  memo.value=li.dataset.memo as string;
  transferType.value=li.dataset.type as string;

  console.log("li-dataset:", li.dataset.code,li.dataset.memo,li.dataset.type);
  dropdownCodesListOpen.value = false; 
  
  //处理List 高亮
  document.querySelectorAll(".active-row").forEach(el => el.classList.remove("active-row"));
  li.classList.add("active-row");

  // 一级 tab
  const mainTabId = (transferType.value === 'TextChat') ? 'chat-tab' : 'file-tab';
  const mainTabEl = document.getElementById(mainTabId);
  if (mainTabEl) {
    const tab = new Tab(mainTabEl);
    tab.show(); // 触发 Bootstrap 内部切换
  }

  // 二级 tab（FileSend / FileReceive）
  if (transferType.value === 'FileSend' || transferType.value === 'FileReceive') {
    const subTabId = (transferType.value === 'FileSend') ? 'send-file-tab' : 'receive-file-tab';
    const subTabEl = document.getElementById(subTabId);
    if (subTabEl) {
      const subTab = new Tab(subTabEl);
      subTab.show();
    }
  }
}

function openDropdown() {
  dropdownCodesListOpen.value = true;
}

//function closeDropdown() {
//  dropdownCodesListOpen.value = false;
//}

// 点击外部关闭下拉
const clickOutsideHandler = (event: MouseEvent) => {
  const target = event.target as HTMLElement;
  if (wrapperRef.value && !wrapperRef.value.contains(target)) {
    dropdownCodesListOpen.value = false;
  }
}
/*
function onInput(event:Event) {
  const el = (event.target as HTMLInputElement);
  if(el.id=="inputCode"){
    if (/[a-zA-Z0-9]/.test(el.value) && !hasWarned.value){
      darkAlert("不建议自定义Code,发送失败率太高。\n建议清空Code或点击”New“按钮开启新任务，由程序自动生成Code。\n\n如是接收任务，直接粘帖Code即可。\n\n");
      hasWarned.value=true;
    }
  }
}
*/
function onKeydown(event: KeyboardEvent) {

  if (event.ctrlKey || event.metaKey) return;
  // 检查是否是字母或数字键（A-Z / 0-9）
  if (/^[a-zA-Z0-9]$/.test(event.key)) {
    if (!hasWarned.value) {
      darkAlert(
        "⚠️ 不建议自定义Code，发送失败率太高。\n\n" +
        "点击『New』按钮或清空Code后，直接开始文件或文本发送即可，\nCode会在发送后自动生成。\n\n" +
        "【 如是接收任务，直接粘贴Code 】\n\n"
      );
      hasWarned.value = true;
    }
  }
}

function newProcess(){
  crocCode.value="";
  memo.value="";
  hasWarned.value=false;
}
function toggleFileMode() {
  isFolder.value = false;
} 
function toggleFolderMode() {
  isFolder.value = true;
}
function onClickFileSend(){
  if (transferType.value!==typeSend.value) newProcess();
  transferType.value=typeSend.value;
}
function onClickFileReceive(){
  if (transferType.value!==typeReceive.value) newProcess();
  transferType.value=typeReceive.value;
}
function onClickTextChat(){
  if (transferType.value!==typeTextChat.value) newProcess();

  transferType.value=typeTextChat.value;
}
function isWaiting(code:string):boolean{
  return waitingCodesList.value.includes(code);
}
function deleteCode(code:string){
  const index=waitingCodesList.value.indexOf(code);
  if (index!==-1) {
    waitingCodesList.value.splice(index,1);
  }
  console.log(waitingCodesList); 
}

function getTime():string{
  return new Date().toLocaleString();
  
}

// message funciton --->

// 生成完整消息记录

function chatGetRecords(code:string):string {

  if(!code || ! chatProcessList) return "";

  const exist= chatProcessList.value.find( c => c.croc_code == code);

  const msgs =Array.isArray(exist?.msgList) ? exist!.msgList : [];

  //let text = "";
  if (!exist || !exist.msgList){
    return "";
  }
// 构造文本（额外使用可选链/默认值防止 record 本身异常）
  return msgs.map(r => {
    const t = r?.type ?? "";
    const ts = r?.timestamp ?? "";
    const msg = r?.msg ?? "";
    return `[ ${t} ] ${ts}\n${msg}\n\n`;
  }).join("");

}
// 添加消息
function msgAdd(code:string,msg:string,fromOrTo:string){
  //查找会话
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  const newMsg: txtMsg ={
      croc_code:code,
      type: fromOrTo,// To or From
      timestamp: getTime(),
      msg:msg
  };
  if (exist){
    exist.msgList.push( newMsg );
  }else{
    darkAlert("错误：记录聊天消息时，找不到对应聊天会话。\n\n");
  }
}
// Get the listening status of this chatProcess
function msgListeningStatusGet(code:string):boolean {
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(exist){
    return exist.isListening;
  }
  return false; 
}

// Set the listening status of this chatProcess
function msgListeningStatusSet(code:string,status:boolean) {
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(exist){
    exist.isListening=status;
  }
}
function msgChatEstablishedStatusGet(code:string,status:boolean) {
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(exist){
    return exist.isChatEstablished;
  }
}
function msgChatEstablishedStatusSet(code:string,status:boolean) {
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(exist){
    exist.isChatEstablished=status;
  }
}
// 更新最后一条发送信息是否已收到。
function msgUpdateLastMsgStatus(code:string){
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(!exist){
    darkAlert("错误：更新聊天记录状态时，找不到对应聊天会话\n\n");
    return;
  }
  const msg=exist.msgList.findLast(c => c.croc_code==code && c.type=="To");
  if (!msg){
    darkAlert("错误：更新聊天记录状态时，找不到对应聊天消息\n\n");
    return;
  }
  msg.msg += " (Received)";
}
// 添加聊天任务到列表
function msgAddProcess(code:string,memo:string){
  const exist= chatProcessList.value.find( c => c.croc_code == code && c.type==typeTextChat.value);
  if(!exist){
    chatProcessList.value.push({
      croc_code:code,
      memo:memo,
      type:typeTextChat.value,
      isListening:false,
      isChatEstablished:false,
      msgList:[]
    })
  }
}

function msgProcessIsInList(code:string):boolean{
  const exist= chatProcessList.value.some( c => c.croc_code == code );
  return exist;
}
// <-------message funciton 

// file transfer function ------->
function fileTransProcessUpdate(code:string,type:string,files:fileItem[]){
  
  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
  if(!exist){
    darkAlert("错误：更新文件传输态时，找不到对应进程信息\n\n");
    return;
  }
  exist.files=files;
}
function fileTransStatusUpdate(code:string,type:string,transStatus:string){
  
  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
  if(!exist){
    darkAlert("错误：更新文件传输态时，找不到对应进程信息\n\n");
    return;
  }
  exist.status=transStatus;
}
//function fileTranStatusGet(code:string,type:string):string{

//  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
//  if (exist) return exist.status;
//  return "";
//}
function fileTransProcessAdd(code:string,type:string,memo:string,in_files:fileItem[]){
  
  console.log("fileTransProcessAdd,files:",in_files);
  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
  console.log("fileTransProcessAdd,exist:",exist);

  if(!exist){
    const copyFiles=Array.isArray(in_files) ?  [...in_files] : [];
    console.log("in if(!exist)");
    fileProcessList.value.push({
      croc_code:code,
      type:type,
      memo:memo,
      status:"Pending",
      files:copyFiles
    });
  }

  console.log("fileTransProcessAdd,fileProcessList:",fileProcessList.value);
}
function fileTransIsInList(code:string,type:string):boolean{
  const exist= fileProcessList.value.some( c => c.croc_code == code && c.type==type);
  return exist;
}
// <--------file transfer function

async function selectSaveFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    title: "选择保存目录/Select Save Folder",
  });
  if (typeof selected === "string") {
    savePath.value = selected;
  }
  console.log(selected);
}
async function sendFiles() {
  if (sendPaths.value.length === 0) {
    darkAlert("请先选择要发送的文件或目录。\nPlease select files or folders to send first.\n\n");
    return;
  }
  if (crocCode.value.trim()!=="" && isWaiting(crocCode.value)) {
    darkAlert("上一次发送等待对方接收完成。\nThe previous sending is waiting for receiving.\n\n");
    return;
  }
  //isSending.value = true;
  if (crocCode.value.trim()!==""){
    waitingCodesList.value.push(crocCode.value.trim());
  }
  
  // Implement file sending logic here
  await invoke("send_files", { files: sendPaths.value, code: crocCode.value,isFolder: isFolder.value });
  
}
async function receiveFiles() {
  if (!crocCode.value || crocCode.value.trim() === "") {
    darkAlert("请输入发送方生成的Code。\nPlease enter a Code from sender.\n\n");
    return;
  }
  if (!savePath.value || savePath.value.trim() === "") {
    darkAlert("请选择保存目录。\nPlease select a save folder.\n\n");
    return;
  }
  // Implement file receiving logic here
  await invoke("receive_files", { savePath: savePath.value, code: crocCode.value });

  console.log("Save to:", savePath.value, "Code:", crocCode.value);
}
async function sendText() {
  if(inputText.value.trim()==="")
  {
    darkAlert("请输入发送文本。\nEnter text first.\n\n");
    return;
  }
  if(crocCode.value.trim()!=="" && isWaiting(crocCode.value.trim())) {
    darkAlert("Code: "+ crocCode.value+"\n\n最后一次发送的消息对方还未接收，等待接收完成。\nThe last sent msg has not been received.\nWaiting for the reception to complete.\n\n")
    return;
  }
  // 如果已发送，对方还未回复，程序处于监听状态，不能重复发送。
  if(msgListeningStatusGet(crocCode.value)){
    darkAlert("对方回复后才能发送下一条。\n Can not send before the last msg be replied.");
    return;
  }
  if(crocCode.value.trim()!=="" && !isWaiting(crocCode.value.trim())) {
    waitingCodesList.value.push(crocCode.value);
  }

  await invoke("send_text", { msg:inputText.value, code: crocCode.value });
}
async function receiveText() {
  if(crocCode.value.trim()===""){
    darkAlert("请输入Code。\nEnter Code first.\n\n");
    return;
  }
  await invoke("receive_text", {  code: crocCode.value });
}
onMounted(async () => {
  savePath.value = await homeDir();
  document.addEventListener("click", clickOutsideHandler);

  listenSendError = await listen("croc-send-error", (event) => {
    const message = event.payload as emitInfo;
    darkAlert("Code: "+message.croc_code+"\n\n"+message.info+"\n\n");
    //isSending.value = false;
    if(isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }
    console.error("croc send error:", message);
  });
  listenReceiveError = await listen("croc-receive-error", (event) => {
    const message = event.payload as string;
    //darkAlert("Code: "+message.croc_/code+"\n\n"+message.info);
    darkAlert(message+"\n\n");
    console.error("croc receive error:", message);
  });
  listenCode = await listen("croc-code",async (event) => {
    const code = event.payload as string;
    crocCode.value = code;
    memo.value="";
    if(!isWaiting(code)){
      waitingCodesList.value.push(code);
    }
      // 会话是否已在列表中
      if (!fileTransIsInList(code,typeSend.value)){
        const input_memo=  "Send-"+(fileSendCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
        fileTransProcessAdd(code,typeSend.value,input_memo,tempPaths.value);
        console.log("listen croc-code,tempPaths:",tempPaths.value);
        tempPaths.value=[];
        memo.value=input_memo;
        //darkAlert(memo);
      }
    console.log("listen croc-code ,fileProcessList:", fileProcessList.value);
    console.log("Received croc code:", crocCode.value);
  });

  listenSendProgress = await listen("croc-send-file-progress", (event) => {
    const progress = event.payload as emitProgress; 
    
    if (fileTransIsInList(progress.croc_code,typeSend.value)){
      fileTransProcessUpdate(progress.croc_code,typeSend.value,progress.files);
    } else {
      tempPaths.value=progress.files;

      console.log("错误：文件发送进度更新，未找到对应进程会话,tempPaths:",tempPaths.value);
    }
    // 是当前传输进程才更新，否则会混乱。

    console.log("Send progress update:", progress);
    console.log("fileProcessList:",fileProcessList.value);
  });
  listenStatus = await listen("croc-send-file-status", (event) => {
    const st = event.payload as emitInfo;

    fileTransStatusUpdate(st.croc_code,typeSend.value,st.info);
    console.log("Send file status update:", st);

  });
  listenReady = await listen("croc-send-file-ready", (event) => {
    const message = event.payload as emitInfo;
    darkAlert("Code: "+message.croc_code+"\n\n"+message.info+"\n\n");
    if (!isWaiting(message.croc_code)){
      waitingCodesList.value.push(message.croc_code)
    }
    console.log("Croc ready:", message);
  });
  listenSendFileDone = await listen("croc-send-file-done", (event) => {
    const message = event.payload as emitInfo;
    //isSending.value = false;
    if (message.croc_code.trim().length===0){
      darkAlert("发送完成，无Code\n\n")
    } else{
      if( isWaiting(message.croc_code)){
        deleteCode(message.croc_code);
      }
    }

    fileTransStatusUpdate(message.croc_code,typeSend.value,"All sent");
    //sendStatus.value="All sent"
    console.log("Croc send done:", message);
  });
  listenSendFileSuccess = await listen("croc-send-file-success", (event) => {
    const message = event.payload as emitInfo;
    darkAlert("Code:"+message.croc_code+"\n\n"+message.info+"\n\n\n");
    //is Sending？
    if(isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }

    fileTransStatusUpdate(message.croc_code,typeSend.value,"All sent");
    console.log("Croc send success:", message);
  });
  listenReceiveFileProgress = await listen("croc-receive-file-progress",async (event) => {
    //receivePaths.value = event.payload as fileItem[]; 
    const pr=event.payload as emitProgress;

    if (pr.croc_code.trim().length===0){
      darkAlert("错误：更新Receive file进度时，Code为空。");
      return;
    }
      // 会话是否已在列表中
    if (fileTransIsInList(pr.croc_code,typeReceive.value)){
        fileTransProcessUpdate(pr.croc_code,typeReceive.value,pr.files);
        //darkAlert(memo);
    }
    else{
          // not the first, mark a memo.
          //const input_memo= await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          const input_memo= "Receive-"+(fileReceiveCount.value as number + 1); // await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          fileTransProcessAdd(pr.croc_code,typeReceive.value,input_memo,pr.files);
          memo.value=input_memo;
    }
    console.log("Receive progress update:", pr.files);
  });

  /*
  listenReceiveFileDone = await listen("croc-receive-file-done", (event) => {
    const message = event.payload as string;
    console.log("Croc receive done:", message);
    receiveStatus.value="Received all"
  });
*/
  listenReceiveFileSuccess = await listen("croc-receive-file-success", (event) => {
    const msg= event.payload as emitInfo;
    //receiveStatus.value="Received all"

    fileTransStatusUpdate(msg.croc_code,typeReceive.value,"Received all");
    darkAlert(msg.info+"\n\n");
    console.log("Croc receive success:", fileProcessList.value);
  });

  listenReceiveStatus = await listen("croc-receive-file-status", async(event) => {
    const st= event.payload as emitInfo;
    //receiveStatus.value=st.info;
    if (st.croc_code.trim().length===0){
      darkAlert("错误：更新Receive file状态时，Code为空。");
      return;
    }
    if (!fileTransIsInList(st.croc_code,typeReceive.value)){
          const input_memo= "Receive-"+(fileReceiveCount.value as number + 1); // await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          fileTransProcessAdd(st.croc_code,typeReceive.value,input_memo,[]);
          memo.value=input_memo;
    }
    fileTransStatusUpdate(st.croc_code,typeReceive.value,st.info);
    console.log("Overall receive status update:", st);
  });

  listenSendTextCode = await listen("croc-send-text-code", async(event) => {
    const code = event.payload as string;
    crocCode.value = code.trim();

    // 如果此code没有在等待接收，则置为在等待接收状态。
    if(!isWaiting(code.trim())){
      waitingCodesList.value.push(code.trim());
    }

    
      // 会话是否已在列表中
      if (!msgProcessIsInList(code)){
        const input_memo= "Chat-"+(textChatCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
        msgAddProcess(code,input_memo);
        memo.value=input_memo;
        //darkAlert(memo);
      }
    // record the msg
    msgAdd(code,inputText.value,"To");

    inputText.value="";
    //darkAlert(memo);
    console.log("Received croc code:", crocCode.value);
  });
  listenSendTextSuccess = await listen("croc-send-text-success", (event) => {
    const message = event.payload as emitInfo;
    // latest msg add "(Received)"
    msgUpdateLastMsgStatus(message.croc_code);
    if (isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }
    // set listening status to true
    msgListeningStatusSet(message.croc_code,true);
    msgChatEstablishedStatusSet(message.croc_code,true);
    // if other side received the msg, continuely listen the coming msg.
    invoke("start_chat_listener",{code: message.croc_code} );
    console.log("Croc receive success:", message);
  });
  listenSendTextStatus = await listen("croc-send-text-status",(event)=>{
    const message = event.payload as emitInfo;
    console.log(message);
  });
  listenReceiveTextStatus = await listen("croc-receive-text-status",(event)=>{
    const message = event.payload as emitInfo;
    console.log(message);
  });
  listenReceiveTextMsg = await listen("croc-receive-text-msg",async(event)=>{
    const msg = event.payload as emitInfo;
    //darkAlert(message.info);
      // 会话是否已在列表中
      if (!msgProcessIsInList(msg.croc_code)){
        const input_memo=  "Chat-"+(textChatCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
        msgAddProcess(msg.croc_code,input_memo);
        memo.value=input_memo;
      } 
    // record the msg
    msgAdd(msg.croc_code,msg.info,"From");
    // set isListenning status to false
    msgListeningStatusSet(msg.croc_code,false);

    console.log(msg);
  });

  interface versionInfo {
    tag_name:string,
    body:string
  }
  try {
      curVersion.value = await getVersion();
      curVersion.value="v"+curVersion.value;

      let latest = await invoke('check_update') as versionInfo;
      latestVersion.value = latest.tag_name;
      latestVersionDesc.value = latest.body;

      console.log(latest);
  } catch (e) {
      console.log(e);
  }

});

watch(chatText,async ()=>{
  await nextTick(); //等待TextArea渲染
  if (chatText.value && chatArea.value){
    chatArea.value.scrollTop=chatArea.value.scrollHeight;
  }
});
watch(memo,(newMemo)=> {
  if(transferType.value===typeTextChat.value){

    const item= chatProcessList.value.find( p => p.croc_code=== crocCode.value );
    if(item){
      item.memo=newMemo;
    }
  } else {
    const item= fileProcessList.value.find( p => p.croc_code=== crocCode.value && p.type===transferType.value );
    if(item){
      item.memo=newMemo;
    }
  }
});

onBeforeUnmount(() => {
  listenSendError?.();
  listenReceiveError?.();
  listenSendProgress?.();
  listenStatus?.();
  listenCode?.();
  listenReady?.();
  listenSendFileDone?.();
  listenSendFileSuccess?.();
  listenReceiveFileProgress?.();
//  listenReceiveFileDone?.();
  listenReceiveFileSuccess?.();
  listenReceiveStatus?.();
  listenSendTextSuccess?.();
  listenSendTextStatus?.();
  listenSendTextCode?.();
  listenReceiveTextMsg?.();
  listenReceiveTextStatus?.();
  document.removeEventListener("click", clickOutsideHandler);
});

</script>

<template>
  <main class="container-fluid p-4 pt-2" style="z-index:1000; " >
    <div class="row mb-0 align-items-end" >
      <!-- nav tabs -->
      <div class="col-5 mb-0" style="margin-bottom:0px;padding-bottom:0px;" >
        <ul class="nav nav-tabs mb-0 " id="topTab" role="tablist">
          <li class="nav-item" role="presentation">
            <button class="nav-link active"  style="margin-left:10px;"  id="file-tab" data-bs-toggle="tab" data-bs-target="#file-pane" type="button" role="tab">
              文件传输<br>File Transfer
            </button>
          </li>
          <li class="nav-item" role="presentation">
            <button class="nav-link" @click="onClickTextChat" id="chat-tab" data-bs-toggle="tab" data-bs-target="#chat-pane" type="button" role="tab">
              文本聊天<br>Text Chat
            </button>
          </li>
        </ul>
      </div>
      <div class="col-7 mb-2 justify-content-end" ref="wrapperRef" style="margin-bottom:0px;padding-bottom:0px; position:relative">
        <div class="input-group mb-0 mt-0" style="float:right;">
          <span class="input-group-text text-white bg-secondary" id="basic-addon2">Code</span>

          <input type="text" @keydown="onKeydown" id="inputCode" ref="inputCodeRef" class="form-control " v-model="crocCode" @focus="openDropdown" 
          title="发送时，可输入自定义或留空自动生成传输代码&#10;接收时，输入对方的传输代码&#10;连续或来回传输时，可保持Code不变
When sending,enter custom Code or leave it blank to generate Code automatically.
When receiving,enter the Code provided by other side.&#10;When transmitting continuously or back and forth,the Code can be kept unchanged." 
          placeholder="">
          <input type="text" v-model="memo"  class="form-control flex-shrink-0" style = "width:80px;flex:0 0 100px;text-align:center;" title="Code别名，用于多任务切换时便于记忆。&#10;可修改！&#10;Remarks of Code,remember conveniently when multi-task switch.&#10;Modifyable!" 
          placeholder="别名可改">
          <button type="button" @click="newProcess" class="btn input-group btn-success btn-outline-warning flex-shrink-0" style = "width:60px;text-align:center; padding-left:0px;padding-right:0px;"
          title="点击开始新任务,之前的任务会后台继续运行。&#10;点击Code输入框可切换任务。&#10;Start a new transfer,the transfers before will still running.&#10;Click input box of Code can switch the tasks.">New</button>
        </div>
        <!--
        <ul v-show="dropdownCodesListOpen" ref="dropdownRef" class="list-group position-absolute w-100" style="z-index: 9001; top: 100%; left: 0;">
          <li v-for="code in codesList" :key="code.croc_code" @click="selectCode($event.currentTarget as HTMLElement)" :data-type="code.type" :data-code="code.croc_code" :data-memo="code.memo" :data-status="code.status" class="list-group-item list-group-item-action bg-secondary text-white" style="cursor: pointer;">
            [ {{ code.type }} ] [ {{  code.croc_code }} ] [ {{ code.memo }} ] 
          </li>
        </ul>
        -->
        <ul v-show="dropdownCodesListOpen"
            ref="dropdownRef"
            class="list-group position-absolute w-100"
            style="z-index: 9001; top: 100%; left: 0;">

          <!-- 表头 -->
          <div class="list-group-item bg-secondary text-white"
              style="display: grid; grid-template-columns: 90px 300px 100px; font-weight: bold; cursor: default;">
            <div>Type</div>
            <div>Code</div>
            <div>Memo</div>
          </div>

          <!-- 表格行（每行可点） -->
          <div v-for="code in codesList"
              :key="code.croc_code"
              @click="selectCode($event.currentTarget as HTMLElement)"
              :data-type="code.type"
              :data-code="code.croc_code"
              :data-memo="code.memo"
               class="list-group-item list-group-item-action bg-secondary text-white" :class="{'active-row': code.type===transferType && code.croc_code===crocCode }"
              style="display: grid; grid-template-columns: 90px 300px 100px; cursor: pointer;">
            <div>{{ code.type }}</div>
            <div>{{ code.croc_code }}</div>
            <div>{{ code.memo }}</div>
          </div>

        </ul>
      </div>
    </div>
    <!-- tab content -->
    <div class="tab-content pt-0" style="border:1px solid #ddd;" id="myTabContent">
      <div class="tab-pane fade pt-3 show active "  id="file-pane" role="tabpanel" aria-labelledby="file-tab">
        <!-- File Transfer content -->
        <ul class="nav nav-tabs mb-0" style="margin-left:10px;" id="secondTab" role="tablist">
          <li class="nav-item" role="presentation">
            <button class="nav-link active" @click="onClickFileSend" id="send-file-tab" data-bs-toggle="tab" data-bs-target="#send-file-pane" type="button" role="tab">
              发送/Send
            </button>
          </li>
          <li class="nav-item" role="presentation">
            <button class="nav-link" @click="onClickFileReceive" id="receive-file-tab" data-bs-toggle="tab" data-bs-target="#receive-file-pane" type="button" role="tab">
              接收/Receive
            </button>
          </li>
        </ul>
        <!-- Second tab content -->
        <div class="tab-content pt-0" style="height:calc(99vh - 120px);border-top:1px solid #ddd;" id="fileTabContent">
          <!-- Send File content -->
          <div class="tab-pane fade show active"  id="send-file-pane" style="padding:1rem;" role="tabpanel" aria-labelledby="send-file-tab">
            <div class="fixed-pane-container mb-0">
              <div class="header-area mb-0 pb-0">
                <div class="row">
                  <div class="col-2 mb-3">
                    <div class="input-group mb-0 mt-0 ">
                      <button class="btn" @click="toggleFileMode" :class="isFolder ? 'btn-secondary' : 'btn-success btn-outline-warning' " style="width:34px; padding-left:0px;padding-right:0px;" title="切换为发送文件/Toggle to send files">
                        <img src="/assets/file.svg"  width="24" height="24" alt="Icon">
                      </button>
                      <button class="btn" @click="toggleFolderMode" :class="isFolder ? 'btn-success btn-outline-warning' : 'btn-secondary' " style="width:34px; padding-left:0px; padding-right:0px;" title="切换为发送目录/Toggle to send folders">
                        <img src="/assets/folder.svg"  width="24" height="24" alt="Icon">
                      </button>
                    </div>
                  </div>

                  <div class="col-7 mb-0">
                    <button class="btn btn-success" @click="selectFile" title="点击选择要发送的文件或目录。&#10;Click to select files or folders to send.">{{ isFolder ? '选择目录/Select Folders' : '选择文件/Select Files' }}</button>
                  </div>
                  <div class="col-3 mb-0 justify-content-end" style="display:flex; ">
                    <span title="请先选择要发送的文件或目录，然后点击发送。&#10;接收完成前不能继续发送。&#10;Please select files or folders first,then click to send.&#10;Cannot send again before current transfer is done.">
                      <button class="btn btn-warning" @click="sendFiles"  :disabled="!sendPaths || sendPaths.length === 0 || isSending">发送/Send</button>
                    </span>
                  </div>
                  <div class="col-12 mb-0 mt-0" v-show="sendPaths.length>0">
                    <span>状态/Status &nbsp;: &nbsp;</span>
                    <span style="color:lightgreen;" v-show="sendPaths.length>0">{{  sendStatus   }}</span>
                  </div>
                </div>
              </div> <!-- header-area row end -->
              <div class="scrollable-area">
                <div class="row">
                  <div class="col-12">
                    <div class="content scrollable flex-height" id="sendTable" v-if="sendPaths && sendPaths.length > 0">
                      <table class="table  table-sm text-white " v-show=" sendPaths">
                        <thead style="  background-color: #333;">
                          <tr>
                            <th class="text-start fs-6 " style="width: 40px;">No.</th>
                            <th class="text-start fs-6 " title=""> 状态/Status</th>
                            <th class="text-start fs-6 " title=""> 文件/File </th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr v-for="(path ,index) in sendPaths" :class="{ 'directory-row':path.is_dir }" :key="path.file">
                            <td class="text-center fs-6 " style="width: 20px;" title="">{{ index+1 }}</td>
                            <td class="text-start fs-6 "  > {{ path.status }} </td>
                            <td class="text-start fs-6 "  :title="path.file" > {{ path.file }} </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>
                </div>
              </div> <!-- scrollable area end -->
            </div> <!-- fixed pane container end -->
          </div>
          <!-- Receive File content -->
          <div class="tab-pane fade "  id="receive-file-pane" style="padding:1rem;" role="tabpanel" aria-labelledby="receive-file-tab">
            <div class="fixed-pane-container">
              <div class="header-area pb-0">
                <div class="row">
                  <div class="col-9 mb-3">
                    <div class="input-group mb-0 mt-0" >
                      <span class="input-group-text text-white bg-secondary" id="">保存到/SaveTo</span>
                      <input type="text" class="form-control" v-model="savePath" title="接收文件的保存位置/Where to save the files." >
                      <button class="btn btn-success" style="width:34px; padding-left:0px; padding-right:0px;" @click="selectSaveFolder" title="点击选择保存目录。&#10;Click to select save folder.">
                        <img src="/assets/folder.svg"  width="24" height="24" alt="Icon">
                      </button>
                    </div>
                  </div>
                  <div class="col-3 mb-2">
                    <span style="float:right;" title="输入对方Code后，点击接收文件。&#10;After entering the Code from other side,click to receive files.">
                      <button class="btn btn-warning" @click="receiveFiles">接收/Receive</button>
                    </span>
                  </div>
                  <div class="col-12 mb-0 mt-0 " v-show="receivePaths.length>0" style="margin-top:0px;">
                    <span>状态/Status &nbsp;: &nbsp;</span>
                    <span style="color:lightgreen;" >{{  receiveStatus   }}</span>
                  </div>
                </div>
              </div>
              <div class="scrollable-area mt-0" >
                <div class="row">
                  <div class="col-12">
                    <div class="content scrollable flex-height" id="receiveTable" v-if="receivePaths && receivePaths.length > 0">
                      <table class="table  table-sm text-white mt-0" v-show=" receivePaths">
                        <thead style="  background-color: #333;">
                          <tr>
                            <th class="text-start fs-6 " style="width: 40px;">No.</th>
                            <th class="text-start fs-6 " title=""> 状态/Status</th>
                            <th class="text-start fs-6 " title=""> 文件/File </th>
                          </tr>
                        </thead>
                        <tbody>
                          <tr v-for="(path ,index) in receivePaths" :class="{ 'directory-row':path.is_dir }" :key="path.file">
                            <td class="text-center fs-6 " style="width: 20px;" title="">{{ index+1 }}</td>
                            <td class="text-start fs-6 "  > {{ path.status }} </td>
                            <td class="text-start fs-6 "  :title="path.file" > {{ path.file }} </td>
                          </tr>
                        </tbody>
                      </table>
                    </div>
                  </div>
                </div>
              </div> <!-- scrollable area end -->
            </div> <!-- fixed pane container end -->
          </div> <!-- Receive File content end -->
        </div>
      </div>

      <div class="tab-pane fade p-0 " id="chat-pane"   role="tabpanel" aria-labelledby="chat-tab">
        <!-- Text Chat content -->
        <div class="fixed-pane-container p-0 m-0">
          <div class="header-area p-3 " >
            <div class="row">
              <div class="col-9">
                <textarea class="form-control pl-0 " v-model="inputText" style="border::1px solid #ddd;height:83px; resize:none;" placeholder="在这里输入消息/Enter message here" >
                </textarea>
              </div>
              <div class="col-3 mb-0 justify-content-end" style="display:flex; ">
                <div class="row justify-content-end">
                  <div class="col-12 mb-2">
                    <span class="m-0 p-0 flex " style="float:right;" 
                      title="发送文字后把Code告知对方以接收。&#10;接收完成前不能继续发送。&#10;After sending the message,inform the recipient of the Code so they can receive it.&#10;Cannot send again until the recipient has finished receiving it.">
                      <button class="btn btn-warning" @click="sendText" style="width:115px;" :disabled="!inputText || inputText.trim().length === 0 || isSending">发送/Send</button>
                    </span>
                  </div>

                  <div class="col-12">
                    <span  style="float:right;" title="输入对方Code后，点击按钮接收信息。&#10;After entering the Code from sender,click to receive the message.">
                      <button class="btn btn-warning" @click="receiveText" >接收/Receive</button>
                    </span>
                  </div>
                </div>
              </div>
            </div>
          </div>
          <div class="text-scrollable-area  mt-0" >
            <div class="row">
              <div class="col-12 " >
                <textarea ref="chatArea" class="form-control pl-0 bg-dark text-white " v-model="chatText" style="height:calc(99vh - 200px);border:none; border-top:1px solid #ddd; resize:none;" readonly>
                </textarea>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

    <div class="  text-white">
        <div class="row justify-content-end">
            <div class="col-auto">
            <span title="点我访问软件主页，可提Bug或建议。"> 
              <a href="https://gitee.com/vvvvvx/croc-gui" style="color:inherit;text-decoration:none;" target="_blank">Developed by Viaco.</a>&emsp; Email : 106324221@qq.com&emsp;
            </span>
            <span :class="[(curVersion < latestVersion && latestVersion!='') ?  'blink':'']" v-html="versionText" :title="versionTitle">
            </span>
        </div>
        </div>
    </div>
      <!-- Bootstrap JS (with Popper) -->
  </main>
</template>

<style scoped>
    .list-group-item.active-row {
      background-color: #198754 !important; /* Bootstrap success 绿色 */
      color: white !important;
    }
    /* 美化 tab 样式 */
    .nav-tabs  {
      border-bottom: none;
      margin-bottom: 1rem;
    }
    .nav-tabs .nav-link  {
      position: relative;
      clip-path: polygon(0% 0%, 90% 0%, 100% 90%, 100% 100%, 10% 100%, 0% 10%);
      box-shadow: 0 2px 6px rgba(0,0,0,0.2);

      border: none;
      border-radius: 2px;
      margin-right: 0.5rem;
      padding: 0.2rem 1.2rem;
      background-color:#6c757d; /*#f1f3f5;*/
      color: white;/*#495057;*/
      font-weight: 500;
      transition: all 0.2s ease;
    }
    .nav-tabs .nav-link:hover  {
      transform: translateY(-2px);

      background-color: #e9ecef;
      color: #212529;
    }
    .nav-tabs .nav-link.active{
      background-color: #198754;
      color: #fff;
      box-shadow: 0 2px 6px rgba(13, 110, 253, 0.3);
      flex-shrink: 0;
    }
    html,body {
      height: 100%;
      margin: 0;
      overflow:hidden;
      min-width:1000px;
    }

    .tab-content {
      height:calc(100vh - 90px);
      overflow: hidden; /*auto;*/
      background:  #2c3137;
      border-radius: 10px;
     /* border: 1px solid #ddd;*/
      color: #fff;
     /* padding: 1rem;*/
      padding-top:1rem;
      padding-left:0px;
      padding-right:0px;
      padding-bottom:0px;
      box-shadow: 0 4px 12px rgba(0,0,0,0.05);

    }
    .tab-pane {
      height:100%;
      width:100%;
    }
    .directory-row {
      font-weight: bold;
      color: #ffc107; /*#198754; #0d6efd;*/
    }
    .fixed-pane-container{
      overflow-y:hidden;
      overflow-x:hidden;
      display:flex;
      flex-direction:column;
      height:100%;
    }
    .header-area{
      position:sticky;
      top:0;
      z-index:10;
      padding-bottom:1rem;
    }
    .scrollable-area{
      max-height:calc(99vh - 240px);
      overflow-y:auto;
      overflow-x:hidden;
    }
    .text-scrollable-area{
      max-height:calc(99vh - 140px);
      overflow-y:auto;
      overflow-x:hidden;
    }
    .table{
      width:100%;
      margin-bottom:0;
    }
    .table th,.table td {
      white-space: nowrap;  /*防止文本换行*/
      overflow: hidden;
      /*text-overflow: ellipsis; 超长文本省略号*/
    }
    .table thead {
      position:sticky;
      top:0;
      z-index:5;
      box-shadow:0 2px 4px rgba(0,0,0,0.2 );
      background-color:#333;
    }

    
        /*
.logo.vite:hover {
  filter: drop-shadow(0 0 2em #747bff);
}

.logo.vue:hover {
  filter: drop-shadow(0 0 2em #249b73);
}


:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  line-height: 24px;
  font-weight: 400;

  color: #0f0f0f;
  background-color: #f6f6f6;

  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

.container {
  margin: 0;
  padding-top: 10vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  text-align: center;
}

.logo {
  height: 6em;
  padding: 1.5em;
  will-change: filter;
  transition: 0.75s;
}

.logo.tauri:hover {
  filter: drop-shadow(0 0 2em #24c8db);
}

.row {
  display: flex;
  justify-content: center;
}

a {
  font-weight: 500;
  color: #646cff;
  text-decoration: inherit;
}

a:hover {
  color: #535bf2;
}

h1 {
  text-align: center;
}

input,
button {
  border-radius: 8px;
  border: 1px solid transparent;
  padding: 0.6em 1.2em;
  font-size: 1em;
  font-weight: 500;
  font-family: inherit;
  color: #0f0f0f;
  background-color: #ffffff;
  transition: border-color 0.25s;
  box-shadow: 0 2px 2px rgba(0, 0, 0, 0.2);
}

button {
  cursor: pointer;
}

button:hover {
  border-color: #396cd8;
}
button:active {
  border-color: #396cd8;
  background-color: #e8e8e8;
}

input,
button {
  outline: none;
}

#greet-input {
  margin-right: 5px;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  a:hover {
    color: #24c8db;
  }

  input,
  button {
    color: #ffffff;
    background-color: #0f0f0f98;
  }
  button:active {
    background-color: #0f0f0f69;
  }
}
*/
</style>
