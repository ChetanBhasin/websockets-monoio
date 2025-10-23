# Security Policy

## Supported Versions

We support the latest version of websockets-monoio with security updates.

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability, please send an email to [chetan@example.com](mailto:chetan@example.com) instead of opening a public issue.

Please include:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact
- Any suggested fixes

We will acknowledge receipt within 48 hours and provide a detailed response within 5 business days.

## Known Advisory Status

### RUSTSEC-2025-0057 (fxhash unmaintained)

**Status**: Acknowledged, not a security vulnerability
**Impact**: Low - maintenance status only
**Mitigation**: This is a transitive dependency from `monoio` that is used for internal hash operations. The advisory indicates the crate is no longer maintained but does not represent a security vulnerability.

**Resolution Plan**:
1. Monitor monoio updates for migration to `rustc-hash` or alternative
2. Consider contributing to monoio if needed
3. No immediate action required as this is not a security issue

The advisory is ignored in our CI pipeline with the specific flag `--ignore RUSTSEC-2025-0057` to prevent false positive failures while maintaining vigilance for actual security vulnerabilities.

## Dependencies

We regularly audit our dependencies for security vulnerabilities using:
- `cargo audit` in CI/CD
- Dependabot for automated updates
- Manual review of security advisories

## Security Best Practices

When using websockets-monoio:
- Always use TLS (`wss://`) for production connections
- Validate and sanitize all incoming WebSocket data
- Implement proper authentication and authorization
- Use connection timeouts and rate limiting
- Keep dependencies updated