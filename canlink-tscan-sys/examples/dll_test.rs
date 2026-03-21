//! Simple DLL loading test to verify libTSCAN.dll

use std::env;
use std::ffi::CString;

#[cfg(windows)]
use winapi::um::libloaderapi::{FreeLibrary, GetProcAddress, LoadLibraryA};

fn main() {
    #[cfg(windows)]
    unsafe {
        println!("🔍 Testing libTSCAN.dll loading...\n");

        // Get the executable directory
        let exe_path = env::current_exe().expect("Failed to get executable path");
        let exe_dir = exe_path
            .parent()
            .expect("Failed to get executable directory");

        println!("Executable directory: {}", exe_dir.display());

        // Check if DLLs exist
        let libtscan_path = exe_dir.join("libTSCAN.dll");
        let libtsh_path = exe_dir.join("libTSH.dll");

        println!("Looking for libTSCAN.dll at: {}", libtscan_path.display());
        println!("  Exists: {}", libtscan_path.exists());
        println!("Looking for libTSH.dll at: {}", libtsh_path.display());
        println!("  Exists: {}\n", libtsh_path.exists());

        if !libtscan_path.exists() {
            println!("❌ libTSCAN.dll not found!");
            return;
        }

        // Try to load the DLL with full path
        let dll_path_str = libtscan_path.to_str().expect("Invalid path");
        let dll_name = CString::new(dll_path_str).unwrap();
        let h_dll = LoadLibraryA(dll_name.as_ptr());

        if h_dll.is_null() {
            println!("❌ Failed to load libTSCAN.dll");
            println!("   Error: The DLL or one of its dependencies could not be loaded");
            println!("   Make sure libTSH.dll is also present");
            return;
        }

        println!("✓ Successfully loaded libTSCAN.dll\n");

        // Test loading some functions
        let functions = [
            "initialize_lib_tscan",
            "finalize_lib_tscan",
            "tscan_scan_devices",
            "tscan_connect",
            "tscan_disconnect_by_handle",
            "tscan_config_can_by_baudrate",
            "tscan_transmit_can_async",
            "tsfifo_receive_can_msgs",
        ];

        println!("Checking function exports:");
        let mut found_count = 0;
        for func_name in &functions {
            let c_name = CString::new(*func_name).unwrap();
            let proc_addr = GetProcAddress(h_dll, c_name.as_ptr());

            if proc_addr.is_null() {
                println!("  ❌ {} - NOT FOUND", func_name);
            } else {
                println!("  ✓ {} - Found at 0x{:X}", func_name, proc_addr as usize);
                found_count += 1;
            }
        }

        println!(
            "\nResult: {}/{} functions found",
            found_count,
            functions.len()
        );

        FreeLibrary(h_dll);

        if found_count == functions.len() {
            println!("\n✅ All functions found! DLL is compatible.");
        } else {
            println!("\n⚠️ Some functions missing. DLL may be incompatible.");
        }
    }

    #[cfg(not(windows))]
    {
        println!("This test only works on Windows");
    }
}
