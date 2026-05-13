# DebugProxy

A MITM proxy for intercepting and modifying HTTP/HTTPS traffic. 
<img width="1666" height="980" alt="image" src="https://github.com/user-attachments/assets/20804a37-52f5-4c32-8e25-4818dc032f4c" />

## Features

- **HTTPS Interception**: Generates a custom Root CA to decrypt SSL traffic.
- **Request/Response Scripting**: Use JavaScript to modify traffic on the fly. Scripts have access to request/response headers and bodies.
- **Breakpoints**: Use `await proxy.breakpoint()` in your scripts to pause traffic and manually modify it in the Interceptor UI.
- **SSL Bypass**: Define host patterns to bypass decryption for specific domains.
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
   - On your mobile device, go to **Wi-Fi Settings** and tap your network.
   - Set **HTTP Proxy** to **Manual**.
   - Enter your computer's **Local IP** (shown in the app header) and port (default: `8080`).

2. **Download Certificate**:
   Open a browser on your device to access the setup page:
   
   - **Android**: Navigate to `http://proxy.local` (or `http://<YOUR_COMPUTER_IP>:8080/proxy.local`)
   - **iOS**: You **must** use the full IP address: `http://<YOUR_COMPUTER_IP>:8080/proxy.local`
   
   *(Note: iOS does not resolve `proxy.local` correctly during the initial setup phase. Always use the IP address shown in the app header for iOS.)*
   
    Tap **Download Certificate** to save the `.crt` file.

3. **Trust the Certificate**:
   - **iOS**:
     1. Open **Settings** > **Profile Downloaded** and tap **Install**.
     2. Go to **Settings** > **General** > **About** > **Certificate Trust Settings**.
     3. Enable full trust for **Debug Proxy Root CA**.
   - **Android**: Go to **Settings** > **Security** > **Advanced** > **Encryption & credentials** > **Install from storage** > **CA certificate** and select the file.

## Planned Features

- [ ] **TypeScript Scripting**: Type-safe request/response modification.
- [ ] **VPN Mode**: System-wide proxying via local VPN interface.
- [ ] **WebSocket Support**: Intercept and modify WS/WSS frames.
- [ ] **Protobuf Support**: Automatic decoding/encoding for gRPC traffic.
- [ ] **Session Export**: Save and load captured traffic sessions (HAR format support).
- [ ] **Device Management** 
- [ ] **Desktop Proxy**

## Contributing

Contributions are welcome. Please keep pull requests small and focused on a single feature or fix. 

## License

This project is licensed under the MIT License.
