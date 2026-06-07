use std::{ffi::CStr, ptr};

use khronos_egl as egl;

use crate::recorder::{manager::unavailable_gpu_capture_health, types::RecorderGpuCaptureHealth};

const EGL_PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;
const EGL_IMAGE_DMA_BUF_IMPORT_EXT: &str = "EGL_EXT_image_dma_buf_import";
const EGL_IMAGE_DMA_BUF_IMPORT_MODIFIERS_EXT: &str = "EGL_EXT_image_dma_buf_import_modifiers";
const EGL_MESA_PLATFORM_SURFACELESS_EXT: &str = "EGL_MESA_platform_surfaceless";
const EGL_EXT_PLATFORM_BASE_EXT: &str = "EGL_EXT_platform_base";
const GL_VENDOR: u32 = 0x1F00;
const GL_RENDERER: u32 = 0x1F01;
const GL_VERSION: u32 = 0x1F02;

type EglInstance = egl::DynamicInstance<egl::EGL1_5>;
type GlGetString = unsafe extern "system" fn(u32) -> *const u8;

pub(super) fn collect_gpu_capture_health() -> RecorderGpuCaptureHealth {
    match probe_gpu_capture() {
        Ok(probe) => probe.into_health(),
        Err(error) => unavailable_gpu_capture_health(error),
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GpuCaptureProbe {
    cleanup_error: Option<String>,
    display_extensions: Vec<String>,
    egl_client_apis: Vec<String>,
    egl_platform: String,
    egl_vendor: Option<String>,
    egl_version: Option<String>,
    opengl_available: bool,
    renderer: Option<String>,
    vendor: Option<String>,
    version: Option<String>,
}

impl GpuCaptureProbe {
    fn into_health(self) -> RecorderGpuCaptureHealth {
        let dmabuf_import_available =
            has_extension(&self.display_extensions, EGL_IMAGE_DMA_BUF_IMPORT_EXT);
        let modifier_import_available = Some(has_extension(
            &self.display_extensions,
            EGL_IMAGE_DMA_BUF_IMPORT_MODIFIERS_EXT,
        ));

        RecorderGpuCaptureHealth {
            available: self.opengl_available && dmabuf_import_available,
            cleanup_error: self.cleanup_error.clone(),
            dmabuf_import_available,
            egl_available: true,
            egl_client_apis: self.egl_client_apis,
            egl_platform: Some(self.egl_platform),
            egl_vendor: self.egl_vendor,
            egl_version: self.egl_version,
            error: health_error(self.opengl_available, dmabuf_import_available),
            modifier_import_available,
            opengl_available: self.opengl_available,
            renderer: self.renderer,
            vendor: self.vendor,
            version: self.version,
        }
    }
}

fn probe_gpu_capture() -> Result<GpuCaptureProbe, String> {
    let lib = unsafe { libloading::Library::new("libEGL.so.1") }
        .map_err(|error| format!("EGL library is unavailable: {error}."))?;
    let egl = unsafe { EglInstance::load_required_from(lib) }
        .map_err(|error| format!("EGL 1.5 entrypoints are unavailable: {error}."))?;
    let client_extensions = query_egl_extensions(&egl, None).unwrap_or_default();
    let selected_display = egl_display(&egl, &client_extensions)?;
    egl.initialize(selected_display.display)
        .map_err(|error| format!("EGL display initialization failed: {error}."))?;

    let probe_result = probe_initialized_display(&egl, selected_display);
    match (probe_result, egl.terminate(selected_display.display)) {
        (Ok(probe), Ok(())) => Ok(probe),
        (Ok(mut probe), Err(error)) => {
            append_cleanup_error(
                &mut probe.cleanup_error,
                format!("EGL display cleanup failed: {error}."),
            );
            Ok(probe)
        }
        (Err(probe_error), Ok(())) => Err(probe_error),
        (Err(probe_error), Err(error)) => Err(format!(
            "{probe_error} EGL display cleanup failed: {error}."
        )),
    }
}

fn probe_initialized_display(
    egl: &EglInstance,
    selected_display: SelectedEglDisplay,
) -> Result<GpuCaptureProbe, String> {
    let display = selected_display.display;
    let display_extensions = query_egl_extensions(egl, Some(display)).unwrap_or_default();
    let egl_client_apis = query_egl_client_apis(egl, display).unwrap_or_default();
    let egl_vendor = query_egl_string(egl, display, egl::VENDOR);
    let egl_version = query_egl_string(egl, display, egl::VERSION);

    egl.bind_api(egl::OPENGL_API)
        .map_err(|error| format!("EGL OpenGL API binding failed: {error}."))?;
    let config = choose_probe_config(egl, display)?;
    let surface = egl
        .create_pbuffer_surface(display, config, &[egl::WIDTH, 1, egl::HEIGHT, 1, egl::NONE])
        .map_err(|error| format!("EGL pbuffer creation failed: {error}."))?;
    let context = match egl.create_context(display, config, None, &[egl::NONE]) {
        Ok(context) => context,
        Err(error) => {
            let _ = egl.destroy_surface(display, surface);
            return Err(format!("EGL OpenGL context creation failed: {error}."));
        }
    };

    let probe_result = (|| {
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|error| format!("EGL OpenGL context activation failed: {error}."))?;
        Ok(GpuCaptureProbe {
            cleanup_error: None,
            display_extensions,
            egl_client_apis,
            egl_platform: selected_display.platform.to_string(),
            egl_vendor,
            egl_version,
            opengl_available: true,
            renderer: gl_string(egl, GL_RENDERER),
            vendor: gl_string(egl, GL_VENDOR),
            version: gl_string(egl, GL_VERSION),
        })
    })();

    let _ = egl.make_current(display, None, None, None);
    let destroy_context = egl.destroy_context(display, context);
    let destroy_surface = egl.destroy_surface(display, surface);
    let cleanup_error = cleanup_error(destroy_context, destroy_surface);

    match probe_result {
        Ok(mut probe) => {
            probe.cleanup_error = cleanup_error;
            Ok(probe)
        }
        Err(error) => {
            if let Some(cleanup_error) = cleanup_error {
                Err(format!("{error} {cleanup_error}"))
            } else {
                Err(error)
            }
        }
    }
}

fn choose_probe_config(egl: &EglInstance, display: egl::Display) -> Result<egl::Config, String> {
    let attributes = [
        egl::SURFACE_TYPE,
        egl::PBUFFER_BIT,
        egl::RENDERABLE_TYPE,
        egl::OPENGL_BIT,
        egl::RED_SIZE,
        8,
        egl::GREEN_SIZE,
        8,
        egl::BLUE_SIZE,
        8,
        egl::NONE,
    ];

    egl.choose_first_config(display, &attributes)
        .map_err(|error| format!("EGL probe config selection failed: {error}."))?
        .ok_or_else(|| "EGL did not expose a pbuffer OpenGL config.".to_string())
}

#[derive(Clone, Copy, Debug)]
struct SelectedEglDisplay {
    display: egl::Display,
    platform: &'static str,
}

fn egl_display(
    egl: &EglInstance,
    client_extensions: &[String],
) -> Result<SelectedEglDisplay, String> {
    if has_extension(client_extensions, EGL_EXT_PLATFORM_BASE_EXT)
        && has_extension(client_extensions, EGL_MESA_PLATFORM_SURFACELESS_EXT)
    {
        let display = unsafe {
            egl.get_platform_display(
                EGL_PLATFORM_SURFACELESS_MESA,
                ptr::null_mut(),
                &[egl::ATTRIB_NONE],
            )
        }
        .map_err(|error| format!("EGL surfaceless display creation failed: {error}."))?;
        return Ok(SelectedEglDisplay {
            display,
            platform: "surfaceless",
        });
    }

    let display = unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }
        .ok_or_else(|| "EGL default display is unavailable.".to_string())?;
    Ok(SelectedEglDisplay {
        display,
        platform: "default",
    })
}

