<script setup lang="ts">
import { onMounted,onBeforeUnmount ,ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";
import { listen,UnlistenFn } from "@tauri-apps/api/event";
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
interface emitStatus{
  croc_code:string,
  status:string
}

const isFolder = ref(false); // File or Folder mode
const isFileTransfer= ref(true); //FileTransfer or TextChat
const sendPaths=ref<fileItem[]>([]); // Selected file or folder paths to send
const receivePaths=ref<fileItem[]>([]); // received file or folder paths 
const crocCode = ref<string>(""); // Croc code for transfer
const waitingCodesList = ref<string[]>([]); //Croc codes which are waiting for receiving
const savePath=ref<string>(""); // Application directory path
const sendStatus=ref<string>("Pending"); // Overall send status
const receiveStatus=ref<string>(""); // Overall receive status
const isSending=ref<boolean>(false); // Whether currently sending files
const chatText=ref<string>("聊天记录")
const inputText=ref<string>("")

let listenError: UnlistenFn | null = null;
let listenSendProgress: UnlistenFn | null = null;
let listenStatus: UnlistenFn | null = null;
let listenCode: UnlistenFn | null = null;
let listenReady: UnlistenFn | null = null;
let listenSendFileDone: UnlistenFn | null = null;
let listenReceiveFileProgress: UnlistenFn | null = null;
let listenReceiveFileDone: UnlistenFn | null = null;
let listenSendFileSuccess: UnlistenFn | null = null;
let listenReceiveFileSuccess: UnlistenFn | null = null;
let listenReceiveStatus: UnlistenFn | null = null;

async function selectFile() {
  const selected = await open({
    multiple: true,
    directory: isFolder.value,
    title: isFileTransfer ? "选择目录/Select Folders":"选择文件/Select files",
  });
  if (Array.isArray(selected)) {
    sendPaths.value = selected.map((path) => ({ file: path, status: "待发送/Pending" ,is_dir: isFolder.value }));
  } else if (typeof selected === "string") {
    sendPaths.value = [{ file: selected, status: "待发送/Pending" ,is_dir: isFolder.value }];
  } else {
    sendPaths.value = [];
  }
  console.log(sendPaths);
}
function toggleFileMode() {
  isFolder.value = false;
} 
function toggleFolderMode() {
  isFolder.value = true;
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
    alert("请先选择要发送的文件或目录。\nPlease select files or folders to send first.");
    return;
  }
  if (crocCode.value.trim()!=="" && isWaiting(crocCode.value)) {
    alert("上一次发送等待对方接收完成。\nThe previous sending is waiting for receiving.");
    return;
  }
  //isSending.value = true;
  if (crocCode.value.trim()!==""){
    waitingCodesList.value.push(crocCode.value.trim());
  }
  
  // Implement file sending logic here
  await invoke("send_files", { files: sendPaths.value, code: crocCode.value });
  
}
async function receiveFiles() {
  if (!crocCode.value || crocCode.value.trim() === "") {
    alert("请输入发送方生成的Code。\nPlease enter a Code from sender.");
    return;
  }
  if (!savePath.value || savePath.value.trim() === "") {
    alert("请选择保存目录。\nPlease select a save folder.");
    return;
  }
  // Implement file receiving logic here
  await invoke("receive_files", { savePath: savePath.value, code: crocCode.value });

  console.log("Save to:", savePath.value, "Code:", crocCode.value);
}
async function sendText() {
  if(inputText.value.trim()==="")
  {
    alert("请输入发送文本。\nEnter text first.");
    return;
  }
}
async function receiveText() {

}
onMounted(async () => {
  savePath.value = await homeDir();

  listenError = await listen("croc-error", (event) => {
    const message = event.payload as emitInfo;
    alert("Code: "+message.croc_code+"\n\n"+message.info);
    //isSending.value = false;
    if(isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }
    console.error("File transfer error:", message);
  });
  listenCode = await listen("croc-code", (event) => {
    const code = event.payload as string;
    crocCode.value = code;
    if(!isWaiting(code)){
      waitingCodesList.value.push(code);
    }
    console.log("Received croc code:", crocCode.value);
  });

  listenSendProgress = await listen("croc-send-file-progress", (event) => {
    const progress = event.payload as emitProgress; 
    // 是当前传输进程才更新，否则会混乱。
    if (crocCode.value===progress.croc_code){
      sendPaths.value = progress.files;
    }
    console.log("Send progress update:", sendPaths.value);
    /*
    const payload = event.payload as { file: string; status: string };
    const index = sendPaths.value.findIndex((item) => item.file === payload.file);
    if (index !== -1) {
      sendPaths.value[index].status = payload.status;
    }*/
  });
  listenStatus = await listen("croc-send-file-status", (event) => {
    const status = event.payload as emitStatus;
    if (crocCode.value===status.croc_code){
      sendStatus.value = status.status;
    }
    console.log("Overall status update:", sendStatus.value);

  });
  listenReady = await listen("croc-send-file-ready", (event) => {
    const message = event.payload as emitInfo;
    alert("Code: "+message.croc_code+"\n\n"+message.info);
    if (!isWaiting(message.croc_code)){
      waitingCodesList.value.push(message.croc_code)
    }
    console.log("Croc ready:", message);
  });
  listenSendFileDone = await listen("croc-send-file-done", (event) => {
    const message = event.payload as emitInfo;
    //isSending.value = false;
    if (message.croc_code.trim().length===0){
      alert("发送完成，无Code")
    } else{
      if( isWaiting(message.croc_code)){
        deleteCode(message.croc_code);
      }

    }

    console.log("Croc send done:", message);
  });
  listenSendFileSuccess = await listen("croc-send-file-success", (event) => {
    const message = event.payload as emitInfo;
    alert("Code:"+message.croc_code+"\n\n"+message.info);
    //isSending.value = false;
    if(isWaiting(message.croc_code)){
      deleteCode(message.croc_code);
    }
    console.log("Croc send success:", message);
  });
  listenReceiveFileProgress = await listen("croc-receive-file-progress", (event) => {
    receivePaths.value = event.payload as fileItem[]; 
    console.log("Receive progress update:", receivePaths.value);
  });
  listenReceiveFileDone = await listen("croc-receive-file-done", (event) => {
    const message = event.payload as string;
    console.log("Croc receive done:", message);
  });
  listenReceiveFileSuccess = await listen("croc-receive-file-success", (event) => {
    const message = event.payload as string;
    alert(message);
    console.log("Croc receive success:", message);
  });
  listenReceiveStatus = await listen("croc-receive-file-status", (event) => {
    receiveStatus.value = event.payload as string;
    console.log("Overall receive status update:", receiveStatus.value);
  });
});

