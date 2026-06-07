use std::{ffi::c_void, mem, ptr};

use khronos_egl as egl;

use crate::recorder::{
    capture_runtime::{CapturedDmaBufFrame, CapturedFrame, CapturedFrameFormat},
    errors::{capture_adapter_failed_error, capture_format_unsupported_error},
    types::RecorderError,
};

const EGL_PLATFORM_SURFACELESS_MESA: egl::Enum = 0x31DD;
const EGL_LINUX_DMA_BUF_EXT: egl::Enum = 0x3270;
const EGL_LINUX_DRM_FOURCC_EXT: egl::Attrib = 0x3271;
const EGL_DMA_BUF_PLANE0_FD_EXT: egl::Attrib = 0x3272;
const EGL_DMA_BUF_PLANE0_OFFSET_EXT: egl::Attrib = 0x3273;
const EGL_DMA_BUF_PLANE0_PITCH_EXT: egl::Attrib = 0x3274;
const EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT: egl::Attrib = 0x3443;
const EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT: egl::Attrib = 0x3444;
const DRM_FORMAT_MOD_INVALID: u64 = (1_u64 << 56) - 1;

const GL_TEXTURE_2D: u32 = 0x0DE1;
const GL_TEXTURE_MIN_FILTER: u32 = 0x2801;
const GL_TEXTURE_MAG_FILTER: u32 = 0x2800;
const GL_TEXTURE_WRAP_S: u32 = 0x2802;
const GL_TEXTURE_WRAP_T: u32 = 0x2803;
const GL_LINEAR: i32 = 0x2601;
const GL_CLAMP_TO_EDGE: i32 = 0x812F;
const GL_RGBA: u32 = 0x1908;
const GL_UNSIGNED_BYTE: u32 = 0x1401;
const GL_FRAMEBUFFER: u32 = 0x8D40;
const GL_READ_FRAMEBUFFER: u32 = 0x8CA8;
const GL_COLOR_ATTACHMENT0: u32 = 0x8CE0;
const GL_FRAMEBUFFER_COMPLETE: u32 = 0x8CD5;

type EglInstance = egl::DynamicInstance<egl::EGL1_5>;
type GlGenTextures = unsafe extern "system" fn(i32, *mut u32);
type GlDeleteTextures = unsafe extern "system" fn(i32, *const u32);
type GlBindTexture = unsafe extern "system" fn(u32, u32);
type GlTexParameteri = unsafe extern "system" fn(u32, u32, i32);
type GlEglImageTargetTexture2DOes = unsafe extern "system" fn(u32, *const c_void);
type GlGenFramebuffers = unsafe extern "system" fn(i32, *mut u32);
type GlDeleteFramebuffers = unsafe extern "system" fn(i32, *const u32);
type GlBindFramebuffer = unsafe extern "system" fn(u32, u32);
type GlFramebufferTexture2D = unsafe extern "system" fn(u32, u32, u32, u32, i32);
type GlCheckFramebufferStatus = unsafe extern "system" fn(u32) -> u32;
type GlReadPixels = unsafe extern "system" fn(i32, i32, i32, i32, u32, u32, *mut c_void);

pub(super) struct LinuxGpuFrameCompositor {
    egl: EglInstance,
    display: egl::Display,
    context: egl::Context,
    surface: egl::Surface,
    gl: GlFns,
}

struct GlFns {
    gen_textures: GlGenTextures,
    delete_textures: GlDeleteTextures,
    bind_texture: GlBindTexture,
    tex_parameteri: GlTexParameteri,
    egl_image_target_texture_2d_oes: GlEglImageTargetTexture2DOes,
    gen_framebuffers: GlGenFramebuffers,
    delete_framebuffers: GlDeleteFramebuffers,
    bind_framebuffer: GlBindFramebuffer,
    framebuffer_texture_2d: GlFramebufferTexture2D,
    check_framebuffer_status: GlCheckFramebufferStatus,
    read_pixels: GlReadPixels,
}

struct ImportedTexture {
    image: egl::Image,
    texture: u32,
}

struct Framebuffer {
    id: u32,
}

