use std::{
    env,
    path::{Path, PathBuf},
};

fn main() {
    build_whisper_cpp();
    tauri_build::build()
}

fn build_whisper_cpp() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let whisper_dir = manifest_dir.join("vendor/whisper.cpp");
    let header = whisper_dir.join("include/whisper.h");
    let ggml_include_dir = whisper_dir.join("ggml/include");
    let install_dir = cmake::Config::new(&whisper_dir)
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("GGML_STATIC", "ON")
        .define("GGML_CCACHE", "OFF")
        .define("GGML_NATIVE", "OFF")
        .define("GGML_OPENMP", "OFF")
        .define("WHISPER_BUILD_TESTS", "OFF")
        .define("WHISPER_BUILD_EXAMPLES", "OFF")
        .define("WHISPER_BUILD_SERVER", "OFF")
        .define(
            "GGML_VULKAN",
            if cfg!(feature = "whisper-vulkan") {
                "ON"
            } else {
                "OFF"
            },
        )
        .define(
            "GGML_CUDA",
            if cfg!(feature = "whisper-cuda") {
                "ON"
            } else {
                "OFF"
            },
        )
        .build();

    println!(
        "cargo:rustc-link-search=native={}",
        install_dir.join("lib").display()
    );
    println!("cargo:rustc-link-lib=static=whisper");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib=static=ggml-base");
    println!("cargo:rustc-link-lib=static=ggml-cpu");
    if cfg!(feature = "whisper-vulkan") {
        println!("cargo:rustc-link-lib=static=ggml-vulkan");
        println!("cargo:rustc-link-lib=dylib=vulkan");
    }
    if cfg!(feature = "whisper-cuda") {
        link_cuda_backend();
    }
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
    if cfg!(target_os = "macos") {
        println!("cargo:rustc-link-lib=dylib=c++");
    }

    println!("cargo:rerun-if-changed={}", whisper_dir.display());
    println!("cargo:rerun-if-changed={}", header.display());

    let bindings = bindgen::Builder::default()
        .header(header.to_string_lossy())
        .clang_arg(format!("-I{}", whisper_dir.join("include").display()))
        .clang_arg(format!("-I{}", ggml_include_dir.display()))
        .allowlist_function("whisper_.*")
        .allowlist_type("whisper_.*")
        .allowlist_var("WHISPER_.*")
        .generate()
        .expect("generate whisper.cpp C API bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    bindings
        .write_to_file(out_path.join("whisper_bindings.rs"))
        .expect("write whisper.cpp bindings");
}

fn link_cuda_backend() {
    let library_dirs = cuda_library_dirs();
    if library_dirs.is_empty() {
        println!(
            "cargo:warning=whisper-cuda is enabled, but no CUDA Toolkit library directory was found"
        );
    }
    for directory in library_dirs {
        println!("cargo:rustc-link-search=native={}", directory.display());
    }

    println!("cargo:rustc-link-lib=static=ggml-cuda");
    println!("cargo:rustc-link-lib=static=cudart_static");
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=dylib=cublas");
        println!("cargo:rustc-link-lib=dylib=cublasLt");
        println!("cargo:rustc-link-lib=dylib=cuda");
    } else {
        println!("cargo:rustc-link-lib=static=cublas_static");
        println!("cargo:rustc-link-lib=static=cublasLt_static");
        println!("cargo:rustc-link-lib=static=culibos");
        println!("cargo:rustc-link-lib=dylib=cuda");
        if cfg!(target_os = "linux") {
            println!("cargo:rustc-link-lib=dylib=dl");
            println!("cargo:rustc-link-lib=dylib=pthread");
            println!("cargo:rustc-link-lib=dylib=rt");
        }
    }
}

fn cuda_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    for variable in ["CUDA_PATH", "CUDA_HOME", "CUDAToolkit_ROOT"] {
        if let Some(root) = env::var_os(variable).map(PathBuf::from) {
            add_cuda_root_library_dirs(&mut dirs, &root);
        }
    }

    for root in [
        PathBuf::from("/usr/local/cuda"),
        PathBuf::from("/opt/cuda"),
        PathBuf::from(r"C:\Program Files\NVIDIA GPU Computing Toolkit\CUDA"),
    ] {
        add_cuda_root_library_dirs(&mut dirs, &root);
    }

    dirs
}

fn add_cuda_root_library_dirs(dirs: &mut Vec<PathBuf>, root: &Path) {
    for relative in ["lib64", "lib/x64", "lib"] {
        let directory = root.join(relative);
        if directory.is_dir() && !dirs.contains(&directory) {
            dirs.push(directory);
        }
    }
}