onBeforeUnmount(() => {
  listenError?.();
  listenSendProgress?.();
  listenStatus?.();
  listenCode?.();
  listenReady?.();
  listenSendFileDone?.();
  listenSendFileSuccess?.();
  listenReceiveFileProgress?.();
  listenReceiveFileDone?.();
  listenReceiveFileSuccess?.();
  listenReceiveStatus?.();
});

</script>

<template>
  <main class="container-fluid p-4 pt-2" style="z-index:1000;" >
    <div class="row mb-0 align-items-end" >
      <!-- nav tabs -->
      <div class="col-5 mb-0" style="margin-bottom:0px;padding-bottom:0px;" >
        <ul class="nav nav-tabs mb-0 " id="topTab" role="tablist">
          <li class="nav-item" role="presentation">
            <button class="nav-link active " style="margin-left:10px;"  id="file-tab" data-bs-toggle="tab" data-bs-target="#file-pane" type="button" role="tab">
              文件传输<br>File Transfer
            </button>
          </li>
          <li class="nav-item" role="presentation">
            <button class="nav-link"  id="chat-tab" data-bs-toggle="tab" data-bs-target="#chat-pane" type="button" role="tab">
              文本聊天<br>Text Chat
            </button>
          </li>
        </ul>
      </div>
      <div class="col-7 mb-2 justify-content-end" style="margin-bottom:0px;padding-bottom:0px;">
        <div class="input-group mb-0 mt-0" style="width:400px; float:right;">
          <span class="input-group-text text-white bg-secondary" id="basic-addon2">Code</span>
          <input type="text" class="form-control " v-model="crocCode" 
          title="发送时，可输入自定义或留空自动生成传输代码&#10;接收时，输入对方的传输代码&#10;连续或来回传输时，可保持Code不变
