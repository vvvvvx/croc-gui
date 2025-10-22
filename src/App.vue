<script setup lang="ts">
import { onMounted,onBeforeUnmount,ref,nextTick,watch,computed} from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { downloadDir } from "@tauri-apps/api/path";
import { listen,UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from '@tauri-apps/api/app';
//import { ElMessageBox } from "element-plus";
import { darkAlert,darkConfirmRemember } from "./utils/dialog";
import * as sets from "./utils/setting";
//import { loadConfig,saveConfig } from "./utils/configManager";
//import { AppConfig } from "./config";
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
  newArrival:boolean, //if there are new files arrive ,for notification
  savePath:String, // this Process save to where.
  isFolder:boolean,// is Send folder
  last:boolean,// if the last process before jump to other tab. display the last process when jump back.
  zip:boolean, // zip status
  exclude:string, // exclude status
  files:fileItem[],
}
// 文本发送类
interface chatProcess{
  croc_code:string, //main code
  type:string, //TextChat or FileSend or FileReceive
  clientRole:string // Sender | Receiver
  memo:string, // mark the code to who
  isListening:boolean, // if true ,must wait the back msg first.
  isChatEstablished:boolean,
  newArrival: boolean, // if there are new messages,for notification
  last:boolean,// if the last process before jump to other tab. display the last process when jump back.
  msgList:txtMsg[],
}
interface Process { //for droplist
  croc_code :string,
  type:string,
  memo:string,
  status:string,
  newArrival:boolean, // if there are new message or files ，for  notification
  last: boolean // if the last process before jump to other tab. display the last process when jump back.
}


enum Type {
  FileSend = "FileSend",
  FileReceive="FileReceive",
  TextChat="TextChat",
  FileTrans="FileTrans"
}
enum Role{
  Sender = "Sender",
  Receiver = "Receiver"
}
const curVersion=ref(''); //当前版本号
const latestVersion=ref(''); //最新版本号
const latestVersionDesc=ref(''); //最新版本描述
const versionText=computed(()=>{return ((curVersion.value.toLowerCase() < latestVersion.value.toLowerCase()) && latestVersion.value!='') ?  `<a href="https://gitee.com/vvvvvx/croc-gui/releases" target="_blank" style="text-decoration:none;color:green;">有新版本/New version available.</a>`:`<a href="https://gitee.com/vvvvvx/croc-gui/releases" target="_blank" style="text-decoration:none;color:white;">Version: ${curVersion.value}</a>` ;}); //版本号显示文本
const versionTitle=computed(()=>{return ((curVersion.value.toLowerCase() < latestVersion.value.toLowerCase()) && latestVersion.value!='') ? `当前版本：${curVersion.value}  最新版本：${latestVersion.value} \n\n新版本更新：\n${latestVersionDesc.value}`:"点击我查看版本更新信息。"}); //版本号鼠标悬停提示

let rememberChoice : boolean | null = null; //ConfirmBox choice remember

const isFolder = ref(false); // File or Folder mode
const hasWarned = ref(false);// custom Code warning,just once
const zip = ref(false);        // 发送前是否先打包压缩
const exclude = ref<string>("");     // 排除哪些文件，以逗号分隔
//const isFileTransfer= ref(true); //FileTransfer or TextChat
const crocCode = ref<string>(""); // Croc code for transfer
const memo = ref<string>("");// Code memo
const transferType = ref<string>(Type.FileSend as string);// FileSend | FileReceive | TextChat
const remFileTab = ref<string>(Type.FileSend as string); // when leave FileTab,remember FileSend or FileReceive;
const waitingCodesList = ref<string[]>([]); //Croc codes which are waiting for receiving
//const savePath=ref<string>(""); // the directory to save the received files
const savePathTmp=ref<string>(""); // the directory to save the received files
const isSending=ref<boolean>(false); // Whether currently sending files
const inputText=ref<string>(""); // text to send
const fileProcessList = ref<fileProcess[]>([]); // for multi sending
const chatProcessList = ref<chatProcess[]>([]); // for multi chatting
const chatArea=ref<HTMLTextAreaElement | null>(null); //聊天窗口TextArea
const sendPathsTmp = ref<fileItem[]>([]); //在发送文件时，在生成code之前，临时保存文件列表。
const dropdownCodesListOpen = ref(false);
const wrapperRef = ref<HTMLElement | null>(null);
const Tab = (window as any).bootstrap?.Tab;

const config =ref< sets.AppConfig >( {
        transfers: 8,
        //zip: false,
        //exclude: "",
        overwrite: false,
        multicast: "",
        ip: "",
        local: false,
        relay: "",
        relay6: "",
        relay_passwd: "",
        proxy_socks5: "",
        proxy_http: "",
      });

const sendStatus= computed (()=>{   // current send status
  const fp=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == Type.FileSend
  );
  return fp ? fp.status: "";
});
const receiveStatus= computed (()=>{   // current receive status
  const fp=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == Type.FileReceive
  );
  return fp ? fp.status: "";
});
const fileReceiveCount = computed (()=>{
  return fileProcessList.value.filter(fp => fp.type== Type.FileReceive).length;
});

const fileSendCount = computed (()=>{
  return fileProcessList.value.filter(fp => fp.type== Type.FileSend).length;
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
    status:fp.status,
    newArrival:fp.newArrival,
    last: fp.last
  }));

  // 再把 chatProcessList 转换成 Process
  const chatProcesses: Process[] = chatProcessList.value.map(cp => ({
    croc_code: cp.croc_code,
    type: cp.type,  // TextChat / FileSend / FileReceive
    memo: cp.memo,
    status:"",
    newArrival:cp.newArrival,
    last: cp.last
  }));

  // 合并数组
  return [...fileProcesses, ...chatProcesses];

}); 
// the directory to save the received files
const savePath= computed<string>(()=> {
  const fp=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == Type.FileReceive
  );
  console.log("savePath:",fp ? fp.savePath: savePathTmp.value);
  console.log("savePathTmp:",savePathTmp.value);
  const result=fp ? fp.savePath : savePathTmp.value;

  return String(result);
});