impl LinuxGpuFrameCompositor {
    pub(super) fn new() -> Result<Self, RecorderError> {
        let lib = unsafe { libloading::Library::new("libEGL.so.1") }.map_err(|error| {
            capture_adapter_failed_error(format!("EGL library is unavailable: {error}."))
        })?;
        let egl = unsafe { EglInstance::load_required_from(lib) }.map_err(|error| {
            capture_adapter_failed_error(format!("EGL 1.5 entrypoints are unavailable: {error}."))
        })?;
        let display = egl_display(&egl)?;
        egl.initialize(display).map_err(|error| {
            capture_adapter_failed_error(format!("EGL display initialization failed: {error}."))
        })?;
        egl.bind_api(egl::OPENGL_API).map_err(|error| {
            capture_adapter_failed_error(format!("EGL OpenGL API binding failed: {error}."))
        })?;
        let config = choose_config(&egl, display)?;
        let surface = egl
            .create_pbuffer_surface(display, config, &[egl::WIDTH, 1, egl::HEIGHT, 1, egl::NONE])
            .map_err(|error| {
                capture_adapter_failed_error(format!("EGL pbuffer creation failed: {error}."))
            })?;
        let context = egl
            .create_context(display, config, None, &[egl::NONE])
            .map_err(|error| {
                capture_adapter_failed_error(format!(
                    "EGL OpenGL context creation failed: {error}."
                ))
            })?;
        egl.make_current(display, Some(surface), Some(surface), Some(context))
            .map_err(|error| {
                capture_adapter_failed_error(format!(
                    "EGL OpenGL context activation failed: {error}."
                ))
            })?;
        let gl = GlFns::load(&egl)?;

        Ok(Self {
            egl,
            display,
            context,
            surface,
            gl,
        })
    }

    pub(super) fn read_frame_to_rgba(
        &self,
        frame: &CapturedDmaBufFrame,
    ) -> Result<CapturedFrame, RecorderError> {
        frame.validate()?;
        let imported = self.import_frame(frame)?;
        let source_fbo =
            self.create_framebuffer(imported.texture, "GPU compositor DMA-BUF source")?;
        let mut rgba = vec![0_u8; frame.width as usize * frame.height as usize * 4];
        unsafe {
            (self.gl.bind_framebuffer)(GL_READ_FRAMEBUFFER, source_fbo.id);
            (self.gl.read_pixels)(
                0,
                0,
                frame.width as i32,
                frame.height as i32,
                GL_RGBA,
                GL_UNSIGNED_BYTE,
                rgba.as_mut_ptr().cast::<c_void>(),
            );
        }
        self.destroy_framebuffer(source_fbo);
        self.destroy_imported_texture(imported);

        CapturedFrame::new(
            frame.source_id.clone(),
            frame.timestamp_ms,
            frame.width,
            frame.height,
            frame.width * 4,
            CapturedFrameFormat::Rgba,
            rgba_bottom_up_to_top_down(&rgba, frame.width, frame.height),
        )
    }

    fn import_frame(&self, frame: &CapturedDmaBufFrame) -> Result<ImportedTexture, RecorderError> {
        let attribs = dmabuf_import_attribs(
            frame.width,
            frame.height,
            frame.fourcc,
            frame.fd_raw_for_egl()?,
            frame.offset,
            frame.stride,
            frame.modifier,
        );
        let image = self
            .egl
            .create_image(
                self.display,
                unsafe { egl::Context::from_ptr(egl::NO_CONTEXT) },
                EGL_LINUX_DMA_BUF_EXT,
                unsafe { egl::ClientBuffer::from_ptr(ptr::null_mut()) },
                &attribs,
            )
            .map_err(|error| {
                capture_adapter_failed_error(format!(
                    "EGL DMA-BUF import failed for capture source {}: {error}.",
                    frame.source_id
                ))
            })?;
        let texture = self.gen_texture();
        unsafe {
            (self.gl.bind_texture)(GL_TEXTURE_2D, texture);
            self.set_texture_params();
            (self.gl.egl_image_target_texture_2d_oes)(
                GL_TEXTURE_2D,
                image.as_ptr().cast::<c_void>(),
            );
        }

        Ok(ImportedTexture { image, texture })
    }

