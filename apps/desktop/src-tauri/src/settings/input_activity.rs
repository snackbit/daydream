use std::{env, fs, io, path::Path, process::Command};

use super::types::{InputActivityRepairResult, InputActivityStatus, InputActivityStatusReason};

const INPUT_GROUP: &str = "input";
const LOGIN_GUIDANCE: &str = "Log out and back in, then reopen Daydream.";

pub(crate) fn input_activity_status() -> InputActivityStatus {
    platform_input_activity_status()
}

pub(super) fn repair_input_activity_permissions() -> InputActivityRepairResult {
    repair_input_activity_permissions_with_runner(&SystemRepairCommandRunner)
}

#[cfg(target_os = "linux")]
fn platform_input_activity_status() -> InputActivityStatus {
    let mut status = linux_probe_input_activity_status(Path::new("/dev/input"));
    enrich_linux_repair_status(&mut status);
    status
}

#[cfg(not(target_os = "linux"))]
fn platform_input_activity_status() -> InputActivityStatus {
    InputActivityStatus {
        available: false,
        candidate_device_count: 0,
        current_user: current_username(),
        message: "Input activity capture is only implemented on Linux.".to_string(),
        permission_denied_device_count: 0,
        platform: env::consts::OS.to_string(),
        readable_device_count: 0,
        reason: InputActivityStatusReason::UnsupportedPlatform,
        repair_available: false,
        user_in_input_group: false,
    }
}

#[cfg(target_os = "linux")]
fn linux_probe_input_activity_status(input_dir: &Path) -> InputActivityStatus {
    let current_user = current_username();
    let Ok(entries) = fs::read_dir(input_dir) else {
        return InputActivityStatus {
            available: false,
            candidate_device_count: 0,
            current_user,
            message: "Linux input device directory /dev/input is unavailable.".to_string(),
            permission_denied_device_count: 0,
            platform: "linux".to_string(),
            readable_device_count: 0,
            reason: InputActivityStatusReason::DevInputMissing,
            repair_available: false,
            user_in_input_group: false,
        };
    };

    let mut candidate_device_count = 0;
    let mut permission_denied_device_count = 0;
    let mut readable_device_count = 0;
    let mut supported_device_count = 0;

    for entry in entries.flatten() {
        let file_name = entry.file_name();
        if !file_name.to_string_lossy().starts_with("event") {
            continue;
        }
        candidate_device_count += 1;
        match evdev::Device::open(entry.path()) {
            Ok(device) => {
                readable_device_count += 1;
                if evdev_device_supports_activity(&device) {
                    supported_device_count += 1;
                }
            }
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                permission_denied_device_count += 1;
            }
            Err(_error) => {}
        }
    }

    let (available, reason, message) = if supported_device_count > 0 {
        (
            true,
            InputActivityStatusReason::Available,
            "Keyboard and mouse activity capture can read Linux input devices.".to_string(),
        )
    } else if permission_denied_device_count > 0 {
        (
            false,
            InputActivityStatusReason::PermissionDenied,
            "Daydream cannot read Linux input devices. Repair permissions, then log out and back in."
                .to_string(),
        )
    } else if candidate_device_count == 0 {
        (
            false,
            InputActivityStatusReason::DevInputMissing,
            "Linux input event devices were not found under /dev/input.".to_string(),
        )
    } else {
        (
            false,
            InputActivityStatusReason::NoSupportedDevices,
            "No readable keyboard or mouse input devices were found.".to_string(),
        )
    };

    InputActivityStatus {
        available,
        candidate_device_count,
        current_user,
        message,
        permission_denied_device_count,
        platform: "linux".to_string(),
        readable_device_count,
        reason,
        repair_available: false,
        user_in_input_group: false,
    }
}