// Selected file or folder paths to send
const sendPaths= computed<fileItem[]>(()=> {
  console.log("fileProcessList:",fileProcessList);
  const progress=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == Type.FileSend
  );
  console.log("sendPaths:",progress ? progress.files : sendPathsTmp.value);
  console.log("sendPathsTmp:",sendPathsTmp.value);
  return progress ? progress.files : sendPathsTmp.value;
});

const receivePaths = computed<fileItem[]>(() => {
  const progress=fileProcessList.value.find(
    p => p.croc_code === crocCode.value && p.type == Type.FileReceive
  );
  //如果找不到进程记录，就显示临时内容。
  return progress ? progress.files : [];

}); // received file or folder paths 
const chatEstablished = computed<boolean>(()=>{

  if(crocCode.value.trim().length===0) return false;
  return msgChatEstablishedStatusGet(crocCode.value);
});
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
let listenReceiveFileDir: UnlistenFn | null = null;
let listenReceiveStatus: UnlistenFn | null = null;
let listenSendTextSuccess: UnlistenFn | null = null;
let listenSendTextStatus: UnlistenFn | null = null;
let listenSendTextCode: UnlistenFn | null = null;
let listenReceiveTextMsg :UnlistenFn | null = null;
let listenReceiveTextStatus :UnlistenFn | null = null;
let listenConfirm :UnlistenFn | null = null;

async function selectFile() {
  //sendPathsTmp.value=[];
  const selected = await open({
    multiple: true,
    directory: isFolder.value,
    title: isFolder.value ? "选择目录/Select Folders":"选择文件/Select files",
  });
  if (Array.isArray(selected)) {
    //sendPaths.value = selected.map((path) => ({ file: path, status: "待发送/Pending" ,is_dir: isFolder.value }));
    sendPathsTmp.value = selected.map((path) => ({ file: path, status: "待发送/Pending" ,is_dir: isFolder.value }));
  } else if (typeof selected === "string") {
    //sendPaths.value = [{ file: selected, status: "待发送/Pending" ,is_dir: isFolder.value }];
    sendPathsTmp.value = [{ file: selected, status: "待发送/Pending" ,is_dir: isFolder.value }];
  } else {
    //sendPaths.value = [];
    sendPathsTmp.value = [];
  }
  if(sendPathsTmp.value.length!==0){
    newProcess();
  }
  //console.log(sendPaths);
  console.log("selectFile(),sendPathsTmp:",sendPathsTmp.value);
}


