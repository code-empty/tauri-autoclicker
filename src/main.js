const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// DOM 元素
let app;
let hoursInput, minutesInput, secondsInput, msInput;
let mouseButtonSelect, clickTypeSelect;
let hotkeyInput, recordBtn, hotkeyContainer;
let actionBtn, btnText;
let statusBadge, statusText, footerHotkey;

let isRunning = false;
let currentHotkey = "F8";
let isRecording = false;

// 初始化
window.addEventListener("DOMContentLoaded", async () => {
  // 获取 DOM 元素
  app = document.getElementById("app");
  hoursInput = document.getElementById("hours");
  minutesInput = document.getElementById("minutes");
  secondsInput = document.getElementById("seconds");
  msInput = document.getElementById("ms");
  mouseButtonSelect = document.getElementById("mouse-button");
  clickTypeSelect = document.getElementById("click-type");
  hotkeyInput = document.getElementById("hotkey");
  recordBtn = document.getElementById("record-btn");
  hotkeyContainer = document.getElementById("hotkey-container");
  actionBtn = document.getElementById("action-btn");
  btnText = document.getElementById("btn-text");
  statusBadge = document.getElementById("status-badge");
  statusText = document.getElementById("status-text");
  footerHotkey = document.getElementById("footer-hotkey");

  // 绑定配置项更改事件，实时同步到 Rust
  const configElements = [hoursInput, minutesInput, secondsInput, msInput, mouseButtonSelect, clickTypeSelect];
  configElements.forEach(el => {
    el.addEventListener("change", syncSettings);
    el.addEventListener("input", syncSettings);
  });

  // 启动/停止按钮点击
  actionBtn.addEventListener("click", toggleClicker);

  // 录制快捷键按钮
  recordBtn.addEventListener("click", () => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  });

  // 监听 Rust 发送的状态更改事件
  await listen("clicker-status-changed", (event) => {
    isRunning = event.payload;
    updateUIState(isRunning);
  });

  // 获取初始状态
  isRunning = await invoke("get_status");
  updateUIState(isRunning);
  syncSettings(); // 同步初始配置
});

// 状态和 UI 更新
function updateUIState(running) {
  if (running) {
    app.classList.add("running");
    statusText.textContent = "连点中";
    btnText.textContent = `停止连点 (${currentHotkey})`;
  } else {
    app.classList.remove("running");
    statusText.textContent = "未激活";
    btnText.textContent = `启动连点 (${currentHotkey})`;
  }
}

// 同步设置到 Rust 后端
async function syncSettings() {
  // 确保输入值合理
  const hours = Math.max(0, parseInt(hoursInput.value) || 0);
  const minutes = Math.max(0, parseInt(minutesInput.value) || 0);
  const seconds = Math.max(0, parseInt(secondsInput.value) || 0);
  const ms = Math.max(1, parseInt(msInput.value) || 1); // 毫秒最小 1ms

  const interval = (hours * 3600000) + (minutes * 60000) + (seconds * 1000) + ms;
  const button = parseInt(mouseButtonSelect.value);
  const clickType = parseInt(clickTypeSelect.value);

  // 调用 Rust 命令更新设置 (注意：Tauri 自动将 click_type 转换为驼峰 clickType)
  await invoke("update_settings", { interval, button, clickType });
}

// 切换连点状态
async function toggleClicker() {
  if (isRunning) {
    await invoke("stop_clicker");
  } else {
    await invoke("start_clicker");
  }
}

// 快捷键录制逻辑
function startRecording() {
  isRecording = true;
  hotkeyContainer.classList.add("recording");
  recordBtn.textContent = "按键中...";
  hotkeyInput.value = "请按下快捷键...";
  window.addEventListener("keydown", handleHotkeyRecord, true);
}

function stopRecording() {
  isRecording = false;
  hotkeyContainer.classList.remove("recording");
  recordBtn.textContent = "录制";
  hotkeyInput.value = currentHotkey;
  window.removeEventListener("keydown", handleHotkeyRecord, true);
}

async function handleHotkeyRecord(e) {
  e.preventDefault();
  e.stopPropagation();

  const key = e.key;

  // 忽略单独按下的修饰键
  if (["Control", "Shift", "Alt", "Meta", "CapsLock", "NumLock"].includes(key)) {
    return;
  }

  // Escape 键取消录制
  if (key === "Escape") {
    stopRecording();
    return;
  }

  // 构建快捷键组合字符串
  let modifiers = [];
  if (e.ctrlKey) modifiers.push("Control");
  if (e.shiftKey) modifiers.push("Shift");
  if (e.altKey) modifiers.push("Alt");
  if (e.metaKey) modifiers.push("Command");

  let mainKey = key;
  if (mainKey === " ") {
    mainKey = "Space";
  } else if (mainKey.length === 1) {
    mainKey = mainKey.toUpperCase();
  } else {
    // 确保首字母大写
    mainKey = mainKey.charAt(0).toUpperCase() + mainKey.slice(1);
  }

  modifiers.push(mainKey);
  const newHotkey = modifiers.join("+");

  try {
    // 尝试在 Rust 后端注册热键
    await invoke("register_hotkey", { hotkey: newHotkey });
    currentHotkey = newHotkey;
    footerHotkey.textContent = newHotkey;
    
    // 更新界面
    updateUIState(isRunning);
    stopRecording();
  } catch (err) {
    alert(`快捷键 [${newHotkey}] 注册失败: ${err}\n(可能已被系统或其他程序占用)`);
    stopRecording();
  }
}

