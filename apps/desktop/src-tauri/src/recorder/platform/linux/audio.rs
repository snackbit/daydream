use std::{cell::RefCell, rc::Rc};

use pulse::{
    callbacks::ListResult,
    context::{Context, FlagSet as ContextFlagSet, State as ContextState},
    mainloop::threaded::Mainloop,
    operation,
    proplist::{properties::APPLICATION_NAME, Proplist},
};

use super::super::super::{
    errors::audio_error,
    types::{AudioSource, AudioSourceKind, AudioSourceList, AudioSourceRole, RecorderError},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AudioProbe {
    pub(super) desktop_default_source_id: Option<String>,
    pub(super) error: Option<String>,
    pub(super) microphone_default_source_id: Option<String>,
    pub(super) server_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PulseServer {
    default_sink_name: Option<String>,
    default_source_name: Option<String>,
    server_name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PulseSource {
    description: Option<String>,
    monitor_of_sink_name: Option<String>,
    name: String,
}

pub(super) fn probe_audio_server() -> AudioProbe {
    match query_pulse_sources() {
        Ok((server, sources)) => {
            let audio_sources = normalize_sources(&server, sources);
            AudioProbe {
                desktop_default_source_id: audio_sources
                    .iter()
                    .find(|source| {
                        source.is_default && source.roles.contains(&AudioSourceRole::DesktopAudio)
                    })
                    .map(|source| source.id.clone()),
                error: None,
                microphone_default_source_id: audio_sources
                    .iter()
                    .find(|source| {
                        source.is_default && source.roles.contains(&AudioSourceRole::Microphone)
                    })
                    .map(|source| source.id.clone()),
                server_name: server.server_name,
            }
        }
        Err(error) => AudioProbe {
            desktop_default_source_id: None,
            error: Some(error),
            microphone_default_source_id: None,
            server_name: None,
        },
    }
}

pub(super) fn get_audio_sources() -> Result<AudioSourceList, RecorderError> {
    let (server, sources) = query_pulse_sources().map_err(audio_error)?;

    Ok(AudioSourceList {
        sources: normalize_sources(&server, sources),
    })
}

fn query_pulse_sources() -> Result<(PulseServer, Vec<PulseSource>), String> {
    let mut mainloop =
        Mainloop::new().ok_or_else(|| "Failed to create PulseAudio main loop.".to_string())?;
    let mut proplist =
        Proplist::new().ok_or_else(|| "Failed to create PulseAudio property list.".to_string())?;
    proplist
        .set_str(APPLICATION_NAME, "Daydream")
        .map_err(|error| format!("Failed to set PulseAudio application name: {error:?}"))?;
    let context = Rc::new(RefCell::new(
        Context::new_with_proplist(&mainloop, "Daydream", &proplist)
            .ok_or_else(|| "Failed to create PulseAudio context.".to_string())?,
    ));

    {
        let mainloop_ptr = &mut mainloop as *mut Mainloop;
        context
            .borrow_mut()
            .set_state_callback(Some(Box::new(move || unsafe {
                (*mainloop_ptr).signal(false);
            })));
    }

    context
        .borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|error| format!("Failed to connect to PulseAudio: {error}"))?;

    mainloop
        .start()
        .map_err(|error| format!("Failed to start PulseAudio main loop: {error}"))?;
    mainloop.lock();

    let ready = loop {
        match context.borrow().get_state() {
            ContextState::Ready => break true,
            ContextState::Failed | ContextState::Terminated => break false,
            _ => mainloop.wait(),
        }
    };
    context.borrow_mut().set_state_callback(None);

    if !ready {
        mainloop.unlock();
        mainloop.stop();
        return Err("PulseAudio context failed before becoming ready.".to_string());
    }

    let server = Rc::new(RefCell::new(None));
    let server_done = Rc::new(RefCell::new(false));
    {
        let server_ref = Rc::clone(&server);
        let server_done_ref = Rc::clone(&server_done);
        let mainloop_ptr = &mut mainloop as *mut Mainloop;
        let operation = context.borrow().introspect().get_server_info(move |info| {
            *server_ref.borrow_mut() = Some(PulseServer {
                default_sink_name: info
                    .default_sink_name
                    .as_ref()
                    .map(|value| value.to_string()),
                default_source_name: info
                    .default_source_name
                    .as_ref()
                    .map(|value| value.to_string()),
                server_name: info.server_name.as_ref().map(|value| value.to_string()),
            });
            *server_done_ref.borrow_mut() = true;
            unsafe {
                (*mainloop_ptr).signal(false);
            }
        });
        while operation.get_state() != operation::State::Done || !*server_done.borrow() {
            mainloop.wait();
        }
    }

    let sources = Rc::new(RefCell::new(Vec::new()));
    let source_list_done = Rc::new(RefCell::new(false));
    {
        let sources_ref = Rc::clone(&sources);
        let source_list_done_ref = Rc::clone(&source_list_done);
        let mainloop_ptr = &mut mainloop as *mut Mainloop;
        let operation = context
            .borrow()
            .introspect()
            .get_source_info_list(move |result| match result {
                ListResult::Item(info) => {
                    if let Some(name) = &info.name {
                        sources_ref.borrow_mut().push(PulseSource {
                            description: info.description.as_ref().map(|value| value.to_string()),
                            monitor_of_sink_name: info
                                .monitor_of_sink_name
                                .as_ref()
                                .map(|value| value.to_string()),
                            name: name.to_string(),
                        });
                    }
                }
                ListResult::End => {
                    *source_list_done_ref.borrow_mut() = true;
                    unsafe {
                        (*mainloop_ptr).signal(false);
                    }
                }
                ListResult::Error => {
                    *source_list_done_ref.borrow_mut() = true;
                    unsafe {
                        (*mainloop_ptr).signal(false);
                    }
                }
            });
        while operation.get_state() != operation::State::Done || !*source_list_done.borrow() {
            mainloop.wait();
        }
    }

    mainloop.unlock();
    context.borrow_mut().disconnect();
    mainloop.stop();

    let server = server
        .borrow()
        .clone()
        .ok_or_else(|| "PulseAudio did not return server info.".to_string())?;
    let sources = sources.borrow().clone();

    Ok((server, sources))
}

fn normalize_sources(server: &PulseServer, sources: Vec<PulseSource>) -> Vec<AudioSource> {
    sources
        .into_iter()
        .map(|source| normalize_source(server, source))
        .collect()
}

fn normalize_source(server: &PulseServer, source: PulseSource) -> AudioSource {
    let is_monitor = source.monitor_of_sink_name.is_some() || source.name.ends_with(".monitor");
    let kind = if is_monitor {
        AudioSourceKind::Monitor
    } else {
        AudioSourceKind::Input
    };
    let roles = if is_monitor {
        vec![AudioSourceRole::DesktopAudio]
    } else {
        vec![AudioSourceRole::Microphone]
    };
    let is_default = if is_monitor {
        source.monitor_of_sink_name.as_deref() == server.default_sink_name.as_deref()
            || source.name
                == server
                    .default_sink_name
                    .as_ref()
                    .map(|sink| format!("{sink}.monitor"))
                    .unwrap_or_default()
    } else {
        Some(source.name.as_str()) == server.default_source_name.as_deref()
    };

    AudioSource {
        display_name: source.description.unwrap_or_else(|| source.name.clone()),
        id: source.name,
        is_default,
        kind,
        roles,
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_sources, PulseServer, PulseSource};
    use crate::recorder::types::{AudioSourceKind, AudioSourceRole};

    #[test]
    fn maps_default_sink_monitor_to_desktop_audio() {
        let sources = normalize_sources(
            &PulseServer {
                default_sink_name: Some("evo4_stereo".to_string()),
                default_source_name: Some("evo4_mic_stereo".to_string()),
                server_name: Some("PulseAudio (on PipeWire 1.6.6)".to_string()),
            },
            vec![
                PulseSource {
                    description: Some("EVO4 monitor".to_string()),
                    monitor_of_sink_name: Some("evo4_stereo".to_string()),
                    name: "evo4_stereo.monitor".to_string(),
                },
                PulseSource {
                    description: Some("EVO4 mic".to_string()),
                    monitor_of_sink_name: None,
                    name: "evo4_mic_stereo".to_string(),
                },
            ],
        );

        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].kind, AudioSourceKind::Monitor);
        assert_eq!(sources[0].roles, vec![AudioSourceRole::DesktopAudio]);
        assert!(sources[0].is_default);
        assert_eq!(sources[1].kind, AudioSourceKind::Input);
        assert_eq!(sources[1].roles, vec![AudioSourceRole::Microphone]);
        assert!(sources[1].is_default);
    }
}
