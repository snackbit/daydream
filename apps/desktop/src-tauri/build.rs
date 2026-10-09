use std::{
    env,
    path::{Path, PathBuf},
};

#[path = "build_support/resource_staging.rs"]
mod resource_staging;
use resource_staging::copy_if_changed;

fn main() {
    build_whisper_cpp();
    build_embedding_worker();
    tauri_build::build()
}

fn build_embedding_worker() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let source = manifest_dir.join("embedding-worker");
    let out = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let install = cmake::Config::new(&source)
        .out_dir(out.join("embedding-cpu"))
        .define("DAYDREAM_MODEL_VALIDATION", "OFF")
        .define("GGML_VULKAN", "OFF")
        .build_target("install")
        .build();
    let name = if cfg!(target_os = "windows") {
        "daydream-embedding-worker.exe"
    } else {
        "daydream-embedding-worker"
    };
    let worker = install.join("bin").join(name);
    println!(
        "cargo:rustc-env=DAYDREAM_EMBEDDING_WORKER={}",
        worker.display()
    );
    // A resource binary remains independent of Whisper's ggml symbols.
    let resources = manifest_dir.join("resources/embedding");
    std::fs::create_dir_all(&resources).expect("create embedding resource directory");
    copy_if_changed(&worker, &resources.join(name)).expect("stage embedding worker resource");
    let gpu_name = if cfg!(target_os = "windows") {
        "daydream-embedding-worker-vulkan.exe"
    } else {
        "daydream-embedding-worker-vulkan"
    };
    if cfg!(feature = "embedding-vulkan") {
        let mut gpu_config = cmake::Config::new(&source);
        gpu_config
            .out_dir(out.join("embedding-vulkan"))
            .define("DAYDREAM_MODEL_VALIDATION", "OFF")
            .define("GGML_VULKAN", "ON")
            .build_target("install");
        if let Some(glslc) = env::var_os("DAYDREAM_VULKAN_GLSLC") {
            gpu_config.define("Vulkan_GLSLC_EXECUTABLE", glslc);
        }
        let gpu_install = gpu_config.build();
        let gpu = gpu_install.join("bin").join(name);
        copy_if_changed(&gpu, &install.join("bin").join(gpu_name))
            .expect("stage development Vulkan worker");
        copy_if_changed(&gpu, &resources.join(gpu_name)).expect("stage Vulkan worker resource");
    } else {
        // A preceding feature-enabled build must not leak its optional binary
        // into a later CPU-only package.
        for stale in [install.join("bin").join(gpu_name), resources.join(gpu_name)] {
            if stale.exists() {
                std::fs::remove_file(stale).expect("remove stale Vulkan worker");
            }
        }
    }
    for (source, name) in [
        ("vendor/llama.cpp/LICENSE", "LLAMA-LICENSE.txt"),
        ("embedding-worker/JSON-LICENSE.txt", "JSON-LICENSE.txt"),
        ("embedding-worker/E5-LICENSE.txt", "E5-LICENSE.txt"),
        ("embedding-worker/SPIRV-LICENSE.txt", "SPIRV-LICENSE.txt"),
    ] {
        copy_if_changed(&manifest_dir.join(source), &resources.join(name))
            .expect("stage embedding license");
    }
    println!("cargo:rerun-if-changed={}", source.display());
    println!("cargo:rerun-if-changed=build_support/resource_staging.rs");
    println!("cargo:rerun-if-env-changed=DAYDREAM_VULKAN_GLSLC");
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir.join("vendor/llama.cpp").display()
    );
}

fn build_whisper_cpp() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let whisper_dir = manifest_dir.join("vendor/whisper.cpp");
    let header = whisper_dir.join("include/whisper.h");
    let ggml_include_dir = whisper_dir.join("ggml/include");
    let install_dir = cmake::Config::new(&whisper_dir)
        .out_dir(PathBuf::from(env::var("OUT_DIR").expect("out dir")).join("whisper"))
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
