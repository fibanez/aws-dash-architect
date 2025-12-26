# <img src="dash-icon.png" width="160" height="160" align="left" padding="20" alt="Dash Architect Icon"> AWS Dash

See all your AWS accounts, all your regions, in one window.

<div align="right">
Powered by <a href="https://github.com/fibanez/stood">Stood Agent Library</a> <img src="https://github.com/fibanez/stood/raw/main/stood-icon.png" width="15" height="15">
</div>
<br clear="left">

> **Alpha Release**: AWS Dash is in active development. Features may change as we work toward stable release.

&nbsp;

AWS environments grow. Accounts multiply. Regions scatter. Finding what you need becomes a hunt across consoles and CLI sessions.

AWS Dash brings everything into one place. Browse resources across accounts and regions the way you browse files on your computer. No context switching. No tab sprawl.

&nbsp;

### Talk to your infrastructure

Describe what you want in plain English. AI agents handle the rest—finding resources, analyzing configurations, executing multi-step operations. They understand your environment and work across account boundaries.

&nbsp;

### Built for real AWS scale

- 93 services, nearly 200 resource types
- Direct AWS API integration with smart caching
- Works with AWS Identity Center

&nbsp;

## Get Started

Requires Rust 1.81+ and AWS Identity Center.

```bash
git clone https://github.com/aws/aws-dash-architect.git
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
