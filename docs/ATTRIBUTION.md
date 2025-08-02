# Attribution and Licensing

This document provides attribution information for third-party assets and resources used in this project.

## AWS Architecture Icons

This application uses AWS Architecture Icons provided by Amazon Web Services, Inc. and its affiliates.

**Source**: [AWS Architecture Icons](https://aws.amazon.com/architecture/icons/)  
**License**: Used under AWS's permitted usage terms for customers creating architecture diagrams  
**Usage**: CloudFormation resource visualization and UI elements  
**Version**: Architecture Icons Release 18 (February 2024)

### Icon Categories Used

- **Architecture Service Icons (16px)**: 117 icons from various AWS service categories
- **Architecture Group Icons (32px)**: 1 icon (AWS Cloud default)  
- **Resource Icons (48px)**: 16 specific resource type icons

### Permitted Usage

According to [AWS Architecture Icons page](https://aws.amazon.com/architecture/icons/), AWS allows customers and partners to use these toolkits and assets to create architecture diagrams and incorporate them into:

- Architecture diagrams
- Whitepapers  
- Presentations
- Data sheets
- Posters

### Icon Management

Icons are automatically matched to CloudFormation resource types using the `generate_resource_icons.py` script, which maps AWS service names to corresponding icon files. The application maintains only the subset of icons actually referenced in the codebase (134 out of 3,971 total icons) for efficiency.

### Updates

AWS releases architecture icon packages quarterly. This application may be updated periodically to include new icons for newly supported AWS services.

---

**Note**: For questions about licensing obligations for commercial applications, AWS recommends contacting trademarks@amazon.com.