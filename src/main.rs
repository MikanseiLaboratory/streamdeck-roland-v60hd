mod actions;
mod pool;
mod settings;
mod tally;

use std::collections::HashMap;
use std::env;

use actions::{build_job, Gesture, AUTO, COMPOSITION, CUT, SELECT_AUX, SELECT_PGM, SELECT_PST};
use futures::{SinkExt, StreamExt};
use pool::{ConnectionStatus, EndpointKey, Pool, Work};
use roland_rs::devices::v60hd::{PanelStatus, TallyColor};
use settings::{ActionSettings, EndpointInfo, PiMessage, PiOut};
use streamdeck_rs::registration::RegistrationParams;
use streamdeck_rs::{ImagePayload, Message, MessageOut, StreamDeckSocket, Target};
use tally::{image_data_uri, LightBinding, TallyLight};
use tokio::sync::{mpsc, oneshot};

type SdSocket = StreamDeckSocket<(), ActionSettings, PiMessage, PiOut>;

enum Outgoing {
    ShowOk {
        context: String,
    },
    ShowAlert {
        context: String,
    },
    ToPi {
        action: String,
        context: String,
        payload: PiOut,
    },
    SetImage {
        context: String,
        image: Option<String>,
    },
    GetSettings {
        context: String,
    },
    Log {
        message: String,
    },
}

struct KeyWatch {
    endpoint: Option<EndpointKey>,
    binding: LightBinding,
}

struct Plugin {
    pool: Pool,
    open_pi: HashMap<String, String>,
    watches: HashMap<String, KeyWatch>,
    tally_states: HashMap<EndpointKey, [TallyColor; 8]>,
    panel_states: HashMap<EndpointKey, PanelStatus>,
    last_light: HashMap<String, Option<TallyLight>>,
    outgoing: mpsc::UnboundedSender<Outgoing>,
}

#[tokio::main]
async fn main() {
    let params = RegistrationParams::from_args(env::args()).expect("Stream Deck registration args");
    let mut socket: SdSocket = StreamDeckSocket::connect(params.port, params.event, params.uuid)
        .await
        .expect("connect to Stream Deck");

    let (outgoing_tx, mut outgoing_rx) = mpsc::unbounded_channel();
    let (status_tx, mut status_rx) = mpsc::unbounded_channel();
    let (idle_tx, mut idle_rx) = mpsc::unbounded_channel();
    let (tally_tx, mut tally_rx) = mpsc::unbounded_channel();
    let (panel_tx, mut panel_rx) = mpsc::unbounded_channel();

    let mut plugin = Plugin {
        pool: Pool::new(status_tx, idle_tx, tally_tx, panel_tx),
        open_pi: HashMap::new(),
        watches: HashMap::new(),
        tally_states: HashMap::new(),
        panel_states: HashMap::new(),
        last_light: HashMap::new(),
        outgoing: outgoing_tx,
    };

    loop {
        tokio::select! {
            msg = socket.next() => {
                match msg {
                    Some(Ok(message)) => plugin.handle(message),
                    Some(Err(e)) => eprintln!("streamdeck read error: {e:?}"),
                    None => break,
                }
            }
            out = outgoing_rx.recv() => {
                let Some(out) = out else { break };
                if let Err(e) = send_out(&mut socket, out).await {
                    eprintln!("streamdeck write error: {e:?}");
                }
            }
            status = status_rx.recv() => {
                let Some((key, status)) = status else { break };
                plugin.on_status(key, status);
            }
            idle = idle_rx.recv() => {
                let Some((key, generation)) = idle else { break };
                plugin.pool.apply_idle(key, generation);
            }
            tally = tally_rx.recv() => {
                let Some((key, updates)) = tally else { break };
                plugin.on_tally(key, updates);
            }
            panel = panel_rx.recv() => {
                let Some((key, panel)) = panel else { break };
                plugin.on_panel(key, panel);
            }
        }
    }
}