    fn create_framebuffer(&self, texture: u32, label: &str) -> Result<Framebuffer, RecorderError> {
        let mut id = 0;
        unsafe {
            (self.gl.gen_framebuffers)(1, &mut id);
            (self.gl.bind_framebuffer)(GL_FRAMEBUFFER, id);
            (self.gl.framebuffer_texture_2d)(
                GL_FRAMEBUFFER,
                GL_COLOR_ATTACHMENT0,
                GL_TEXTURE_2D,
                texture,
                0,
            );
            let status = (self.gl.check_framebuffer_status)(GL_FRAMEBUFFER);
            if status != GL_FRAMEBUFFER_COMPLETE {
                (self.gl.delete_framebuffers)(1, &id);
                return Err(capture_adapter_failed_error(format!(
                    "{label} framebuffer is incomplete: 0x{status:x}."
                )));
            }
        }

        Ok(Framebuffer { id })
    }

    fn gen_texture(&self) -> u32 {
        let mut texture = 0;
        unsafe {
            (self.gl.gen_textures)(1, &mut texture);
        }
        texture
    }

    unsafe fn set_texture_params(&self) {
        (self.gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MIN_FILTER, GL_LINEAR);
        (self.gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_MAG_FILTER, GL_LINEAR);
        (self.gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_S, GL_CLAMP_TO_EDGE);
        (self.gl.tex_parameteri)(GL_TEXTURE_2D, GL_TEXTURE_WRAP_T, GL_CLAMP_TO_EDGE);
    }

    fn destroy_imported_texture(&self, imported: ImportedTexture) {
        self.delete_texture(imported.texture);
        let _ = self.egl.destroy_image(self.display, imported.image);
    }

    fn destroy_framebuffer(&self, framebuffer: Framebuffer) {
        unsafe {
            (self.gl.delete_framebuffers)(1, &framebuffer.id);
        }
    }

    fn delete_texture(&self, texture: u32) {
        unsafe {
            (self.gl.delete_textures)(1, &texture);
        }
    }
}

fn dmabuf_import_attribs(
    width: u32,
    height: u32,
    fourcc: u32,
    fd: egl::Attrib,
    offset: u32,
    stride: u32,
    modifier: u64,
) -> Vec<egl::Attrib> {
    let mut attribs = vec![
        egl::WIDTH as egl::Attrib,
        width as egl::Attrib,
        egl::HEIGHT as egl::Attrib,
        height as egl::Attrib,
        EGL_LINUX_DRM_FOURCC_EXT,
        fourcc as egl::Attrib,
        EGL_DMA_BUF_PLANE0_FD_EXT,
        fd,
        EGL_DMA_BUF_PLANE0_OFFSET_EXT,
        offset as egl::Attrib,
        EGL_DMA_BUF_PLANE0_PITCH_EXT,
        stride as egl::Attrib,
    ];

    if modifier != DRM_FORMAT_MOD_INVALID {
        attribs.extend([
            EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
            (modifier & 0xffff_ffff) as egl::Attrib,
            EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT,
            (modifier >> 32) as egl::Attrib,
        ]);
    }

    attribs.push(egl::ATTRIB_NONE);
    attribs
}

impl Drop for LinuxGpuFrameCompositor {
    fn drop(&mut self) {
        let _ = self.egl.make_current(self.display, None, None, None);
        let _ = self.egl.destroy_context(self.display, self.context);
        let _ = self.egl.destroy_surface(self.display, self.surface);
        let _ = self.egl.terminate(self.display);
    }
}

impl GlFns {
    fn load(egl: &EglInstance) -> Result<Self, RecorderError> {
        Ok(Self {
            gen_textures: gl_proc(egl, "glGenTextures")?,
            delete_textures: gl_proc(egl, "glDeleteTextures")?,
            bind_texture: gl_proc(egl, "glBindTexture")?,
            tex_parameteri: gl_proc(egl, "glTexParameteri")?,
            egl_image_target_texture_2d_oes: gl_proc(egl, "glEGLImageTargetTexture2DOES")?,
            gen_framebuffers: gl_proc(egl, "glGenFramebuffers")?,
            delete_framebuffers: gl_proc(egl, "glDeleteFramebuffers")?,
            bind_framebuffer: gl_proc(egl, "glBindFramebuffer")?,
            framebuffer_texture_2d: gl_proc(egl, "glFramebufferTexture2D")?,
            check_framebuffer_status: gl_proc(egl, "glCheckFramebufferStatus")?,
            read_pixels: gl_proc(egl, "glReadPixels")?,
        })
    }
}