function selectCode(li: HTMLElement) {
  // remember current Process first
  remCurProcess();

  // then change Process
  crocCode.value = li.dataset.code as string;
  memo.value=li.dataset.memo as string;
  transferType.value=li.dataset.type as string;
  
  if(transferType.value===Type.FileSend){
    const exist = fileProcessList.value.find(fp => fp.type===Type.FileSend && fp.croc_code === crocCode.value);
    if(exist){
      isFolder.value=exist.isFolder;
      zip.value=exist.zip;
      exclude.value=exist.exclude;
    }

  }
  
  console.log("li-dataset:", li.dataset.code,li.dataset.memo,li.dataset.type);
  dropdownCodesListOpen.value = false; 
 
  // set newArrival to false
  newArrivalStatusSet(crocCode.value,transferType.value,false);
  //处理List 高亮
  document.querySelectorAll(".active-row").forEach(el => el.classList.remove("active-row"));
  li.classList.add("active-row");
  
  switchTab();
}
// for transferType,jump to right tab
function switchTab(){

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
/*
function shoudListBlink(code:string):boolean {
  const exist= chatProcessList.value.find( c => c.croc_code == code && c.newArrival);
  if(exist){
    console.log("shoudListBlink:",exist.croc_code!==crocCode.value)
    return (exist.croc_code!==crocCode.value || transferType.value!==Type.TextChat)
  }
  return false;
}
*/
function shoudInputBlink():boolean {
  const exist = codesList.value.find( c => c.newArrival===true);
  if(exist) return true;
  return false;
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
function onSavePathInput(el:HTMLElement){
  savePathTmp.value=(el as HTMLInputElement).value ;
}

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
//function chatEstablished():boolean{
//  if(crocCode.value.trim()==="") return false;
//  return msgChatEstablishedStatusGet(crocCode.value);
//}
function newProcess(){
  crocCode.value="";
  memo.value="";
  hasWarned.value=false;
  zip.value=false;
  exclude.value="";
  //document.getElementById("inputCode")?.focus();
}
// switch to send files
function toggleFileMode() {
  isFolder.value = false;
  selectFile();
} 
// switch to send folders
function toggleFolderMode() {
  isFolder.value = true;
  selectFile();
}
function onClickFileTab(){
  // newProcess();
  // remember current Process first at this Tab.
  remCurProcess();
  transferType.value=remFileTab.value;
  //switchTab();
  // reload the last Process at new Tab
  reloadLastProcess(transferType.value);
  newArrivalStatusSet(crocCode.value,transferType.value,false);
  
}
function onClickFileSend(){
  // remember current Process first at this Tab.
  remCurProcess();
  transferType.value=Type.FileSend ;
  console.log("onClickFileTab transferType:",transferType.value);
  // reload the last Process at new Tab
  reloadLastProcess(transferType.value);
  newArrivalStatusSet(crocCode.value,transferType.value,false);
}
function onClickFileReceive(){
  // remember current Process first at this Tab.
  remCurProcess();
  transferType.value=Type.FileReceive ;
  // reload the last Process at new Tab
  reloadLastProcess(transferType.value);
  newArrivalStatusSet(crocCode.value,transferType.value,false);
}
function onClickTextChat(){
  console.log("onClickTextChat transferType:",transferType.value);
  // remember current Process first at this Tab.
  remCurProcess();
  transferType.value=Type.TextChat ;
  // reload the last Process at new Tab
  reloadLastProcess(transferType.value);
  newArrivalStatusSet(crocCode.value,transferType.value,false);
}
function isWaiting(code:string):boolean{
  return waitingCodesList.value.includes(code);
}
function isMainCode(code:string):boolean {
  const [code1st,]=getCodeParts(code);
  // s1111 Sender's Code,
  // r2222 Receiver's Code
  if(code1st ==="" || code1st==="s1111" || code1st==="r2222") return false;
  return true;
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

// Code:1234-goods-boooks-funny
// return [1234, goods-boooks-funny]
function getCodeParts(code:string):[string,string]{
  const parts= code.split("-");
  if(parts.length!==4) return ["",""];
  return [ parts[0],  parts[1]+"-"+parts[2]+"-"+parts[3] ];
}

function chatGetRecords(code:string):string {

  console.log("chatGetRecords code:",code);
  //alert("chatGetRecords code:"+code);
  if(!code || ! chatProcessList) return "";

  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));

  const msgs =Array.isArray(exist?.msgList) ? exist!.msgList : [];
  
  console.log("chatGetRecords msgs:",msgs);
  //alert("chatGetRecords msgs:"+msgs);

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
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));
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
/*
function msgListeningStatusGet(code:string):boolean {
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));
  if(exist){
    return exist.isListening;
  }
  return false; 
}
*/
// Set the listening status of this chatProcess
function msgListeningStatusSet(code:string,status:boolean) {
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));
  if(exist){
    exist.isListening=status;
  }
}
// Code:1234-goods-boooks-funny
// codePart: goods-boooks-funny
function msgGetMainCode(codePart:string):string{
  const parts=codePart.split("-");
  let code="----";
  if(parts.length===3) code=codePart;
  if(parts.length===4) code=parts[1]+"-"+parts[2]+"-"+parts[3];
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));

  if(exist) return exist.croc_code;
  return "";
}
function  msgGetRole(mainCode:string):string{
  const [,code3Last]=getCodeParts(mainCode);
  const exist= chatProcessList.value.find( c => c.croc_code.includes( code3Last));
  if(exist){
    return exist.clientRole;
  }
  return "";

}
function msgChatEstablishedStatusGet(code:string):boolean {
  if(code.trim().length===0) return false;
  const exist= chatProcessList.value.find( c => c.croc_code.includes( code));
  if(exist){
    return exist.isChatEstablished;
  }
  return false;
}
function msgChatEstablishedStatusSet(code:string,status:boolean) {
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code));
  if(exist){
    exist.isChatEstablished=status;
  }
}
/*
function msgHasNewMsgStatusGet(code:string):boolean {
  if(code.trim().length===0) return false;
  const exist= chatProcessList.value.find( c => c.croc_code == code);
  if(exist){
    return exist.hasNewMsg;
  }
  return false;
}
*/
function newArrivalStatusSet(code:string,type:string,status:boolean) {
  const exist = type===Type.TextChat ? chatProcessList.value.find( c => c.croc_code == code && c.type===type) : fileProcessList.value.find(c => c.croc_code===code && c.type===type);
  if(exist){
    exist.newArrival=status;
  }
}
// 更新最后一条发送信息是否已收到。
function msgUpdateLastMsgStatus(code:string){
  const exist= chatProcessList.value.find( c => c.croc_code.includes( code));
  if(!exist){
    darkAlert("错误：更新聊天记录状态时，找不到对应聊天会话\n\n");
    return;
  }
  const msg=exist.msgList.findLast(c => c.croc_code.includes( code) && c.type=="To");
  if (!msg){
    darkAlert("错误：更新聊天记录状态时，找不到对应聊天消息\n\n");
    return;
  }
  msg.msg += " (Received)";
}
// 添加聊天任务到列表
function msgAddProcess(code:string,memo:string,role:string){
  const exist= chatProcessList.value.find( c => c.croc_code.includes(code) && c.type==Type.TextChat);
  if(!exist){
    chatProcessList.value.push({
      croc_code:code,
      memo:memo,
      type:Type.TextChat,
      clientRole:role,
      isListening:false,
      isChatEstablished:false,
      newArrival:false,
      last:false,
      msgList:[]
    })
  } else if(exist.clientRole!==role){
     darkAlert("本程序不能接收本程序消息\n\nThis program cannot receive messages from itself.\n\n") 
  }

}

function msgProcessIsInList(code:string):boolean{
  const exist= chatProcessList.value.some( c => c.croc_code.includes(code) );
  return exist;
}
// <-------message funciton 