async fn send_out(socket: &mut SdSocket, out: Outgoing) -> Result<(), String> {
    let result = match out {
        Outgoing::ShowOk { context } => socket.send(MessageOut::ShowOk { context }).await,
        Outgoing::ShowAlert { context } => socket.send(MessageOut::ShowAlert { context }).await,
        Outgoing::ToPi {
            action,
            context,
            payload,
        } => {
            socket
                .send(MessageOut::SendToPropertyInspector {
                    action,
                    context,
                    payload,
                })
                .await
        }
        Outgoing::SetImage { context, image } => {
            socket
                .send(MessageOut::SetImage {
                    context,
                    payload: ImagePayload {
                        image,
                        target: Target::Both,
                        state: None,
                    },
                })
                .await
        }
        Outgoing::GetSettings { context } => socket.send(MessageOut::GetSettings { context }).await,
        Outgoing::Log { message } => {
            socket
                .send(MessageOut::LogMessage {
                    payload: streamdeck_rs::LogMessagePayload { message },
                })
                .await
        }
    };
    result.map_err(|e| format!("{e:?}"))
}

impl Plugin {
    fn handle(&mut self, message: Message<(), ActionSettings, PiMessage>) {
        match message {
            Message::WillAppear {
                action,
                context,
                payload,
                ..
            } => {
                self.watch_key(action, context, payload.settings);
            }
            Message::DidReceiveSettings {
                action,
                context,
                payload,
                ..
            } => {
                self.watch_key(action, context.clone(), payload.settings);
                self.push_status(&context);
            }
            Message::WillDisappear { context, .. } => {
                self.unwatch_key(&context);
                self.pool.unpin(&context);
            }
            Message::KeyDown {
                action,
                context,
                payload,
                ..
            } => self.run_action(action, context, payload.settings, Gesture::Down),
            Message::KeyUp {
                action,
                context,
                payload,
                ..
            } => self.run_action(action, context, payload.settings, Gesture::Up),
            Message::PropertyInspectorDidAppear {
                action, context, ..
            } => {
                self.open_pi.insert(context.clone(), action);
                if self.pool.status_for_context(&context).is_none() {
                    let _ = self.outgoing.send(Outgoing::GetSettings {
                        context: context.clone(),
                    });
                }
                self.push_status(&context);
            }
            Message::PropertyInspectorDidDisappear { context, .. } => {
                self.open_pi.remove(&context);
            }
            Message::SendToPlugin {
                action,
                context,
                payload,
            } => self.on_pi_message(action, context, payload),
            _ => {}
        }
    }

    fn on_pi_message(&mut self, action: String, context: String, payload: PiMessage) {
        if payload.property_inspector.as_deref() == Some("propertyInspectorConnected") {
            self.open_pi.insert(context.clone(), action);
            self.push_status(&context);
            return;
        }
        if payload.command.as_deref() == Some("test_connection") {
            self.start_ver_test(action, context, payload);
        }
    }

    fn start_ver_test(&self, action: String, context: String, payload: PiMessage) {
        let host = payload.host.unwrap_or_default();
        let host = host.trim().to_string();
        if host.is_empty() {
            self.push_status_value(&context, "Enter a host".to_string());
            return;
        }
        self.push_status_value(&context, "Testing…".to_string());
        let outgoing = self.outgoing.clone();
        tokio::spawn(async move {
            let status = match ver_probe(&host).await {
                Ok((product, version)) => format!("Connected ({product} {version})"),
                Err(e) => format!("Failed ({e})"),
            };
            let _ = outgoing.send(Outgoing::ToPi {
                action,
                context,
                payload: PiOut::state(status, Vec::new()),
            });
        });
    }

    fn on_tally(&mut self, key: EndpointKey, updates: Vec<(u8, TallyColor)>) {
        let slot = self
            .tally_states
            .entry(key.clone())
            .or_insert([TallyColor::Dark; 8]);
        for (index, state) in updates {
            if let Some(entry) = slot.get_mut(index as usize) {
                *entry = state;
            }
        }
        self.refresh_endpoint_images(&key);
    }

    fn on_panel(&mut self, key: EndpointKey, panel: PanelStatus) {
        self.panel_states.insert(key.clone(), panel);
        self.refresh_endpoint_images(&key);
    }

    fn watch_key(&mut self, action: String, context: String, settings: ActionSettings) {
        let endpoint = EndpointKey::from_settings(&settings);
        self.pool.pin(&context, endpoint.clone());
        self.watches.insert(
            context.clone(),
            KeyWatch {
                endpoint,
                binding: LightBinding::from_action(&action, &settings),
            },
        );
        self.refresh_tally_image(&context);
    }

    fn unwatch_key(&mut self, context: &str) {
        self.watches.remove(context);
        self.last_light.remove(context);
        let _ = self.outgoing.send(Outgoing::SetImage {
            context: context.to_string(),
            image: None,
        });
    }

