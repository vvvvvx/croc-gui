// src/utils/dialog.ts
import { h, render, ref } from "vue";
import { ElDialog, ElInput, ElButton } from "element-plus";
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

export async function darkAlert(message: string, title = "提示"): Promise<void> {
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
          closeOnClickModal: true,
          closeOnPressEscape: true,
          showClose: false,
          customClass: "dark-dialog",
          style:` 
            width: 500px; 
            text-align:center; 
            z-index:9999;
            background-color:#333;
            color:#fff;`,
        },
        {
          // 自定义标题 slot
          title: () => h("div", { style: "color:white;" }, title),

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
