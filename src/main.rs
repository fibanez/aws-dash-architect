#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use tracing_subscriber::prelude::*;

fn init_logging() {
    // Check if tokio-console profiling is requested
    // To enable: TOKIO_CONSOLE=1 RUSTFLAGS="--cfg tokio_unstable" cargo run
    let use_tokio_console = std::env::var("TOKIO_CONSOLE")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false);

    if use_tokio_console {
        // Initialize tokio-console for async task profiling
        console_subscriber::init();
        eprintln!("tokio-console profiling enabled - connect with: tokio-console");
        eprintln!("NOTE: File logging disabled when using tokio-console");
        return;
    }

    // Standard file-based logging
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
        // Stood agent library set to INFO by default (use RUST_LOG=stood=debug for more detail)
        let filter = tracing_subscriber::EnvFilter::builder()
            .parse("awsdash=info,stood=info,aws_sdk_cloudformation=info,aws_sdk_bedrockruntime=info,aws_config=warn,aws_sigv4=warn,aws_smithy_runtime=warn,aws_smithy_runtime_api=warn,hyper=warn,aws_smithy_http=warn,aws_endpoint=warn")
            .expect("Failed to parse env filter");

        let (filter, reload_handle) = tracing_subscriber::reload::Layer::new(filter);

        let subscriber = tracing_subscriber::registry().with(filter).with(
            tracing_subscriber::fmt::layer()
                .with_writer(move || file.try_clone().expect("Failed to clone file handle"))
                .with_ansi(false), // No ANSI colors in file
        );

        // Set the global default subscriber
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");

        // Store reload handle for dynamic toggling (in lib.rs)
        awsdash::set_tracing_reload_handle(reload_handle);

        tracing::info!("Tracing initialized to log file: {:?}", log_path);
        tracing::info!("Log levels: awsdash=info, stood=info, AWS SDKs=info/warn");
        tracing::info!("To increase verbosity: RUST_LOG=stood=debug cargo run");
        eprintln!("Both logging and tracing going to: {:?}", log_path);

        // Note: stood library logs at INFO level by default
        // Set RUST_LOG=stood=debug or stood=trace for more detailed agent execution logs
        tracing::info!(
            "Stood agent framework logging enabled (set RUST_LOG=stood=debug for verbose output)"
        );
    }
}

fn main() -> eframe::Result {
    init_logging();

    // Clean up old agent log files (keep 50 most recent)
    match awsdash::app::agent_framework::AgentLogger::cleanup_old_logs(50) {
        Ok(deleted) if deleted > 0 => {
            tracing::info!("Cleaned up {} old agent log files", deleted);
        }
        Ok(_) => {} // No files deleted
        Err(e) => {
            tracing::warn!("Failed to clean up old agent logs: {}", e);
        }
    }

    // Clean up old debug log files (keep 20 most recent)
    match awsdash::app::agent_framework::AgentLogger::cleanup_old_debug_logs(20) {
        Ok(deleted) if deleted > 0 => {
            tracing::info!("Cleaned up {} old debug log files", deleted);
        }
        Ok(_) => {}
        Err(e) => {
            tracing::warn!("Failed to clean up old debug logs: {}", e);
        }
    }

    // Initialize V8 platform (required for JavaScript execution)
    awsdash::app::agent_framework::initialize_v8_platform()
        .expect("Failed to initialize V8 platform");

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
        "AWS Dash",
        native_options,
        Box::new(|cc| {
            // Install image loaders to support SVG and other image formats
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(awsdash::DashApp::new(cc)))
        }),
    )
}
