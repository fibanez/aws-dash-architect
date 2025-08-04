# <img src="dash-icon.png" width="160" height="160" align="left" padding="20" alt="Dash Architect Icon"> AWS Dash Architect

**A unified desktop environment for architecting compliant AWS solutions**
<div align="right">
Powered by <a href="https://github.com/fibanez/stood">Stood Agent Library</a> <img src="https://github.com/fibanez/stood/raw/main/stood-icon.png" width="15" height="15">
</div>
<br clear="left">
&nbsp;<br>
AWS Dash Architect transforms the fragmented workflow of cloud architecture design by bringing together specialized AI agents, real-time compliance validation using CloudFormation Guard and CloudFormation template validation against official AWS schemas, easy cross-account resource management with multi-account and multi-region capabilities, and seamless deployment into a single desktop experience.
&nbsp;<br>

# <img src="aws-dash-architect-1.png" width="800" align="center" padding="20" alt="AWS Dash Architect Desktop"> 
> ⚠️ **Alpha Release**: AWS Dash Architect is currently in active development. Features and APIs may change as we work toward the first stable release.

&nbsp;<br>
## 📚 Documentation

- **[User Guide](docs/userguide/)**: Incomplete user documentation
- **[Technical Documentation](docs/technical/)**: Architecture and development guides

<br clear="left">

## 🚀 Quick Start

### Installation

**This is a Rust project requiring Rust 1.81 or later.**

#### Install Rust (if not already installed)
- **All platforms**: Visit [rustup.rs](https://rustup.rs/) and follow the installation instructions
- **Linux/macOS**: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Windows**: Download and run [rustup-init.exe](https://win.rustup.rs/), install Build Tools for Visual Studio with C++ option, cmake, and NASM

#### Build from source
```bash
git clone https://github.com/aws/aws-dash-architect.git
cd aws-dash-architect
cargo build --release
```

#### Run the application
- **Linux/macOS**: `./target/release/awsdash`
- **Windows**: `.\target\release\awsdash.exe`

### Prerequisites

⚠️ **AWS Identity Center is Required**: AWS Dash Architect requires AWS Identity Center (formerly AWS SSO) with specific permission sets and roles configured before first use.

### Setup

1. **[Setup IAM Identity Center](docs/userguide/setup-iam-identity-center.md)** - Configure AWS Identity Center with required permissions
2. **[Login to AWS Dash Architect](docs/userguide/login-aws-dash-architect.md)** - Complete first launch and authentication

## ✨ Key Features

### 🤖 Specialized AI Agent Team 🚧 *Under Development*

Three specialized AI agents will work together to create compliant architectures:

- **🎯 Operations Agent**: Helps interact with AWS environment for operations related tasks
- **🔨 Compliance Agent**: Updates cloudformation templates with compliance violations
- 💰 **Pricing Agent**: Helps generate pricing estimates from CloudFormation templates
- **🏛️ Architect Agent**: Designs solutions following AWS best practices and compliance standards

> **Development Status**: The specialized AI agent architecture is currently being implemented using the Stood Agent Library framework. This will enable natural language architecture design with intelligent collaboration between specialized agents.

### 🛡️ Proactive Compliance Verification 🚧 *Under Development*

Built-in compliance validation against 30+ regulatory frameworks:

- **FedRAMP** (Low and Moderate)
- **CMMC** (Levels 1-5)
- **NIST** (800-53, 800-171, 800-172, CSF)
- **HIPAA**, **PCI DSS**, **SOX**
- **International standards** (ACSC, APRA, ENS, and more)

Compliance issues will be caught during design—not after deployment.

> **Development Status**: Compliance verification will be implemented using [AWS CloudFormation Guard](https://github.com/aws-cloudformation/cloudformation-guard), an open-source policy-as-code evaluation tool. This will enable real-time validation of CloudFormation templates against regulatory frameworks and custom organizational policies during the architecture design process.

### 🌐 Cross-Account AWS Explorer ✅ *Complete*

Unified resource discovery and visualization:

- **Multi-Account and Multi-Region Visibility**: See all your AWS resources across accounts and regions in one interface
- **Resource Discovery**: Find existing resources to incorporate into new designs
- **Service Availability**: Verify service availability by region before designing
- **Parameter Discovery**: Extract configuration details from existing resources

> **Status**: The AWS Explorer is fully functional and provides comprehensive cross-account resource discovery with real-time caching and efficient API usage.

### 📊 Advanced Architecture Visualization 🚧 *Under Development*

Automatic diagram generation that will show:

- **New Architecture Components**: Your designed solution
- **Existing Resource Integration**: How new resources connect to existing infrastructure
- **Cross-Account Relationships**: Dependencies spanning multiple AWS accounts
- **Compliance Boundaries**: Visual indicators of security and compliance zones

> **Development Status**: Advanced visualization features are being developed to automatically generate comprehensive architecture diagrams based on CloudFormation templates and existing resource relationships. This will include interactive diagrams with zoom, filtering, and compliance overlay capabilities.

### ⚡ From POC to Production in Days, Not Months

Unlike traditional approaches where you build POCs with console clicks and then rebuild everything:

- **Compliant from Day One**: All prototyping generates production-ready, compliant infrastructure
- **Infrastructure as Code**: Every experiment automatically creates proper CloudFormation templates
- **No Step Backward**: Eliminates the traditional "rebuild for production" phase
- **75% Faster Time-to-Market**: Move directly from validated POC to production deployment

## 🎯 Who Should Use AWS Dash Architect?

### Primary Users
- **Cloud Architects** designing compliant AWS solutions
- **AWS Solutions Architects** 
- **AWS Professional Services** consultants
- **Innovation Teams** building POCs and MVPs
- **DevOps Engineers** managing multi-account environments

### Use Cases
- **Rapid Prototyping**: Build compliant POCs without sacrificing production readiness
- **Compliance Audits**: Validate architectures against regulatory requirements
- **Multi-Account Management**: Design solutions spanning multiple AWS accounts
- **Cost Optimization**: Accurate cost estimation during the design phase
- **Architecture Documentation**: Generate diagrams and documentation automatically

## 📄 License

AWS Dash Architect is licensed under the [Apache License 2.0](LICENSE).

---

**Transform your AWS architecture workflow. Design compliant. Deploy confidently. Ship faster.**

*AWS Dash Architect - Where compliance meets velocity.*
