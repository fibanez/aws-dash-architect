# <img src="dash-icon.png" width="160" height="160" align="left" padding="20" alt="Dash Architect Icon"> AWS Dash

All your AWS accounts. All your regions. One place.

<br clear="left"></br>
&nbsp;
<p></p>

> **Alpha Release**: AWS Dash is in active development. Features may change as we work toward stable release.

[![Release](https://img.shields.io/github/v/release/fibanez/aws-dash?include_prereleases&style=for-the-badge)](https://github.com/fibanez/aws-dash/releases/latest)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg?style=for-the-badge)](/LICENSE-APACHE)
&nbsp;

AWS environments grow. Accounts multiply. Regions scatter. Finding what you need becomes a hunt across consoles and CLI sessions.

AWS Dash brings everything into one place. Browse resources across accounts and regions. Build pages that fit your workflows. No context switching. No tab sprawl.

&nbsp;

### Make it yours

Create dashboards tailored to your environment. Track what matters to your team. Automate your specific operations. Generic tools serve everyone the same way. Dash becomes yours.

&nbsp;

### Talk to your infrastructure

Describe what you want in plain English. AI agents handle the rest—finding resources, analyzing configurations, executing multi-step operations. They understand your environment and work across account and region boundaries.

&nbsp;

### Built for real AWS scale

- 82 AWS services, 177 resource types
- Direct AWS API integration with smart caching
- Works with AWS Identity Center

&nbsp;

## Download

Pre-built binaries for Linux, Windows, and macOS:

**[Download Latest Release](https://github.com/fibanez/aws-dash/releases/latest)**

### Installation

**macOS:** Extract the zip, then remove the quarantine attribute before first run:
```bash
xattr -c AWSDash.app
```
Then double-click AWSDash.app or drag it to Applications.

**Linux (AppImage):** Download, make executable, and run:
```bash
chmod +x AWSDash-x86_64.AppImage
./AWSDash-x86_64.AppImage
```

**Windows:** Extract the zip and run `awsdash.exe`.

&nbsp;

## Build from Source

For the latest development version or to compile yourself. Requires Rust 1.81+.

```bash
git clone https://github.com/fibanez/aws-dash.git
cd aws-dash
cargo build --release
./target/release/awsdash
```

&nbsp;

## Documentation

- [User Guide](docs/userguide/)
- [Technical Docs](docs/technical/)
- [Changelog](docs/CHANGELOG.md)
- [Setup Identity Center](docs/userguide/setup-iam-identity-center.md)

&nbsp;

## License

[Apache License 2.0](LICENSE)
