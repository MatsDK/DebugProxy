# DebugProxy

A MITM proxy for intercepting and modifying HTTP/HTTPS traffic. 
<img width="1666" height="980" alt="image" src="https://github.com/user-attachments/assets/20804a37-52f5-4c32-8e25-4818dc032f4c" />

## Features

- **HTTPS Interception**: Generates a custom Root CA to decrypt SSL traffic.
- **Request/Response Scripting**: Use JavaScript to modify traffic on the fly. Scripts have access to request/response headers and bodies.
- **SSL Bypass**: Define host patterns to bypass decryption for specific domains.
- **Traffic History**: Capture and inspect request/response data including timing and size.
- **Config Management**: Backup and restore all settings, including proxy rules and scripts, to JSON files.

## Installation

### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Node.js](https://nodejs.org/) & `pnpm`

### Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/MatsDK/DebugProxy.git
   ```

2. Install dependencies:
   ```bash
   pnpm install
   ```

3. Run:
   ```bash
   pnpm tauri dev
   ```

## Usage

1. **Root CA**: Click "Settings" -> "Download Root CA" and install it in your system/browser trust store to intercept HTTPS calls.

## Mobile Usage

1. **Proxy Settings**:
   - Find your computer's local IP address (displayed in the DebugProxy header).
   - On your mobile device, open WiFi settings and set the **HTTP Proxy** to **Manual**.
   - Enter your computer's IP and the proxy port (default: `8080`).

2. **Install Certificate**:
   - Ensure the DebugProxy is running on your computer. You should be able to see `CONNECT` requests from your mobile device in the traffic history.
   - Open a browser on your mobile device and navigate to `http://proxy.local`.
   - Download the CA certificate.

3. **Trust the Certificate**:
   - **iOS**: 
     1. Open the downloaded profile in *Settings > Profile Downloaded* and click **Install**.
     2. Go to *Settings > General > About > Certificate Trust Settings*. 
     3. Enable full trust for "DebugProxy CA".
   - **Android**: Go to *Settings > Security > Advanced > Encryption & credentials > Install from storage > CA certificate*. Select the downloaded file.

## Planned Features

- [ ] **TypeScript Scripting**: Type-safe request/response modification.
- [ ] **Breakpoints**: Pause traffic for manual inspection and modification.
- [ ] **VPN Mode**: System-wide proxying via local VPN interface.
- [ ] **WebSocket Support**: Intercept and modify WS/WSS frames.
- [ ] **Protobuf Support**: Automatic decoding/encoding for gRPC traffic.

## Contributing

Contributions are welcome. Please keep pull requests small and focused on a single feature or fix. 

## License

This project is licensed under the MIT License.
