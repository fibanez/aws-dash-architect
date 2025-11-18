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
        // Enable Stood agent library debugging for agent execution visibility (can be toggled via UI)
        let filter = tracing_subscriber::EnvFilter::builder()
            .parse("awsdash=trace,stood=trace,aws_sdk_cloudformation=trace,aws_sdk_bedrockruntime=trace,aws_config=trace,aws_sigv4=trace,aws_smithy_runtime=trace,aws_smithy_runtime_api=trace,hyper=trace,aws_smithy_http=trace,aws_endpoint=trace")
            .expect("Failed to parse env filter");

        let (filter, reload_handle) = tracing_subscriber::reload::Layer::new(filter);

        let subscriber = tracing_subscriber::registry()
            .with(filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_writer(move || file.try_clone().expect("Failed to clone file handle"))
                    .with_ansi(false) // No ANSI colors in file
            );

        // Set the global default subscriber
        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");

        // Store reload handle for dynamic toggling (in lib.rs)
        awsdash::set_tracing_reload_handle(reload_handle);

        tracing::info!("Tracing initialized to log file: {:?}", log_path);
        tracing::info!(
            "MAXIMUM AWS SDK TRACE logging enabled for CloudFormation deployment troubleshooting"
        );
        tracing::warn!(
            "🚨 SECURITY WARNING: TRACE level logging may expose AWS credentials in logs"
        );
        eprintln!("Both logging and tracing going to: {:?}", log_path);

        // Note: stood library debug traces are already captured by the tracing subscriber above
        // with stood=trace enabled. All stood internal operations, tool executions, and
        // agent lifecycle events are logged to the same file.
        // The RUST_LOG environment variable controls the verbosity (stood=trace for full debug).
        tracing::info!(
            "Stood agent framework logging enabled via tracing subscriber (RUST_LOG controls verbosity)"
        );
    }
}

fn main() -> eframe::Result {
    init_logging();

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
        "AWS Dash Architect",
        native_options,
        Box::new(|cc| {
            // Install image loaders to support SVG and other image formats
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Ok(Box::new(awsdash::DashApp::new(cc)))
        }),
    )
}
