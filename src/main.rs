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

        // Create a file for tracing output
        let file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&log_path)
            .expect("Failed to open log file");

        // Configure tracing with unified formatting for all logs
        // Stood agent library set to INFO by default (use RUST_LOG=stood=debug for more detail)
        // GUI framework (eframe, egui, wgpu) logs are also captured via tracing-log bridge
        let filter = tracing_subscriber::EnvFilter::builder()
            .parse("awsdash=info,stood=info,eframe=info,egui=warn,wgpu=warn,wgpu_core=warn,wgpu_hal=warn,naga=warn,winit=warn,aws_sdk_cloudformation=info,aws_sdk_bedrockruntime=info,aws_config=warn,aws_sigv4=warn,aws_smithy_runtime=warn,aws_smithy_runtime_api=warn,hyper=warn,aws_smithy_http=warn,aws_endpoint=warn")
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

        // Bridge log crate events to tracing (for eframe, egui, glow, etc.)
        // This must be done AFTER setting the tracing subscriber
        tracing_log::LogTracer::init().expect("Failed to initialize log-to-tracing bridge");

        // Store reload handle for dynamic toggling (in lib.rs)
        awsdash::set_tracing_reload_handle(reload_handle);

        tracing::info!("Logging initialized to: {:?}", log_path);
        tracing::info!("Log levels: awsdash=info, stood=info, GUI=info/warn (wgpu), AWS SDKs=info/warn");
    }
}

fn init_perf_timing_path() {
    // Set perf timing log path for stood library
    // Only active in debug builds when stood is compiled with perf-timing feature
    #[cfg(debug_assertions)]
    {
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let perf_log_path = proj_dirs
                .data_dir()
                .join("logs")
                .join("agent_perf_timing.log");
            std::env::set_var("PERF_TIMING_LOG_PATH", &perf_log_path);
            tracing::debug!(
                "Set PERF_TIMING_LOG_PATH for stood library: {:?}",
                perf_log_path
            );
        }
    }
}

fn setup_panic_handler() {
    // Install a panic handler that writes to a crash log file
    // This catches panics even if normal logging hasn't been initialized yet
    std::panic::set_hook(Box::new(|panic_info| {
        let crash_msg = format!(
            "AWS Dash crashed!\n\
             Panic occurred at: {}\n\
             Details: {}\n\
             Backtrace:\n{:?}\n",
            panic_info
                .location()
                .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
                .unwrap_or_else(|| "unknown location".to_string()),
            panic_info
                .payload()
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| panic_info.payload().downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("unknown panic"),
            std::backtrace::Backtrace::force_capture()
        );

        // Try to write to crash log file
        if let Some(proj_dirs) = directories::ProjectDirs::from("com", "", "awsdash") {
            let log_dir = proj_dirs.data_dir().join("logs");
            let _ = std::fs::create_dir_all(&log_dir);
            let crash_log_path = log_dir.join("crash.log");

            if let Ok(mut file) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&crash_log_path)
            {
                use std::io::Write;
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "\n=== CRASH at {} ===\n{}", timestamp, crash_msg);
            }

            // Also write to stderr (visible in console builds)
            eprintln!("\n{}", crash_msg);
            eprintln!("Crash log written to: {:?}", crash_log_path);
        } else {
            // Fallback: at least print to stderr
            eprintln!("\n{}", crash_msg);
        }
    }));
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up panic handler BEFORE anything else to catch early crashes
    // This writes panic info to a file even if logging isn't initialized yet
    setup_panic_handler();

    let args: Vec<String> = std::env::args().collect();
    if let Some((url, title)) = awsdash::app::webview::parse_webview_args(&args) {
        awsdash::app::webview::run_webview(url, title)?;
        return Ok(());
    }

    init_logging();
    init_perf_timing_path();

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
    )?;

    Ok(())
}
