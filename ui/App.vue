<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

interface HistorySummary {
  id: number;
  preview: string;
  at: string;
  kind: string;
}

interface PageResult {
  items: HistorySummary[];
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
}

interface ConnectionInfo {
  status: string;
  generated_psk: string | null;
  is_hosting: boolean;
  is_joining: boolean;
  preview: string;
}

const showConfig = ref(false);
const connectionStatus = ref("未连接");
const currentPreview = ref("");
const searchInput = ref("");
const page = ref(0);
const pageSize = ref(9);
const pageResult = ref<PageResult>({
  items: [],
  page: 0,
  page_size: 9,
  total: 0,
  total_pages: 1,
});
const selectedIndex = ref<number | null>(null);
const hostPort = ref("3939");
const joinAddr = ref("127.0.0.1:3939");
const pskInput = ref("");
const generatedPsk = ref<string | null>(null);
const isHosting = ref(false);
const isJoining = ref(false);
const errorMsg = ref("");

const unlistens: UnlistenFn[] = [];

const isConnected = computed(() => connectionStatus.value.includes("已连接"));

async function refreshConnection() {
  const info = await invoke<ConnectionInfo>("get_connection_status");
  connectionStatus.value = info.status;
  generatedPsk.value = info.generated_psk;
  isHosting.value = info.is_hosting;
  isJoining.value = info.is_joining;
  currentPreview.value = info.preview;
}

async function refreshHistory() {
  pageResult.value = await invoke<PageResult>("search_history", {
    query: searchInput.value,
    page: page.value,
    pageSize: pageSize.value,
  });
  if (pageResult.value.page !== page.value) {
    page.value = pageResult.value.page;
  }
}

async function applyItem(id: number) {
  await invoke("apply_history_item", { id });
  await refreshConnection();
}

async function deleteItem(id: number) {
  await invoke("delete_history_item", { id });
  await refreshHistory();
}

async function onSearch() {
  page.value = 0;
  selectedIndex.value = null;
  await refreshHistory();
}

async function prevPage() {
  if (page.value > 0) {
    page.value -= 1;
    selectedIndex.value = null;
    await refreshHistory();
  }
}

async function nextPage() {
  if (page.value + 1 < pageResult.value.total_pages) {
    page.value += 1;
    selectedIndex.value = null;
    await refreshHistory();
  }
}

