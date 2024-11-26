use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};
use futures_util::FutureExt;

#[proc_macro_attribute]
pub fn test_with_server(_attr: TokenStream, item: TokenStream) -> TokenStream {
  let input = parse_macro_input!(item as ItemFn);
  let func_name = &input.sig.ident;
  let block = &input.block;

  let expanded = quote! {
    #[tokio::test]
    #[serial]
    async fn #func_name() {
      use tokio::process::{Command, Child};
      use std::time::{Duration, Instant};
      use tokio::time::sleep;

      // Function to start the server
      async fn start_server() -> Result<Child, String> {
        let mut child = Command::new("cargo")
          .arg("run")
          .stdout(std::process::Stdio::null()) // Suppress output
          .stderr(std::process::Stdio::null()) // Suppress output
          .spawn()
          .map_err(|e| format!("Failed to start server process: {}", e))?;

        // Poll the server for readiness (wait for it to bind)
        let start = Instant::now();
        while start.elapsed() < Duration::from_secs(5) {
          match child.try_wait() {
            Ok(Some(status)) => return Err(format!("Server exited early with status: {}", status)),
            Ok(None) => sleep(Duration::from_millis(100)).await,
            Err(e) => return Err(format!("Error while waiting for server: {}", e)),
          }
        }

        Ok(child)
      }

      // Start the server and handle errors
      let mut server = start_server()
        .await
        .unwrap_or_else(|err| {
          panic!("Server failed to start: {}", err);
        });

      // keep trying to connect to server until it is ready:
      //
      let mut client = reqwest::Client::new();
      let mut start = Instant::now();
      loop {
        if start.elapsed() > Duration::from_secs(3600) {
          panic!("Server failed to start in time");
        }
        match client.get("http://127.0.0.1:3000/status").send().await {
          Ok(response) => {
            if response.status().is_success() {
              break;
            }
          }
          Err(_) => {sleep(Duration::from_millis(100)).await}
        }
      }

      tprintln!("Server started after {} seconds", start.elapsed().as_secs());

      tprintln!("Running test: {}", stringify!(#func_name));
      // Run the test block
      let result = std::panic::AssertUnwindSafe(async { #block })
                .catch_unwind()
                .await;

      // Stop the server
      let _ = server.kill().await;

      // If the test panicked, propagate the panic
      if let Err(err) = result {
        std::panic::resume_unwind(err);
      }
    }
  };

  TokenStream::from(expanded)
}