trait EglFd {
    fn fd_raw_for_egl(&self) -> Result<egl::Attrib, RecorderError>;
}

impl EglFd for CapturedDmaBufFrame {
    fn fd_raw_for_egl(&self) -> Result<egl::Attrib, RecorderError> {
        use std::os::fd::AsRawFd;

        self.fd.as_raw_fd().try_into().map_err(|_error| {
            capture_format_unsupported_error(format!(
                "Capture source {} reported an invalid DMA-BUF fd.",
                self.source_id
            ))
        })
    }
}

fn egl_display(egl: &EglInstance) -> Result<egl::Display, RecorderError> {
    let display = unsafe {
        egl.get_platform_display(
            EGL_PLATFORM_SURFACELESS_MESA,
            ptr::null_mut(),
            &[egl::ATTRIB_NONE],
        )
    }
    .or_else(|_error| {
        unsafe { egl.get_display(egl::DEFAULT_DISPLAY) }.ok_or(egl::Error::BadDisplay)
    })
    .map_err(|error| {
        capture_adapter_failed_error(format!("EGL display creation failed: {error}."))
    })?;

    Ok(display)
}

fn choose_config(egl: &EglInstance, display: egl::Display) -> Result<egl::Config, RecorderError> {
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
        .map_err(|error| {
            capture_adapter_failed_error(format!("EGL GPU compositor config failed: {error}."))
        })?
        .ok_or_else(|| capture_adapter_failed_error("EGL did not expose an OpenGL config."))
}

fn gl_proc<T: Copy>(egl: &EglInstance, name: &str) -> Result<T, RecorderError> {
    let ptr = egl.get_proc_address(name).ok_or_else(|| {
        capture_adapter_failed_error(format!("OpenGL entrypoint {name} is unavailable."))
    })?;

    Ok(unsafe { mem::transmute_copy::<extern "system" fn(), T>(&ptr) })
}

fn rgba_bottom_up_to_top_down(rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
    let row_len = width as usize * 4;
    let mut output = vec![0; row_len * height as usize];

    for y in 0..height as usize {
        let source_y = height as usize - 1 - y;
        let source_start = source_y * row_len;
        let output_start = y * row_len;
        output[output_start..output_start + row_len]
            .copy_from_slice(&rgba[source_start..source_start + row_len]);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::{
        dmabuf_import_attribs, rgba_bottom_up_to_top_down, DRM_FORMAT_MOD_INVALID,
        EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT,
    };
    use khronos_egl as egl;

    #[test]
    fn readback_conversion_flips_gl_bottom_up_rows_to_top_down_rgba() {
        let rgba = vec![
            0, 0, 255, 255, 255, 255, 255, 255, 255, 0, 0, 255, 0, 255, 0, 255,
        ];
        let top_down = rgba_bottom_up_to_top_down(&rgba, 2, 2);

        assert_eq!(
            top_down,
            vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,]
        );
    }

    #[test]
    fn dmabuf_import_attribs_omit_implicit_modifier() {
        let attribs =
            dmabuf_import_attribs(1920, 1080, 875_713_112, 10, 0, 7680, DRM_FORMAT_MOD_INVALID);

        assert!(!attribs.contains(&EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT));
        assert!(!attribs.contains(&EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT));
        assert_eq!(attribs.last().copied(), Some(egl::ATTRIB_NONE));
    }

    #[test]
    fn dmabuf_import_attribs_include_explicit_modifier() {
        let modifier = 0x1122_3344_5566_7788;
        let attribs = dmabuf_import_attribs(1920, 1080, 875_713_112, 10, 0, 7680, modifier);

        assert!(attribs
            .windows(2)
            .any(|pair| pair == [EGL_DMA_BUF_PLANE0_MODIFIER_LO_EXT, 0x5566_7788]));
        assert!(attribs
            .windows(2)
            .any(|pair| pair == [EGL_DMA_BUF_PLANE0_MODIFIER_HI_EXT, 0x1122_3344]));
        assert_eq!(attribs.last().copied(), Some(egl::ATTRIB_NONE));
    }
}