#[cfg(target_os = "linux")]
fn evdev_device_supports_activity(device: &evdev::Device) -> bool {
    let keyboard_like = device.supported_keys().is_some_and(|keys| {
        keys.contains(evdev::KeyCode::KEY_A) || keys.contains(evdev::KeyCode::KEY_ENTER)
    });
    let mouse_like = device.supported_relative_axes().is_some()
        || device.supported_absolute_axes().is_some()
        || device.supported_keys().is_some_and(|keys| {
            keys.contains(evdev::KeyCode::BTN_LEFT) || keys.contains(evdev::KeyCode::BTN_RIGHT)
        });

    keyboard_like || mouse_like
}

#[cfg(target_os = "linux")]
fn enrich_linux_repair_status(status: &mut InputActivityStatus) {
    let input_group = linux_group(INPUT_GROUP);
    let Some(input_group) = input_group else {
        status.repair_available = false;
        return;
    };
    status.user_in_input_group = linux_process_has_group(input_group.gid)
        || status
            .current_user
            .as_ref()
            .is_some_and(|user| input_group.members.iter().any(|member| member == user));
    status.repair_available = !status.available
        && status.reason == InputActivityStatusReason::PermissionDenied
        && !status.user_in_input_group
        && status.current_user.is_some()
        && find_executable("pkexec").is_some()
        && find_usermod().is_some();

    if !status.available && status.user_in_input_group {
        status.message =
            "The current user is in the input group, but this Daydream process cannot read input devices yet. Log out and back in, then reopen Daydream."
                .to_string();
    }
}

trait RepairCommandRunner {
    fn run(&self, program: &Path, args: &[String]) -> io::Result<RepairCommandOutput>;
}

struct RepairCommandOutput {
    code: Option<i32>,
    stderr: String,
    stdout: String,
}

struct SystemRepairCommandRunner;

impl RepairCommandRunner for SystemRepairCommandRunner {
    fn run(&self, program: &Path, args: &[String]) -> io::Result<RepairCommandOutput> {
        let output = Command::new(program).args(args).output()?;
        Ok(RepairCommandOutput {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            stdout: String::from_utf8_lossy(&output.stdout).trim().to_string(),
        })
    }
}

fn repair_input_activity_permissions_with_runner(
    runner: &dyn RepairCommandRunner,
) -> InputActivityRepairResult {
    #[cfg(not(target_os = "linux"))]
    {
        let _ = runner;
        return InputActivityRepairResult {
            message: "Input activity permission repair is only available on Linux.".to_string(),
            status: input_activity_status(),
            success: false,
        };
    }

    #[cfg(target_os = "linux")]
    {
        let mut status = input_activity_status();
        if status.available {
            return InputActivityRepairResult {
                message: "Input activity capture is already available.".to_string(),
                status,
                success: true,
            };
        }
        let Some(user) = status.current_user.clone() else {
            return InputActivityRepairResult {
                message: "Could not determine the current user for permission repair.".to_string(),
                status,
                success: false,
            };
        };
        if !valid_linux_username(&user) {
            return InputActivityRepairResult {
                message: "Current username is not valid for permission repair.".to_string(),
                status,
                success: false,
            };
        }
        let Some(pkexec) = find_executable("pkexec") else {
            return InputActivityRepairResult {
                message: "pkexec is not available on this system.".to_string(),
                status,
                success: false,
            };
        };
        let Some(usermod) = find_usermod() else {
            return InputActivityRepairResult {
                message: "usermod is not available on this system.".to_string(),
                status,
                success: false,
            };
        };
        if linux_group(INPUT_GROUP).is_none() {
            return InputActivityRepairResult {
                message: "Linux input group does not exist on this system.".to_string(),
                status,
                success: false,
            };
        }

        let args = vec![
            usermod.to_string_lossy().to_string(),
            "-aG".to_string(),
            INPUT_GROUP.to_string(),
            user,
        ];
        match runner.run(&pkexec, &args) {
            Ok(output) if output.code == Some(0) => {
                status = input_activity_status();
                InputActivityRepairResult {
                    message: format!("Permission repair completed. {LOGIN_GUIDANCE}"),
                    status,
                    success: true,
                }
            }
            Ok(output) => InputActivityRepairResult {
                message: command_failure_message(output),
                status,
                success: false,
            },
            Err(error) => InputActivityRepairResult {
                message: format!("Permission repair failed: {error}"),
                status,
                success: false,
            },
        }
    }
}

