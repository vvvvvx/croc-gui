// src/utils/dialog.ts
import { h, render, ref } from "vue";
import { ElDialog, ElInput, ElButton,ElCheckbox } from "element-plus";
import "element-plus/dist/index.css";

export async function askUserInput(title = "请输入内容"): Promise<string> {
  return new Promise((resolve) => {
    // 创建挂载容器
    const container = document.createElement("div");
    document.body.appendChild(container);

    // 输入框内容
    const inputValue = ref("");

    // 提交逻辑
    const handleConfirm = () => {
      if (!inputValue.value.trim()) {
        alert("输入不能为空！");
        return;
      }
      cleanup();
      resolve(inputValue.value.trim());
    };

    // 清理逻辑
    const cleanup = () => {
      render(null, container);
      container.remove();
    };

    // 渲染 Vue 组件到容器
    render(
      h(
        ElDialog,
        {
          modelValue: true,
          title,
          closeOnClickModal: false,
          closeOnPressEscape: false,
          showClose: false, // ❌ 不允许关闭
          customClass: "dark-dialog",
          style:` 
            width: 400px; 
            text-align:center; 
            z-index:9999;
            background-color:#333;
            color:#fff;`,
        },
        {
          title:() => h("div",{style: "color:white"},title),
          default: () =>
            h(ElInput, {
              modelValue: inputValue.value,
              "onUpdate:modelValue": (val: string) => (inputValue.value = val),
              placeholder: "请输入文字...",
              autofocus: true,
              style: "color :white;",
            }),
          footer: () =>
            h(
              "div",
              { style: "text-align: center;" },
              [
                h(
                  ElButton,
                  {
                    type: "primary",
                    onClick: handleConfirm,
                  },
                  () => "确定"
                ),
              ]
            ),
        }
      ),
      container
    );
  });
}

export async function darkAlert(message: string, title = "提  示"): Promise<void> {
  return new Promise((resolve) => {
    // 创建挂载容器
    const container = document.createElement("div");
    document.body.appendChild(container);

    const handleClose = () => {
      cleanup();
      resolve();
    };

    const cleanup = () => {
      render(null, container);
      container.remove();
    };

    render(
      h(
        ElDialog,
        {
          modelValue: true,
          closeOnClickModal: false,
          closeOnPressEscape: false,
          showClose: true,
          customClass: "dark-dialog",
          style:` 
            width: 500px; 
            text-align:center; 
            z-index:1003;
            background-color:#333;
            color:#fff;`,
        },
        {
          // 自定义标题 slot
          title: () => h("div", { style: "color:white;font-weight:bold;font-size:22px;" }, title),

          // 默认内容 slot
          default: () => h("div", { style: "color:white; margin-top: 10px;white-space: pre-wrap;" }, message),

          // footer slot
          footer: () =>
            h(
              "div",
              { style: "text-align:center;" },
              [
                h(
                  ElButton,
                  {
                    type: "success",
                    onClick: handleClose,
                  },
                  () => "确定"
                ),
              ]
            ),
        }
      ),
      container
    );
  });
}


export async function darkConfirm(message: string, title = "确认"): Promise<boolean> {
  return new Promise((resolve) => {
    // 创建挂载容器
    const container = document.createElement("div");
    document.body.appendChild(container);

    const handleConfirm = () => {
      cleanup();
      resolve(true);
    };

    const handleCancel = () => {
      cleanup();
      resolve(false);
    };

    const cleanup = () => {
      render(null, container);
      container.remove();
    };

    // 渲染对话框
    render(
      h(
        ElDialog,
        {
          modelValue: true,
          closeOnClickModal: false,
          closeOnPressEscape: false,
          showClose: true,
          customClass: "dark-dialog",
          style: `
            width: 500px;
            text-align: center;
            z-index: 1003;
            background-color: #333;
            color: #fff;
          `,
        },
        {
          // 标题 slot
          title: () => h("div", { style: "color:white;" }, title),

          // 内容 slot
          default: () =>
            h(
              "div",
              { style: "color:white; margin-top:10px; white-space: pre-wrap;" },
              message
            ),

          // 底部按钮 slot
          footer: () =>
            h(
              "div",
              { style: "text-align:center;" },
              [
                h(
                  ElButton,
                  {
                    type: "success",
                    onClick: handleConfirm,
                  },
                  () => "是"
                ),
                h(
                  ElButton,
                  {
                    type: "danger",
                    onClick: handleCancel,
                  },
                  () => "否"
                ),
              ]
            ),
        }
      ),
      container
    );
  });
}

export async function darkConfirmRemember(
  message: string,
  title = "确认/Confirm"
): Promise<{ answer: boolean; remember: boolean }> {
  return new Promise((resolve) => {
    const container = document.createElement("div");
    document.body.appendChild(container);

    const rememberChoice = ref(false);

    const cleanup = () => {
      render(null, container);
      container.remove();
    };

    const handleConfirm = () => {
      cleanup();
      resolve({ answer: true, remember: rememberChoice.value });
    };

    const handleCancel = () => {
      cleanup();
      resolve({ answer: false, remember: rememberChoice.value });
    };

    render(
      h(
        ElDialog,
        {
          modelValue: true,
          closeOnClickModal: false,
          closeOnPressEscape: false,
          showClose: true,
          customClass: "dark-dialog",
          style: `
            width: 500px;
            text-align: center;
            z-index: 1003;
            background-color: #333;
            color: #fff;
          `,
        },
        {
          title: () => h("div", { style: "color:white;" }, title),

          default: () =>
            h("div", { style: "color:white; margin-top:10px; white-space: pre-wrap;" }, [
              h("div", { style: "margin-bottom: 10px;" }, message),
              h(
                ElCheckbox,
                {
                  modelValue: rememberChoice.value,
                  "onUpdate:modelValue": (val: any) => (rememberChoice.value = !!val),
                  style: "margin-top: 10px; color:white;",
                },
                () => "记住我的选择/Remember my choice"
              ),
            ]),

          footer: () =>
            h(
              "div",
              { style: "text-align:center;" },
              [
                h(
                  ElButton,
                  { type: "success", onClick: handleConfirm },
                  () => "是/Yes"
                ),
                h(
                  ElButton,
                  { type: "danger", onClick: handleCancel },
                  () => "否/No"
                ),
              ]
            ),
        }
      ),
      container
    );
  });
}