fn query_egl_extensions(egl: &EglInstance, display: Option<egl::Display>) -> Option<Vec<String>> {
    egl.query_string(display, egl::EXTENSIONS)
        .ok()
        .map(|extensions| split_extensions(&extensions.to_string_lossy()))
}

fn query_egl_client_apis(egl: &EglInstance, display: egl::Display) -> Option<Vec<String>> {
    query_egl_string(egl, display, egl::CLIENT_APIS).map(|apis| split_extensions(&apis))
}

fn query_egl_string(egl: &EglInstance, display: egl::Display, name: egl::Int) -> Option<String> {
    egl.query_string(Some(display), name)
        .ok()
        .map(|value| value.to_string_lossy().to_string())
}

fn gl_string(egl: &EglInstance, name: u32) -> Option<String> {
    let get_string = egl.get_proc_address("glGetString")?;
    let get_string: GlGetString = unsafe { std::mem::transmute(get_string) };
    let value = unsafe { get_string(name) };
    if value.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(value.cast::<std::ffi::c_char>()) };
    Some(value.to_string_lossy().to_string())
}

fn split_extensions(extensions: &str) -> Vec<String> {
    extensions
        .split_ascii_whitespace()
        .map(ToString::to_string)
        .collect()
}

fn has_extension(extensions: &[String], extension: &str) -> bool {
    extensions.iter().any(|candidate| candidate == extension)
}

fn health_error(opengl_available: bool, dmabuf_import_available: bool) -> Option<String> {
    let mut errors = Vec::new();
    if !opengl_available {
        errors.push("EGL is available, but OpenGL context probing failed.".to_string());
    }
    if !dmabuf_import_available {
        errors.push("EGL is available, but DMA-BUF import is not supported.".to_string());
    }

    if errors.is_empty() {
        None
    } else {
        Some(errors.join(" "))
    }
}

