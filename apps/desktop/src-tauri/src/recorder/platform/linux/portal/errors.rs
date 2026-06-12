use super::super::super::super::{
    errors::{dependency_error, permission_error},
    types::RecorderError,
};

pub(super) fn portal_error(error: ashpd::Error) -> RecorderError {
    if matches!(
        error,
        ashpd::Error::Response(ashpd::desktop::ResponseError::Cancelled)
            | ashpd::Error::Portal(ashpd::PortalError::Cancelled(_))
    ) {
        return permission_error(
            "recorder_portal_permission_denied",
            "ScreenCast source selection was denied or cancelled.",
        );
    }

    dependency_error(
        "recorder_portal_error",
        format!("ScreenCast portal request failed: {error}"),
    )
}
