# UFW Firewall & Network Security Manual

A practical guide to workstation network security, firewall configuration, and local service isolation on **msUI** (Arch Linux / Hyprland).

---

## 1. Security Architecture & Philosophy

The msUI desktop environment follows a **hardened workstation security model**:

1. **Default Deny Incoming (`DROP`)**:
   All unsolicited incoming packets from any network are immediately dropped by the kernel netfilter. Your computer is completely invisible to port scanners on public or untrusted Wi-Fi.
2. **Default Allow Outgoing (`ACCEPT`)**:
   All outbound client connections (web browsing, downloading, git pushes, package updates) are permitted without hindrance.
3. **Default Deny Routed (`DROP`)**:
   Packet forwarding between interfaces is disabled to prevent accidental routing.
4. **LAN-Scoped Service Exposure**:
   Services designed for local device-to-device communication (such as **LocalSend** and **msUI File Share**) are explicitly restricted to your local network subnet (e.g. `192.168.1.0/24`). They are **never** opened to the public internet or globally routable IPv6.
5. **Rate-Limited Remote Access**:
   SSH (port 22) is rate-limited. If an external host attempts 6 or more connections within 30 seconds, UFW blocks that IP address to prevent brute-force credential stuffing.

---

## 2. Port & Service Mapping

| Service | Protocol | Port(s) | Allowed From | Purpose |
|:---|:---:|:---:|:---:|:---|
| **LocalSend (Transfer)** | TCP | `53317` | LAN Subnet only | File transfer payload & HTTP API |
| **LocalSend (Discovery)** | UDP | `53317` | LAN Subnet only | Multicast device discovery on Wi-Fi |
| **msui-fshare** | TCP | `43821:43830` | LAN Subnet only | High-speed phone-to-PC file transfer |
| **SSH** | TCP | `22` | Rate-limited | Remote terminal administration |
| **Web (HTTP/HTTPS)** | TCP | `80`, `443` | **BLOCKED** | Closed by default on workstation |
| **Ollama AI** | TCP | `11434` | **Localhost only** | Bound to `127.0.0.1` (no network access) |
| **Aria2 RPC** | TCP | `6800` | **Localhost only** | Bound to `127.0.0.1` (browser extension RPC) |

> [!NOTE]
> Services bound to `127.0.0.1` (like Ollama and Aria2 RPC) cannot be accessed over the network regardless of firewall rules because the operating system only accepts loopback traffic for them.

---

## 3. Quick Setup & One-Shot Hardening

To automatically install UFW, detect your current Wi-Fi subnet, and deploy the hardened configuration:

```bash
firewall-setup
```

*(Alternatively, run `$HOME/Projects/msui/dotfiles/ufw/setup-firewall.sh`)*

---

## 4. Practical Everyday Commands

### View Firewall Status
```bash
# Detailed view with policies and rule comments
sudo ufw status verbose

# Numbered view (essential for deleting specific rules)
sudo ufw status numbered
```

### Allowing a Temporary Development Port
When building web applications (e.g. Vite on 5173, Next.js on 3000, Django on 8000):

```bash
# Allow only from your local Wi-Fi subnet (RECOMMENDED):
sudo ufw allow from 192.168.1.0/24 to any port 3000 proto tcp comment 'Dev: Next.js'
sudo ufw allow from 192.168.1.0/24 to any port 5173 proto tcp comment 'Dev: Vite'

# Allow from any machine (use with caution on public Wi-Fi):
sudo ufw allow 3000/tcp comment 'Dev: Next.js public'
```

### Deleting a Rule
The easiest way is to use the rule number:
```bash
# 1. List numbered rules
sudo ufw status numbered

# 2. Delete by index (e.g., rule number 4)
sudo ufw delete 4
```

Or delete by exact rule specification:
```bash
sudo ufw delete allow 3000/tcp
```

### Restricting Access to a Specific Device
To allow only a specific IP (e.g. your phone at `192.168.1.15`) access to a port:
```bash
sudo ufw allow from 192.168.1.15 to any port 8080 proto tcp comment 'Phone testing only'
```

---

## 5. Live Monitoring & Troubleshooting

### Inspecting Blocked Connections in Real Time
UFW logs all dropped connection attempts to the system journal:

```bash
# Watch live blocked packets
sudo journalctl -u ufw -f

# Or search recent dropped packets in kernel dmesg
sudo dmesg | grep '[UFW BLOCK]' | tail -n 20
```

### Verifying Active Listening Ports vs Firewall
To see what services are actively listening on your PC:
```bash
ss -tuln
```
* If the `Local Address` column says `127.0.0.1:<port>`, it is private to this machine.
* If it says `0.0.0.0:<port>` or `*:<port>`, it is listening on all network interfaces and protected by UFW.

---

## 6. Emergency Reset

If you ever need to reset your firewall back to clean default msUI protection:

```bash
# Option 1: Re-run automated setup script
firewall-setup

# Option 2: Manual reset
sudo ufw --force reset
sudo ufw default deny incoming
sudo ufw default allow outgoing
sudo ufw default deny routed
sudo ufw allow from 192.168.1.0/24 to any port 43821:43830 proto tcp
sudo ufw allow from 192.168.1.0/24 to any port 53317 proto tcp
sudo ufw allow from 192.168.1.0/24 to any port 53317 proto udp
sudo ufw limit 22/tcp
sudo ufw --force enable
```