// file transfer function ------->
function fileTransProcessUpdate(code:string,type:string,files:fileItem[]){
  
  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
  if(!exist){
    //darkAlert("错误：更新文件传输态时，找不到对应进程信息\n\n");
    console.log("错误：更新文件传输态时，找不到对应进程信息");
    return;
  }
  exist.files=files;
}
function fileTransStatusUpdate(code:string,type:string,transStatus:string){
  
  const exist= fileProcessList.value.find( c => c.croc_code == code && c.type==type);
  if(!exist){
    //darkAlert("错误：更新文件传输态时，找不到对应进程信息\n\n");
    console.log("错误：更新文件传输态时，找不到对应进程信息");
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
    //const copyFiles=Array.isArray(in_files) ?  [...in_files] : [];
    fileProcessList.value.push({
      croc_code:code,
      type:type,
      memo:memo,
      status:"Pending",
      newArrival:false,
      isFolder:isFolder.value,
      last: false,
      savePath: type===Type.FileReceive ? savePath.value : "" ,
      zip:zip.value,
      exclude:exclude.value,
      files:in_files
    });
  }

  console.log("fileTransProcessAdd,fileProcessList:",fileProcessList.value);
}
function fileTransIsInList(code:string,type:string):boolean{
  const exist= fileProcessList.value.some( c => c.croc_code == code && c.type==type);
  return exist;
}
// remember current Process,when switch to this Tab,display it.
function remCurProcess(){
  const exist= transferType.value===Type.TextChat ? 
    chatProcessList.value.find( c => c.croc_code === crocCode.value ) : 
    fileProcessList.value.find( c => c.croc_code === crocCode.value && c.type===transferType.value);
  
  
  if(exist) {
    exist.last=true;
    // remember the FileTab
    if(transferType.value!==Type.TextChat) remFileTab.value=transferType.value;
    console.log("remCurProcess:",exist);
  } 
}
function reloadLastProcess(type:string){
  
  console.log("reloadLastProcess transferType:",type);
  console.log("reloadLastProcess codesList:",codesList.value);
  
  const exist= transferType.value===Type.TextChat ? chatProcessList.value.find( c => c.last === true) : fileProcessList.value.find( c => c.last=== true && c.type===transferType.value);
  //const exist= codesList.value.find( c => c.last===true && c.type===type);
  if(exist){
    transferType.value=exist.type;
    crocCode.value=exist.croc_code;
    memo.value=exist.memo;
    exist.last=false;
    if (transferType.value === Type.FileSend){
      zip.value=(exist as fileProcess).zip;
      exclude.value=(exist as fileProcess).exclude;
    }
    console.log("reloadLastProcess:",exist);
  }else{
    newProcess();
  }
}
// <--------file transfer function

