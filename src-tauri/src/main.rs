#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use tauri::{ WindowUrl, window::{ WindowBuilder, Window } };
use notify::{Watcher, RecursiveMode, RecommendedWatcher, Config};
use std::path::PathBuf;
use std::fs::read;
use std::sync::mpsc;


// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
	format!("Hello, {}! You've been greeted from Rust!", name)
}

fn main() {
	tauri::Builder::default()
		.invoke_handler(tauri::generate_handler![greet])
		.setup(|app| {

			let pwd = std::env::current_dir().unwrap();
			let assets_path: PathBuf = pwd.join("watch-me").into();
			let index_path = assets_path.join("index.html");

			let window = WindowBuilder::new(
					&app.handle(),
					String::from("label"),
					WindowUrl::App("index.html".into())
				)
				.on_web_resource_request(move |request, response| {
					let uri = request.uri();
					match uri {
						"tauri://localhost" => {
							let mutable_response = response.body_mut();
							match read(index_path.clone()) {
								Ok(index_html) => *mutable_response = index_html, // TODO! Check if there are better ways of dealing with errors here
								Err(e) => println!("Unable to read file."),
							}
						},
						_ => ()
					}
				})
				.inner_size(1000.0, 700.0)
				.title("Tauri-App")
				.build()
				.expect("Failed to build window.");


			// channel for message passing from the ui folder watcher to the main application
			let (tx, rx) = mpsc::channel();

			println!("Watching file changes in folder {:?}", assets_path.as_path());

			let watch_handle = std::thread::spawn(move || {

				let (tx_watcher, rx_watcher) = std::sync::mpsc::channel();

				let mut watcher = match RecommendedWatcher::new(tx_watcher, Config::default()) {
					Ok(w) => w,
					Err(e) => panic!("Failed to create file system watcher: {:?}", e),
				};

				match watcher.watch(assets_path.as_path(), RecursiveMode::Recursive) {
					Ok(()) => (),
					Err(e) => {
						println!("Failed to watch: {:?}", e);
						panic!("Failed to watch.");
					}
				};

				// keep listening to rx_watcher to keep the thread running
				for res in rx_watcher {
					match res {
						Ok(event) => {
							println!("event: {:?}", event);
							tx.send(String::from("Reload")).unwrap();
						},
						Err(e) => println!("watch error: {:?}", e),
						}
				}

			});

			// Reload window if file has changed in ui folder
			match rx.recv() {
				Ok(_) => {
					println!("File change detected. Reloading tauri window...");
					match window.eval("location.reload()") {
						Ok(()) => (),
						Err(e) => println!("Failed to reload window: {:?}", e),
					};
				},
				Err(_) => (),
			}

			watch_handle.join().unwrap();

		Ok(())
	})
		.run(tauri::generate_context!())
		.expect("error while running tauri application");
}
