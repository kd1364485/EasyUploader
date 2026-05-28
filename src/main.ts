import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

let selectedPathEl: HTMLElement | null;
let selectFolderBtn: HTMLButtonElement | null;
let generateZipBtn: HTMLButtonElement | null;
let zipStatusEl: HTMLElement | null;
let currentPath: string = "";

let extensionInput: HTMLInputElement | null;
let addExtensionBtn: HTMLButtonElement | null;
let extensionsTagsEl: HTMLElement | null;
let loadExtensionsBtn: HTMLButtonElement | null;
let saveExtensionsBtn: HTMLButtonElement | null;
let extensionsStatusEl: HTMLElement | null;

let extensionsList: string[] = [];

async function selectFolder() {
  try {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "フォルダを選択",
    });
    
    if (selected && typeof selected === "string") {
      currentPath = selected;
      if (selectedPathEl) {
        selectedPathEl.textContent = `選択されたパス: ${selected}`;
      }
      if (generateZipBtn) {
        generateZipBtn.disabled = false;
      }
    }
  } catch (error) {
    console.error("フォルダ選択エラー:", error);
  }
}

async function generateZip() {
  console.log("generateZip関数が呼ばれました");
  console.log("currentPath:", currentPath);

  if (!currentPath) {
    console.log("currentPathが空です");
    if (zipStatusEl) {
      zipStatusEl.textContent = "フォルダが選択されていません";
    }
    return;
  }

  try {
    console.log("テキストファイル生成を開始します");
    if (zipStatusEl) {
      zipStatusEl.textContent = "テキストファイル生成中...";
    }

    const result = await invoke("generate_markdown", {
      path: currentPath,
    });

    console.log("テキストファイル生成完了:", result);
    if (zipStatusEl) {
      zipStatusEl.textContent = `テキストファイル生成完了: ${result}`;
    }
  } catch (error) {
    console.error("テキストファイル生成エラー:", error);
    if (zipStatusEl) {
      zipStatusEl.textContent = `テキストファイル生成に失敗しました: ${error}`;
    }
  }
}

function renderTags() {
  if (!extensionsTagsEl) return;

  extensionsTagsEl.innerHTML = extensionsList.map((ext) => `
    <span class="inline-flex items-center gap-2 px-3 py-1.5 bg-gradient-to-r from-purple-500/20 to-blue-500/20 border border-purple-500/30 rounded-full text-sm text-purple-300 hover:from-purple-500/40 hover:to-blue-500/40 hover:border-purple-500/50 hover:scale-105 hover:shadow-lg hover:shadow-purple-500/20 transition-all duration-300 cursor-pointer">
      <span>.${ext}</span>
      <button onclick="removeExtension('${ext}')" class="text-purple-400 hover:text-red-400 transition-colors duration-200">
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </span>
  `).join('');
}

function addExtension() {
  if (!extensionInput) return;

  const ext = extensionInput.value.trim().toLowerCase();
  if (ext && !extensionsList.includes(ext)) {
    extensionsList.push(ext);
    renderTags();
    extensionInput.value = "";
    if (extensionsStatusEl) {
      extensionsStatusEl.textContent = "設定を保存してください";
    }
  }
}

function removeExtension(ext: string) {
  extensionsList = extensionsList.filter((e) => e !== ext);
  renderTags();
  if (extensionsStatusEl) {
    extensionsStatusEl.textContent = "設定を保存してください";
  }
}

async function loadExtensions() {
  try {
    const extensions = await invoke<string[]>("get_extensions");
    extensionsList = extensions;
    renderTags();
    if (extensionsStatusEl) {
      extensionsStatusEl.textContent = "設定を読み込みました";
    }
  } catch (error) {
    console.error("拡張子読み込みエラー:", error);
    if (extensionsStatusEl) {
      extensionsStatusEl.textContent = `設定の読み込みに失敗しました: ${error}`;
    }
  }
}

async function saveExtensions() {
  try {
    await invoke("save_extensions", { extensions: extensionsList });
    if (extensionsStatusEl) {
      extensionsStatusEl.textContent = "設定を保存しました";
    }
  } catch (error) {
    console.error("拡張子保存エラー:", error);
    if (extensionsStatusEl) {
      extensionsStatusEl.textContent = `設定の保存に失敗しました: ${error}`;
    }
  }
}

window.addEventListener("DOMContentLoaded", () => {
  selectedPathEl = document.querySelector("#selected-path");
  selectFolderBtn = document.querySelector("#select-folder-btn");
  generateZipBtn = document.querySelector("#generate-zip-btn");
  zipStatusEl = document.querySelector("#zip-status");

  extensionInput = document.querySelector("#extension-input");
  addExtensionBtn = document.querySelector("#add-extension-btn");
  extensionsTagsEl = document.querySelector("#extensions-tags");
  loadExtensionsBtn = document.querySelector("#load-extensions-btn");
  saveExtensionsBtn = document.querySelector("#save-extensions-btn");
  extensionsStatusEl = document.querySelector("#extensions-status");

  selectFolderBtn?.addEventListener("click", selectFolder);
  generateZipBtn?.addEventListener("click", generateZip);
  addExtensionBtn?.addEventListener("click", addExtension);
  loadExtensionsBtn?.addEventListener("click", loadExtensions);
  saveExtensionsBtn?.addEventListener("click", saveExtensions);

  // Enterキーで拡張子を追加
  extensionInput?.addEventListener("keypress", (e) => {
    if (e.key === "Enter") {
      addExtension();
    }
  });

  // グローバル関数として登録
  (window as any).removeExtension = removeExtension;

  // 初期ロード時に拡張子設定を読み込む
  loadExtensions();
});
