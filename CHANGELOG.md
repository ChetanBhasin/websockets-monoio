# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-10-23

### Added
- Initial release of websockets-monoio
- High-performance WebSocket client built on monoio runtime
- Support for both `ws://` and `wss://` (TLS) connections
- Zero-copy operations where possible
- Auto-pong and auto-close handling
- Custom header support for authentication
- Comprehensive error handling with thiserror
- Full io_uring support on Linux
- Cross-platform compatibility (Linux, macOS, Windows)

### Features
- **WsClient**: Main client struct for WebSocket connections
- **TLS Support**: Secure connections via monoio-rustls
- **URL Parsing**: Support for ws:// and wss:// schemes
- **HTTP Upgrade**: Complete WebSocket handshake implementation
- **Stream Abstraction**: Unified interface for plain TCP and TLS streams

### Dependencies
- monoio 0.2 (async runtime with io_uring)
- fastwebsockets-monoio 0.10 (WebSocket protocol implementation)
- rustls 0.23 (TLS implementation)
- monoio-rustls 0.4 (TLS integration for monoio)
- And other essential dependencies for networking and crypto

[Unreleased]: https://github.com/ChetanBhasin/websockets-monoio/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/ChetanBhasin/websockets-monoio/releases/tag/v0.1.0