When sending,enter custom Code or leave it blank to generate Code automatically.
When receiving,enter the Code provided by other side.&#10;When transmitting continuously or back and forth,the Code can be kept unchanged." 
          placeholder="">
        </div>
      </div>
    </div>
    <!-- tab content -->
    <div class="tab-content pt-0" style="border:1px solid #ddd;" id="myTabContent">
      <div class="tab-pane fade show active pt-3" id="file-pane" role="tabpanel" aria-labelledby="file-tab">
        <!-- File Transfer content -->
        <ul class="nav nav-tabs mb-0" style="margin-left:10px;" id="secondTab" role="tablist">
          <li class="nav-item" role="presentation">
            <button class="nav-link active"  id="send-file-tab" data-bs-toggle="tab" data-bs-target="#send-file-pane" type="button" role="tab">
              发送/Send
            </button>
          </li>
          <li class="nav-item" role="presentation">
            <button class="nav-link"  id="receive-file-tab" data-bs-toggle="tab" data-bs-target="#receive-file-pane" type="button" role="tab">
              接收/Receive
            </button>
          </li>
        </ul>
        <!-- Second tab content -->
        <div class="tab-content pt-0" style="height:calc(99vh - 120px);border-top:1px solid #ddd;" id="fileTabContent">
          <!-- Send File content -->
          <div class="tab-pane fade show active" id="send-file-pane" style="padding:1rem;" role="tabpanel" aria-labelledby="send-file-tab">
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
                  <div class="col-12 mb-0 mt-0">
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
          <div class="tab-pane fade " id="receive-file-pane" style="padding:1rem;" role="tabpanel" aria-labelledby="receive-file-tab">
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
                  <div class="col-12 mb-0 mt-0 " style="margin-top:0px;">
                    <span>状态/Status &nbsp;: &nbsp;</span>
                    <span style="color:lightgreen;" v-show="receivePaths.length>0">{{  receiveStatus   }}</span>
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

      <div class="tab-pane fade p-0 " id="chat-pane"  role="tabpanel" aria-labelledby="chat-tab">
        <!-- Text Chat content -->
        <div class="fixed-pane-container p-0 m-0">
          <div class="header-area p-3 " >
            <div class="row">
              <div class="col-9">
                <textarea class="form-control pl-0 " v-model="inputText" style="border::1px solid #ddd;height:80px; resize:none;" placeholder="在这里输入消息/Enter message here" >
                </textarea>
              </div>
              <div class="col-3 mb-0 justify-content-end" style="display:flex; ">
                <div class="row justify-content-end">
                  <div class="col-12 mb-2">
                    <span class="m-0 p-0 flex " style="float:right;" 
                      title="发送文字后把Code告知对方以接收。&#10;接收完成前不能继续发送。&#10;After sending the message,inform the recipient of the Code so they can receive it.&#10;Cannot send again until the recipient has finished receiving it.">
                      <button class="btn btn-warning" @click="sendText" style="width:120px;" :disabled="!inputText || inputText.trim().length === 0 || isSending">发送/Send</button>
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
                <textarea class="form-control pl-0 bg-dark text-white " v-model="chatText" style="height:calc(99vh - 200px);border:none; border-top:1px solid #ddd; resize:none;" readonly>
                </textarea>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>

      <!-- Bootstrap JS (with Popper) -->
  </main>
</template>

<style scoped>

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
