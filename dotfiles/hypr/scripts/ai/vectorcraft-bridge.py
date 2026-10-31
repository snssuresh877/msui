#!/usr/bin/env python3
"""
msUI VectorCraft AI Bridge
Translates natural language user prompts into VectorCraft JSON-RPC control protocol commands
using local Ollama inference (qwen2.5-coder:1.5b).
"""

import sys
import json
import socket
import urllib.request
import urllib.error

OLLAMA_URL = "http://127.0.0.1:11434/api/chat"
MODEL = "qwen2.5-coder:1.5b"
VECTORCRAFT_PORT = 7979

SYSTEM_PROMPT = """You are an AI assistant controlling VectorCraft (an open-source vector graphics design application).
Translate the user's design request into VectorCraft JSON-RPC control commands.
VectorCraft supports JSON-lines methods over port 7979:
1. Method "engine.execute" with params {"command": "<command_id>", "params": {<args>}}
   Common commands:
   - "shape.rectangle": {"x": 100, "y": 100, "width": 200, "height": 120, "rx": 10, "ry": 10}
   - "shape.ellipse": {"cx": 200, "cy": 200, "rx": 80, "ry": 80}
   - "shape.line": {"x1": 50, "y1": 50, "x2": 250, "y2": 250}
   - "shape.star": {"cx": 200, "cy": 200, "points": 5, "inner_radius": 30, "outer_radius": 70}
   - "style.fill": {"color": "#ff3366"}
   - "style.stroke": {"color": "#000000", "width": 2}
   - "document.export": {"format": "svg", "path": "/path/to/out.svg"}
2. Method "ui.screenshot" with params {"path": "/tmp/vectorcraft_shot.png"}
3. Method "document.inspect" with params {}

Output ONLY valid JSON or a list of JSON objects (one per line) matching the VectorCraft control protocol.
Do NOT output markdown backticks or commentary."""

def send_to_vectorcraft(command_json: dict) -> dict:
    try:
        s = socket.create_connection(("127.0.0.1", VECTORCRAFT_PORT), timeout=3.0)
        line = json.dumps(command_json) + "\n"
        s.sendall(line.encode("utf-8"))
        res = s.recv(4096).decode("utf-8")
        s.close()
        return json.loads(res.strip()) if res.strip() else {"status": "ok"}
    except Exception as e:
        return {"error": str(e)}

def prompt_ai(user_input: str) -> str:
    payload = {
        "model": MODEL,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": user_input}
        ],
        "stream": False
    }
    req = urllib.request.Request(
        OLLAMA_URL,
        data=json.dumps(payload).encode("utf-8"),
        headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(req, timeout=30.0) as resp:
        result = json.loads(resp.read().decode("utf-8"))
        return result.get("message", {}).get("content", "").strip()

def main():
    if len(sys.argv) < 2:
        print("Usage: vectorcraft-bridge.py <natural language prompt>")
        sys.exit(1)

    prompt = " ".join(sys.argv[1:])
    print(f"🤖 Querying {MODEL} for: '{prompt}'...")

    try:
        ai_reply = prompt_ai(prompt)
    except Exception as e:
        print(f"❌ Error communicating with local AI engine: {e}")
        print("Ensure local AI engine is running: ~/.config/hypr/scripts/ai/ai-control.sh start")
        sys.exit(1)

    print("\n📦 AI Generated VectorCraft Command:")
    print(ai_reply)

    # Clean markdown if present
    clean_json = ai_reply.strip()
    if clean_json.startswith("```"):
        lines = clean_json.splitlines()
        clean_json = "\n".join(lines[1:-1])

    # Try sending to VectorCraft
    try:
        cmd = json.loads(clean_json)
        print("\n🚀 Executing on VectorCraft (port 7979)...")
        result = send_to_vectorcraft(cmd)
        print("Result:", json.dumps(result, indent=2))
    except json.JSONDecodeError:
        print("\n⚠️  AI response was not raw JSON. Lines:")
        for line in clean_json.splitlines():
            line = line.strip()
            if line.startswith("{") and line.endswith("}"):
                try:
                    c = json.loads(line)
                    res = send_to_vectorcraft(c)
                    print(f"Executed: {line} -> {res}")
                except Exception as ex:
                    print(f"Failed line: {line} ({ex})")

if __name__ == "__main__":
    main()