async function selectSaveFolder() {
  const selected = await open({
    multiple: false,
    directory: true,
    title: "选择保存目录/Select Save Folder",
  });
  if (typeof selected === "string") {
    savePathTmp.value = selected;
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
  if(!sets.isEmpty(exclude.value) && !sets.isExclude(exclude.value)) {
    darkAlert("Exclude格式错误\nWrong [exclude] format\n\n");
    return;
  }
  //isSending.value = true;
  if (crocCode.value.trim()!==""){
    waitingCodesList.value.push(crocCode.value.trim());
  }
  // Implement file sending logic here
  await invoke("send_files", { files: sendPaths.value, code: crocCode.value,isFolder: isFolder.value,zip:zip.value,exclude:exclude.value });
  
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
  rememberChoice=null;
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
 // if( crocCode.value.trim().length>0 && msgListeningStatusGet(crocCode.value) ){
 //   darkAlert("对方回复后才能发送下一条。\n Can not send before the last msg be replied.");
 //   return;
 // }
  
  const role = msgGetRole(crocCode.value.trim())
  const [,code3Last] =getCodeParts(crocCode.value);
  // s1111 for Sender sending
  // r2222 for Receiver sending.
  const code= role=== Role.Sender ? "s1111-"+code3Last : "r2222-"+code3Last;

  if(crocCode.value.trim()!=="" && !isWaiting(crocCode.value.trim())) {
    //无role信息，则是第一次发起会话
    if(role===""){
      waitingCodesList.value.push(crocCode.value);
    }else{
    // 否则是回复已有会话
      if(!isWaiting(code)) {
        waitingCodesList.value.push(code);
      } else {
        darkAlert("对方收到消息后才可继续发送新消息\n\nWait until the previous message is received before sending a new one.\n\n")
        return;
      }
    }

  }
  if(role===""){
    await invoke("send_text", { msg:inputText.value, code: crocCode.value });
  }else{
    await invoke("send_text", { msg:inputText.value, code: code});
  }
}
async function receiveText() {
  if(crocCode.value.trim()===""){
    darkAlert("请输入Code。\nEnter Code first.\n\n");
    return;
  }
  rememberChoice=null;
  // savePath 用于处理误在聊天界面接收文件
  await invoke("receive_text", {  code: crocCode.value,savePath:savePath.value });
}
async function showSettingDlg(){
    const dlg=document.getElementById("settingDlg");
    if(dlg) dlg.style.display="block";
    //getById("closeRuningAlertBtn").focus();

}
function closeSettingDlg(){
    const dlg=document.getElementById("settingDlg");
    if(dlg) dlg.style.display="none";
};
async function onSaveConfig(){
  let ip=config.value.ip; 
  if( !sets.isEmpty(ip) && !sets.isProxy(ip)){
    darkAlert("Wrong [Local IP] format.\n\n");
    return;
  }

  let multicast=config.value.multicast; 
  if( !sets.isEmpty(multicast) && !sets.isMulticast(multicast)){
    darkAlert("Wrong [Multicast] format.\nValid range is: 224.0.0.0 - 239.255.255.255\n\n");
    return;
  }
  let transfers=config.value.transfers; 
  if( !sets.isEmpty(String(transfers)) && !sets.isNum(String(transfers))){
    darkAlert("Wrong [Transfers] format.\n\nNot the bigger the better, recommended 4-16\n");
    return;
  }
  let relay=config.value.relay; 
  if( !sets.isEmpty(relay) && !sets.isIPv4Port(relay)){
    darkAlert("Wrong [Relay-IPv4] format.\n\n");
    return;
  }
  let relay6=config.value.relay6; 
  if( !sets.isEmpty(relay6) && !sets.isIPv6Port(relay6)){
    darkAlert("Wrong [Relay-IPv6] format.\n\n");
    return;
  }
  let socks5=config.value.proxy_socks5; 
  if( !sets.isEmpty(socks5) && !sets.isProxy(socks5)){
    darkAlert("Wrong [socks5] format.\n\n");
    return;
  }
  let http=config.value.proxy_http; 
  if( !sets.isEmpty(http) && !sets.isProxy(http)){
    darkAlert("Wrong [http] format.\n\n");
    return;
  }
  try{
    await sets.saveConfig(config.value);
    darkAlert("Config saved.\n\n");
    closeSettingDlg();
  } catch(e){
    darkAlert("Failed to save config.\n\n"+e+"\n\n");
  } 
  
}

async function onReloadConfig() {
  config.value = await sets.loadConfig();
}
/*
*/
onMounted(async () => {
  //savePath.value = await homeDir();
  savePathTmp.value = await downloadDir();

  // Read config file
  config.value = await sets.loadConfig();
  console.log( config.value);

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

  listenCode = await listen("croc-code",(event) => {
    const code = event.payload as string;
    crocCode.value = code;
    memo.value="";
    if(!isWaiting(code)){
      waitingCodesList.value.push(code);
    }
      // 会话是否已在列表中
      if (!fileTransIsInList(code,Type.FileSend)){
        const input_memo=  "Send-"+(fileSendCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
        fileTransProcessAdd(code,Type.FileSend,input_memo,sendPathsTmp.value);
        console.log("listen croc-code,sendPathsTmp:",sendPathsTmp.value);
        sendPathsTmp.value=[];
        memo.value=input_memo;
        //darkAlert(memo);
      }
    console.log("listen croc-code ,fileProcessList:", fileProcessList.value);
    console.log("Received croc code:", crocCode.value);
  });

  listenSendProgress = await listen("croc-send-file-progress", (event) => {
    const progress = event.payload as emitProgress; 
    
    if (fileTransIsInList(progress.croc_code,Type.FileSend)){
      fileTransProcessUpdate(progress.croc_code,Type.FileSend,progress.files);
    } else {
      sendPathsTmp.value=progress.files;

      console.log("错误：文件发送进度更新，未找到对应进程会话,sendPathsTmp:",sendPathsTmp.value);
    }
    // 是当前传输进程才更新，否则会混乱。

    console.log("Send progress update:", progress);
    console.log("fileProcessList:",fileProcessList.value);
  });

  listenStatus = await listen("croc-send-file-status", (event) => {
    const st = event.payload as emitInfo;

    fileTransStatusUpdate(st.croc_code,Type.FileSend,st.info);
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

    fileTransStatusUpdate(message.croc_code,Type.FileSend,"All sent");
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

    fileTransStatusUpdate(message.croc_code,Type.FileSend,"All sent");
    console.log("Croc send success:", message);
  });

  listenReceiveFileProgress = await listen("croc-receive-file-progress",(event) => {
    //receivePaths.value = event.payload as fileItem[]; 
    const pr=event.payload as emitProgress;

    if (pr.croc_code.trim().length===0){
      darkAlert("错误：更新Receive file进度时，Code为空。");
      return;
    }
      // 会话是否已在列表中
    if (fileTransIsInList(pr.croc_code,Type.FileReceive)){
        fileTransProcessUpdate(pr.croc_code,Type.FileReceive,pr.files);
        //darkAlert(memo);
    }
    else{
          // not the first, mark a memo.
          //const input_memo= await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          const input_memo= "Receive-"+(fileReceiveCount.value as number + 1); // await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          fileTransProcessAdd(pr.croc_code,Type.FileReceive,input_memo,pr.files);
          memo.value=input_memo;
          // if receive files at TextChat tab
          if(transferType.value!==Type.FileReceive){
            newArrivalStatusSet(pr.croc_code,Type.FileReceive ,true);
            console.log("File newArrival:",true);
          }
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

    fileTransStatusUpdate(msg.croc_code,Type.FileReceive,"Received all");
    darkAlert(msg.info+"\n\n");
    console.log("Croc receive success:", fileProcessList.value);
  });

  listenReceiveFileDir= await listen("croc-receive-file-dir", (event) => {
    const msg= event.payload as emitInfo;
    //receiveStatus.value="Received all"

    darkAlert("Code; "+msg.croc_code+"\n\n"+ msg.info+"\n\n");

    console.log(msg.info );
  });
  listenReceiveStatus = await listen("croc-receive-file-status", (event) => {
    const st= event.payload as emitInfo;
    //receiveStatus.value=st.info;
    if (st.croc_code.trim().length===0){
      darkAlert("错误：更新Receive file状态时，Code为空。");
      return;
    }
     //如果在fileTab接收Text,会导致ProcessList错误添加。但如果无此代码，则文件接收状态不会更新。
    if (!fileTransIsInList(st.croc_code,Type.FileReceive)){
          const input_memo= "Receive-"+(fileReceiveCount.value as number + 1); // await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
          fileTransProcessAdd(st.croc_code,Type.FileReceive,input_memo,[]);
          memo.value=input_memo;
    }
    
    fileTransStatusUpdate(st.croc_code,Type.FileReceive,st.info);
    console.log("Overall receive status update:", st);
  });

  listenSendTextCode = await listen("croc-send-text-code", (event) => {
    const code = event.payload as string;

    //const [code1st,code3Last]=getCodeParts(code);

    // 如果此code没有在等待接收，则置为在等待接收状态。
    if(!isWaiting(code.trim())){
      waitingCodesList.value.push(code.trim());
    }
    let mainCode="";
    if (isMainCode(code)){
      crocCode.value = code.trim();
      mainCode=crocCode.value;
    }else{
      mainCode=msgGetMainCode(code)
    }
    // 会话是否已在列表中
    if (isMainCode(code) && !msgProcessIsInList(mainCode)){
      const input_memo= "Chat-"+(textChatCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
      msgAddProcess(code,input_memo,Role.Sender);
      memo.value=input_memo;
      //darkAlert(memo);
    }
    // record the msg
    msgAdd(mainCode,inputText.value,"To");

    inputText.value="";
    //darkAlert("Code: "+code+"\n\n聊天已就绪，把Code发给对方开始聊天。\n【Code已复制，直接粘贴】\n\nChat ready, provide the Code to recipient to chat.\n【Code copied to clipboard】");
    //darkAlert(memo);
    console.log("Received croc code:", crocCode.value);
  });

  listenSendTextSuccess = await listen("croc-send-text-success", (event) => {
    const message = event.payload as emitInfo;

    const mainCode=msgGetMainCode(message.croc_code);
    // latest msg add "(Received)"
    msgUpdateLastMsgStatus(mainCode);

    if (isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }
    // record the msg
    //msgAdd(message.croc_code,inputText.value,"To");

    //inputText.value="";
    // set listening status to true
    msgListeningStatusSet(mainCode,true);
    msgChatEstablishedStatusSet(mainCode,true);

    if(isMainCode(message.croc_code)){
      // if other side received the msg, continuely listen the coming msg.
      // 
      const [_,code3Last]=getCodeParts(mainCode);
      const role = msgGetRole(mainCode);
      // Sender listen Receiver's Code
      // Receiver listen Sender's Code
      // r2222 = Receiver's send code
      // s1111 = Sender's send code
      const code = role===Role.Sender ? "r2222-"+code3Last : "s1111-"+code3Last;
      invoke("start_chat_listener",{code: code} );
    }
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

  listenReceiveTextMsg = await listen("croc-receive-text-msg",(event)=>{
    const msg = event.payload as emitInfo;

    const mainCode=msgGetMainCode(msg.croc_code);
    
    //darkAlert(message.info);
    // 会话是否已在列表中
    if (isMainCode(msg.croc_code) && !msgProcessIsInList(msg.croc_code)){
      const input_memo=  "Chat-"+(textChatCount.value as number + 1); //await askUserInput("给新任务Code起个别名，以便区别查看多任务:");
      msgAddProcess(msg.croc_code,input_memo,Role.Receiver);
      memo.value=input_memo;
      // 如果是FileReceive界面接收的文本，则在fileProcessList中删除同Code项
      if(transferType.value===Type.FileReceive){
        const index=fileProcessList.value.findIndex(fp => fp.croc_code===msg.croc_code && fp.type===Type.FileReceive );
        if(index!==-1) fileProcessList.value.splice(index,1);
      }
      
      // 开始持续监听发送方消息
      // Sender listen Receiver's Code
      // Receiver listen Sender's Code
      // r2222 = Receiver's send code
      // s1111 = Sender's send code
      const [_,code3Last]=getCodeParts(msg.croc_code);
      invoke("start_chat_listener",{code: "s1111-"+code3Last} );
    } 
    // record the msg
    msgAdd(mainCode,msg.info,"From");
    // set isListenning status to false
    msgListeningStatusSet(mainCode,false);
    // set hasNewMsg status to true
    if(mainCode!==crocCode.value || transferType.value!==Type.TextChat){
      newArrivalStatusSet(mainCode,Type.TextChat ,true);
      console.log("newArrival:",true);
    }
    // 如果transferType不是‘TextChat’，则说明是user在FileReceive界面接收了Text.应该切换到TextChat界面
    //if (transferType!==Type.TextChat){
    //  transferType.value=Type.TextChat;
    //  switchTab();
    //}
    console.log(msg);
  });

  // stdin input
  listenConfirm = await listen("croc_confirm",async (event)=>{
    const msg = event.payload as emitInfo;

    if(rememberChoice!==null){
      await invoke("write_stdin",{
        code:msg.croc_code,
        input:rememberChoice ? "y" : 'n'});
      return;
    }

    const { answer,remember } =await  darkConfirmRemember("Code: "+msg.croc_code+"\n\n"+msg.info+"\n\n") ;
    if(remember){
      rememberChoice=answer;
    }
    console.log("answer:",answer,"remember:",remember);
    await invoke("write_stdin",{
      code:msg.croc_code,
      input:answer ? "y" : 'n'});

    console.log("Confirm info:",msg);
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
  if(transferType.value===Type.TextChat){

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
  listenReceiveFileDir?.();
  listenReceiveStatus?.();
  listenSendTextSuccess?.();
  listenSendTextStatus?.();
  listenSendTextCode?.();
  listenReceiveTextMsg?.();
  listenReceiveTextStatus?.();
  listenConfirm?.();
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
            <button class="nav-link active" @click="onClickFileTab" style="margin-left:10px;"  id="file-tab" data-bs-toggle="tab" data-bs-target="#file-pane" type="button" role="tab">
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

          <input type="text" class="form-control" :class="{ 'blink': shoudInputBlink() }" @keydown="onKeydown" id="inputCode" ref="inputCodeRef" v-model="crocCode" @focus="openDropdown" @click="openDropdown"
          title="发送时，可输入自定义或留空自动生成传输代码&#10;接收时，输入对方的传输代码&#10;连续或来回传输时，可保持Code不变
When sending,enter custom Code or leave it blank to generate Code automatically.
When receiving,enter the Code provided by other side.&#10;When transmitting continuously or back and forth,the Code can be kept unchanged." 
          placeholder="">
          <input type="text" v-model="memo"  class="form-control flex-shrink-0"  :class="{ 'blink': shoudInputBlink() }" style = "width:80px;flex:0 0 100px;text-align:center;" title="Code别名，用于多任务切换时便于记忆。&#10;可修改！&#10;Remarks of Code,remember conveniently when multi-task switch.&#10;Modifyable!" 
          placeholder="editable">
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
            style="z-index: 1001; top: 100%; left: 0;">

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
               class="list-group-item list-group-item-action text-white bg-secondary" 
               :class="{'active-row': code.type===transferType && code.croc_code===crocCode ,'blink': code.newArrival }"
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
                  <div class="col-4 mb-3">
                    <div class="input-group mb-0 mt-0 ">
                      <button class="btn" @click="toggleFileMode" :class="isFolder ? 'btn-secondary' : 'btn-success btn-outline-warning' " style=" padding-left:3px;padding-right:3px;" title="选择要发送的文件&#10;Select files to send.">
                        <img src="/assets/file.svg"  width="24" height="24" alt="Icon">SelectFiles
                      </button>&nbsp;
                      <button class="btn" @click="toggleFolderMode" :class="isFolder ? 'btn-success btn-outline-warning' : 'btn-secondary' " style=" padding-left:3px; padding-right:3px;" title="选择要发送的目录&#10;Select folders to send. ">
                        SelectFolders<img src="/assets/folder.svg"  width="24" height="24" alt="Icon">
                      </button>
                      <!--
                      &nbsp;&nbsp;&nbsp;&nbsp;
                      <button class="btn btn-success" @click="selectFile" title="点击选择要发送的文件或目录。&#10;Click to select files or folders to send.">{{ isFolder ? 'SelectFolders' : 'SelectFiles' }}</button>
                      -->
                      </div>
                  </div>

                  <div class="col-6 mb-4  d-flex">
                    <div class="form-check mb-1 mt-2" v-show="isFolder"  title="发送前先压缩成zip,选择Zip则Exclude无效&#10;Zip files before sendding.If select zip,then Exclude invalid." >
                      <label class="form-check-label text-white " for="zip" >Zip</label>
                      <input type="checkbox" id="zip"  v-model="zip"  class=" form-check-input"   title="" placeholder="">
                    </div>
                      &nbsp;&nbsp;&nbsp;&nbsp;
                    <div class="input-group mb-1 mt-0 " v-show="isFolder && !zip" style="width:60%;" title="发送目录时，排除文件名的特征清单，特征间以英文逗号分隔。&#10;如：'pdf, 纪要' 表示文件名包含‘纪要’的或扩展名为pdf的，都不会发送。&#10;&#10;Exclude patterns when sending a folder. Separate multiple patterns with commas.&#10;Example: 'pdf, summary' means files whose names contain “summary” or have the “.pdf” extension will be excluded from sending." >
                      <span class="input-group-text text-white bg-secondary  " >Exclude</span>
                      <input type="text" v-model="exclude"  class="form-control"    placeholder="eg.  'docx , summary' ">
                    </div>
                  </div>
                  <div class="col-2 mb-0 justify-content-end" style="display:flex; ">
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
                  <div class="col-10 mb-3 ">
                    <div class="d-flex">
                    <div class="input-group mb-0 mt-0" >
                      <span class="input-group-text text-white bg-secondary" id="">保存到/SaveTo</span>
                      <input type="text" class="form-control" v-model="savePath" @input="onSavePathInput($event.currentTarget as HTMLElement)" title="接收文件的保存位置/Where to save the files." >
                      <button class="btn btn-success" style="width:34px; padding-left:0px; padding-right:0px;" @click="selectSaveFolder" title="点击选择保存目录。&#10;Click to select save folder.">
                        <img src="/assets/folder.svg"  width="24" height="24" alt="Icon">
                      </button>
                    </div>
                <!--
                    &nbsp;&nbsp;
                    <div class="form-check mb-1 mt-2" style="width:280px;" title="目录中有同名文件时自动覆盖或断点续传&#10;Auto resume or overwrite when there are same files in the dir." >
                      <label class="form-check-label text-white " for="overwrite" >Auto Resume/Overwrite</label>
                      <input type="checkbox" id="overwrite"  v-model="config.overwrite"  class=" form-check-input"   title="" placeholder="">
                    </div>
                -->
                  </div>
                  </div>
                  <div class="col-2 mb-2">
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
                    <span  style="float:right;" v-show="!chatEstablished" title="输入对方Code后，点击按钮接收信息。&#10;After entering the Code from sender,click to receive the message.">
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
      <div class="row">
        <div class="col-4 justify-content-begin">
          <a class="text-success" @click="showSettingDlg" style="font-weight:bold;cursor:pointer">Advanced Settings</a>
        </div>

        <div class="col-8">
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
      </div>
    </div>
    <div id="settingDlg" class="custom-setting" >
      <fieldset class="border p-3 rounded mb-3 mt-0 pt-1 pb-1">
        <legend class="float-none w-auto px-0 text-white mb-0 fs-6">Advanced Settings</legend>

        <div class="row">
          <div class="d-flex p-1">
            <div class="form-check mb-1 mt-0" style="padding-right:20px;" title="强制使用本地连接&#10;Force to use only local connections" >
              <label class="form-check-label text-white " for="local" >Force Local Connections</label>
              <input type="checkbox" id="local"  v-model="config.local"  class=" form-check-input"   title="" placeholder="">
            </div>
            <div class="form-check mb-1 mt-0" title="目录中有同名文件时自动覆盖或断点续传&#10;Auto resume or overwrite when there are same files in the dir." >
              <label class="form-check-label text-white " for="overwrite" >Auto Resume/Overwrite</label>
              <input type="checkbox" id="overwrite"  v-model="config.overwrite"  class=" form-check-input"   title="" placeholder="">
            </div>
          </div>
          <div class="col-6 p-1">
            <div class="input-group mb-1 mt-0" title="用于局域网自动发现，进行本地连接快速传输，默认239.255.255.250&#10;Use for local discover to connect locally,default: 239.255.255.250 ">
              <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >Multicast</span>
              <input type="text" v-model="config.multicast"  class="form-control mb-0 mt-0 p-1"   title="用于局域网自动发现，进行本地连接快速传输，默认239.255.255.250&#10;Use for local discover to connect locally,default: 239.255.255.250 " placeholder="def. 239.255.255.250">
            </div>
          </div>
          <div class="col-6 p-1">
            <div class="input-group mb-1 mt-0" title="本机IP,例：10.2.2.123:9009&#10;Self PC's IP, eg. 10.2.2.123:9009" >
              <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >Local IP</span>
              <input type="text" v-model="config.ip"  class="form-control mb-0 mt-0 p-1"   title="" placeholder="eg. 10.0.0.1:9009">
            </div>
          </div>
          <fieldset class="border p-2 rounded mb-1 pb-1 pt-1">
            <legend class="float-none w-auto px-0 mb-0 text-white fs-6">Send Options</legend>
            <div class="row">
              <div class="col-4 " >
                <div class="input-group mb-0 mt-0 p-0 " title="并行传输的端口数，并非越大越好，建议4-16,默认4。&#10;Number of ports to use for transfers, default 4." >
                  <span class="input-group-text text-white bg-secondary mb-0 mt-0 p-1" >Tansfers </span>
                  <input type="number" v-model="config.transfers"  class="form-control mb-0 mt-0 p-1"   title="" placeholder="">
                </div>
              </div>
            </div>
          </fieldset>

          <fieldset class="border p-2 rounded mb-1 pb-1 pt-1">
            <legend class="float-none w-auto px-0 mb-0 text-white fs-6">Relay</legend>
            <div class="col-12">
              <div class="input-group mb-1 mt-0" title="默认使用官方中继，可能绕道海外，可自建中继。发送方和接收方须使用同一中继。&#10;&#10;By default, the official relay is used, which may route through overseas servers.&#10;You can also set up your own relay. Both sender and receiver must use the same relay." >
                <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >Relay-IPv4</span>
                <input type="text" v-model="config.relay"  class="form-control mb-0 mt-0 p-1"   title="默认使用官方中继，可能绕道海外，可自建中继。发送方和接收方须使用同一中继。&#10;&#10;By default, the official relay is used, which may route through overseas servers.&#10;You can also set up your own relay. Both sender and receiver must use the same relay." placeholder="eg. 125.178.3.210:9009">
              </div>
            </div>
            <div class="col-12">
              <div class="input-group mb-1 mt-0" title="默认使用官方中继，可能绕道海外，可自建中继。发送方和接收方须使用同一中继。&#10;&#10;By default, the official relay is used, which may route through overseas servers.&#10;You can also set up your own relay. Both sender and receiver must use the same relay.">
                <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >Relay-IPv6</span>
                <input type="text" v-model="config.relay6"  class="form-control mb-0 mt-0 p-1"   title="默认使用官方中继，可能绕道海外，可自建中继。发送方和接收方须使用同一中继。&#10;&#10;By default, the official relay is used, which may route through overseas servers.&#10;You can also set up your own relay. Both sender and receiver must use the same relay." placeholder="eg. [2a01:4ff:1f0:eb5d::1]:9009">
              </div>
            </div>
            <div class="col-12">
              <div class="input-group mb-1 mt-0" >
                <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >&nbsp;Password</span>
                <input type="text" v-model="config.relay_passwd"  class="form-control mb-0 mt-0 p-1"   title="" placeholder="">
              </div>
            </div>
          </fieldset>
          <fieldset class="border p-2 rounded mb-0 pb-1 pt-1">
            <legend class="float-none w-auto pb-0 mb-0 text-white fs-6">Proxy</legend>
            <div class="row">
              <div class="col-6" style="padding-left:12px;padding-right:4px;">
                <div class="input-group mb-1 mt-0" >
                  <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >socks5</span>
                  <input type="text" v-model="config.proxy_socks5"  class="form-control mb-0 mt-0 p-1"   title="" placeholder="eg. 125.178.3.21:9001">
                </div>
              </div>
              <div class="col-6" style="padding-right:12px;padding-left:4px;">
                <div class="input-group mb-1 mt-0" >
                  <span class="input-group-text text-white bg-secondary  mb-0 mt-0 p-1" >http</span>
                  <input type="text" v-model="config.proxy_http"  class="form-control mb-0 mt-0 p-1"   title="" placeholder="eg. 125.178.3.21:9000">
                </div>
              </div>
            </div>
          </fieldset>
        </div>
      </fieldset>
      <div class="row">
        <div class="col-12">
          <button @click="onSaveConfig" id="" class="btn btn-success">&ensp;Save&ensp;</button>&nbsp;&nbsp;
          <button @click="onReloadConfig" id="" class="btn btn-success">Reload</button>&nbsp;&nbsp;
          <button @click="closeSettingDlg" id="btnCloseSettingDlg" class="btn btn-warning ">Cancel</button>
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
    .blink  {
      animation: blink 1s infinite;
    }
    .list-group .list-blink .list-group-item.blink  {
      animation: list-blink 1s infinite;
    }
    @keyframes blink {
      0%,100%{
        background-color : white;
      }
      50% {
        background-color: #198754; /*bg-success*/
        color: white;
      }
    }
    @keyframes list-blink {
      0%,100%{
        background-color : #6c757d !important; /*bg-secondary*/
      }
      50% {
        background-color: #198754 !important; /*bg-success */
        color: white !important;
      }
    }
    .list-group-item.blink::after {
      content: '';
      position: absolute;
      inset: 0;
      background: #198754;
      opacity: 0;
      z-index: 0;
      animation: flash-bg 1s infinite;
    }

    @keyframes flash-bg {
      0%, 100% { opacity: 0; }
      80% { opacity: 0.8; }
    }

    .custom-setting{
      display: none; 
      background:#333; 
      position: fixed; 
      top: 50%; 
      left: 50%; 
      border-radius: 10px;
      transform: translate(-50%,-50%); 
      padding: 20px; 
      border: 2px solid #ccc; 
      box-shadow: 0 0 10px rgba(0,0,0,0.2); 
      text-align:center; 
      width:60%;
      z-index: 1002;
    }

</style>