fn command_failure_message(output: RepairCommandOutput) -> String {
    let detail = if !output.stderr.is_empty() {
        output.stderr
    } else if !output.stdout.is_empty() {
        output.stdout
    } else {
        "No error output was returned.".to_string()
    };
    format!("Permission repair failed: {detail}")
}

fn current_username() -> Option<String> {
    env::var("USER")
        .ok()
        .or_else(|| env::var("LOGNAME").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| valid_linux_username(value))
}

fn valid_linux_username(username: &str) -> bool {
    !username.is_empty()
        && username.len() <= 32
        && username
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
struct LinuxGroup {
    gid: u32,
    members: Vec<String>,
}

#[cfg(target_os = "linux")]
fn linux_group(name: &str) -> Option<LinuxGroup> {
    let groups = fs::read_to_string("/etc/group").ok()?;
    groups.lines().find_map(|line| {
        let mut parts = line.split(':');
        let group_name = parts.next()?;
        let _password = parts.next()?;
        let gid = parts.next()?.parse::<u32>().ok()?;
        let members = parts
            .next()
            .unwrap_or_default()
            .split(',')
            .filter(|member| !member.is_empty())
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if group_name == name {
            Some(LinuxGroup { gid, members })
        } else {
            None
        }
    })
}

#[cfg(target_os = "linux")]
fn linux_process_has_group(gid: u32) -> bool {
    let Ok(status) = fs::read_to_string("/proc/self/status") else {
        return false;
    };
    status.lines().any(|line| {
        line.strip_prefix("Groups:").is_some_and(|groups| {
            groups
                .split_whitespace()
                .filter_map(|value| value.parse::<u32>().ok())
                .any(|candidate| candidate == gid)
        })
    })
}

fn find_usermod() -> Option<std::path::PathBuf> {
    [
        "/usr/sbin/usermod",
        "/sbin/usermod",
        "/usr/bin/usermod",
        "/bin/usermod",
    ]
    .into_iter()
    .map(std::path::PathBuf::from)
    .find(|path| path.is_file())
    .or_else(|| find_executable("usermod"))
}

fn find_executable(name: &str) -> Option<std::path::PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|directory| directory.join(name))
        .find(|path| path.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CapturingRunner {
        code: Option<i32>,
    }

    impl RepairCommandRunner for CapturingRunner {
        fn run(&self, program: &Path, args: &[String]) -> io::Result<RepairCommandOutput> {
            assert_eq!(
                program.file_name().and_then(|name| name.to_str()),
                Some("pkexec")
            );
            assert!(args.iter().any(|arg| arg.ends_with("usermod")));
            assert!(args.contains(&"-aG".to_string()));
            assert!(args.contains(&INPUT_GROUP.to_string()));
            Ok(RepairCommandOutput {
                code: self.code,
                stderr: String::new(),
                stdout: String::new(),
            })
        }
    }

    #[test]
    fn linux_username_validation_rejects_command_payloads() {
        assert!(valid_linux_username("felixf"));
        assert!(valid_linux_username("daydream.user-1"));
        assert!(!valid_linux_username(""));
        assert!(!valid_linux_username("bad user"));
        assert!(!valid_linux_username("bad;user"));
        assert!(!valid_linux_username("../bad"));
    }

    #[test]
    fn command_failure_prefers_stderr() {
        let message = command_failure_message(RepairCommandOutput {
            code: Some(1),
            stderr: "denied".to_string(),
            stdout: "ignored".to_string(),
        });

        assert_eq!(message, "Permission repair failed: denied");
    }

    #[test]
    fn repair_runner_contract_is_fixed() {
        let runner = CapturingRunner { code: Some(0) };
        let output = runner
            .run(
                Path::new("/usr/bin/pkexec"),
                &[
                    "/usr/sbin/usermod".to_string(),
                    "-aG".to_string(),
                    "input".to_string(),
                    "felixf".to_string(),
                ],
            )
            .expect("runner");

        assert_eq!(output.code, Some(0));
    }
}
