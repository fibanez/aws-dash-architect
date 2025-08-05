#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use clap::Parser;

/// AWS Dash Architect - AWS CloudFormation Template Editor
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Automatically trigger login on startup
    #[arg(long, env = "AWS_DASH_AUTO_LOGIN")]
    auto_login: bool,

    /// AWS Identity Center URL (e.g., https://mycompany.awsapps.com/start/)
    #[arg(long, env = "AWS_IDENTITY_CENTER_URL")]
    identity_url: Option<String>,

    /// AWS Region for Identity Center (e.g., us-east-1)
    #[arg(long, env = "AWS_IDENTITY_CENTER_REGION")]
    region: Option<String>,

    /// Default AWS role name to assume
    #[arg(long, env = "AWS_DEFAULT_ROLE")]
    role: Option<String>,
}

fn init_logging() {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
        let log_dir = proj_dirs.data_dir().join("logs");
        let _ = std::fs::create_dir_all(&log_dir);

        let log_path = log_dir.join("awsdash.log");
        // Initialize file logging with simple_logging
        let _ = simple_logging::log_to_file(&log_path, log::LevelFilter::Debug);
        // Create a file for tracing that's the same as the logging file
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path)
            .expect("Failed to open log file");

        // Configure tracing to use the same file as log
        // Enable MAXIMUM AWS SDK logging for debugging CloudFormation deployment issues
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::builder()
                    .parse("awsdash=trace,aws_sdk_cloudformation=trace,aws_sdk_bedrockruntime=trace,aws_config=trace,aws_sigv4=trace,aws_smithy_runtime=trace,aws_smithy_runtime_api=trace,hyper=trace,aws_smithy_http=trace,aws_endpoint=trace")
                    .expect("Failed to parse env filter")
            )
            .with_writer(move || file.try_clone().expect("Failed to clone file handle"))
            .with_ansi(false) // No ANSI colors in file
            .finish();

        // Set the global default subscriber
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");

        tracing::info!("Tracing initialized to log file: {:?}", log_path);
        tracing::info!(
            "MAXIMUM AWS SDK TRACE logging enabled for CloudFormation deployment troubleshooting"
        );
        tracing::warn!(
            "🚨 SECURITY WARNING: TRACE level logging may expose AWS credentials in logs"
        );
        eprintln!("Both logging and tracing going to: {:?}", log_path);
    }
}

fn main() -> eframe::Result {
    init_logging();

    // Parse command-line arguments
    let args = Args::parse();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 300.0])
            .with_min_inner_size([300.0, 220.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(&include_bytes!("../assets/icon-256.png")[..])
                    .expect("Failed to load icon"),
            ),
        ..Default::default()
    };

    eframe::run_native(
        "AWS Dash Architect",
        native_options,
        Box::new(move |cc| {
            // Install image loaders to support SVG and other image formats
            egui_extras::install_image_loaders(&cc.egui_ctx);

            // Create app with automation parameters
            let mut app = awsdash::DashApp::new(cc);
            
            // Set up automation parameters if provided
            if args.auto_login {
                app.set_auto_login(
                    args.identity_url,
                    args.region,
                    args.role,
                );
            }

            Ok(Box::new(app))
        }),
    )
}