async function startHost() {
  errorMsg.value = "";
  const port = Number(hostPort.value);
  try {
    await invoke("start_host", { port });
    await refreshConnection();
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function startJoin() {
  errorMsg.value = "";
  try {
    await invoke("start_join", { addr: joinAddr.value, psk: pskInput.value });
    await refreshConnection();
  } catch (e) {
    errorMsg.value = String(e);
  }
}

async function disconnect() {
  await invoke("disconnect");
  await refreshConnection();
}

function onKeydown(e: KeyboardEvent) {
  if (showConfig.value) return;
  const items = pageResult.value.items;
  if (e.key === "ArrowUp") {
    e.preventDefault();
    if (selectedIndex.value === null) {
      selectedIndex.value = items.length > 0 ? items.length - 1 : null;
    } else if (selectedIndex.value > 0) {
      selectedIndex.value -= 1;
    }
  } else if (e.key === "ArrowDown") {
    e.preventDefault();
    if (selectedIndex.value === null) {
      selectedIndex.value = items.length > 0 ? 0 : null;
    } else if (selectedIndex.value + 1 < items.length) {
      selectedIndex.value += 1;
    }
  } else if (e.key === "PageUp") {
    e.preventDefault();
    void prevPage();
  } else if (e.key === "PageDown") {
    e.preventDefault();
    void nextPage();
  } else if (e.key === "Enter" && selectedIndex.value !== null) {
    e.preventDefault();
    const item = items[selectedIndex.value];
    if (item) void applyItem(item.id);
  } else if (/^[1-9]$/.test(e.key)) {
    const local = Number(e.key) - 1;
    const item = items[local];
    if (item) {
      selectedIndex.value = local;
      void applyItem(item.id);
    }
  }
}

onMounted(async () => {
  window.addEventListener("keydown", onKeydown);
  await refreshConnection();
  await refreshHistory();
  unlistens.push(await listen("connection-changed", () => void refreshConnection()));
  unlistens.push(await listen("history-updated", () => void refreshHistory()));
  unlistens.push(
    await listen<string>("clipboard-preview-updated", (ev) => {
      currentPreview.value = ev.payload;
    }),
  );
  unlistens.push(
    await listen<string>("psk-generated", (ev) => {
      generatedPsk.value = ev.payload;
      void refreshConnection();
    }),
  );
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown);
  unlistens.forEach((u) => u());
});
</script>

<template>
  <div class="app">
    <header class="topbar">
      <h1>剪切板共享</h1>
      <span class="status">{{ isConnected ? "🟢" : "⚪" }} {{ connectionStatus }}</span>
      <button class="btn ghost" @click="showConfig = !showConfig">
        {{ showConfig ? "← 返回" : "配置" }}
      </button>
    </header>

    <main v-if="!showConfig" class="history-view">
      <section class="card">
        <h2>当前内容</h2>
        <p>{{ currentPreview || "暂无内容" }}</p>
      </section>

      <section class="card search">
        <input
          v-model="searchInput"
          placeholder="搜索历史…"
          @input="onSearch"
        />
      </section>

      <div class="pager">
        <button class="btn" :disabled="page <= 0" @click="prevPage">◀</button>
        <span>第 {{ page + 1 }} / {{ pageResult.total_pages }} 页</span>
        <button
          class="btn"
          :disabled="page + 1 >= pageResult.total_pages"
          @click="nextPage"
        >
          ▶
        </button>
        <label>
          每页:
          <select v-model.number="pageSize" @change="onSearch">
            <option :value="9">9</option>
            <option :value="15">15</option>
            <option :value="30">30</option>
          </select>
        </label>
      </div>

      <section v-if="pageResult.items.length === 0" class="empty">暂无历史记录</section>
      <ul class="list">
        <li
          v-for="(item, i) in pageResult.items"
          :key="item.id"
          :class="{ selected: selectedIndex === i }"
          @click="selectedIndex = i; applyItem(item.id)"
        >
          <span class="num">{{ (i % 9) + 1 }}</span>
          <div class="body">
            <div class="preview">{{ item.preview }}</div>
            <div class="meta">{{ item.at }} · {{ item.kind }}</div>
          </div>
          <button class="btn danger" @click.stop="deleteItem(item.id)">删除</button>
        </li>
      </ul>
    </main>

    <main v-else class="config-view">
      <p v-if="errorMsg" class="error">{{ errorMsg }}</p>

      <section class="card">
        <h2>作为主机 (Host)</h2>
        <label>
          监听端口
          <input v-model="hostPort" />
        </label>
        <div class="row">
          <button v-if="!isHosting" class="btn primary" @click="startHost">开始监听</button>
          <button v-else class="btn danger" @click="disconnect">停止监听</button>
        </div>
        <div v-if="generatedPsk" class="psk">
          <div>分享此密钥给连接方：</div>
          <code>{{ generatedPsk }}</code>
        </div>
      </section>

      <section class="card">
        <h2>连接到主机 (Join)</h2>
        <label>
          主机地址
          <input v-model="joinAddr" placeholder="192.168.1.100:3939" />
        </label>
        <label>
          PSK 密钥
          <input v-model="pskInput" placeholder="主机提供的密钥" />
        </label>
        <div class="row">
          <button v-if="!isJoining" class="btn primary" @click="startJoin">连接</button>
          <button v-else class="btn danger" @click="disconnect">取消</button>
        </div>
      </section>
    </main>
  </div>
</template>

<style>
:root {
  font-family: "Segoe UI", "PingFang SC", "Microsoft YaHei", sans-serif;
  color: #1a1a1a;
  background: #f3f3f3;
}

* {
  box-sizing: border-box;
}

body {
  margin: 0;
}

.app {
  min-height: 100vh;
  display: flex;
  flex-direction: column;
}

.topbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  background: #282828;
  color: #fff;
}

.topbar h1 {
  margin: 0;
  font-size: 18px;
}

.status {
  flex: 1;
  font-size: 13px;
}

.history-view,
.config-view {
  padding: 16px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.card {
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 14px;
}

.card h2 {
  margin: 0 0 8px;
  font-size: 14px;
  color: #555;
}

.search input,
.config-view input {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid #ccc;
  border-radius: 6px;
}

.pager {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
}

.list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.list li {
  display: flex;
  align-items: center;
  gap: 10px;
  background: #fff;
  border: 1px solid #ddd;
  border-radius: 8px;
  padding: 10px 12px;
  cursor: pointer;
}

.list li.selected {
  border-color: #6496dc;
  background: #dcebff;
}

.num {
  width: 24px;
  color: #666;
  font-weight: 600;
}

.body {
  flex: 1;
  min-width: 0;
}

.preview {
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.meta {
  font-size: 11px;
  color: #888;
  margin-top: 2px;
}

.empty {
  text-align: center;
  color: #888;
  padding: 32px 0;
}

.btn {
  border: none;
  border-radius: 6px;
  padding: 6px 12px;
  background: #e8e8e8;
  cursor: pointer;
}

.btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.btn.ghost {
  background: #464646;
  color: #fff;
}

.btn.primary {
  background: #198754;
  color: #fff;
}

.btn.danger {
  background: #dc3545;
  color: #fff;
}

.row {
  display: flex;
  gap: 8px;
  margin-top: 10px;
}

.config-view label {
  display: block;
  margin-top: 8px;
  font-size: 13px;
}

.psk {
  margin-top: 12px;
  padding: 10px;
  background: #f5f5f5;
  border-radius: 6px;
  word-break: break-all;
}

.error {
  color: #b00020;
}
</style>
