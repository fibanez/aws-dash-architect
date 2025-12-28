# <img src="dash-icon.png" width="160" height="160" align="left" padding="20" alt="Dash Architect Icon"> AWS Dash

See all your AWS accounts, all your regions, in one window.

<br clear="left"></br>
&nbsp;
<p></p>

> **Alpha Release**: AWS Dash is in active development. Features may change as we work toward stable release.

&nbsp;

AWS environments grow. Accounts multiply. Regions scatter. Finding what you need becomes a hunt across consoles and CLI sessions.

AWS Dash brings everything into one place. Browse resources across accounts and regions the way you browse files on your computer. No context switching. No tab sprawl.

&nbsp;

### Talk to your infrastructure

Describe what you want in plain English. AI agents handle the rest—finding resources, analyzing configurations, executing multi-step operations. They understand your environment and work across account and region boundaries.

&nbsp;

### Built for real AWS scale

- 92 AWS services, 174 resource types
- Direct AWS API integration with smart caching
- Works with AWS Identity Center

&nbsp;

## Download

Pre-built binaries for Linux, Windows, and macOS:

**[Download Latest Release](https://github.com/fibanez/aws-dash-architect/releases/latest)**

### Installation

**macOS:** Extract the zip, then remove the quarantine attribute before first run:
```bash
xattr -cr AWSDash.app
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
git clone https://github.com/fibanez/aws-dash-architect.git
cd aws-dash-architect
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
