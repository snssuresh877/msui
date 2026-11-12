# msUI Local AI Manager & llama.cpp Manual

A high-performance, low-RAM local AI runtime manager built natively in Rust for **msUI** (Arch Linux / Hyprland), powered by **`llama.cpp`** (`llama-server`).

---

## 1. Why llama.cpp Instead of Ollama?

Traditional Ollama runs a continuous Go daemon in the background on boot, allocates uncompressed float context caches, and keeps heavy models resident in memory. On an **8 GB RAM laptop** (7.1 GB usable), this leads to ZRAM thrashing and UI stuttering.

The **msUI Local AI Manager** solves this with a strict zero-waste architecture:
1. **0 MB Idle RAM**: No background daemon. When stopped, `llama-server` is killed and 100% of RAM is returned to the OS.
2. **4-Bit Quantized Context (`-ctk q4_0 -ctv q4_0`)**: Slashes KV-cache memory usage by up to 70%.
3. **Flash Attention (`-fa`)**: Lowers memory bandwidth overhead on CPU.
4. **Hard Memory Caps**: Enforces a strict context window (default: 2,048 tokens).
5. **Memory Sentinel**: Checks available system memory via `/proc/meminfo` before launching any model. If available RAM is below the safety threshold, it refuses to launch to protect desktop responsiveness.
6. **OpenAI-Compatible API**: Serves a high-speed local HTTP API at `http://127.0.0.1:8080/v1` for any editor, browser extension, or script.

---

## 2. Model Storage Layout

Models are organized in standard GGUF format:

```text
~/.local/share/msui/ai/
├── models/
│   ├── qwen2.5-coder-1.5b-instruct-q4_k_m.gguf
│   └── qwen2.5-coder-0.5b-instruct-q4_k_m.gguf
├── config/
│   └── settings.toml
├── logs/
│   └── llama-server.log
└── runtime.json
```

---

## 3. Recommended Models for 8 GB RAM

| Model Preset | Size on Disk | Active RAM | Best For |
|:---|:---:|:---:|:---|
| **`qwen-0.5b`** | **380 MB** | **~600 MB** | Fast command-line queries, scripts, minimal resource usage |
| **`qwen-1.5b`** *(Recommended)* | **980 MB** | **~1.2 GB** | Balanced coding, general reasoning, safe on 8 GB RAM |
| **`qwen-3b`** | **1.9 GB** | **~2.1 GB** | Complex single-shot tasks (requires at least 3 GB free RAM) |

---

## 4. CLI Commands & Quick Reference

### Check Runtime & System RAM Status
```bash
msui ai status
# or simply:
ai status
```

### Download a Model
Download using built-in presets (accelerated with multi-connection `aria2c` / `curl`):
```bash
# Recommended for 8GB RAM:
msui ai pull qwen-1.5b

# Ultra-lightweight (380 MB):
msui ai pull qwen-0.5b

# Direct link from Hugging Face:
msui ai pull https://huggingface.co/TheBloke/TinyLlama-1.1B-Chat-v1.0-GGUF/resolve/main/tinyllama-1.1b-chat-v1.0.Q4_K_M.gguf
```

### List Installed Models
```bash
msui ai models
```

### Start & Stop Local AI Engine
```bash
# Start default model
msui ai start

# Start specific model
msui ai start qwen2.5-coder-1.5b

# Stop runtime and reclaim 100% RAM
msui ai stop

# Restart runtime
msui ai restart
```

### Delete a Model
```bash
msui ai delete qwen-3b
```

### Ask a Question from Terminal
```bash
msui ai chat "How do I reverse a string in Rust without allocating?"
```

---

## 5. Integrating with Editors & Applications

`llama-server` provides an **OpenAI-compatible** endpoint while running:

* **Base URL**: `http://127.0.0.1:8080/v1`
* **API Key**: `not-needed` (or any string)
* **Model Name**: Use the installed model name (or `default`)

### Testing via curl
```bash
curl http://127.0.0.1:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{"role": "user", "content": "Hello!"}],
    "temperature": 0.7
  }'
```

---

## 6. Configuration Reference (`settings.toml`)

Stored at `~/.local/share/msui/ai/config/settings.toml`:

```toml
port = 8080
host = "127.0.0.1"
context_size = 2048
cpu_threads = 4
auto_unload_mins = 5
min_free_ram_mb = 1200
quantize_kv = true
flash_attention = true
default_model = "qwen2.5-coder-1.5b-instruct-q4_k_m"
```