fn cleanup_error(
    destroy_context: Result<(), egl::Error>,
    destroy_surface: Result<(), egl::Error>,
) -> Option<String> {
    let mut cleanup_error = None;
    if let Err(error) = destroy_context {
        append_cleanup_error(
            &mut cleanup_error,
            format!("EGL OpenGL context cleanup failed: {error}."),
        );
    }
    if let Err(error) = destroy_surface {
        append_cleanup_error(
            &mut cleanup_error,
            format!("EGL pbuffer cleanup failed: {error}."),
        );
    }
    cleanup_error
}

fn append_cleanup_error(cleanup_error: &mut Option<String>, message: String) {
    match cleanup_error {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(&message);
        }
        None => *cleanup_error = Some(message),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        has_extension, split_extensions, GpuCaptureProbe, EGL_IMAGE_DMA_BUF_IMPORT_EXT,
        EGL_IMAGE_DMA_BUF_IMPORT_MODIFIERS_EXT,
    };

    #[test]
    fn extension_parser_requires_exact_tokens() {
        let extensions = split_extensions("EGL_EXT_image_dma_buf_import EGL_EXT_platform_base");

        assert!(has_extension(&extensions, EGL_IMAGE_DMA_BUF_IMPORT_EXT));
        assert!(!has_extension(&extensions, "EGL_EXT_image_dma_buf"));
    }

    #[test]
    fn health_requires_dmabuf_import_but_reports_modifier_support() {
        let health = GpuCaptureProbe {
            cleanup_error: None,
            display_extensions: split_extensions(&format!(
                "{EGL_IMAGE_DMA_BUF_IMPORT_EXT} {EGL_IMAGE_DMA_BUF_IMPORT_MODIFIERS_EXT}"
            )),
            egl_client_apis: vec!["OpenGL".to_string()],
            egl_platform: "surfaceless".to_string(),
            egl_vendor: Some("EGL vendor".to_string()),
            egl_version: Some("1.5".to_string()),
            opengl_available: true,
            renderer: Some("renderer".to_string()),
            vendor: Some("vendor".to_string()),
            version: Some("4.6".to_string()),
        }
        .into_health();

        assert!(health.available);
        assert!(health.dmabuf_import_available);
        assert_eq!(health.modifier_import_available, Some(true));
        assert!(health.opengl_available);
        assert_eq!(health.error, None);
        assert_eq!(health.cleanup_error, None);
        assert_eq!(health.egl_client_apis, vec!["OpenGL"]);
        assert_eq!(health.egl_platform.as_deref(), Some("surfaceless"));
        assert_eq!(health.egl_vendor.as_deref(), Some("EGL vendor"));
        assert_eq!(health.egl_version.as_deref(), Some("1.5"));
    }

    #[test]
    fn health_degrades_without_dmabuf_import() {
        let health = GpuCaptureProbe {
            cleanup_error: None,
            display_extensions: Vec::new(),
            egl_client_apis: Vec::new(),
            egl_platform: "default".to_string(),
            egl_vendor: None,
            egl_version: None,
            opengl_available: true,
            renderer: None,
            vendor: None,
            version: None,
        }
        .into_health();

        assert!(!health.available);
        assert!(!health.dmabuf_import_available);
        assert_eq!(health.modifier_import_available, Some(false));
        assert!(health.opengl_available);
        assert!(health.error.expect("error").contains("DMA-BUF"));
    }

    #[test]
    fn health_preserves_successful_probe_facts_when_cleanup_fails() {
        let health = GpuCaptureProbe {
            cleanup_error: Some(
                "EGL OpenGL context cleanup failed: EGL is not initialized.".to_string(),
            ),
            display_extensions: split_extensions(EGL_IMAGE_DMA_BUF_IMPORT_EXT),
            egl_client_apis: vec!["OpenGL_ES".to_string(), "OpenGL".to_string()],
            egl_platform: "surfaceless".to_string(),
            egl_vendor: Some("NVIDIA".to_string()),
            egl_version: Some("1.5".to_string()),
            opengl_available: true,
            renderer: Some("NVIDIA GeForce RTX 4090".to_string()),
            vendor: Some("NVIDIA Corporation".to_string()),
            version: Some("4.6.0 NVIDIA".to_string()),
        }
        .into_health();

        assert!(health.available);
        assert!(health.dmabuf_import_available);
        assert!(health.opengl_available);
        assert_eq!(health.renderer.as_deref(), Some("NVIDIA GeForce RTX 4090"));
        assert!(health
            .cleanup_error
            .as_deref()
            .expect("cleanup error")
            .contains("cleanup failed"));
        assert_eq!(health.error, None);
    }
}