    fn refresh_endpoint_images(&mut self, key: &EndpointKey) {
        for context in self.pool.contexts_for(key) {
            self.refresh_tally_image(&context);
        }
    }

    fn refresh_tally_image(&mut self, context: &str) {
        let Some(watch) = self.watches.get(context) else {
            return;
        };
        let tally = watch
            .endpoint
            .as_ref()
            .and_then(|endpoint| self.tally_states.get(endpoint));
        let panel = watch
            .endpoint
            .as_ref()
            .and_then(|endpoint| self.panel_states.get(endpoint));
        let light = watch.binding.light(tally, panel);
        if self.last_light.get(context) == Some(&light) {
            return;
        }
        self.last_light.insert(context.to_string(), light);
        let image = light.map(image_data_uri);
        let _ = self.outgoing.send(Outgoing::SetImage {
            context: context.to_string(),
            image,
        });
    }

    fn on_status(&mut self, key: EndpointKey, status: ConnectionStatus) {
        self.pool.set_status(key.clone(), status.clone());
        if matches!(status, ConnectionStatus::Retrying { .. }) {
            self.tally_states.remove(&key);
            self.panel_states.remove(&key);
            self.refresh_endpoint_images(&key);
        }
        for context in self.pool.contexts_for(&key) {
            self.push_status_value(&context, status.label());
        }
    }

    fn push_status(&self, context: &str) {
        let label = self
            .pool
            .status_for_context(context)
            .map(ConnectionStatus::label)
            .unwrap_or_else(|| "Not connected".to_string());
        self.push_status_value(context, label);
    }

    fn push_status_value(&self, context: &str, label: String) {
        let Some(action) = self.open_pi.get(context) else {
            return;
        };
        let endpoints = self
            .pool
            .endpoint_list()
            .into_iter()
            .map(|(key, status)| EndpointInfo {
                host: key.host,
                status,
            })
            .collect();
        let _ = self.outgoing.send(Outgoing::ToPi {
            action: action.clone(),
            context: context.to_string(),
            payload: PiOut::state(label, endpoints),
        });
    }

    fn run_action(
        &mut self,
        action: String,
        context: String,
        settings: ActionSettings,
        gesture: Gesture,
    ) {
        let job = match build_job(&action, &settings, gesture) {
            Ok(Some(job)) => job,
            Ok(None) => return,
            Err(e) => {
                let _ = self.outgoing.send(Outgoing::Log { message: e });
                if gesture == Gesture::Down {
                    let _ = self.outgoing.send(Outgoing::ShowAlert { context });
                }
                return;
            }
        };
        let Some(key) = EndpointKey::from_settings(&settings) else {
            if gesture == Gesture::Down {
                let _ = self.outgoing.send(Outgoing::ShowAlert { context });
            }
            return;
        };
        let tx = self.pool.sender(&key);
        let outgoing = self.outgoing.clone();
        let show_feedback = gesture == Gesture::Down;
        let skip_ok = matches!(
            action.as_str(),
            SELECT_PGM | SELECT_PST | SELECT_AUX | COMPOSITION | CUT | AUTO
        );
        tokio::spawn(async move {
            let (reply_tx, reply_rx) = oneshot::channel();
            if tx
                .send(Work::Exec {
                    job,
                    reply: reply_tx,
                })
                .is_err()
            {
                if show_feedback {
                    let _ = outgoing.send(Outgoing::ShowAlert { context });
                }
                return;
            }
            match reply_rx.await {
                Ok(Ok(())) => {
                    if show_feedback && !skip_ok {
                        let _ = outgoing.send(Outgoing::ShowOk { context });
                    }
                }
                Ok(Err(e)) => {
                    let _ = outgoing.send(Outgoing::Log { message: e });
                    if show_feedback {
                        let _ = outgoing.send(Outgoing::ShowAlert { context });
                    }
                }
                Err(_) => {
                    if show_feedback {
                        let _ = outgoing.send(Outgoing::ShowAlert { context });
                    }
                }
            }
        });
    }
}

async fn ver_probe(host: &str) -> Result<(String, String), String> {
    let mut client = roland_rs::AsyncV60HdClient::connect(host)
        .await
        .map_err(|e| e.to_string())?;
    client.ver().await.map_err(|e| e.to_string())
}
