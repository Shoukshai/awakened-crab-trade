//! Serial settings transaction worker. X11 reads remain event-driven on their
//! own thread, so registering shortcuts or persisting settings never blocks GTK.
use super::xwayland_shortcuts::Backend;
use crate::{hotkeys::Hotkeys, settings::Settings};
use std::sync::{atomic::AtomicBool, mpsc::Receiver, Arc};
pub struct Request {
    pub epoch: u64,
    pub settings: Settings,
}
pub enum Update {
    Backend {
        connection: u64,
        result: Result<(), String>,
    },
    Applied {
        epoch: u64,
        settings: Box<Settings>,
        connection: u64,
        available: bool,
    },
    Failed {
        epoch: u64,
        error: String,
    },
}
fn attach<A, R>(
    active: Arc<AtomicBool>,
    actions: A,
    reports: R,
    connection: u64,
) -> Result<Arc<Backend>, String>
where
    A: Fn(&'static str) + Clone + Send + 'static,
    R: Fn(Update) + Clone + Send + 'static,
{
    // Start the listener before any proposed keys are acquired. Even a slow
    // disk commit must not leave a synchronous keyboard grab without a reader.
    let backend = Arc::new(Backend::new(&Hotkeys::disabled())?);
    let reader = backend.clone();
    std::thread::spawn(move || {
        if let Err(error) = reader.listen(active, actions) {
            reports(Update::Backend {
                connection,
                result: Err(error),
            });
        }
    });
    Ok(backend)
}
pub fn run<A, R>(
    initial: Settings,
    active: Arc<AtomicBool>,
    requests: Receiver<Request>,
    actions: A,
    reports: R,
) where
    A: Fn(&'static str) + Clone + Send + 'static,
    R: Fn(Update) + Clone + Send + 'static,
{
    let mut connection = 1;
    let mut backend = match attach(active.clone(), actions.clone(), reports.clone(), connection) {
        Ok(backend) => Some(backend),
        Err(error) => {
            reports(Update::Backend {
                connection,
                result: Err(error),
            });
            None
        }
    };
    if let Some(backend) = backend.as_ref() {
        if let Err(error) = backend.replace_and_notify(
            &initial.hotkeys,
            || Ok(()),
            |_, _| {
                reports(Update::Backend {
                    connection,
                    result: Ok(()),
                })
            },
        ) {
            reports(Update::Backend {
                connection,
                result: Err(error),
            });
        }
    }
    while let Ok(request) = requests.recv() {
        if backend.as_ref().is_none_or(|b| !b.is_alive()) {
            connection += 1;
            match attach(active.clone(), actions.clone(), reports.clone(), connection) {
                Ok(new) => backend = Some(new),
                Err(error) => {
                    reports(Update::Failed {
                        epoch: request.epoch,
                        error,
                    });
                    continue;
                }
            }
        }
        let epoch = request.epoch;
        let settings = request.settings;
        let applied = Box::new(settings.clone());
        let published = reports.clone();
        let result = backend.as_ref().unwrap().replace_and_notify(
            &settings.hotkeys,
            || settings.save(),
            move |_, available| {
                published(Update::Applied {
                    epoch,
                    settings: applied,
                    connection,
                    available,
                });
            },
        );
        if let Err(error) = result {
            reports(Update::Failed { epoch, error });
        } else if !backend.as_ref().unwrap().is_alive() {
            reports(Update::Backend {
                connection,
                result: Err("Connexion aux raccourcis perdue après l’enregistrement.".into()),
            });
        }
    }
}
