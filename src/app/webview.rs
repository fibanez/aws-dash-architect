use std::env;
use std::process::Command;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

pub fn spawn_webview_process(url: String, title: String) -> std::io::Result<()> {
    let current_exe = env::current_exe()?;

    Command::new(current_exe)
        .arg("--webview")
        .arg("--title")
        .arg(title)
        .arg("--url")
        .arg(url)
        .spawn()?;

    Ok(())
}

pub fn parse_webview_args(args: &[String]) -> Option<(String, String)> {
    if !args.iter().any(|arg| arg == "--webview") {
        return None;
    }

    let mut title = "AWS Console".to_string();
    let mut url = "https://console.aws.amazon.com/".to_string();

    for i in 0..args.len() {
        if args[i] == "--title" && i + 1 < args.len() {
            title = args[i + 1].clone();
        } else if args[i] == "--url" && i + 1 < args.len() {
            url = args[i + 1].clone();
        }
    }

    Some((url, title))
}

pub fn run_webview(url: String, title: String) -> wry::Result<()> {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title(title).build(&event_loop).unwrap();

    let builder = WebViewBuilder::new().with_url(&url);

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let _webview = builder.build(&window)?;

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let _webview = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox)?
    };

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            *control_flow = ControlFlow::Exit;
        }
    });
}
